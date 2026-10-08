# Smart Migrate - Production ProGuard / R8 Rules
# OWASP MASVS Resiliency and Shrinking Configuration

# Keep Android Application Components & Entrypoints
-keep class com.smartmigrate.client.MainActivity { *; }
-keep class com.smartmigrate.client.StreamEngine** { *; }
-keep class com.smartmigrate.client.StreamDiagnostics { *; }
-keep class com.smartmigrate.client.StreamConnectionState { *; }

# Keep Compose Runtime & UI components
-keep class androidx.compose.runtime.** { *; }
-keepclassmembers class * {
    @androidx.compose.runtime.Composable *;
}

# Keep Coroutines internals
-keepnames class kotlinx.coroutines.internal.MainDispatcherFactory {}
-keepnames class kotlinx.coroutines.CoroutineExceptionHandler {}

# Line numbers and source attributes for release crash diagnostics
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
