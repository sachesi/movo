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
    # The build file falls back to versionCode 4 when nothing is passed in, which would ship
    # every release built here as the same version forever. Derive both from git instead of
    # relying on that fallback: the commit count is monotonic, so it always outgrows the last
    # release even if a tag is missing a bump.
    version_name="$(git describe --tags --abbrev=0 2>/dev/null)" || { echo "No git tag reachable for the release version; tag the commit first (e.g. git tag v0.4.0)" >&2; exit 1; }
    version_name="${version_name#v}"
    version_code="$(git rev-list --count HEAD 2>/dev/null)" || { echo "Unable to read the git commit count for the release version code" >&2; exit 1; }
    cd android
    ANDROID_HOME="$sdk" ANDROID_NDK_HOME="$ndk" ./gradlew assembleRelease -Pmovo.versionCode="$version_code" -Pmovo.versionName="$version_name"
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
    # Kept alongside the APK it belongs to: retrace needs the exact mapping for a release build to
    # turn an obfuscated stack trace back into real names, and R8 overwrites this file on every run.
    mapping="app/build/outputs/mapping/release/mapping.txt"
    test -f "$mapping" || { echo "R8 mapping file not found: $PWD/$mapping" >&2; exit 1; }
    mapping_output="$(dirname "$output")/movo-release-$version_name-mapping.txt"
    cp "$mapping" "$mapping_output"
    # Under the name the release asset carries, so `sha256sum -c` matches a downloaded copy.
    checksum_output="$(dirname "$output")/movo-$version_name.apk.sha256"
    (cd "$(dirname "$output")" && cp -f movo-release.apk "movo-$version_name.apk" && sha256sum "movo-$version_name.apk" > "$checksum_output")
    echo "APK: $(dirname "$output")/movo-$version_name.apk"
    echo "Checksum: $checksum_output"
    echo "Mapping: $mapping_output"

# Run tests and lint, then build the debug APK.
apk-check:
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
    test -d "$sdk" || { echo "Android SDK not found: $sdk" >&2; exit 1; }
    ndk="${ANDROID_NDK_HOME:-$sdk/ndk/29.0.14206865}"
    test -d "$ndk" || { echo "Android NDK 29.0.14206865 not found: $ndk" >&2; exit 1; }
    command -v cargo-ndk >/dev/null || { echo "Install cargo-ndk: cargo install cargo-ndk --locked" >&2; exit 1; }
    cd android
    ANDROID_HOME="$sdk" ANDROID_NDK_HOME="$ndk" ./gradlew testDebugUnitTest lintDebug assembleDebug assembleDebugAndroidTest
    echo "APK: $PWD/app/build/outputs/apk/debug/app-debug.apk"

# Run the instrumentation tests on a connected device or emulator; the television suite expects
# a TV profile (Android TV/Google TV emulator image or hardware) to run against.
apk-test-device:
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
    test -d "$sdk" || { echo "Android SDK not found: $sdk" >&2; exit 1; }
    ndk="${ANDROID_NDK_HOME:-$sdk/ndk/29.0.14206865}"
    test -d "$ndk" || { echo "Android NDK 29.0.14206865 not found: $ndk" >&2; exit 1; }
    command -v cargo-ndk >/dev/null || { echo "Install cargo-ndk: cargo install cargo-ndk --locked" >&2; exit 1; }
    cd android
    ANDROID_HOME="$sdk" ANDROID_NDK_HOME="$ndk" ./gradlew connectedDebugAndroidTest
    echo "Report: $PWD/app/build/reports/androidTests/connected/"

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
    # xgettext has no Rust mode; C mode is close enough but reads a Rust lifetime such as
    # 'static as an unterminated character constant on every occurrence, which drowns out
    # any warning that would matter. Filter that one message out; nothing else is silenced.
    xgettext --from-code=UTF-8 --language=C --keyword=tr --keyword=trf --keyword=trn:1,2 \
        --package-name=Movo --package-version=0.4.2 \
        --output=po/movo.pot $(find crates/movo-desktop/src -name '*.rs' | sort) \
        2> >(grep -v 'unterminated character constant' >&2)
    for language in ru uk; do
        msgmerge --quiet --no-fuzzy-matching --update --backup=none \
            "po/$language.po" po/movo.pot
    done

