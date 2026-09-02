# Movo

Movo is an unofficial client for the HDRezka streaming site, written in Rust. One
provider engine (`movo-core`) is shared by two front ends: a GTK 4 / Libadwaita
desktop application for Linux, and a Jetpack Compose application for Android
phones, tablets and TV.

The project is not affiliated with HDRezka. It talks to the public site the same
way a browser does, and it needs a real account for anything that touches
favorites, history or premium streams.

## Status

Version 0.1.0, usable but young. The desktop client was rewritten on relm4 and
the Android client is the newer of the two. Expect the provider layer to need
maintenance whenever the site changes its markup or its stream encoding.

## What it does

- Browse the catalog, collections and genre listings, with search, filters and
  search suggestions.
- Open a title for its description, ratings, cast, comments, translator list and
  episode list.
- Sign in, and read and write favorites, watch history and episode notifications
  through the account rather than a local copy.
- Play a stream at a chosen quality with subtitles: through mpv or another
  desktop player on Linux, through Media3 on Android.
- Resume where playback stopped. Resume positions are stored locally per
  account; everything else is read back from the account.
- Follow the system language. Russian and Ukrainian translations are included.

There is no download subsystem, and none is planned.

## Requirements

Linux desktop:

- Rust (2021 edition toolchain)
- GTK 4.16 or newer and Libadwaita 1.6 or newer, with development headers
- gettext
- mpv, or any player that can open a URL, for playback

Android:

- JDK 17 or 21
- Android SDK 36 and Build Tools 36
- Android NDK 29.0.14206865
- `cargo-ndk`, and the `aarch64-linux-android`, `armv7-linux-androideabi` and
  `x86_64-linux-android` Rust targets

Android 6.0 (API 23) is the minimum supported release.

## Build and run

The `Justfile` holds every build recipe; `just` on its own lists them.

Desktop:

```sh
just run          # compile the translation catalogs, then run the client
just check        # cargo fmt, clippy and the test suite
```

Android:

```sh
export ANDROID_HOME=/path/to/Android/Sdk
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"

just apk          # debug APK at android/app/build/outputs/apk/debug/app-debug.apk
just apk-check    # unit tests, lint, and both debug APKs
just apk-install  # build and install on a connected device
```

`just apk-release` produces a signed release APK and expects `MOVO_KEYSTORE`,
`MOVO_KEY_ALIAS` and `MOVO_KEYSTORE_PASSWORD` in the environment.

## Layout

```
crates/movo-core       provider engine: session, auth, catalog, streams, storage
crates/movo-desktop    GTK 4 / Libadwaita client (relm4)
crates/movo-android    JNI bridge exposing movo-core to the Android app
android/               Jetpack Compose application
docs/authentication.md account, session and stored-data contract
po/, locale/           translation sources and compiled catalogs
```

## Accounts and stored data

Session material is kept in the platform's own store: the Secret Service keyring
on Linux, an Android Keystore-encrypted file on Android. Passwords are never
written to disk. The only account data Movo keeps locally is the playback second
needed to resume a title, because the site exposes no way to read an exact
position back. Favorites, history and watched state live in the account, and
every write is verified by reading the authoritative state back.

`docs/authentication.md` describes the contract in full.

## Limitations

- One provider only (`hdrzk.org`), and the site's markup is not a stable API.
- Tests that talk to the live site are marked `#[ignore]`; run them with
  `cargo test --workspace -- --ignored` when you want to check the provider
  against reality.
- Desktop playback shells out to an external player; there is no in-window video
  surface.
- The desktop client targets GNOME. It runs elsewhere, but the styling assumes
  Libadwaita.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
