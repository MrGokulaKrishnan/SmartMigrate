package com.smartmigrate.client

import android.content.Context

/**
 * Persists the paired host address, session token, and device ID to SharedPreferences so that
 * the app can reconnect silently after a process restart, backgrounding, or Wi-Fi loss without
 * requiring the user to re-enter the PIN.
 *
 * These values are stored in the app's private EncryptedSharedPreferences-compatible store.
 * They never leave the device — no secret reaches logs or network.
 */
object SessionPreferences {
    private const val PREFS_NAME = "sm_session"
    private const val KEY_HOST = "host_address"
    private const val KEY_TOKEN = "session_token"
    private const val KEY_DEVICE_ID = "device_id"
    private const val KEY_HOST_DEVICE_ID = "host_device_id"

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    /** Save pairing result after a successful PIN exchange. */
    fun savePairing(
        context: Context,
        hostAddress: String,
        sessionToken: String,
        deviceId: String,
        hostDeviceId: String = ""
    ) {
        prefs(context).edit()
            .putString(KEY_HOST, hostAddress)
            .putString(KEY_TOKEN, sessionToken)
            .putString(KEY_DEVICE_ID, deviceId)
            .putString(KEY_HOST_DEVICE_ID, hostDeviceId)
            .apply()
    }

    /** Load the last saved host address (empty string if none). */
    fun loadHost(context: Context): String =
        prefs(context).getString(KEY_HOST, "") ?: ""

    /** Load the last saved session token (empty string if none). */
    fun loadToken(context: Context): String =
        prefs(context).getString(KEY_TOKEN, "") ?: ""

    /** Load the last saved Android device ID. */
    fun loadDeviceId(context: Context): String =
        prefs(context).getString(KEY_DEVICE_ID, SmpClient.DEFAULT_DEVICE_ID) ?: SmpClient.DEFAULT_DEVICE_ID

    /** Load the last saved Windows host device ID. */
    fun loadHostDeviceId(context: Context): String =
        prefs(context).getString(KEY_HOST_DEVICE_ID, "") ?: ""

    /** Returns true if there is a saved pairing (host + token are both non-empty). */
    fun hasSavedPairing(context: Context): Boolean {
        val host = loadHost(context)
        val token = loadToken(context)
        return host.isNotBlank() && token.isNotBlank()
    }

    /** Clears saved pairing data (e.g., after deliberate disconnect or revocation). */
    fun clear(context: Context) {
        prefs(context).edit().clear().apply()
    }
}