# Run the desktop app from source with the development catalogs.
run: mo
    cargo run -p movo

# Regenerate the Android raster icons from the vector masters in design/icons.
icons:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v rsvg-convert >/dev/null || { echo "Install librsvg (rsvg-convert)" >&2; exit 1; }
    src=design/icons
    res=android/app/src/main/res
    # The masters are built from drop-shadow filters, which VectorDrawable cannot express, so the
    # adaptive icon layers ship as bitmaps. Only the monochrome layer stays a vector.
    for bucket in mdpi:108 hdpi:162 xhdpi:216 xxhdpi:324 xxxhdpi:432; do
        density="${bucket%%:*}"; size="${bucket##*:}"
        mkdir -p "$res/drawable-$density"
        for layer in ic_launcher_background ic_launcher_foreground; do
            rsvg-convert -w "$size" -h "$size" "$src/$layer.svg" -o "$res/drawable-$density/$layer.png"
        done
    done
    # Launcher icon, 48dp. Used on Android 6 to 7.1, which predate adaptive icons.
    for bucket in mdpi:48 hdpi:72 xhdpi:96 xxhdpi:144 xxxhdpi:192; do
        density="${bucket%%:*}"; size="${bucket##*:}"
        mkdir -p "$res/mipmap-$density"
        rsvg-convert -w "$size" -h "$size" "$src/icon_phone.svg" -o "$res/mipmap-$density/ic_launcher.png"
    done
    # Leanback banner: 320x180, at the density every television reports.
    mkdir -p "$res/drawable-xhdpi"
    rsvg-convert -w 320 -h 180 "$src/banner_tv_minimal_320x180.svg" -o "$res/drawable-xhdpi/tv_banner.png"
    echo "Regenerated the icons under $res"

# Install the desktop entry and icons for the current user.
install-desktop:
    #!/usr/bin/env bash
    set -euo pipefail
    # The shell resolves a window's icon through the icon theme by application ID, so the app
    # runs without one until these land where the theme can see them.
    share="${XDG_DATA_HOME:-$HOME/.local/share}"
    data=crates/movo-desktop/data
    install -Dm644 "$data/io.github.sachesi.Movo.desktop" "$share/applications/io.github.sachesi.Movo.desktop"
    install -Dm644 "$data/io.github.sachesi.Movo.metainfo.xml" \
        "$share/metainfo/io.github.sachesi.Movo.metainfo.xml"
    install -Dm644 "$data/icons/hicolor/scalable/apps/io.github.sachesi.Movo.svg" \
        "$share/icons/hicolor/scalable/apps/io.github.sachesi.Movo.svg"
    install -Dm644 "$data/icons/hicolor/symbolic/apps/io.github.sachesi.Movo-symbolic.svg" \
        "$share/icons/hicolor/symbolic/apps/io.github.sachesi.Movo-symbolic.svg"
    install -Dm644 "$data/icons/hicolor/512x512/apps/io.github.sachesi.Movo.png" \
        "$share/icons/hicolor/512x512/apps/io.github.sachesi.Movo.png"
    if command -v gtk-update-icon-cache >/dev/null; then
        gtk-update-icon-cache -qtf "$share/icons/hicolor" || true
    fi
    if command -v update-desktop-database >/dev/null; then
        update-desktop-database -q "$share/applications" || true
    fi
    echo "Installed to $share"
    command -v movo >/dev/null ||
        echo "note: the entry runs 'movo', which is not on PATH yet. cargo install --path crates/movo-desktop"

# Format, lint and test the Rust workspace. Tests against the live provider are ignored by default.
check:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
