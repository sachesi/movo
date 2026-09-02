default:
    @just --list

# Build the debug Android APK.
apk:
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
    test -d "$sdk" || { echo "Android SDK not found: $sdk" >&2; exit 1; }
    ndk="${ANDROID_NDK_HOME:-$sdk/ndk/29.0.14206865}"
    test -d "$ndk" || { echo "Android NDK 29.0.14206865 not found: $ndk" >&2; exit 1; }
    command -v cargo-ndk >/dev/null || { echo "Install cargo-ndk: cargo install cargo-ndk --locked" >&2; exit 1; }
    test -n "${JAVA_HOME:-}" || test ! -d /tmp/movo-jdk21 || export JAVA_HOME=/tmp/movo-jdk21
    cd android
    ANDROID_HOME="$sdk" ANDROID_NDK_HOME="$ndk" ./gradlew assembleDebug
    echo "APK: $PWD/app/build/outputs/apk/debug/app-debug.apk"

# Build, align, sign, and verify the release Android APK.
apk-release:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${MOVO_KEYSTORE:?Set MOVO_KEYSTORE to the release keystore path}"
    : "${MOVO_KEY_ALIAS:?Set MOVO_KEY_ALIAS to the release key alias}"
    : "${MOVO_KEYSTORE_PASSWORD:?Set MOVO_KEYSTORE_PASSWORD}"
    export MOVO_KEY_PASSWORD="${MOVO_KEY_PASSWORD:-$MOVO_KEYSTORE_PASSWORD}"
    test -f "$MOVO_KEYSTORE" || { echo "Release keystore not found: $MOVO_KEYSTORE" >&2; exit 1; }
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
    test -d "$sdk" || { echo "Android SDK not found: $sdk" >&2; exit 1; }
    ndk="${ANDROID_NDK_HOME:-$sdk/ndk/29.0.14206865}"
    test -d "$ndk" || { echo "Android NDK 29.0.14206865 not found: $ndk" >&2; exit 1; }
    build_tools="$sdk/build-tools/36.0.0"
    test -x "$build_tools/zipalign" || { echo "zipalign not found: $build_tools" >&2; exit 1; }
    test -x "$build_tools/apksigner" || { echo "apksigner not found: $build_tools" >&2; exit 1; }
    command -v cargo-ndk >/dev/null || { echo "Install cargo-ndk: cargo install cargo-ndk --locked" >&2; exit 1; }
    test -n "${JAVA_HOME:-}" || test ! -d /tmp/movo-jdk21 || export JAVA_HOME=/tmp/movo-jdk21
    cd android
    ANDROID_HOME="$sdk" ANDROID_NDK_HOME="$ndk" ./gradlew assembleRelease
    unsigned="$PWD/app/build/outputs/apk/release/app-release-unsigned.apk"
    output="$PWD/app/build/outputs/apk/release/movo-release.apk"
    test -f "$unsigned" || { echo "Unsigned release APK not found: $unsigned" >&2; exit 1; }
    "$build_tools/zipalign" -f -P 16 4 "$unsigned" "$output"
    "$build_tools/apksigner" sign \
        --ks "$MOVO_KEYSTORE" \
        --ks-key-alias "$MOVO_KEY_ALIAS" \
        --ks-pass env:MOVO_KEYSTORE_PASSWORD \
        --key-pass env:MOVO_KEY_PASSWORD \
        --v4-signing-enabled false \
        "$output"
    "$build_tools/zipalign" -c -P 16 4 "$output"
    "$build_tools/apksigner" verify --verbose --print-certs "$output"
    echo "APK: $output"

# Run tests and lint, then build the debug APK.
apk-check:
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
    test -d "$sdk" || { echo "Android SDK not found: $sdk" >&2; exit 1; }
    ndk="${ANDROID_NDK_HOME:-$sdk/ndk/29.0.14206865}"
    test -d "$ndk" || { echo "Android NDK 29.0.14206865 not found: $ndk" >&2; exit 1; }
    command -v cargo-ndk >/dev/null || { echo "Install cargo-ndk: cargo install cargo-ndk --locked" >&2; exit 1; }
    test -n "${JAVA_HOME:-}" || test ! -d /tmp/movo-jdk21 || export JAVA_HOME=/tmp/movo-jdk21
    cd android
    ANDROID_HOME="$sdk" ANDROID_NDK_HOME="$ndk" ./gradlew testDebugUnitTest lintDebug assembleDebug assembleDebugAndroidTest
    echo "APK: $PWD/app/build/outputs/apk/debug/app-debug.apk"

# Build and install the debug APK on a connected device.
apk-install: apk
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
    "$sdk/platform-tools/adb" install -r android/app/build/outputs/apk/debug/app-debug.apk

# Compile the translation catalogs the desktop app loads at run time.
mo:
    #!/usr/bin/env bash
    set -euo pipefail
    for language in ru uk; do
        mkdir -p "locale/$language/LC_MESSAGES"
        msgfmt --check-format --statistics \
            -o "locale/$language/LC_MESSAGES/movo.mo" "po/$language.po"
    done

# Re-extract translatable strings and merge them into the catalogs.
pot:
    #!/usr/bin/env bash
    set -euo pipefail
    xgettext --from-code=UTF-8 --language=C --keyword=tr --keyword=trf \
        --package-name=Movo --package-version=0.1.0 \
        --output=po/movo.pot $(find crates/movo-desktop/src -name '*.rs' | sort)
    for language in ru uk; do
        msgmerge --quiet --no-fuzzy-matching --update --backup=none \
            "po/$language.po" po/movo.pot
    done

# Run the desktop app from source with the development catalogs.
run: mo
    cargo run -p movo

# Format, lint and test the Rust workspace. Tests against the live provider are ignored by default.
check:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets
    cargo test --workspace
