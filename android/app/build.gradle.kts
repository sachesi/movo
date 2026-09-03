import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

/**
 * Release signing, read from a `keystore.properties` next to this build file that is never
 * committed. Absent it the release build stays unsigned rather than quietly falling back to the
 * debug key, which would ship a build anyone can re-sign over.
 */
val releaseKeystore = rootProject.file("keystore.properties")
    .takeIf { it.exists() }
    ?.let { file -> Properties().apply { file.inputStream().use(::load) } }

android {
    namespace = "org.movo.app"
    compileSdk = 36

    defaultConfig {
        applicationId = "org.movo.app"
        minSdk = 23
        targetSdk = 36
        versionCode = (findProperty("movo.versionCode") as String?)?.toInt() ?: 1
        versionName = findProperty("movo.versionName") as String? ?: "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }
    signingConfigs {
        releaseKeystore?.let { properties ->
            create("release") {
                storeFile = rootProject.file(properties.getProperty("storeFile"))
                storePassword = properties.getProperty("storePassword")
                keyAlias = properties.getProperty("keyAlias")
                keyPassword = properties.getProperty("keyPassword")
            }
        }
    }
    buildTypes {
        release {
            signingConfig = signingConfigs.findByName("release")
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
    sourceSets["main"].jniLibs.srcDir(layout.buildDirectory.dir("generated/jniLibs"))
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
}

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

val buildRust by tasks.registering(Exec::class) {
    workingDir = rustRoot
    // Declared so Gradle can skip the cargo invocation when nothing in the core changed.
    inputs.dir(File(rustRoot, "crates")).withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(File(rustRoot, "Cargo.toml"))
    inputs.file(File(rustRoot, "Cargo.lock"))
    outputs.dir(project.layout.buildDirectory.dir("generated/jniLibs"))
    commandLine("cargo", "ndk", "-t", "arm64-v8a", "-t", "armeabi-v7a", "-t", "x86_64", "-o", project.layout.buildDirectory.dir("generated/jniLibs").get().asFile, "build", "--release", "-p", "movo-android")
}
tasks.named("preBuild").configure { dependsOn(buildRust) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.06.01")
    implementation(composeBom)
    androidTestImplementation(composeBom)
    implementation("androidx.activity:activity-compose:1.12.3")
    implementation("androidx.profileinstaller:profileinstaller:1.4.1")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.tv:tv-material:1.1.0")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.animation:animation")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.10.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.10.0")
    implementation("androidx.media3:media3-exoplayer:1.11.0")
    implementation("androidx.media3:media3-exoplayer-hls:1.11.0")
    implementation("androidx.media3:media3-ui:1.11.0")
    implementation("androidx.media3:media3-session:1.11.0")
    implementation("androidx.datastore:datastore-preferences:1.1.0")
    implementation("androidx.compose.material3:material3-window-size-class")
    implementation("androidx.window:window:1.5.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
    implementation("io.coil-kt.coil3:coil-compose:3.3.0")
    implementation("io.coil-kt.coil3:coil-network-okhttp:3.3.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.10.2")
    testImplementation("androidx.arch.core:core-testing:2.2.0")
    testImplementation("org.robolectric:robolectric:4.16")
    testImplementation("androidx.test:core:1.7.0")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
