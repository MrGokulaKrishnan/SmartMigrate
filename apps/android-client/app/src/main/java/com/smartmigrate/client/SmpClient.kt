package com.smartmigrate.client

import android.os.SystemClock
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
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

    /**
     * Handshake with Windows host to verify SMP/1 protocol and capabilities.
     */
    suspend fun getHello(host: String, port: Int = 7890): Result<DeviceHello> = withContext(Dispatchers.IO) {
        try {
            val url = URL("http://$host:$port/smp/hello")
            val conn = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = 3000
                readTimeout = 3000
                setRequestProperty("Accept", "application/json")
                setRequestProperty("User-Agent", "SmartMigrate-Android/0.1.0")
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
                    appVersion = json.optString("appVersion", "0.1.0"),
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
        try {
            val url = URL("http://$host:$port/smp/pair")
            val conn = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "POST"
                connectTimeout = 4000
                readTimeout = 4000
                doOutput = true
                setRequestProperty("Content-Type", "application/json; charset=utf-8")
                setRequestProperty("Accept", "application/json")
                setRequestProperty("User-Agent", "SmartMigrate-Android/0.1.0")
            }

            val body = JSONObject().apply {
                put("requesterDeviceId", requesterDeviceId)
                put("requesterName", requesterName)
                put("code", pin.trim())
                put("requestedPermissions", JSONArray(listOf("ViewScreen", "ControlMouse", "SendFiles", "ReceiveFiles", "Clipboard")))
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
        try {
            val url = URL("http://$host:$port/smp/session")
            val conn = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "POST"
                connectTimeout = 4000
                readTimeout = 4000
                doOutput = true
                setRequestProperty("Content-Type", "application/json; charset=utf-8")
                setRequestProperty("Accept", "application/json")
                setRequestProperty("Authorization", "Bearer $sessionToken")
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
                    streamUrl = json.optString("streamUrl", "/live?token=$sessionToken"),
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
        val t0 = SystemClock.elapsedRealtime()
        try {
            val url = URL("http://$host:$port/smp/ping")
            val conn = (url.openConnection() as HttpURLConnection).apply {
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
     * Injects remote mouse action (left_click, right_click, move, scroll) to host PC.
     */
    suspend fun sendInput(
        host: String,
        port: Int = 7890,
        action: String,
        x: Int,
        y: Int,
        scrollDelta: Int = 0,
        sequence: Long = 1L,
        deviceId: String = "sm-android-client"
    ): Result<Boolean> = withContext(Dispatchers.IO) {
        try {
            val url = URL("http://$host:$port/smp/input")
            val conn = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "POST"
                connectTimeout = 2000
                readTimeout = 2000
                doOutput = true
                setRequestProperty("Content-Type", "application/json; charset=utf-8")
                setRequestProperty("Accept", "application/json")
            }

            val body = JSONObject().apply {
                put("deviceId", deviceId)
                put("action", action)
                put("x", x)
                put("y", y)
                put("scrollDelta", scrollDelta)
                put("sequence", sequence)
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
     * Queries the Windows host for live diagnostic status across all subsystems.
     */
    suspend fun getDiagnostics(host: String, port: Int = 7890): Result<DiagnosticsReport> = withContext(Dispatchers.IO) {
        try {
            val url = URL("http://$host:$port/smp/diagnostics")
            val conn = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = 3000
                readTimeout = 3000
                setRequestProperty("Accept", "application/json")
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
