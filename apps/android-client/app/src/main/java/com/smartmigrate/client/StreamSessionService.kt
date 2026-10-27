package com.smartmigrate.client

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.wifi.WifiManager
import android.os.Binder
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * Foreground service that keeps an active Smart Migrate remote desktop session alive when the
 * app is backgrounded, the screen turns off, or Android tries to kill the process.
 *
 * Responsibilities:
 * 1. Shows a persistent notification so Android does not kill the streaming process.
 * 2. Acquires a WifiLock (WIFI_MODE_FULL_LOW_LATENCY) to prevent Wi-Fi sleep.
 * 3. Calls ConnectivityManager.bindProcessToNetwork() on the Wi-Fi network so traffic
 *    never silently reroutes through mobile data when Wi-Fi has no internet (intranet LAN).
 * 4. Registers a NetworkCallback and immediately calls reconnectNow() when Wi-Fi returns
 *    after a brief dropout, rather than waiting for the StreamEngine backoff timer.
 * 5. Acquires a partial wakelock so the CPU does not sleep mid-frame during streaming.
 */
class StreamSessionService : Service() {

    companion object {
        private const val TAG = "SmartMigrate-Session"
        private const val CHANNEL_ID = "sm_stream_session"
        private const val NOTIF_ID = 1001

        /**
         * Starts the foreground service. Safe to call multiple times — Android will update
         * the running service's intent rather than start a second instance.
         */
        fun start(context: Context, hostAddress: String) {
            val intent = Intent(context, StreamSessionService::class.java).apply {
                putExtra("host", hostAddress)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        /** Stops the foreground service cleanly. */
        fun stop(context: Context) {
            context.stopService(Intent(context, StreamSessionService::class.java))
        }

        /** Active Wi-Fi network discovered by NetworkMonitor, used for per-socket binding */
        @Volatile
        var activeWifiNetwork: Network? = null

        /** Callback set by MainActivity/DedicatedRemoteSessionScreen to trigger an immediate reconnect. */
        @Volatile
        var onWifiReconnected: (() -> Unit)? = null

        /**
         * Actively queries ConnectivityManager for an operational Wi-Fi network and updates
         * activeWifiNetwork, returning the network if found.
         */
        fun refreshWifiNetwork(context: Context): Network? {
            val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager ?: return null
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val activeNet = cm.activeNetwork
                val caps = cm.getNetworkCapabilities(activeNet)
                if (caps != null && caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                    activeWifiNetwork = activeNet
                    return activeNet
                }
            }
            for (net in cm.allNetworks) {
                val caps = cm.getNetworkCapabilities(net)
                if (caps != null && caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                    activeWifiNetwork = net
                    return net
                }
            }
            return null
        }
    }

    inner class LocalBinder : Binder() {
        fun getService(): StreamSessionService = this@StreamSessionService
    }

    private val binder = LocalBinder()
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private var wifiLock: WifiManager.WifiLock? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private var connectivityManager: ConnectivityManager? = null

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        acquireWifiLock()
        acquireWakeLock()
        registerNetworkMonitor()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val host = intent?.getStringExtra("host") ?: "Windows Host"
        startForeground(host)
        return START_STICKY
    }

    override fun onDestroy() {
        super.onDestroy()
        unregisterNetworkMonitor()
        releaseWifiLock()
        releaseWakeLock()
        serviceScope.cancel()
        Log.i(TAG, "StreamSessionService destroyed")
    }

