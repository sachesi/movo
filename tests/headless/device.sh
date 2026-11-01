#!/usr/bin/env bash
# Runs the instrumented tests on a device against the fake provider on this
# machine. With nothing connected it boots the emulator AVD named by MOVO_AVD
# (default movo-test) without a window and shuts it down afterwards. Screens taken
# on the device land in target/headless/device.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
port="${MOVO_FAKE_PORT:-18767}"
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
ndk="${ANDROID_NDK_HOME:-$sdk/ndk/29.0.14206865}"
adb="$sdk/platform-tools/adb"
avd="${MOVO_AVD:-movo-test}"
emulator_pid=""
provider_pid=""

cleanup() {
    [ -n "$provider_pid" ] && kill "$provider_pid" 2>/dev/null || true
    if [ -n "$emulator_pid" ]; then
        "$adb" emu kill >/dev/null 2>&1 || true
        wait "$emulator_pid" 2>/dev/null || true
    fi
    return 0
}
trap cleanup EXIT

"$adb" start-server >/dev/null
if ! "$adb" get-state >/dev/null 2>&1; then
    # Software rendering through ANGLE: the emulator's other headless GPU modes
    # crash on start in a container.
    "$sdk/emulator/emulator" -avd "$avd" -no-window -no-audio -no-boot-anim \
        -gpu swangle_indirect -no-snapshot -no-metrics -feature -Vulkan >"$root/target/emulator.log" 2>&1 &
    emulator_pid=$!
fi
"$adb" wait-for-device
until [ "$("$adb" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do
    sleep 2
done

cd "$root"
cargo build -p movo-fake-provider
target/debug/movo-fake-provider "0.0.0.0:$port" >/dev/null &
provider_pid=$!

# 10.0.2.2 is how an emulator reaches the machine it runs on. A real device
# needs MOVO_PROVIDER_FROM_DEVICE, or `adb reverse` and 127.0.0.1.
provider="${MOVO_PROVIDER_FROM_DEVICE:-http://10.0.2.2:$port}"
cd android
ANDROID_HOME="$sdk" ANDROID_NDK_HOME="$ndk" ./gradlew connectedDebugAndroidTest --console=plain \
    -Pmovo.testProvider="$provider" \
    -Pandroid.testInstrumentationRunnerArguments.movoProvider="$provider"
echo "Report: $PWD/app/build/reports/androidTests/connected/"

mkdir -p "$root/target/headless/device"
"$adb" pull /sdcard/Android/data/org.movo.app/files/shots/. "$root/target/headless/device/" >/dev/null 2>&1 || true
