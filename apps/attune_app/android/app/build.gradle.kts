import java.io.FileInputStream
import java.util.Properties
import org.gradle.api.GradleException

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

    val resolvedVersionName: String = run {
        // 1. Explicit property passed from Gradle command line (e.g. -Pversion-name=...)
        val propName = project.findProperty("version-name")?.toString()?.trim()
        if (!propName.isNullOrEmpty()) {
            return@run propName
        }
        // 2. Value resolved by Flutter Gradle plugin (from --build-name or pubspec.yaml)
        val flName = flutter.versionName?.trim()
        if (!flName.isNullOrEmpty()) {
            return@run flName
        }
        // 3. Fallback: inspect assets/build_info.json etched by builder
        val buildInfoFile = rootProject.file("../assets/build_info.json")
        if (buildInfoFile.exists()) {
            try {
                val content = buildInfoFile.readText()
                val match = Regex(""""full_version"\s*:\s*"([^"]+)"""").find(content)
                    ?: Regex(""""version"\s*:\s*"([^"]+)"""").find(content)
                if (match != null && match.groupValues[1].isNotBlank()) {
                    return@run match.groupValues[1].trim()
                }
            } catch (_: Exception) {}
        }
        // 4. Fallback: inspect pubspec.yaml directly
        val pubspecFile = rootProject.file("../pubspec.yaml")
        if (pubspecFile.exists()) {
            try {
                val match = Regex("""^version:\s*([^\s+]+)""", RegexOption.MULTILINE).find(pubspecFile.readText())
                if (match != null && match.groupValues[1].isNotBlank()) {
                    return@run match.groupValues[1].trim()
                }
            } catch (_: Exception) {}
        }
        "1.0.0"
    }

    val resolvedVersionCode: Int = run {
        // 1. Explicit property passed from Gradle command line (e.g. -Pversion-code=...)
        val propCode = project.findProperty("version-code")?.toString()?.toIntOrNull()
        if (propCode != null && propCode > 0) {
            return@run propCode
        }
        // 2. Value resolved by Flutter Gradle plugin (from --build-number or pubspec.yaml)
        val flCode = flutter.versionCode
        if (flCode > 0) {
            return@run flCode
        }
        // 3. Fallback: inspect assets/build_info.json etched by builder
        val buildInfoFile = rootProject.file("../assets/build_info.json")
        if (buildInfoFile.exists()) {
            try {
                val content = buildInfoFile.readText()
                val match = Regex(""""build_number"\s*:\s*"([^"]+)"""").find(content)
                val code = match?.groupValues?.get(1)?.toIntOrNull()
                if (code != null && code > 0) {
                    return@run code
                }
            } catch (_: Exception) {}
        }
        // 4. Fallback: inspect pubspec.yaml directly
        val pubspecFile = rootProject.file("../pubspec.yaml")
        if (pubspecFile.exists()) {
            try {
                val match = Regex("""^version:\s*[^\s+]+\+(\d+)""", RegexOption.MULTILINE).find(pubspecFile.readText())
                val code = match?.groupValues?.get(1)?.toIntOrNull()
                if (code != null && code > 0) {
                    return@run code
                }
            } catch (_: Exception) {}
        }
        1
    }

    defaultConfig {
        applicationId = "org.mizproject.attune"
        minSdk = 26
        targetSdk = flutter.targetSdkVersion
        versionCode = resolvedVersionCode
        versionName = resolvedVersionName

        resValue("string", "app_name", "Attune")

        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a", "x86_64"))
        }
    }

    val keyPropertiesFile = rootProject.file("key.properties")
    val keyProperties = Properties()
    if (keyPropertiesFile.exists()) {
        FileInputStream(keyPropertiesFile).use { keyProperties.load(it) }
    }

    val keystorePath = keyProperties.getProperty("storeFile")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("RELEASE_KEYSTORE_FILE")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("ATTUNE_KEYSTORE_FILE")?.trim()?.takeIf { it.isNotEmpty() }
        ?: "attune_release.jks"

    val keystoreFile = project.file(keystorePath).takeIf { it.exists() }
        ?: rootProject.file(keystorePath).takeIf { it.exists() }
        ?: project.file("attune_release.jks").takeIf { it.exists() }

    val storePass = keyProperties.getProperty("storePassword")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("RELEASE_KEYSTORE_PASSWORD")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("ATTUNE_KEYSTORE_PASSWORD")?.trim()?.takeIf { it.isNotEmpty() }
    val alias = keyProperties.getProperty("keyAlias")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("RELEASE_KEY_ALIAS")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("ATTUNE_KEY_ALIAS")?.trim()?.takeIf { it.isNotEmpty() }
    val keyPass = keyProperties.getProperty("keyPassword")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("RELEASE_KEY_PASSWORD")?.trim()?.takeIf { it.isNotEmpty() }
        ?: System.getenv("ATTUNE_KEY_PASSWORD")?.trim()?.takeIf { it.isNotEmpty() }
        ?: storePass

    val hasKeystoreArtifacts = keyPropertiesFile.exists() || (keystoreFile != null && keystoreFile.exists())
    val hasCompleteReleaseSigning = keystoreFile != null && keystoreFile.exists() &&
        !storePass.isNullOrEmpty() && !alias.isNullOrEmpty() && !keyPass.isNullOrEmpty()
    val allowDebugFallbackForRelease =
        System.getenv("ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE").equals("true", ignoreCase = true) ||
        System.getenv("ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE") == "1"

    signingConfigs {
        create("release") {
            if (hasCompleteReleaseSigning) {
                storeFile = keystoreFile
                storePassword = storePass
                keyAlias = alias
                keyPassword = keyPass
            } else if (!hasKeystoreArtifacts && allowDebugFallbackForRelease) {
                // Explicit opt-in debug signing fallback when no release keystore exists
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
            signingConfig = if (hasCompleteReleaseSigning || (!hasKeystoreArtifacts && allowDebugFallbackForRelease)) {
                signingConfigs.getByName("release")
            } else {
                null
            }
        }
    }

    tasks.configureEach {
        if (name.contains("Release", ignoreCase = true) && !hasCompleteReleaseSigning) {
            if (!hasKeystoreArtifacts && allowDebugFallbackForRelease) {
                doFirst {
                    logger.warn(
                        "WARNING: Executing release task '{}' with DEBUG signing fallback " +
                        "(ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE enabled). Do not ship as production.",
                        name
                    )
                }
            } else {
                val missingParts = buildList {
                    if (keystoreFile == null || !keystoreFile.exists()) add("keystore file ($keystorePath)")
                    if (alias.isNullOrEmpty()) add("keyAlias (RELEASE_KEY_ALIAS / ATTUNE_KEY_ALIAS)")
                    if (storePass.isNullOrEmpty()) add("storePassword (RELEASE_KEYSTORE_PASSWORD / ATTUNE_KEYSTORE_PASSWORD)")
                    if (keyPass.isNullOrEmpty()) add("keyPassword (RELEASE_KEY_PASSWORD / ATTUNE_KEY_PASSWORD)")
                }.joinToString(", ")
                doFirst {
                    throw GradleException(
                        "Android release signing credentials are incomplete (missing: $missingParts). " +
                        "Refusing to fall back silently to debug signing for release task '$name'. " +
                        "Set RELEASE_KEYSTORE_PASSWORD and RELEASE_KEY_PASSWORD (or ATTUNE_*) in your environment."
                    )
                }
            }
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
