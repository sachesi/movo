# Authentication and Account Data Plan

## Goal

Movo uses only the official `https://hdrzk.org` provider and requires a verified account session. Authentication secrets live only in the system keyring. Account sync is a first-class, two-sided client operation: Movo imports favorites, history, and watched state from the account and verifies every requested account mutation by reading authoritative state back. Local storage is limited to per-account playback seconds needed for resume because the provider exposes no exact-position read contract.

## Non-negotiable rules

- Never store the account password.
- Never persist cookies or tokens in JSON, settings, logs, errors, or command lines.
- Use the system keyring for persistent session cookies. If it is locked or unavailable, keep the login in memory for the current process only.
- Do not silently fall back to plaintext storage, another provider, local favorites, or stale account state.
- Scope every local account artifact by official-provider user ID.
- Logout must clear the in-memory cookie jar, the keyring entry, cached profile data, and account-scoped UI state.
- Treat server responses as authoritative. Surface failures instead of reporting a successful sync.
- Express mutations as desired state, not blind toggles, and confirm convergence from the server before updating the UI.
- Reconcile remote history deletions into account-scoped local resume data.

## Current defects

- Login accepts a successful JSON response without proving that subsequent authenticated requests work.
- Profile restoration relies on fragile homepage selectors and can substitute the typed login for a failed profile fetch.
- Cookies are written to `session_cookies.json` with lost expiry/security attributes; the current file can be world-readable.
- Logout deletes only the persisted file and leaves the live cookie jar populated.
- Favorites are toggled locally first, use hardcoded category `1`, ignore server failures, and can show local items when the account is empty.
- Server history endpoints are unused; Movo stores only local MPV progress.
- Episode models contain no server watched state, watched label, or dimmed styling.
- History and favorites files are global, so state can leak between accounts.
- History and episode rows do not refresh after playback changes state.
- Tests cover synthetic local structures, not the official account contracts.

## Implementation plan

### 1. Make authentication state truthful

- Define signed-in state from the official authentication cookies and a successful authenticated profile request, not from the login response alone.
- Read the user ID from the authenticated cookie/session contract and load `/user/{id}`.
- Remove the username fallback when profile verification fails.
- Reject login when required cookies are absent, expired, or fail the profile request.
- Preserve HTTP status and content type in account-operation errors; include only a short sanitized body description.
- Keep `AppSettings` profile fields display-only or remove them. They must never authorize a session.

Verify:

- Valid credentials produce an authenticated profile and account-only page access.
- Invalid credentials, HTML responses, missing cookies, and expired sessions remain signed out.
- Restart restoration cannot succeed from a saved username alone.

### 2. Move persistence to the system keyring

- Add the smallest maintained Rust integration for the platform keyring/Secret Service after confirming its current API.
- Store one serialized official-session secret under service `org.gnome.Movo`, keyed by provider and user ID.
- Store cookie names, values, expiry, path, domain, Secure, and HttpOnly metadata needed to reconstruct the jar safely.
- Keep the active non-secret user ID in normal settings only if needed to locate the keyring entry.
- On keyring write failure, keep the verified session in memory and tell the user that login will not survive restart.
- On keyring read failure or locked keyring, remain signed out or session-only; never read a plaintext fallback.

Legacy migration:

1. Read `session_cookies.json` once without logging its contents.
2. Validate the reconstructed session against the official profile endpoint.
3. Write it to the keyring when valid.
4. Remove the plaintext file only after the secret is in memory and the migration attempt has completed. If keyring storage failed, continue session-only and require login after restart.
5. Never recreate the plaintext file.

Verify:

- No password, cookie, or token appears under Movo config/data directories after login.
- Restart restores the account when the keyring is available.
- Locked/unavailable keyring degrades to session-only behavior without plaintext files.
- Migration preserves a valid session and removes the legacy file.

### 3. Fix session lifecycle and logout

- Restore keyring cookies before creating account-dependent views.
- Revalidate the session on startup and clear rejected keyring entries.
- Replace or clear the live cookie jar on logout so already-created clients cannot remain authenticated.
- Delete the corresponding keyring entry and clear cached username/user ID.
- Cancel or invalidate in-flight account requests when account generation changes.
- Disable account-dependent UI immediately on logout.

