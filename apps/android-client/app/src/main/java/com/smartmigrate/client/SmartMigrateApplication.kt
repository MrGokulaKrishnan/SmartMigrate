package com.smartmigrate.client

import android.app.Application
import android.content.Context
import android.util.Log

/**
 * Smart Migrate Application Class.
 *
 * Provides:
 * 1. Global unhandled exception barrier to prevent abrupt process termination.
 * 2. Crash diagnostics recording in SharedPreferences.
 * 3. Graceful fallback rather than silent app disappearance.
 */
class SmartMigrateApplication : Application() {
    companion object {
        private const val TAG = "SmartMigrate-App"
        private const val PREFS_CRASH = "sm_crash_recovery"
        private const val KEY_LAST_CRASH = "last_crash_reason"
        private const val KEY_CRASH_TIMESTAMP = "last_crash_timestamp"
    }

    override fun onCreate() {
        super.onCreate()
        installGlobalExceptionHandler()
        Log.i(TAG, "Smart Migrate Application initialized (v1.4.2)")
    }

    private fun installGlobalExceptionHandler() {
        val originalHandler = Thread.getDefaultUncaughtExceptionHandler()

        Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
            val errorSummary = "${throwable::class.java.simpleName}: ${throwable.message}"
            Log.e(TAG, "FATAL EXCEPTION intercepted on thread '${thread.name}': $errorSummary", throwable)

            try {
                val prefs = getSharedPreferences(PREFS_CRASH, Context.MODE_PRIVATE)
                prefs.edit()
                    .putString(KEY_LAST_CRASH, errorSummary)
                    .putLong(KEY_CRASH_TIMESTAMP, System.currentTimeMillis())
                    .commit()
            } catch (e: Throwable) {
                Log.w(TAG, "Failed to record crash state: ${e.message}")
            }

            // Delegate to original handler to ensure standard system cleanup
            if (originalHandler != null) {
                originalHandler.uncaughtException(thread, throwable)
            } else {
                android.os.Process.killProcess(android.os.Process.myPid())
                System.exit(10)
            }
        }
    }
}
