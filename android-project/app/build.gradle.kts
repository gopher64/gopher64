import groovy.json.JsonSlurper
import java.util.Properties
import java.io.FileInputStream
import java.net.URI
import org.gradle.internal.os.OperatingSystem

plugins {
    alias(libs.plugins.android.application)
}

kotlin {
    jvmToolchain(21)
}

val keystorePropertiesFile = rootProject.file("keystore.properties")
val keystoreProperties = Properties()
if (keystorePropertiesFile.exists()) {
    keystoreProperties.load(FileInputStream(keystorePropertiesFile))
}

android {
    namespace = "io.github.gopher64.gopher64"
    compileSdk {
        version = release(37) {
            minorApiLevel = 2
        }
    }

    ndkVersion = "30.0.16248370"

    defaultConfig {
        applicationId = "io.github.gopher64.gopher64"
        minSdk = 33
        targetSdk = 37
        versionCode = semverToVersionCode(cargoPackageVersion())
        versionName = cargoPackageVersion()
        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "x86_64"))
        }

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        externalNativeBuild {
            cmake {
                arguments += listOf(
                    "-DANDROID_STL=c++_shared",
                    "-DBUILD_SHARED_LIBS=ON",
                )
                abiFilters += "arm64-v8a" // libadrenotools is arm64-only
            }
        }
    }

    signingConfigs {
        if (keystorePropertiesFile.exists()) {
            create("release") {
                keyAlias = keystoreProperties["keyAlias"] as String
                keyPassword = keystoreProperties["keyPassword"] as String
                storeFile = file(keystoreProperties["storeFile"] as String)
                storePassword = keystoreProperties["storePassword"] as String
            }
        }
    }

    buildTypes {
        release {
            if (keystorePropertiesFile.exists()) {
                signingConfig = signingConfigs["release"]
            }
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    packaging {
        jniLibs {
            excludes.add("lib/**/libsevenz_rust2*.so")
            useLegacyPackaging = true
        }
    }

    sourceSets {
        getByName("main") {
            java.directories += sdl3JavaSrcDir().absolutePath
        }
    }

    externalNativeBuild {
        cmake {
            path = file("../libadrenotools/CMakeLists.txt")
        }
    }
}

@Suppress("UNCHECKED_CAST")
fun cargoPackageVersion(packageName: String = "gopher64"): String {
    val repoRoot = rootDir.parentFile
    val jsonText = providers.exec {
        workingDir(repoRoot)
        commandLine("cargo", "metadata", "--format-version", "1", "--no-deps")
    }.standardOutput.asText.get()

    val parsedJson = JsonSlurper().parseText(jsonText) as Map<String, Any>
    val packages = parsedJson["packages"] as List<Map<String, Any>>
    return packages.first { it["name"] == packageName }["version"] as String
}

@Suppress("UNCHECKED_CAST")
fun sdl3JavaSrcDir(): File {
    val repoRoot = rootDir.parentFile
    val jsonText = providers.exec {
        workingDir(repoRoot)
        commandLine("cargo", "metadata", "--format-version", "1")
    }.standardOutput.asText.get()

    val parsedJson = JsonSlurper().parseText(jsonText) as Map<String, Any>
    val packages = parsedJson["packages"] as List<Map<String, Any>>
    val manifestPath = packages.first { it["name"] == "sdl3-src" }["manifest_path"] as String
    return File(manifestPath).parentFile.resolve(
        "SDL/android-project/app/src/main/java"
    )
}

fun semverToVersionCode(version: String): Int {
    val parts = version.substringBefore('-').split(".")
    val (major, minor, patch) = Triple(parts[0].toInt(), parts[1].toInt(), parts[2].toInt())

    return major * 1_000_000 + minor * 1_000 + patch
}

val ndkBuild = tasks.register<Exec>("ndkBuild") {
    val isRelease = gradle.startParameter.taskNames.any { it.endsWith("Release", ignoreCase = true) }
    workingDir = rootDir.parentFile
    val toolchainPath = "$rootDir/android.toolchain.cmake"
    environment("CMAKE_TOOLCHAIN_FILE", toolchainPath)
    environment("CMAKE_GENERATOR", "Ninja")

    var minSdk = android.defaultConfig.minSdk
    var ndkDir = androidComponents.sdkComponents.ndkDirectory.get().asFile.absolutePath
    val sdkDir = androidComponents.sdkComponents.sdkDirectory.get().asFile.absolutePath
    val compileSdk = android.compileSdk
    val compileSdkMinor = android.compileSdkMinor
    environment("ANDROID_JAR", "$sdkDir/platforms/android-$compileSdk.$compileSdkMinor/android.jar")
    environment("ANDROID_NDK_HOME", "$ndkDir")
    environment("ANDROID_NDK_ROOT", "$ndkDir")
    val libClangPath = if (OperatingSystem.current().isWindows) {
        "$ndkDir/toolchains/llvm/prebuilt/windows-x86_64/bin"
    } else {
        "$ndkDir/toolchains/llvm/prebuilt/linux-x86_64/lib"
    }
    environment("LIBCLANG_PATH", libClangPath)

    val jniType = if (isRelease) "release" else "debug"
    val jniLibsFolder = "$rootDir/app/src/$jniType/jniLibs"

    commandLine(
        "cargo", "ndk",
        "--link-libcxx-shared",
        "-P", "$minSdk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-o", jniLibsFolder,
        "build", "--lib",
        "--profile", if (isRelease) "release" else "dev",
    )
}

val sdlLibsArm64 = tasks.register<Copy>("sdlLibsArm64") {
    val isRelease = gradle.startParameter.taskNames.any { it.endsWith("Release", ignoreCase = true) }
    val jniType = if (isRelease) "release" else "debug"
    val jniLibsFolder = "$rootDir/app/src/$jniType/jniLibs/arm64-v8a"

    from("$rootDir/../target/aarch64-linux-android/$jniType")
    into(jniLibsFolder)
    include("libSDL*")
}

val sdlLibsX64 = tasks.register<Copy>("sdlLibsX64") {
    val isRelease = gradle.startParameter.taskNames.any { it.endsWith("Release", ignoreCase = true) }
    val jniType = if (isRelease) "release" else "debug"
    val jniLibsFolder = "$rootDir/app/src/$jniType/jniLibs/x86_64"

    from("$rootDir/../target/x86_64-linux-android/$jniType")
    into(jniLibsFolder)
    include("libSDL*")
}

// Bundled Turnip Vulkan driver, loaded through libadrenotools at runtime. arm64-only.
val turnipDriverUrl =
    "https://github.com/whitebelyash/AdrenoToolsDrivers/releases/download/stu_v2/stable-turnip-sync-V2.zip"
val turnipZip = layout.buildDirectory.file("turnip/stable-turnip-V2.zip")

val turnipDownload = tasks.register("turnipDownload") {
    val zip = turnipZip.get().asFile
    inputs.property("url", turnipDriverUrl)
    outputs.file(zip)

    doLast {
        zip.parentFile.mkdirs()
        URI(turnipDriverUrl).toURL().openStream().use { input ->
            zip.outputStream().use { output -> input.copyTo(output) }
        }
    }
}

val turnipDriverArm64 = tasks.register<Copy>("turnipDriverArm64") {
    dependsOn(turnipDownload)
    val isRelease = gradle.startParameter.taskNames.any { it.endsWith("Release", ignoreCase = true) }
    val jniType = if (isRelease) "release" else "debug"
    val jniLibsFolder = "$rootDir/app/src/$jniType/jniLibs/arm64-v8a"

    from(zipTree(turnipZip)) {
        include("libvulkan_freedreno.so")
    }
    into(jniLibsFolder)
}

tasks.named("preBuild") {
    dependsOn(sdlLibsArm64)
    dependsOn(sdlLibsX64)
    dependsOn(turnipDriverArm64)
}

tasks.named("sdlLibsArm64") {
    dependsOn(ndkBuild)
}

tasks.named("sdlLibsX64") {
    dependsOn(ndkBuild)
}

dependencies {
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.core.ktx)
    implementation(libs.material)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
}