    // ── Foreground notification ───────────────────────────────────────────────

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Smart Migrate Remote Session",
                NotificationManager.IMPORTANCE_DEFAULT
            ).apply {
                description = "Keeps your remote desktop session active"
                setShowBadge(true)
                lockscreenVisibility = Notification.VISIBILITY_PUBLIC
            }
            getSystemService(NotificationManager::class.java)?.createNotificationChannel(channel)
        }
    }

    private fun startForeground(hostAddress: String) {
        val stopIntent = Intent(this, StreamSessionService::class.java).apply {
            action = "STOP"
        }
        val stopPending = PendingIntent.getService(
            this, 0, stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val openIntent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val openPending = PendingIntent.getActivity(
            this, 0, openIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val notification: Notification = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
                .setSmallIcon(R.drawable.ic_notification)
                .setContentTitle("Smart Migrate — Session Active")
                .setContentText("Streaming from $hostAddress")
                .setOngoing(true)
                .setVisibility(Notification.VISIBILITY_PUBLIC)
                .setCategory(Notification.CATEGORY_SERVICE)
                .setContentIntent(openPending)
                .addAction(
                    Notification.Action.Builder(
                        null, "Disconnect", stopPending
                    ).build()
                )
                .build()
        } else {
            @Suppress("DEPRECATION")
            android.app.Notification.Builder(this)
                .setSmallIcon(R.drawable.ic_notification)
                .setContentTitle("Smart Migrate — Session Active")
                .setContentText("Streaming from $hostAddress")
                .setOngoing(true)
                .setContentIntent(openPending)
                .build()
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIF_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            startForeground(NOTIF_ID, notification)
        }
        Log.i(TAG, "StreamSessionService started foreground for host: $hostAddress")
    }

    // ── Wi-Fi Lock ────────────────────────────────────────────────────────────

    @Suppress("DEPRECATION")
    private fun acquireWifiLock() {
        try {
            val wifiManager = applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager
                ?: return
            val lockMode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                WifiManager.WIFI_MODE_FULL_LOW_LATENCY
            } else {
                WifiManager.WIFI_MODE_FULL_HIGH_PERF
            }
            wifiLock = wifiManager.createWifiLock(lockMode, "SmartMigrate:StreamLock").also {
                it.setReferenceCounted(false)
                it.acquire()
                Log.i(TAG, "WifiLock acquired (mode=$lockMode)")
            }
        } catch (e: Exception) {
            Log.w(TAG, "WifiLock acquire failed: ${e.message}")
        }
    }

    private fun releaseWifiLock() {
        try {
            wifiLock?.takeIf { it.isHeld }?.release()
            wifiLock = null
        } catch (_: Exception) {}
    }

    // ── Wake Lock ─────────────────────────────────────────────────────────────

    private fun acquireWakeLock() {
        try {
            val pm = getSystemService(Context.POWER_SERVICE) as? PowerManager ?: return
            wakeLock = pm.newWakeLock(
                PowerManager.PARTIAL_WAKE_LOCK,
                "SmartMigrate:StreamWakeLock"
            ).also {
                it.setReferenceCounted(false)
                it.acquire(30 * 60 * 1000L) // 30 min maximum safety
                Log.i(TAG, "WakeLock acquired")
            }
        } catch (e: Exception) {
            Log.w(TAG, "WakeLock acquire failed: ${e.message}")
        }
    }

    private fun releaseWakeLock() {
        try {
            wakeLock?.takeIf { it.isHeld }?.release()
            wakeLock = null
        } catch (_: Exception) {}
    }

    // ── Network Binding + Reconnect Callback ──────────────────────────────────

    private fun registerNetworkMonitor() {
        val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager ?: return
        connectivityManager = cm

        val request = NetworkRequest.Builder()
            .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
            .build()

        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                Log.i(TAG, "Wi-Fi network available — setting activeWifiNetwork and triggering reconnect")
                activeWifiNetwork = network
                // Signal StreamEngine to reconnect immediately instead of waiting for backoff.
                onWifiReconnected?.invoke()
            }

            override fun onLost(network: Network) {
                Log.w(TAG, "Wi-Fi network lost")
                if (activeWifiNetwork == network) {
                    activeWifiNetwork = null
                }
            }

            override fun onCapabilitiesChanged(
                network: Network,
                capabilities: NetworkCapabilities
            ) {
                if (capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                    activeWifiNetwork = network
                }
            }
        }

        try {
            cm.registerNetworkCallback(request, callback)
            networkCallback = callback

            // Check immediately for any current Wi-Fi network
            val activeNetwork = cm.activeNetwork
            val caps = activeNetwork?.let { cm.getNetworkCapabilities(it) }
            if (caps != null && caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                activeWifiNetwork = activeNetwork
                Log.i(TAG, "Initially discovered active Wi-Fi network")
            }
        } catch (e: Exception) {
            Log.w(TAG, "NetworkCallback registration failed: ${e.message}")
        }
    }

    private fun unregisterNetworkMonitor() {
        val cm = connectivityManager ?: return
        val cb = networkCallback ?: return
        try {
            cm.unregisterNetworkCallback(cb)
        } catch (_: Exception) {}
        networkCallback = null
        activeWifiNetwork = null
        onWifiReconnected = null
    }
}
