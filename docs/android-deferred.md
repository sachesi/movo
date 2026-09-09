# Deferred Android findings

Carried over from the Android client audit (`android/` only, phone and television modes).
Each item below was deliberately left alone during that audit, with the reason recorded at
the time. Line numbers refer to the tree as it stood during that pass.

- **B2** `MainActivity.kt:159,189` the dynamic colour scheme is rebuilt and the
  `UiModeManager` looked up on every recomposition of the root. The root recomposes only on
  a screen, settings, deep-link or fold change, and a dynamic scheme is thirty colour
  reads; remembering it would also risk a stale palette after a wallpaper change now that
  the activity is no longer recreated for every configuration change (A6). Left as is.
- **B3** `home/HomeShell.kt:191` `toTvColorScheme` allocates a television scheme on
  every recomposition of the television shell. It cannot simply be remembered: `MaterialTheme`
  hands out one `ColorScheme` instance and mutates it in place on a theme switch, so a
  `remember` keyed on it would never refresh. The cost is one small object per
  recomposition; left as is.
- **D1** Dependency versions behind the current stable (lint lists activity-compose,
  lifecycle, datastore, window, coroutines, serialization, coil, robolectric, Gradle).
  Not bumped here: each needs its changelog read and the APK re-tested on a device, which
  is a separate change from an audit.
- **D2** `AppLinkWarning` on the `https://hdrzk.org` filter is deliberate and documented in
  the manifest: a third-party client cannot publish the `assetlinks.json` verification
  needs.
- **D3** `account/AccountScreens.kt:492` the favorites chip row routes D-pad Down to the
  grid's focus requester. When the grid is empty or still a skeleton nothing is attached to
  that requester. Whether the focus system tolerates that or throws depends on the Compose
  version; verify on a television with an empty favorites list before touching it.
- **D4** `ui/Skeletons.kt:126` the grid skeleton pulses on television while the home
  skeleton is static. The pulse invalidates draw only, and only while a load is in flight.
- **D5** Every account mutation on the details page (favorite, rating, schedule) re-fetches
  the details and the favorite categories. That is the documented contract in
  `docs/authentication.md`: writes are verified by reading the authoritative state back.
- **D6** The catalog is re-fetched on every return to its tab because `items` is one slot
  shared by Catalog, Search, Favorites and collection paths. Per-tab caching means
  splitting that slot, which is a larger refactor than this pass.
- **D7** `material-icons-extended` is large in debug builds. Release builds are shrunk by
  R8, which keeps only the referenced icons.
- **D8** One global `loading`/`error` slot serves every operation, so a failed background
  write can banner over an unrelated screen. Architectural; out of scope.
- **D9** `hasAdjacentEpisode` scans every season on each call. Its only caller memoises the
  pair per stream, so nothing to gain in the view model.
- **D10** `player/PlayerScreen.kt:625,746` the two overlay gradients are rebuilt on each
  recomposition of the overlay (four times a second while visible). Two small allocations;
  not worth hoisting.
