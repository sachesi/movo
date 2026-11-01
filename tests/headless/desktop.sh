#!/usr/bin/env bash
# Drives the desktop app against the fake provider in a private Wayland session
# and checks what it exposes over accessibility, leaving a screenshot of every
# step in target/headless/desktop. The session driver is `gui-test`, or whatever
# GUI_TEST names; it has to offer start, stop, ui, windows, click, key, type and
# shot. A step on the main loop longer than the app's stall limit fails the run.
#
# Usage: desktop.sh [scenario]
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
gui="${GUI_TEST:-gui-test}"
out="$root/target/headless/desktop"
provider_port="${MOVO_FAKE_PORT:-18765}"
provider="http://127.0.0.1:$provider_port"
only="${1:-}"

command -v "$gui" >/dev/null || {
    echo "session driver not found: $gui (set GUI_TEST)" >&2
    exit 2
}

failures=0
provider_pid=""
state_dir="${GUI_TEST_STATE_DIR:-/tmp/gui-test-$UID}"

cleanup() {
    "$gui" stop >/dev/null 2>&1 || true
    [ -n "$provider_pid" ] && kill "$provider_pid" 2>/dev/null || true
    [ -n "${home_dir:-}" ] && rm -rf "$home_dir"
    return 0
}
trap cleanup EXIT

note() { printf '  %s\n' "$*"; }

fail() {
    printf '  FAILED: %s\n' "$*"
    failures=$((failures + 1))
    return 0
}

views() { "$gui" ui movo 2>/dev/null || true; }

# Reads everything first: with pipefail, a grep that stops at its first match
# would fail the pipeline by closing it on the writer.
has_text() {
    local exposed
    exposed="$(views | cut -f3)"
    grep -qF -- "$1" <<<"$exposed"
}

# Waits for text to be exposed; a failure keeps going so the screenshot and the
# remaining checks of the scenario still come out.
expect() {
    local text="$1" deadline=$((SECONDS + ${2:-10}))
    while [ "$SECONDS" -lt "$deadline" ]; do
        has_text "$text" && return 0
        sleep 0.2
    done
    fail "expected to see: $text"
}

expect_gone() {
    local text="$1" deadline=$((SECONDS + ${2:-10}))
    while [ "$SECONDS" -lt "$deadline" ]; do
        has_text "$text" || return 0
        sleep 0.2
    done
    fail "expected not to see: $text"
}

window_origin() {
    "$gui" windows | awk 'match($0, /[0-9]+x[0-9]+ at [0-9]+,[0-9]+/) {
        split(substr($0, RSTART, RLENGTH), part, " at "); print part[2]; exit }'
}

# Clicks the first exposed view with this role and exact name.
click_view() {
    local role="$1" name="$2" place origin
    place="$(views | awk -F'\t' -v role="$role" -v name="$name" \
        '$2 == role && $3 == name { print $4; exit }')"
    if [ -z "$place" ]; then
        fail "no $role named: $name"
        return 0
    fi
    origin="$(window_origin)"
    read -r x y <<<"$(awk -v place="$place" -v origin="$origin" 'BEGIN {
        split(place, part, " "); split(part[1], at, ","); split(part[2], size, "x")
        split(origin, win, ",")
        printf "%d %d", win[1] + at[1] + size[1] / 2, win[2] + at[2] + size[2] / 2 }')"
    "$gui" click "$x" "$y" >/dev/null
}

tab() { click_view "page tab" "$1"; }

# A rail's first entry, wherever the rail has been laid out.
open_first_poster() {
    "$gui" click 170 280 >/dev/null
}

shot() { "$gui" shot "$out/$1.png" >/dev/null; }

fault() { curl -fsS "$provider/__fake/faults?$1" >/dev/null; }

clear_faults() { curl -fsS "$provider/__fake/clear" >/dev/null; }

requests_to() {
    curl -fsS "$provider/__fake/requests" | grep -cF -- "$1" || true
}

reset_requests() { curl -fsS "$provider/__fake/reset-log" >/dev/null; }

start_app() {
    MOVO_TEST_PROVIDER="$provider" \
        HOME="$home_dir" \
        XDG_CONFIG_HOME="$home_dir/config" \
        XDG_DATA_HOME="$home_dir/data" \
        XDG_CACHE_HOME="$home_dir/cache" \
        PATH="$home_dir/bin:$PATH" \
        LANGUAGE="${LANGUAGE:-en}" \
        "$gui" start --resolution 1400x900 -- "$root/target/debug/movo" >/dev/null
    expect "Hot Now" 20
}

stop_app() {
    "$gui" stop >/dev/null 2>&1 || true
    grep -F "main loop stalled" "$state_dir/app.log" | sed 's/^/  /' >>"$out/stalls.log" || true
    grep -E "panicked|CRITICAL|Gtk-WARNING|GLib-GObject" "$state_dir/app.log" |
        sed 's/^/  /' >>"$out/warnings.log" || true
}

sign_in() {
    "$gui" key ctrl+comma >/dev/null
    expect "Sign In"
    click_view "button" "Sign In"
    expect "Password"
    "$gui" type "${1:-tester}"
    "$gui" key Tab
    "$gui" type "${2:-secret}"
    "$gui" key Return
}

