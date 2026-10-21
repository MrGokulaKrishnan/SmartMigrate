package com.smartmigrate.client

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.SystemClock
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import org.json.JSONArray
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

/**
 * Smart Migrate Protocol (SMP/1) Client.
 *
 * Implements real network requests to the Windows host control plane:
 * - GET /smp/hello (Device capabilities & protocol verification)
 * - POST /smp/pair (Pairing PIN verification & persistent trust token receipt)
 * - POST /smp/session (Session negotiation & display stream URL generation)
 * - GET /smp/ping (Heartbeat & latency RTT measurement)
 * - POST /smp/input (Remote mouse & keyboard injection)
 * - GET /smp/diagnostics (8-point live connectivity diagnostics)
 */
object SmpClient {

    val DEFAULT_DEVICE_ID: String = "sm-android-" + android.os.Build.MODEL.replace(" ", "-").lowercase()

    data class DeviceHello(
        val protocolVersion: Int,
        val protocolName: String,
        val deviceId: String,
        val deviceName: String,
        val platform: String,
        val appVersion: String,
        val capabilities: List<String>,
        val timestampMs: Long
    )

    data class PairResult(
        val status: String,
        val sessionToken: String,
        val hostDeviceId: String,
        val grantedPermissions: List<String>
    )

    data class SessionResult(
        val sessionId: String,
        val status: String,
        val transport: String,
        val streamUrl: String,
        val protocolVersion: Int,
        val heartbeatIntervalMs: Long
    )

    data class DiagnosticsReport(
        val discovery: String,
        val deviceId: String,
        val protocolVersion: String,
        val pairingStatus: String,
        val trustStoreCount: Int,
        val screenCapture: String,
        val streamingPort: Int,
        val overall: String
    )

    data class DiscoveredHost(
        val ip: String,
        val port: Int = 7890,
        val name: String = "Windows Host",
        val id: String = "sm-win-host"
    )

    /**
     * Resolves the active Wi-Fi Network handle, even if mobile data is primary or Wi-Fi lacks internet.
     */
    fun getWifiNetwork(context: Context? = null): Network? {
        val ctx = context ?: try { SmartMigrateApplication.instance } catch (_: Exception) { null } ?: return null
        val cm = ctx.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager ?: return null

        val active = cm.activeNetwork
        if (active != null) {
            val caps = cm.getNetworkCapabilities(active)
            if (caps != null && caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                return active
            }
        }

        for (net in cm.allNetworks) {
            val caps = cm.getNetworkCapabilities(net)
            if (caps != null && caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                return net
            }
        }
        return null
    }