Verify:

- After logout, the same process cannot access `/favorites/`, `/continue/`, or profile pages.
- Restart after logout requires login.
- A request started for account A cannot update UI after switching to account B.

### 4. Make favorites server-authoritative

- Parse bookmark categories from `.b-favorites_content__cats_list_item[data-cat_id]`.
- Parse each title's `.hd-label-row` membership and expose category IDs in `MediaDetails`.
- Let the user select bookmark groups; do not hardcode category `1`.
- Use the official category-specific toggle contract for both add and remove.
- Update the star/category UI only after server success, or roll it back with a visible error.
- Load all requested pages and represent an empty server list as empty. Remove `LocalFavorites` as a signed-in fallback.
- Refresh details and favorites views after a successful mutation.

Verify:

- Multiple bookmark groups display their real counts and contents.
- Add/remove survives restart and matches the website.
- Server errors never leave the local star different from the account.
- An empty account never displays old local favorites.

### 5. Separate server watch state from local resume progress

- Implement the official `/ajax/send_save` request when playback starts or the selected episode changes.
- Load `/continue/` as account history and parse `watched-row` state.
- Implement server remove and watched/unwatched toggles through the official endpoints.
- Parse schedule watched IDs/classes where available and carry `is_watched` through the episode model.
- Keep exact playback seconds locally because the official save contract does not provide full resume-position sync.
- Store local resume data per user ID and media/season/episode key.
- Mark playback completion immediately in local UI and send the corresponding server update.

Verify:

- Starting a movie or episode creates/updates the official account history.
- Server history and watched state survive restart and match the website.
- MPV resume position remains exact for the current account.
- Account A never sees account B's local resume positions.

### 6. Render and refresh watched state

- Show a clear watched label and dim completed/watched episode rows.
- Keep partial local progress as a progress bar without calling it watched.
- Refresh history and episode rows after playback events, account sync, login, logout, and relevant view activation.
- Do not rely on reconstructing the entire window to refresh account state.

Verify:

- Partial episode: normal row plus progress bar.
- Watched episode: watched label plus dimmed row.
- Returning from MPV updates the existing details and history views immediately.
- Logout removes account-only watched state from the visible UI.

### 7. Add focused regression coverage

- Add sanitized fixtures for official login, profile, favorites, continue-history, schedule, and expired-session responses.
- Test request payloads and cookies with a small standard-library local HTTP server; do not add a test framework solely for this.
- Test malformed/HTML account responses and ensure they cannot produce a signed-in state.
- Test keyring serialization separately from the real desktop keyring. Use the dependency's test backend if available; otherwise keep the adapter seam minimal.
- Test logout against the same client instance.
- Test multi-account separation, favorite rollback, watched rendering inputs, and history refresh events.

Verify:

- `cargo test -p movo-core --lib --test auth_test --test storage_test`
- Relevant desktop unit tests for account-state rendering and refresh.
- `cargo check --all`
- `cargo fmt --all -- --check`
- `git diff --check`

## Manual acceptance gate

1. Fresh install: login is required; successful login unlocks content.
2. Restart: session restores from the unlocked system keyring without credentials.
3. Locked keyring: Movo does not read or create plaintext secrets and explains that persistence is unavailable.
4. Favorites: add and remove in multiple groups, restart, and compare with the website.
5. History: start a movie and an episode, confirm both appear under the official account, then remove and toggle watched state.
6. Episodes: partial progress shows a bar; watched episodes are labelled and dimmed.
7. Account switch: no favorites, history, progress, or watched state leaks between users.
8. Logout: account pages become inaccessible immediately, the keyring entry is gone, and restart requires login.

## Stop condition

Done means every manual acceptance item passes and the focused automated checks are green. Do not claim completion from parser tests alone or from a login dialog that merely accepted credentials.

## Out of scope

- Storing or auto-filling the account password.
- Plaintext or custom-encrypted fallback credential files.
- Non-official providers or mirror selection.
- Cross-device synchronization of exact playback seconds beyond what the official service supports.
- Feature parity with other HDRezka clients.
