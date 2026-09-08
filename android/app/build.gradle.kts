import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

android {
    namespace = "org.movo.app"
    compileSdk = 37

    defaultConfig {
        applicationId = "org.movo.app"
        minSdk = 23
        targetSdk = 36
        versionCode = (findProperty("movo.versionCode") as String?)?.toInt() ?: 4
        versionName = findProperty("movo.versionName") as String? ?: "0.3.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        // Matches the ABIs buildRust actually compiles; without this, transitive AndroidX
        // prebuilts pull an x86 lib/ dir into the APK with no native library in it, and a
        // 32-bit x86 device installs the app only to crash on System.loadLibrary.
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    }
    buildTypes {
        release {
            // Left unsigned here on purpose: `just apk-release` builds this, then zipaligns,
            // signs and verifies it itself, so there is exactly one signing path.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    buildFeatures { compose = true }
    // Derives locales_config.xml from the values-* folders, which is what puts the app in the
    // per-app language picker on Android 13 and later.
    androidResources { generateLocaleConfig = true }
    packaging { jniLibs.useLegacyPackaging = false }
    // The unit tests render real composables under Robolectric, which needs the resources.
    testOptions { unitTests.isIncludeAndroidResources = true }
    sourceSets["main"].jniLibs.directories.add(layout.buildDirectory.dir("generated/jniLibs").get().asFile.absolutePath)
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin { compilerOptions { jvmTarget.set(JvmTarget.JVM_17) } }

// Recomposition and stability reports, opt-in so ordinary builds stay fast:
//   ./gradlew assembleDebug -Pmovo.composeMetrics=true
composeCompiler {
    stabilityConfigurationFiles.add(layout.projectDirectory.file("compose_stability.conf"))
    if (project.findProperty("movo.composeMetrics") == "true") {
        metricsDestination = layout.buildDirectory.dir("compose-metrics")
        reportsDestination = layout.buildDirectory.dir("compose-reports")
    }
}

val rustRoot = rootProject.projectDir.parentFile

tasks.withType<Test>().configureEach {
    inputs.file(File(rustRoot, "crates/movo-android/src/lib.rs"))
        .withPathSensitivity(PathSensitivity.RELATIVE)
}

val buildRust = tasks.register<Exec>("buildRust") {
    workingDir = rustRoot
    // Declared so Gradle can skip the cargo invocation when nothing in the core changed.
    inputs.dir(File(rustRoot, "crates")).withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(File(rustRoot, "Cargo.toml"))
    inputs.file(File(rustRoot, "Cargo.lock"))
    outputs.dir(project.layout.buildDirectory.dir("generated/jniLibs"))
    commandLine("cargo", "ndk", "-t", "arm64-v8a", "-t", "armeabi-v7a", "-t", "x86_64", "-o", project.layout.buildDirectory.dir("generated/jniLibs").get().asFile, "build", "--release", "-p", "movo-android")
}
// Only the tasks that package native libs need the Rust build; wiring it to preBuild made
// lint and unit tests each pay for three cross-compiles they never load the library for.
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("JniLibFolders") }.configureEach { dependsOn(buildRust) }

dependencies {
    val composeBom = platform(libs.androidx.compose.bom)
    implementation(composeBom)
    androidTestImplementation(composeBom)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.profileinstaller)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.tv.material)
    implementation(libs.androidx.compose.material.icons.extended)
    implementation(libs.androidx.compose.animation)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.media3.exoplayer)
    implementation(libs.androidx.media3.exoplayer.hls)
    implementation(libs.androidx.media3.ui)
    implementation(libs.androidx.media3.session)
    implementation(libs.androidx.datastore.preferences)
    implementation(libs.androidx.compose.material3.window.size)
    implementation(libs.androidx.window)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.coil.compose)
    implementation(libs.coil.network.okhttp)
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.androidx.arch.core.testing)
    testImplementation(libs.robolectric)
    testImplementation(libs.androidx.test.core)
    testImplementation(composeBom)
    testImplementation(libs.androidx.compose.ui.test.junit4)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
    debugImplementation(libs.androidx.compose.ui.test.manifest)
}
