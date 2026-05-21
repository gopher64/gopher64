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

    var ndkDir = androidComponents.sdkComponents.ndkDirectory.get().asFile.absolutePath
    environment("ANDROID_NDK_HOME", "$ndkDir")
    environment("LIBCLANG_PATH", "$ndkDir/toolchains/llvm/prebuilt/linux-x86_64/musl/lib")

    val jniLibsFolder = layout.projectDirectory.dir("$rootDir/app/src/main/jniLibs")
    project.delete(jniLibsFolder)

    commandLine(
        "cargo", "ndk",
        "--link-libcxx-shared",
        "-P", "34",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-o", "$rootDir/app/src/main/jniLibs",
        "build", "--lib",
        "--profile", if (isRelease) "release" else "dev",
    )
}

tasks.named("preBuild") {
    dependsOn(ndkBuild)
}

dependencies {
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.core.ktx)
    implementation(libs.material)
    implementation(libs.androidx.games.activity)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
}
