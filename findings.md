# Android client audit

Scope: `android/` only, phone and television modes. No new features. Every finding below is
either fixed in this pass (marked **fixed**) or deliberately left alone with the reason
(marked **deferred**). Line numbers refer to the tree before the fixes.

Baseline before any change: `./gradlew testDebugUnitTest lintDebug` green, 42 unit tests,
lint 0 errors / 13 warnings / 1 hint.

## A. Correctness

- **A1 fixed** `core/MovoViewModel.kt:379` `openDetails` cancels the content job. A deep
  link that lands while the television home is still loading cancels that load; on return
  the home shows the skeleton for ever, because nothing re-requests it and no error is set.
  On a phone the same path shows "No titles in this category" for a catalog that was never
  fetched. The cancel buys nothing: the blocking JNI call cannot be interrupted, only its
  result is dropped, and every content load already checks that its tab is still current
  before applying. Removed the cancel.
- **A2 fixed** `core/MovoViewModel.kt:161` `selectTab` clears `items` only for Search, so
  switching Catalog → Favorites shows the catalog's posters under the favorites chips until
  the favorites arrive, and the other way round. Cleared `items` and `page` for every tab.
- **A3 fixed** duplicate lazy-list keys crash. Grids and rows key by URL, and the provider
  gives no uniqueness guarantee: pages of a listing overlap when the ordering shifts between
  requests, and `details/DetailsScreen.kt:450` concatenates directors and actors keyed by
  `url:name`, so a title whose director also acts in it crashes the details page. Paged
  results are now merged with `distinctBy`, home sections and suggestions are de-duplicated
  on arrival, and the concatenated rows on the details page are de-duplicated once per
  title.
- **A4 fixed** `home/HomeShell.kt:529` the navigation rail lists seven tabs plus Settings in
  a fixed column. At compact height (a phone in landscape uses the rail because its width
  is not compact) that is ~490dp of items in ~300dp of window, so the lower items,
  Settings included, are clipped and unreachable. The rail now scrolls.
- **A5 fixed** `player/PlayerScreen.kt:235` the player requests no audio focus and does not
  handle audio becoming noisy. Playback keeps running over a phone call or another app's
  audio and keeps going when headphones are unplugged. Set media audio attributes with
  focus handling and `setHandleAudioBecomingNoisy(true)`.
- **A6 fixed** `AndroidManifest.xml` the activity declares no `configChanges`, and the player
  forces landscape. Opening the player from a portrait phone therefore recreates the whole
  activity, builds the ExoPlayer and MediaSession, tears them down, and builds them again;
  closing it does the same on the way back. Declared
  `orientation|screenSize|screenLayout|smallestScreenSize|keyboardHidden`; Compose already
  relayouts from `LocalConfiguration`, and the window size class and fold tracker follow it.