    /**
     * Requests and binds the Wi-Fi network handle with a short timeout.
     */
    suspend fun requestWifiNetwork(context: Context? = null, timeoutMs: Long = 1000): Network? {
        val existing = getWifiNetwork(context)
        if (existing != null) return existing

        val ctx = context ?: try { SmartMigrateApplication.instance } catch (_: Exception) { null } ?: return null
        val cm = ctx.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager ?: return null

        val req = NetworkRequest.Builder()
            .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
            .build()
        val deferred = CompletableDeferred<Network?>()
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                deferred.complete(network)
            }
            override fun onUnavailable() {
                deferred.complete(null)
            }
        }

        return try {
            cm.requestNetwork(req, callback, timeoutMs.toInt())
            withTimeoutOrNull(timeoutMs) { deferred.await() }
        } catch (_: Exception) {
            null
        } finally {
            try { cm.unregisterNetworkCallback(callback) } catch (_: Exception) {}
        }
    }

    /**
     * Opens an HttpURLConnection explicitly routed through the Wi-Fi network interface.
     */
    fun openHttpConnection(url: URL, wifiNet: Network? = null, context: Context? = null): HttpURLConnection {
        val net = wifiNet ?: getWifiNetwork(context)
        if (net != null) {
            try {
                return net.openConnection(url) as HttpURLConnection
            } catch (_: Exception) {}
        }
        return url.openConnection() as HttpURLConnection
    }

    /**
     * Determines local Wi-Fi IPv4 subnet prefix (e.g., "192.168.0.") and self host octet from LinkProperties.
     */
    fun getSubnetInfo(context: Context? = null, wifiNet: Network? = null): Pair<String, Int> {
        val ctx = context ?: try { SmartMigrateApplication.instance } catch (_: Exception) { null }
        val cm = ctx?.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
        val net = wifiNet ?: getWifiNetwork(ctx)

        if (cm != null && net != null) {
            val lp = cm.getLinkProperties(net)
            if (lp != null) {
                for (la in lp.linkAddresses) {
                    val addr = la.address
                    if (addr is java.net.Inet4Address && !addr.isLoopbackAddress) {
                        val host = addr.hostAddress ?: ""
                        if (!host.startsWith("127.") && !host.startsWith("169.254.")) {
                            val lastDot = host.lastIndexOf('.')
                            if (lastDot > 0) {
                                val prefix = host.substring(0, lastDot + 1)
                                val octet = host.substring(lastDot + 1).toIntOrNull() ?: 0
                                return Pair(prefix, octet)
                            }
                        }
                    }
                }
            }
        }

        // Fallback: search non-cellular interfaces
        try {
            val interfaces = java.net.NetworkInterface.getNetworkInterfaces()
            while (interfaces.hasMoreElements()) {
                val iface = interfaces.nextElement()
                if (iface.isLoopback || !iface.isUp) continue
                val name = iface.name.lowercase()
                if (name.contains("rmnet") || name.contains("pdp") || name.contains("ccmni") || name.contains("dummy")) {
                    continue
                }
                val addrs = iface.inetAddresses
                while (addrs.hasMoreElements()) {
                    val addr = addrs.nextElement()
                    if (addr is java.net.Inet4Address && !addr.isLoopbackAddress) {
                        val host = addr.hostAddress ?: ""
                        if (!host.startsWith("127.") && !host.startsWith("169.254.")) {
                            val lastDot = host.lastIndexOf('.')
                            if (lastDot > 0) {
                                val prefix = host.substring(0, lastDot + 1)
                                val octet = host.substring(lastDot + 1).toIntOrNull() ?: 0
                                return Pair(prefix, octet)
                            }
                        }
                    }
                }
            }
        } catch (_: Exception) {}

        return Pair("", 0)
    }

    /**
     * Determines local Wi-Fi IPv4 subnet prefix (e.g., "192.168.0.")
     */
    fun getLocalSubnetPrefix(context: Context? = null): String {
        return getSubnetInfo(context).first
    }

    /**
     * Returns the phone's current local Wi-Fi IPv4 address.
     */
    fun getLocalIp(context: Context? = null): String {
        val (prefix, octet) = getSubnetInfo(context)
        return if (prefix.isNotEmpty() && octet > 0) "$prefix$octet" else ""
    }

    /**
     * Concurrently probes the entire local subnet for active Smart Migrate Windows hosts.
     */
    suspend fun probeSubnet(
        subnetPrefix: String,
        localHostNum: Int,
        wifiNet: Network?
    ): DiscoveredHost? = coroutineScope {
        if (subnetPrefix.isEmpty()) return@coroutineScope null
        val result = CompletableDeferred<DiscoveredHost?>()

        // Prioritize common host addresses, then probe all 1..254 except self
        val priorityHosts = listOf(1, 2, 3, 4, 5, 10, 20, 50, 100, 101, 102, 150, 200)
        val allHosts = (1..254).toList()
        val candidates = (priorityHosts + allHosts).distinct().filter { it != localHostNum }

        val channel = Channel<String>(candidates.size)
        candidates.forEach { channel.trySend("$subnetPrefix$it") }
        channel.close()

        val concurrency = 64
        val workers = (1..concurrency).map {
            launch(Dispatchers.IO) {
                for (ip in channel) {
                    if (result.isCompleted) break
                    try {
                        val url = URL("http://$ip:7890/smp/hello")
                        val conn = openHttpConnection(url, wifiNet).apply {
                            connectTimeout = 300
                            readTimeout = 400
                            requestMethod = "GET"
                            setRequestProperty("Accept", "application/json")
                            setRequestProperty("User-Agent", "SmartMigrate-Discovery/1.4.4")
                        }
                        if (conn.responseCode == 200) {
                            val text = BufferedReader(InputStreamReader(conn.inputStream)).use { it.readText() }
                            if (text.contains("SMP") || text.contains("protocolVersion") || text.contains("deviceId")) {
                                val json = JSONObject(text)
                                val host = DiscoveredHost(
                                    ip = ip,
                                    port = 7890,
                                    name = json.optString("deviceName", "Windows Host"),
                                    id = json.optString("deviceId", "sm-win-host")
                                )
                                result.complete(host)
                                break
                            }
                        }
                    } catch (_: Exception) {}
                }
            }
        }

        val found = try {
            withTimeoutOrNull(2500) { result.await() }
        } finally {
            workers.forEach { it.cancel() }
        }
        found
    }

    /**
     * Discovers active Smart Migrate Windows Host on the local network via UDP beacon probe on port 7889
     * and parallel subnet candidate scanning, strictly routing over Wi-Fi.
     */
    suspend fun discoverHosts(context: Context? = null, timeoutMs: Long = 2000): DiscoveredHost? = withContext(Dispatchers.IO) {
        val wifiNet = requestWifiNetwork(context, 800) ?: getWifiNetwork(context)
        val (subnetPrefix, localHostNum) = getSubnetInfo(context, wifiNet)

        // 1. Send UDP probe on port 7889 bound to Wi-Fi
        try {
            val socket = java.net.DatagramSocket()
            wifiNet?.bindSocket(socket)
            socket.broadcast = true
            socket.soTimeout = 600

            val probeMsg = "SM_DISCOVERY_PROBE\n".toByteArray(Charsets.UTF_8)
            val broadcastAddr = java.net.InetAddress.getByName("255.255.255.255")
            socket.send(java.net.DatagramPacket(probeMsg, probeMsg.size, broadcastAddr, 7889))

            if (subnetPrefix.isNotEmpty()) {
                try {
                    val directedAddr = java.net.InetAddress.getByName("${subnetPrefix}255")
                    socket.send(java.net.DatagramPacket(probeMsg, probeMsg.size, directedAddr, 7889))
                } catch (_: Exception) {}
            }

            val buf = ByteArray(1024)
            val recvPacket = java.net.DatagramPacket(buf, buf.size)
            socket.receive(recvPacket)

            val resp = String(recvPacket.data, 0, recvPacket.length, Charsets.UTF_8)
            socket.close()

            if (resp.contains("SM_DISCOVERY_RESPONSE")) {
                val fromIp = recvPacket.address.hostAddress ?: ""
                var port = 7890
                var name = "Windows Host"
                var id = "sm-win-host"
                var ip = fromIp

                resp.split(":").forEach { part ->
                    val kv = part.split("=")
                    if (kv.size == 2) {
                        when (kv[0].trim()) {
                            "port" -> port = kv[1].trim().toIntOrNull() ?: 7890
                            "name" -> name = kv[1].trim()
                            "id" -> id = kv[1].trim()
                            "ip" -> if (kv[1].trim().isNotEmpty()) ip = kv[1].trim()
                        }
                    }
                }
                return@withContext DiscoveredHost(ip, port, name, id)
            }
        } catch (_: Exception) {
            // UDP broadcast blocked, dropped by firewall, or timed out
        }

        // 2. Parallel Subnet Probing across all 1..254 addresses
        if (subnetPrefix.isNotEmpty()) {
            val found = probeSubnet(subnetPrefix, localHostNum, wifiNet)
            if (found != null) {
                return@withContext found
            }
        }

        null
    }

    /**
     * Handshake with Windows host to verify SMP/1 protocol and capabilities.
     */
    suspend fun getHello(host: String, port: Int = 7890): Result<DeviceHello> = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) {
            return@withContext Result.failure(IllegalArgumentException("Host IP address is required."))
        }
        try {
            val url = URL("http://$cleanHost:$port/smp/hello")
            val conn = openHttpConnection(url).apply {
                requestMethod = "GET"
                connectTimeout = 3000
                readTimeout = 3000
                setRequestProperty("Accept", "application/json")
                setRequestProperty("User-Agent", "SmartMigrate-Android/1.4.4")
            }

            val code = conn.responseCode
            if (code != 200) {
                return@withContext Result.failure(Exception("Host returned HTTP $code: ${conn.responseMessage}"))
            }

            val responseText = BufferedReader(InputStreamReader(conn.inputStream)).use { it.readText() }
            val json = JSONObject(responseText)

            val capsJson = json.optJSONArray("capabilities") ?: JSONArray()
            val caps = mutableListOf<String>()
            for (i in 0 until capsJson.length()) {
                caps.add(capsJson.getString(i))
            }

            Result.success(
                DeviceHello(
                    protocolVersion = json.optInt("protocolVersion", 1),
                    protocolName = json.optString("protocolName", "SMP/1"),
                    deviceId = json.optString("deviceId", "unknown"),
                    deviceName = json.optString("deviceName", "Windows Host"),
                    platform = json.optString("platform", "Windows"),
                    appVersion = json.optString("appVersion", "1.4.4"),
                    capabilities = caps,
                    timestampMs = json.optLong("timestampMs", System.currentTimeMillis())
                )
            )
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Submits the 6-digit numeric PIN to pair with the Windows Host and receive a session token.
     */
    suspend fun pair(
        host: String,
        port: Int = 7890,
        pin: String,
        requesterDeviceId: String = "sm-android-" + android.os.Build.MODEL.replace(" ", "-").lowercase(),
        requesterName: String = "Android (" + android.os.Build.MODEL + ")"
    ): Result<PairResult> = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) {
            return@withContext Result.failure(IllegalArgumentException("Host IP address is required. Please auto-detect or enter the PC's IP."))
        }
        val cleanPin = pin.trim()
        if (cleanPin.length != 6 || !cleanPin.all { it.isDigit() }) {
            return@withContext Result.failure(IllegalArgumentException("Please enter a valid 6-digit PIN."))
        }
        try {
            val url = URL("http://$cleanHost:$port/smp/pair")
            val conn = openHttpConnection(url).apply {
                requestMethod = "POST"
                connectTimeout = 4000
                readTimeout = 4000
                doOutput = true
                setRequestProperty("Content-Type", "application/json; charset=utf-8")
                setRequestProperty("Accept", "application/json")
                setRequestProperty("User-Agent", "SmartMigrate-Android/1.4.4")
            }

            val body = JSONObject().apply {
                put("requesterDeviceId", requesterDeviceId)
                put("requesterName", requesterName)
                put("code", cleanPin)
                put("requestedPermissions", JSONArray(listOf("ViewScreen", "ControlMouse", "ControlKeyboard", "Clipboard")))
            }

            OutputStreamWriter(conn.outputStream).use { it.write(body.toString()) }

            val code = conn.responseCode
            val stream = if (code in 200..299) conn.inputStream else conn.errorStream
            val responseText = if (stream != null) BufferedReader(InputStreamReader(stream)).use { it.readText() } else ""

            if (code != 200) {
                val errMsg = try {
                    JSONObject(responseText).optString("message", "Pairing rejected (HTTP $code)")
                } catch (_: Exception) {
                    "Pairing rejected (HTTP $code)"
                }
                return@withContext Result.failure(Exception(errMsg))
            }

            val json = JSONObject(responseText)
            val permsJson = json.optJSONArray("grantedPermissions") ?: JSONArray()
            val perms = mutableListOf<String>()
            for (i in 0 until permsJson.length()) {
                perms.add(permsJson.getString(i))
            }

            Result.success(
                PairResult(
                    status = json.optString("status", "APPROVED"),
                    sessionToken = json.getString("sessionToken"),
                    hostDeviceId = json.optString("hostDeviceId", ""),
                    grantedPermissions = perms
                )
            )
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Establishes an active display session with the Windows Host using the pairing session token.
     */
    suspend fun createSession(
        host: String,
        port: Int = 7890,
        clientDeviceId: String,
        sessionToken: String
    ): Result<SessionResult> = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) {
            return@withContext Result.failure(IllegalArgumentException("Host IP address is required."))
        }
        try {
            val url = URL("http://$cleanHost:$port/smp/session")
            val conn = openHttpConnection(url).apply {
                requestMethod = "POST"
                connectTimeout = 4000
                readTimeout = 4000
                doOutput = true
                setRequestProperty("Content-Type", "application/json; charset=utf-8")
                setRequestProperty("Accept", "application/json")
                setRequestProperty("Authorization", "Bearer $sessionToken")
                setRequestProperty("User-Agent", "SmartMigrate-Android/1.4.4")
            }

            val body = JSONObject().apply {
                put("clientDeviceId", clientDeviceId)
                put("sessionToken", sessionToken)
            }

            OutputStreamWriter(conn.outputStream).use { it.write(body.toString()) }

            val code = conn.responseCode
            val stream = if (code in 200..299) conn.inputStream else conn.errorStream
            val responseText = if (stream != null) BufferedReader(InputStreamReader(stream)).use { it.readText() } else ""

            if (code != 200) {
                val errMsg = try {
                    JSONObject(responseText).optString("message", "Session creation failed (HTTP $code)")
                } catch (_: Exception) {
                    "Session creation failed (HTTP $code)"
                }
                return@withContext Result.failure(Exception(errMsg))
            }

            val json = JSONObject(responseText)
            Result.success(
                SessionResult(
                    sessionId = json.optString("sessionId", ""),
                    status = json.optString("status", "SESSION_ESTABLISHED"),
                    transport = json.optString("transport", "LAN_DIRECT"),
                    streamUrl = json.optString("streamUrl", "/live"),
                    protocolVersion = json.optInt("protocolVersion", 1),
                    heartbeatIntervalMs = json.optLong("heartbeatIntervalMs", 2000L)
                )
            )
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Measures round-trip heartbeat latency (RTT) to the Windows host in milliseconds.
     */
    suspend fun ping(host: String, port: Int = 7890): Result<Long> = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) {
            return@withContext Result.failure(IllegalArgumentException("Host IP address is required."))
        }
        val t0 = SystemClock.elapsedRealtime()
        try {
            val url = URL("http://$cleanHost:$port/smp/ping")
            val conn = openHttpConnection(url).apply {
                requestMethod = "GET"
                connectTimeout = 2500
                readTimeout = 2500
            }
            val code = conn.responseCode
            val latency = SystemClock.elapsedRealtime() - t0
            if (code == 200) {
                Result.success(latency)
            } else {
                Result.failure(Exception("Ping returned HTTP $code"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Injects remote input action (touch, mouse, keyboard, hotkey, text) to Windows PC.
     */
    suspend fun sendInput(
        host: String,
        port: Int = 7890,
        action: String,
        x: Int? = null,
        y: Int? = null,
        scrollDelta: Int = 0,
        sequence: Long = 0L,
        deviceId: String = DEFAULT_DEVICE_ID,
        button: String? = null,
        dx: Int? = null,
        dy: Int? = null,
        normalizedX: Float? = null,
        normalizedY: Float? = null,
        sessionToken: String? = null,
        vkCode: Int? = null,
        keyUp: Boolean? = null,
        text: String? = null,
        hotkey: String? = null
    ): Result<Boolean> = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) {
            return@withContext Result.failure(IllegalArgumentException("Host IP address is required."))
        }
        try {
            val url = URL("http://$cleanHost:$port/smp/input")
            val conn = openHttpConnection(url).apply {
                requestMethod = "POST"
                connectTimeout = 1500
                readTimeout = 1500
                doOutput = true
                setRequestProperty("Content-Type", "application/json; charset=utf-8")
                setRequestProperty("Accept", "application/json")
                if (!sessionToken.isNullOrBlank()) {
                    setRequestProperty("Authorization", "Bearer $sessionToken")
                }
            }

            val body = JSONObject().apply {
                put("deviceId", deviceId)
                put("action", action)
                if (button != null) put("button", button)
                if (x != null) put("x", x)
                if (y != null) put("y", y)
                if (dx != null) put("dx", dx)
                if (dy != null) put("dy", dy)
                if (normalizedX != null) put("normalizedX", normalizedX.toDouble())
                if (normalizedY != null) put("normalizedY", normalizedY.toDouble())
                if (scrollDelta != 0) put("scrollDelta", scrollDelta)
                if (sequence > 0) put("sequence", sequence)
                if (vkCode != null) put("vkCode", vkCode)
                if (keyUp != null) put("keyUp", keyUp)
                if (text != null) put("text", text)
                if (hotkey != null) put("hotkey", hotkey)
            }

            OutputStreamWriter(conn.outputStream).use { it.write(body.toString()) }
            val code = conn.responseCode
            if (code == 200) {
                Result.success(true)
            } else {
                Result.failure(Exception("Input injection returned HTTP $code"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Informs the Windows host to cleanly terminate the active display streaming / control session.
     */
    suspend fun disconnectSession(host: String, port: Int = 7890): Result<Boolean> = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) {
            return@withContext Result.failure(IllegalArgumentException("Host IP address is required."))
        }
        try {
            val url = URL("http://$cleanHost:$port/smp/disconnect")
            val conn = openHttpConnection(url).apply {
                requestMethod = "POST"
                connectTimeout = 1500
                readTimeout = 1500
                doOutput = true
            }
            conn.outputStream.close()
            Result.success(conn.responseCode == 200)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Re-establishes the display session using a previously saved pairing token, without
     * requiring the user to re-enter the PIN. Called by StreamEngine after a network drop.
     *
     * If the host has reset its state (e.g., restarted), this returns failure and the caller
     * should direct the user to re-pair. Otherwise it returns a fresh session URL with the
     * same token so /live reconnection proceeds silently.
     */
    suspend fun revalidateSession(
        host: String,
        port: Int = 7890,
        clientDeviceId: String,
        sessionToken: String
    ): Result<SessionResult> {
        if (sessionToken.isBlank()) {
            return Result.failure(Exception("No saved session token — re-pairing required"))
        }
        return createSession(host, port, clientDeviceId, sessionToken)
    }

    /**
     * Fast TCP liveness check: connects to port 7890 with a 1.5s timeout and checks for
     * an HTTP response (any code). Used before retrying a full session after a network drop.
     */
    suspend fun pingHost(host: String, port: Int = 7890): Boolean = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) return@withContext false
        try {
            val url = URL("http://$cleanHost:$port/smp/ping")
            val conn = openHttpConnection(url).apply {
                requestMethod = "GET"
                connectTimeout = 1500
                readTimeout = 1500
            }
            conn.responseCode in 200..499 // any real HTTP response = host is reachable
        } catch (_: Exception) {
            false
        }
    }

    /**
     * Queries the Windows host for live diagnostic status across all subsystems.
     */
    suspend fun getDiagnostics(host: String, port: Int = 7890): Result<DiagnosticsReport> = withContext(Dispatchers.IO) {
        val cleanHost = host.trim()
        if (cleanHost.isBlank()) {
            return@withContext Result.failure(IllegalArgumentException("Host IP address is required."))
        }
        try {
            val url = URL("http://$cleanHost:$port/smp/diagnostics")
            val conn = openHttpConnection(url).apply {
                requestMethod = "GET"
                connectTimeout = 3000
                readTimeout = 3000
                setRequestProperty("Accept", "application/json")
                setRequestProperty("User-Agent", "SmartMigrate-Android/1.4.4")
            }

            val code = conn.responseCode
            if (code != 200) {
                return@withContext Result.failure(Exception("Diagnostics returned HTTP $code"))
            }

            val responseText = BufferedReader(InputStreamReader(conn.inputStream)).use { it.readText() }
            val json = JSONObject(responseText)

            Result.success(
                DiagnosticsReport(
                    discovery = json.optString("discovery", "PASS"),
                    deviceId = json.optString("deviceId", "Windows Host"),
                    protocolVersion = json.optString("protocolVersion", "SMP/1"),
                    pairingStatus = json.optString("pairingStatus", "IDLE"),
                    trustStoreCount = json.optInt("trustStoreCount", 0),
                    screenCapture = json.optString("screenCapture", "READY"),
                    streamingPort = json.optInt("streamingPort", 7890),
                    overall = json.optString("overall", "PASS")
                )
            )
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

