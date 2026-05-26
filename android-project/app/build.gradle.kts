plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "io.github.gopher64.gopher64"
    compileSdk {
        version = release(36) {
            minorApiLevel = 1
        }
    }

    ndkVersion = "27.3.13750724"

    defaultConfig {
        applicationId = "io.github.gopher64.gopher64"
        minSdk = 34
        targetSdk = 36
        versionCode = 1
        versionName = "1.1.20"
        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "x86_64"))
        }

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            signingConfig = signingConfigs["debug"]
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    packaging {
        jniLibs {
            excludes.add("lib/**/libsevenz_rust2*.so")
        }
    }
}

val ndkBuild = tasks.register<Exec>("ndkBuild") {
    val isRelease = gradle.startParameter.taskNames.any { it.endsWith("Release", ignoreCase = true) }
    workingDir = rootDir.parentFile
    val toolchainPath = "$rootDir/android.toolchain.cmake"
    environment("CMAKE_TOOLCHAIN_FILE", toolchainPath)

    var minSdk = android.defaultConfig.minSdk
    var ndkDir = androidComponents.sdkComponents.ndkDirectory.get().asFile.absolutePath
    environment("ANDROID_NDK_HOME", "$ndkDir")
    environment("LIBCLANG_PATH", "$ndkDir/toolchains/llvm/prebuilt/linux-x86_64/musl/lib")

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

tasks.named("preBuild") {
    dependsOn(sdlLibsArm64)
    dependsOn(sdlLibsX64)
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