- **A7 fixed** `core/MovoViewModel.kt:120,146,621,694` three user-facing messages are
  English literals ("Enter login and password", "Operation failed", "Selected quality is
  unavailable") in an app that ships Russian and Ukrainian. Moved to string resources with
  translations.

## B. Performance

- **B1 fixed** `MainActivity.kt:157` `context.settings` is a getter that builds a new `Flow`
  on every read, and `collectAsStateWithLifecycle` keys its producer on the flow instance,
  so every recomposition of the root composable restarted the DataStore collection. The
  flow is now remembered per context.
- **B2 deferred** `MainActivity.kt:159,189` the dynamic colour scheme is rebuilt and the
  `UiModeManager` looked up on every recomposition of the root. The root recomposes only on
  a screen, settings, deep-link or fold change, and a dynamic scheme is thirty colour
  reads; remembering it would also risk a stale palette after a wallpaper change now that
  the activity is no longer recreated for every configuration change (A6). Left as is.
- **B3 deferred** `home/HomeShell.kt:191` `toTvColorScheme` allocates a television scheme on
  every recomposition of the television shell. It cannot simply be remembered: `MaterialTheme`
  hands out one `ColorScheme` instance and mutates it in place on a theme switch, so a
  `remember` keyed on it would never refresh. The cost is one small object per
  recomposition; left as is.
- **B4 fixed** `core/MovoViewModel.kt:358` every appended favorites page re-fetched the
  favorite categories. Only the first page does now.
- **B5 fixed** `core/MovoViewModel.kt:271` `loadSearchFilters` ran through the operation
  wrapper even when the filters were cached, so every visit to Search blipped the global
  progress bar and cleared the error slot. Returns before the wrapper when cached.
- **B6 fixed** `core/NativeBridge.kt:168,186` keystore key resolution and AES-GCM run on the
  main thread during restore and sign-in. Moved onto `Dispatchers.IO`.
- **B7 fixed** `catalog/CatalogScreens.kt:378` boxed `mutableStateOf(-1)` (lint hint), now
  `mutableIntStateOf`.

## C. Polish and consistency

- **C1 fixed** `MainActivity.kt:291`, `account/AccountScreens.kt:119` the sign-in screen
  shows "Movo" twice: the splash frame draws the name and tagline, and the card inside it
  draws the name again. Dropped the card's headline.
- **C2 fixed** `search/SearchScreens.kt:148,155` submitting a search on a phone leaves the
  keyboard covering the results. The keyboard is hidden on submit.
- **C3 fixed** commit d2e0908 says avatars, collection tiles, actor photos and comment
  avatars got the placeholder tile the media cards use, but its diff only configured the
  crossfade. Those images still drew nothing while loading and nothing on failure, and the
  history and details posters paint a background fill under the loaded image instead,
  which the media card comment explains is wasted fill on a television. All of them now
  share one remembered placeholder painter.
- **C4 fixed** `details/DetailsScreen.kt:565` the rating dialog on television claims no
  initial focus, unlike every other television dialog, so the first remote press can land
  nowhere. Cancel takes focus on arrival, matching the sign-out and history dialogs.
- **C5 fixed** `search/SearchScreens.kt:133-196` the phone branch of the search screen still
  carries `isTv` conditionals (`tvFocusScale(isTv)`, a television width) that can never be
  true because the television branch returns above it. Removed.
- **C6 fixed** `settings/SettingsScreen.kt:344` the tab-to-label mapping is duplicated from
  `home/HomeShell.kt:618`. One mapping, shared.
- **C7 fixed** broken indentation in `core/MovoViewModel.kt` (`loadCatalog`, `loadFavorites`,
  `loadHistory`) and `catalog/CatalogScreens.kt` (`CollectionCardContent`,
  `MediaCardContent`), plus redundant same-package imports in `SettingsScreen.kt`.
- **C8 fixed** `app/build.gradle.kts:64` `kotlinOptions` is deprecated on Kotlin 2.x and
  removed in 2.3. Replaced with `kotlin { compilerOptions { jvmTarget } }`.

## E. Second polish pass

- **E1 fixed** `details/DetailsScreen.kt` the error banner was the last item of the page and
  there was no progress indicator, so a failed favorite toggle or a slow rating went
  unreported on any page longer than the screen. Both now overlay the page as they do on
  the home shell; the banner's Retry reloads the title.
- **E2 fixed** `details/DetailsScreen.kt` the description's "More" button was shown by
  character count while the text was cut by line count, so a short description of many
  lines was truncated with no way to open it and a long one of few lines had a button that
  did nothing. The button now follows the measured overflow.
- **E3 fixed** `catalog/CatalogScreens.kt` the media card ignored the rating the provider
  already sends. It joins the year and category line.
- **E4 fixed** `player/PlayerScreen.kt` on a phone only remote keys counted as interaction,
  so a seek drag longer than the timeout had the overlay vanish under the finger. Every
  touch now delays the auto-hide.
- **E5 fixed** `player/PlayerScreen.kt` on a television Back left the player outright even
  with the overlay up. It now takes the overlay down first, as the players around it do,
  and leaves on the next press; a paused player, which keeps its overlay, leaves at once.
- **E6 fixed** `catalog/CatalogScreens.kt` the collections grid paged with a "Load more"
  button while every other grid pages on scroll. The scroll trigger is shared and the
  button is gone.
- **E7 fixed** `details/DetailsScreen.kt` Share was offered on a television, where the
  chooser opens on "no apps can perform this action". Hidden there.
- **E8 fixed** `ui/Skeletons.kt` every empty state showed the same inbox glyph. Each screen
  now shows its own (search, search-off, favorites, history, updates, collections,
  catalog, and cloud-off for a home that did not load).
- **E9 fixed** `MainActivity.kt` the trailer's back button sat under the status bar (edge
  to edge, no insets), and the embed was a black screen until its page loaded. Insets and
  a spinner.
- **E10 fixed** `account/AccountScreens.kt` the sign-in fields carry no autofill content
  types, so password managers did not offer to fill them. Username and password types set.
- **E11 fixed** `ui/MovoWidgets.kt` the choice rows in the playback sheet and the search
  filters were plain chips to a screen reader; they are radio buttons in a group now, as
  the settings chips already were.
- **E12 fixed** `home/HomeShell.kt` the "More" sheet on the phone did not mark which of
  its destinations was the current one; a tick does. The television rail's collapsed
  items name themselves to a screen reader; before, they were unlabeled icons until the
  rail opened.

## F. Third pass, on request

Five items held back from the earlier passes as feature-adjacent, then asked for.

- **F1** `player/PlayerScreen.kt` double tap on a phone seeks by the configured interval,
  backwards on the left half and forwards on the right; a single tap still toggles the
  overlay. The single tap now waits out the double-tap window, as every player does.
- **F2** `home/HomeShell.kt` pull to refresh on the phone's home tabs re-runs the visible
  tab's load through the same path the error banner's Retry uses. The indicator follows a
  flag set by the pull, not the global loading bit, so ordinary loads do not show it.
- **F3** `details/DetailsScreen.kt`, `player/PlayerScreen.kt` translator chips carry the
  flags the provider sends (premium, camera rip, ads, director's cut) and quality chips
  and the player's quality menu mark premium streams. New strings in en/ru/uk.
- **F4** `home/HomeShell.kt`, `details/DetailsScreen.kt` the phone's top bars slide away as
  the content scrolls and return on the first scroll back. Not on a television, which has
  no scroll gesture. The home bar's connection is installed only while the tabs are up,
  so a bar collapsed under Settings does not come back hidden.
- **F5** `player/PlayerScreen.kt` a sync error (a failed history write) fades after six
  seconds instead of sitting over the film until the player closes; a playback error still
  stays until the player recovers.

## D. Deferred

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

## Verification

After the fixes: `./gradlew testDebugUnitTest lintDebug` (results recorded in the final
report), and the instrumentation suite `TvNavigationTest` needs a television emulator or
device, which this environment does not have.
