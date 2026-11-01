import java.io.FileInputStream
import java.util.Properties

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "org.mizproject.attune"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        resValues = true
    }

    defaultConfig {
        applicationId = "org.mizproject.attune"
        minSdk = 26
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName

        resValue("string", "app_name", "Attune")

        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a", "x86_64"))
        }
    }

    signingConfigs {
        create("release") {
            val keyPropertiesFile = rootProject.file("key.properties")
            val keyProperties = Properties()
            if (keyPropertiesFile.exists()) {
                keyProperties.load(FileInputStream(keyPropertiesFile))
            }

            val keystorePath = keyProperties.getProperty("storeFile")
                ?: System.getenv("ATTUNE_KEYSTORE_FILE")
                ?: "attune_release.jks"

            val keystoreFile = project.file(keystorePath).takeIf { it.exists() }
                ?: rootProject.file(keystorePath).takeIf { it.exists() }
                ?: project.file("attune_release.jks").takeIf { it.exists() }

            val storePass = keyProperties.getProperty("storePassword")
                ?: System.getenv("ATTUNE_KEYSTORE_PASSWORD")
            val alias = keyProperties.getProperty("keyAlias")
                ?: System.getenv("ATTUNE_KEY_ALIAS")
            val keyPass = keyProperties.getProperty("keyPassword")
                ?: System.getenv("ATTUNE_KEY_PASSWORD")
                ?: storePass

            if (keystoreFile != null && keystoreFile.exists() && !storePass.isNullOrEmpty() && !alias.isNullOrEmpty()) {
                storeFile = keystoreFile
                storePassword = storePass
                keyAlias = alias
                keyPassword = keyPass
            } else {
                // Graceful fallback to debug signing if release keystore or credentials are not configured
                val debugConfig = signingConfigs.getByName("debug")
                storeFile = debugConfig.storeFile
                storePassword = debugConfig.storePassword
                keyAlias = debugConfig.keyAlias
                keyPassword = debugConfig.keyPassword
            }
        }
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-debug"
            resValue("string", "app_name", "Attune (Debug)")
            signingConfig = signingConfigs.getByName("debug")
        }
        release {
            isMinifyEnabled = false
            isShrinkResources = false
            signingConfig = signingConfigs.getByName("release")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
