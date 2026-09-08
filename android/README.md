# Movo for Android

The Android client uses the same Rust provider engine as the Linux application and presents adaptive Jetpack Compose layouts for phones, tablets, Android TV, and Google TV. Playback uses Media3 with stream quality and subtitle selection.

## Requirements

- JDK 17 or 21
- Android SDK platform 37 (`platforms;android-37.0`) and Android Build Tools 36
- Android NDK 29.0.14206865
- Rust targets `aarch64-linux-android`, `armv7-linux-androideabi`, and `x86_64-linux-android`
- `cargo-ndk`

Set `ANDROID_HOME` and `ANDROID_NDK_HOME`, then build:

```sh
just apk
```

Use `just apk-check` for tests and lint, or `just apk-install` to build and install on a connected device.

The debug APK is written to `app/build/outputs/apk/debug/app-debug.apk`.

Account session cookies are encrypted with an Android Keystore key. Passwords are never stored. Exact resume positions remain local and account-scoped; favorites, history, and watched state are verified against the official account.
