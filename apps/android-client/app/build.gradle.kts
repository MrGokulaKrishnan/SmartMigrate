plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.smartmigrate.client"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.smartmigrate.client"
        minSdk = 26
        targetSdk = 35
        versionCode = 5
        versionName = "1.4.1"
    }

    signingConfigs {
        create("release") {
            val keystoreFile = file("release.keystore")
            if (keystoreFile.exists()) {
                storeFile = keystoreFile
                storePassword = (project.findProperty("SM_KEYSTORE_PASSWORD") as? String)
                    ?: System.getenv("SM_KEYSTORE_PASSWORD")
                    ?: "smartmigrate_keystore_pass"
                keyAlias = (project.findProperty("SM_KEY_ALIAS") as? String)
                    ?: System.getenv("SM_KEY_ALIAS")
                    ?: "smartmigrate_release"
                keyPassword = (project.findProperty("SM_KEY_PASSWORD") as? String)
                    ?: System.getenv("SM_KEY_PASSWORD")
                    ?: "smartmigrate_keystore_pass"
                enableV1Signing = true
                enableV2Signing = true
            } else {
                throw GradleException("release.keystore is required for release build and must not fall back to debug")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            signingConfig = signingConfigs.getByName("release")
        }
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.10.01")
    implementation(composeBom)
    androidTestImplementation(composeBom)

    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.animation:animation")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")

    // CameraX for hardware QR scanner
    implementation("androidx.camera:camera-camera2:1.4.1")
    implementation("androidx.camera:camera-lifecycle:1.4.1")
    implementation("androidx.camera:camera-view:1.4.1")
    implementation("com.google.zxing:core:3.5.3")

    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
}