# Signing in without a keyring raises an alert over the dialog; each needs its
# own Escape, and a key sent while the next one animates in is lost.
close_settings() {
    local deadline=$((SECONDS + 10))
    while has_text "Sign Out" && [ "$SECONDS" -lt "$deadline" ]; do
        "$gui" key Escape
        sleep 0.5
    done
}

scenario_browse() {
    tab Catalog
    expect "Забытый сад 4"
    shot catalog
    tab Search
    "$gui" type "маяк"
    expect "Зимний маяк 10"
    shot search
    tab Collections
    expect "Подборка 2"
    shot collections
    tab Home
    expect "Hot Now"
    shot home
}

scenario_home_keeps_its_place() {
    tab Catalog
    expect "Забытый сад 4"
    reset_requests
    tab Home
    expect "Hot Now"
    sleep 1
    local rails
    rails="$(requests_to "/announce")"
    [ "$rails" = "0" ] || fail "returning to Home fetched it again ($rails)"
}

scenario_details_and_dialogs() {
    open_first_poster
    expect "Описание тайтла номер 1"
    expect "Серия 1"
    shot details-series
    local metadata
    metadata="$(views | cut -f3 | grep -F "IMDb" | head -1)"
    [ "$(grep -o "IMDb" <<<"$metadata" | wc -l)" = "1" ] || fail "rating listed twice: $metadata"
    grep -qF "TV Series · " <<<"$metadata" || fail "title kind not in the app language: $metadata"
    click_view "button" "Comments"
    expect "Зритель 3"
    shot comments
    "$gui" key Escape
    expect_gone "Зритель 3"
    click_view "button" "Иван Актёр"
    expect "Filmography"
    shot actor
    "$gui" key Escape
    "$gui" key alt+Left
    expect "Hot Now"
}

# A page that was left must not keep loading: every details page still alive
# would otherwise fetch itself again each time the account changes.
scenario_closed_pages_stay_closed() {
    open_first_poster
    expect "Описание тайтла номер 1"
    "$gui" key alt+Left
    expect "Hot Now"
    sleep 0.5
    reset_requests
    sign_in tester secret
    expect "Sign Out" 15
    close_settings
    sleep 1
    local reloaded
    reloaded="$(requests_to "/series/")"
    [ "$reloaded" = "0" ] || fail "a closed page fetched itself again ($reloaded)"
}

scenario_markup_stays_text() {
    tab Search
    "$gui" type "разметка"
    expect "<b>Жирный</b>"
    shot markup-title
}

scenario_sign_in() {
    sign_in tester wrong
    expect "Неверный логин или пароль"
    shot sign-in-refused
    "$gui" key Escape
    sleep 0.5
    "$gui" key Escape
    sleep 0.5
    sign_in tester secret
    expect "Sign Out" 15
    shot signed-in
    close_settings
    tab History
    expect "Ночной дом 5"
    shot history
    tab Favorites
    expect "Смотрю"
    shot favorites
    tab Notifications
    expect "Тихий поезд 1"
    shot notifications
}

scenario_provider_down() {
    tab Catalog
    expect "Забытый сад 4"
    fault "path=/&kind=status&value=503"
    tab Home
    tab Search
    "$gui" type "тихий"
    sleep 3
    shot search-provider-down
    clear_faults
    "$gui" key ctrl+a
    "$gui" type "маяк"
    expect "Зимний маяк 10" 15
    shot search-recovered
}

scenario_slow_provider_stays_responsive() {
    fault "path=/series&kind=delay&value=4000&times=1"
    tab Catalog
    expect "Забытый сад 4"
    tab Search
    expect "Filters"
    tab Collections
    expect "Подборка 2"
    shot slow-provider
}

run_scenario() {
    local name="$1"
    [ -n "$only" ] && [ "$only" != "$name" ] && return 0
    echo "== $name"
    home_dir="$(mktemp -d)"
    mkdir -p "$home_dir/bin"
    printf '#!/bin/sh\nprintf "%%s\\n" "$@" > "%s/player-args"\n' "$home_dir" >"$home_dir/bin/mpv"
    chmod +x "$home_dir/bin/mpv"
    clear_faults
    start_app
    "scenario_$name"
    stop_app
    rm -rf "$home_dir"
}

rm -rf "$out"
mkdir -p "$out"
: >"$out/stalls.log"
: >"$out/warnings.log"

"$root/target/debug/movo-fake-provider" "127.0.0.1:$provider_port" >/dev/null &
provider_pid=$!
sleep 0.3

run_scenario browse
run_scenario home_keeps_its_place
run_scenario details_and_dialogs
run_scenario closed_pages_stay_closed
run_scenario markup_stays_text
run_scenario sign_in
run_scenario provider_down
run_scenario slow_provider_stays_responsive

if [ -s "$out/stalls.log" ]; then
    echo "main loop stalls:"
    cat "$out/stalls.log"
    failures=$((failures + 1))
fi
if [ -s "$out/warnings.log" ]; then
    echo "toolkit warnings:"
    sort -u "$out/warnings.log" | cut -c1-200
    failures=$((failures + 1))
fi

echo "screenshots in $out"
if [ "$failures" -gt 0 ]; then
    echo "$failures failed"
    exit 1
fi
echo "all passed"
