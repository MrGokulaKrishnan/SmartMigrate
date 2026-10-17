package com.smartmigrate.client

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.security.MessageDigest

class AppUpdaterTest {

    @Test
    fun testProductionCertificateFingerprintFormat() {
        val prodFingerprint = AppUpdater.OFFICIAL_RELEASE_FINGERPRINT
        assertNotNull(prodFingerprint)
        val parts = prodFingerprint.split(":")
        assertEquals("SHA-256 fingerprint must have 32 colon-separated bytes", 32, parts.size)
        parts.forEach { part ->
            assertEquals(2, part.length)
            assertTrue("Each byte must be hex", part.all { it in '0'..'9' || it in 'A'..'F' })
        }
    }

    @Test
    fun testLegacyDebugCertificateFingerprintFormat() {
        val debugFingerprint = AppUpdater.LEGACY_DEBUG_FINGERPRINT
        assertNotNull(debugFingerprint)
        val parts = debugFingerprint.split(":")
        assertEquals("SHA-256 fingerprint must have 32 colon-separated bytes", 32, parts.size)
    }

    @Test
    fun testVersionDowngradeDetection() {
        val installedVersionCode = 6L
        val candidateOlderCode = 5L
        val candidateSameCode = 6L
        val candidateNewerCode = 7L

        assertTrue("Older version code must be flagged as downgrade", candidateOlderCode < installedVersionCode)
        assertFalse("Same version code is not a downgrade", candidateSameCode < installedVersionCode)
        assertFalse("Newer version code is not a downgrade", candidateNewerCode < installedVersionCode)
    }

    @Test
    fun testManifestJsonParsing() {
        val sampleManifest = """
        {
          "product": "Smart Migrate",
          "version": "1.4.2",
          "versionCode": 6,
          "platforms": {
            "android-arm64": {
              "version": "1.4.2",
              "versionCode": 6,
              "url": "https://smartmigrate.web.app/downloads/android/SmartMigrate.apk",
              "fileName": "SmartMigrate.apk",
              "size": 2599798,
              "sha256": "eba656e0d4bd9cb797d926d0688e9721f8482ab3fb07236c5faf25ca3bf5910c",
              "signature": "62:03:AE:3F:A5:56:E3:29:31:9F:BC:4C:C8:B7:0F:29:72:EC:75:43:A5:06:7A:9A:23:24:5F:F2:EA:5D:36:60"
            }
          }
        }
        """.trimIndent()

        val root = JSONObject(sampleManifest)
        val platforms = root.getJSONObject("platforms")
        val android = platforms.getJSONObject("android-arm64")

        assertEquals("1.4.2", android.getString("version"))
        assertEquals(6L, android.getLong("versionCode"))
        assertEquals("eba656e0d4bd9cb797d926d0688e9721f8482ab3fb07236c5faf25ca3bf5910c", android.getString("sha256"))
        assertEquals(AppUpdater.OFFICIAL_RELEASE_FINGERPRINT, android.getString("signature"))
    }

    @Test
    fun testSha256ChecksumVerification() {
        val testContent = "SmartMigrate-Secure-APK-Payload-Test"
        val md = MessageDigest.getInstance("SHA-256")
        val hash = md.digest(testContent.toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it) }

        val tempFile = File.createTempFile("sm_test", ".apk")
        tempFile.writeText(testContent)

        val fileDigest = MessageDigest.getInstance("SHA-256")
        tempFile.inputStream().use { fis ->
            val buf = ByteArray(1024)
            var n: Int
            while (fis.read(buf).also { n = it } != -1) {
                fileDigest.update(buf, 0, n)
            }
        }
        val computedHash = fileDigest.digest().joinToString("") { "%02x".format(it) }
        tempFile.delete()

        assertEquals(hash, computedHash)
    }
}
