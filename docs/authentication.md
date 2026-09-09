# Authentication and Account Data Contract

## Goal

Movo uses only the official `https://hdrzk.org` provider and requires a verified account session for anything account-scoped. Authentication secrets never touch disk in plain text. Account sync is a first-class, two-sided client operation: Movo imports favorites, history, and watched state from the account and verifies every requested account mutation by reading authoritative state back. Local storage is limited to per-account playback seconds needed for resume, because the provider exposes no exact-position read contract.

## Non-negotiable rules

- Never store the account password.
- Never persist cookies or tokens in JSON, settings, logs, errors, or command lines.
- Use the platform's own secret store for a session that should survive a restart. If it is locked or unavailable, keep the login in memory for the current process only.
- Do not silently fall back to plaintext storage, another provider, local favorites, or stale account state.
- Scope every local account artifact by official-provider user ID.
- Logout clears the in-memory cookie jar, the stored session, cached profile data, and account-scoped UI state.
- Treat server responses as authoritative. Surface failures instead of reporting a successful sync.
- Express mutations as desired state, not blind toggles, and confirm convergence from the server before updating the UI.
- Reconcile remote history deletions into account-scoped local resume data.

## What is stored where

| Data | Location | Notes |
| --- | --- | --- |
| Session cookies | Secret Service keyring (Linux) | `crates/movo-core/src/client/session/credentials.rs`: one `SessionSecret` (provider, user ID, cookies with their expiry/path/domain/Secure/HttpOnly attributes) per account, under service `io.github.sachesi.Movo`, keyed `hdrzk.org:{user_id}`. |
| Session cookies | Android Keystore-encrypted file (Android) | The Rust core has no keyring access on Android (`persist_session`/`restore_session` are stubbed out there); the app exports the session as a string (`RezkaClient::export_session`) after login, encrypts and stores it itself, and hands it back through the `Restore` JNI command (`RezkaClient::import_session`) on the next launch. |
| Signed-in user ID | `settings.json` | The only account field `AppSettings` keeps; every other field is a UI preference. Used only to look up the right keyring entry on Linux. |
| Watch positions | `history-{user_id}.json` (or equivalent, one file per account) | `crates/movo-core/src/storage/history.rs`. Holds the exact playback second per title/season/episode, because the provider does not expose one. Never shared between accounts. |

No password, cookie, or token is ever written to `settings.json`, a log line, or an error message.

## Sync and verification

- Sign-in state comes from a successful authenticated profile request (`auth::check_profile`), not from the login response alone. `RezkaSession::authenticated_user_id` reads the account cookies directly rather than trusting anything cached.
- `RezkaClient::sync_history` reads the account's history and reconciles it with local resume positions (`WatchHistory::reconcile_media`), dropping local entries the account no longer lists unless they were written inside the last few minutes, which protects a resume position saved a moment before the reconciliation runs.
- Every account mutation is expressed as desired state and confirmed by reading it back rather than trusting the request's own success reply: `set_favorite`, `set_history_watched` and `mark_watched` each re-fetch the affected list after posting and only report success once the server actually reflects it.
- A session restore that reaches the provider and gets rejected clears the stored session; one that never reaches the provider (no network) leaves it alone, so a lost connection cannot sign an account out.

## Manual acceptance gate

1. Fresh install: login is required; successful login unlocks content.
2. Restart: session restores from the platform's secret store without credentials.
3. Locked or unavailable keyring: Movo keeps the session in memory for that run only, and does not read or create a plaintext secret.
4. Favorites: add and remove in multiple groups, restart, and compare with the website.
5. History: start a movie and an episode, confirm both appear under the official account, then remove and toggle watched state.
6. Episodes: partial progress shows a bar; watched episodes are labelled and dimmed.
7. Account switch: no favorites, history, progress, or watched state leaks between users.
8. Logout: account pages become inaccessible immediately, the stored session is gone, and restart requires login.

## Out of scope

- Storing or auto-filling the account password.
- Plaintext or custom-encrypted fallback credential files.
- Non-official providers or mirror selection.
- Cross-device synchronization of exact playback seconds beyond what the official service supports.
- Feature parity with other HDRezka clients.
