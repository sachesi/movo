# Testing

`just check` formats, lints and runs every test that needs nothing but Cargo. That includes
`crates/movo-fake-provider/tests/client_flows.rs`, which drives the whole core client over real
HTTP.

## The fake provider

`crates/movo-fake-provider` is a small server that answers in the site's markup with an invented
library of 240 titles, one account (`tester` / `secret`) and that account's favorites and history.
Writes change its state, so a flow can read back what it did. It can also misbehave on request:

    curl 'http://127.0.0.1:18765/__fake/faults?path=/series&kind=delay&value=4000&times=1'

`kind` is one of `status`, `delay`, `garbage`, `empty`, `drop`, `lose-reply`, `limit` and `expired`.
`value` is the status or the milliseconds, `times` limits how often it applies, and `path` matches
any request path containing it. `/__fake/clear` removes them, `/__fake/requests` lists what was
served and `/__fake/reset-log` empties that list.

A client only looks at it when built with the `test-provider` feature, which reads the address from
`MOVO_TEST_PROVIDER`. No package enables that feature, so a shipped binary always talks to the real
site.

    just fake-provider &
    MOVO_TEST_PROVIDER=http://127.0.0.1:18765 cargo run -p movo --features test-provider

## The desktop app without a display

`just test-desktop-headless` runs the app in a private Wayland session, walks it through the
scenarios in `tests/headless/desktop.sh`, and checks what it exposes over accessibility. Each step
leaves a screenshot in `target/headless/desktop`. Pass a scenario name to run only that one.

The script needs a session driver: `gui-test` on the path, or the program `GUI_TEST` names. It has to
offer `start`, `stop`, `ui`, `windows`, `click`, `key`, `type` and `shot`.

Test builds also watch the main loop and log any stretch over 300 ms that it was kept from running.
The run fails on those and on toolkit warnings, so work that belongs on another thread shows up here
even when the screen looks right.

## The Android app without a device

`just test-android-headless` builds the core for this machine, starts the fake provider and runs
`ProviderFlowTest` under Robolectric: the app's Kotlin talking to the real Rust core over JNI and
real HTTP. Rendered screens land in `target/headless/android`. Elsewhere those tests skip themselves,
so `gradlew testDebugUnitTest` needs neither the library nor the provider.

Robolectric has no platform keystore, so everything that stores a session through `SessionStore`
needs a device: `just apk-test-device` runs the instrumented tests on whatever `adb` is connected to.
