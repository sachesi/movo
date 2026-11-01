#!/usr/bin/env bash
# Runs the Android app's Kotlin against the real Rust core and the fake provider,
# on this machine, with no device: the core is built for the host and the JVM
# tests load it over JNI. Screens rendered along the way land in
# target/headless/android.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
port="${MOVO_FAKE_PORT:-18766}"
scratch="$(mktemp -d)"

cleanup() {
    [ -n "${provider_pid:-}" ] && kill "$provider_pid" 2>/dev/null || true
    rm -rf "$scratch"
    return 0
}
trap cleanup EXIT

cd "$root"
cargo build -p movo-fake-provider -p movo-android --features movo-android/test-provider

target/debug/movo-fake-provider "127.0.0.1:$port" >/dev/null &
provider_pid=$!
sleep 0.3

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
cd android
ANDROID_HOME="$sdk" \
    MOVO_TEST_PROVIDER="http://127.0.0.1:$port" \
    MOVO_CORE_LIBRARY_DIR="$root/target/debug" \
    MOVO_ANDROID_SHOTS="$root/target/headless/android" \
    HOME="$scratch" GRADLE_USER_HOME="${GRADLE_USER_HOME:-$HOME/.gradle}" \
    XDG_CONFIG_HOME="$scratch/config" \
    XDG_DATA_HOME="$scratch/data" \
    XDG_CACHE_HOME="$scratch/cache" \
    ./gradlew testDebugUnitTest --rerun --console=plain --tests 'org.movo.app.ProviderFlowTest'
