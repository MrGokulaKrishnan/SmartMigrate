# Smart Migrate Android Client

This is the Kotlin/Jetpack Compose Android client shell. It includes working local navigation, a protected pairing explanation, the supplied Smart Migrate logo, and the same violet liquid-glass visual language as the website and Windows host.

The future client will request pairing explicitly, keep key material in Android Keystore, and stop every active session when a user disconnects or the device trust is revoked. It does not currently simulate pairing, remote control, or streaming.

## Build prerequisites

Install JDK 17, Android SDK API 37, and Gradle 9.1+ (or open the folder in a compatible Android Studio release), then run:

```powershell
gradle :app:assembleDebug
```
