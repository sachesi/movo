use jni::{
    objects::{JClass, JString},
    sys::{jlong, jstring},
    JNIEnv,
};
use movo_core::client::{
    models::{self, CatalogCategory, Translator},
    RezkaClient,
};
use movo_core::error::{ClientError, ErrorKind};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, RwLock};

static CLIENT: LazyLock<RwLock<RezkaClient>> = LazyLock::new(|| RwLock::new(RezkaClient::new()));

/// Serializes account mutations against each other without shutting readers
/// out. The mutations only need `&RezkaClient`; taking the write lock for them
/// blocked every catalog, search and details request for as long as the
/// mutation ran, and some of them poll the provider for several seconds.
static MUTATIONS: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));
static RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().expect("Android runtime"));

/// Reads the app is waiting on, by the id it sent each one under, so it can
/// drop one it stopped waiting for instead of leaving it to run for nobody:
/// a dropped read gives back its place under the provider's request limit and
/// its hold on the client, which a sign-out waits for.
static READS: LazyLock<Mutex<HashMap<u64, Read>>> = LazyLock::new(Default::default);

/// How long a drop is kept for a read that has not started yet. The app only
/// asks for one once the read is on its way, so the read turns up within
/// moments; a drop still waiting after this was for a read that had already
/// answered.
const EARLY_DROP_KEPT: Duration = Duration::from_secs(60);

struct Read {
    dropped: Arc<Notify>,
    /// When the drop arrived, for a read that had not started yet.
    early_since: Option<Instant>,
}

fn reads() -> MutexGuard<'static, HashMap<u64, Read>> {
    let mut reads = READS.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    reads.retain(|_, read| {
        read.early_since
            .is_none_or(|since| now.duration_since(since) < EARLY_DROP_KEPT)
    });
    reads
}

/// What the read sent under `id` waits on besides its reply. A drop that
/// arrived before the read has already been signalled.
fn track(id: u64) -> Arc<Notify> {
    let mut reads = reads();
    let read = reads.entry(id).or_insert_with(|| Read {
        dropped: Arc::default(),
        early_since: None,
    });
    read.early_since = None;
    read.dropped.clone()
}

fn drop_read(id: u64) {
    reads()
        .entry(id)
        .or_insert_with(|| Read {
            dropped: Arc::default(),
            early_since: Some(Instant::now()),
        })
        .dropped
        .notify_one();
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Command {
    Login {
        login: String,
        password: String,
    },
    Restore {
        secret: String,
    },
    Logout,
    Catalog {
        category: CatalogCategory,
        filter: Option<String>,
        page: usize,
    },
    Search {
        query: String,
        page: usize,
    },
    SearchSuggestions {
        query: String,
    },
    Home,
    SearchFilters,
    Collections {
        page: usize,
    },
    Path {
        path: String,
        page: usize,
    },
    Details {
        url: String,
    },
    Actor {
        url: String,
    },
    Comments {
        post_id: i64,
        page: usize,
    },
    Trailer {
        post_id: i64,
    },
    Rate {
        post_id: i64,
        rating: u8,
    },
    LikeComment {
        id: String,
    },
    AccountData,
    ToggleScheduleWatched {
        id: String,
    },
    Episodes {
        post_id: i64,
        translator_id: i64,
        #[serde(default)]
        schedules: Vec<models::ScheduleGroup>,
    },
    MovieStream {
        post_id: i64,
        translator: Translator,
    },
    EpisodeStream {
        post_id: i64,
        translator_id: i64,
        season: i64,
        episode: i64,
    },
    FavoriteCategories,
    Favorites {
        category_id: Option<i64>,
        page: usize,
    },
    SetFavorite {
        url: String,
        post_id: i64,
        category_id: i64,
        favorite: bool,
    },
    History,
    RemoveHistory {
        id: String,
    },
    SetHistoryWatched {
        id: String,
        watched: bool,
    },
    SaveWatch {
        post_id: i64,
        translator_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    },
    SetHiddenCountries {
        countries: String,
    },
    Countries,
    MarkWatched {
        url: String,
        post_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    },
}

impl Command {
    /// Whether the command only reads, which is all the app may drop. A change
    /// runs to the end once sent, even when the app stops waiting for it: the
    /// user asked for it, and one cut off half way leaves nobody knowing
    /// whether it happened.
    fn is_read(&self) -> bool {
        matches!(
            self,
            Command::Catalog { .. }
                | Command::Search { .. }
                | Command::SearchSuggestions { .. }
                | Command::Home
                | Command::SearchFilters
                | Command::Collections { .. }
                | Command::Path { .. }
                | Command::Details { .. }
                | Command::Actor { .. }
                | Command::Comments { .. }
                | Command::Trailer { .. }
                | Command::AccountData
                | Command::Episodes { .. }
                | Command::MovieStream { .. }
                | Command::EpisodeStream { .. }
                | Command::FavoriteCategories
                | Command::Favorites { .. }
                | Command::History
                | Command::Countries
        )
    }
}

/// A failed command, and whether it leaves the stored session worth keeping.
struct Failure {
    message: String,
    kind: ErrorKind,
    /// Set only by `Restore`: the provider turned the stored session down, so
    /// the app should forget it instead of trying again later.
    session_rejected: bool,
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        let kind = movo_core::error::classify(&message);
        Self {
            message,
            kind,
            session_rejected: false,
        }
    }
}

impl From<ClientError> for Failure {
    fn from(error: ClientError) -> Self {
        Self {
            message: error.message,
            kind: error.kind,
            session_rejected: false,
        }
    }
}

async fn invoke_read(command: Command, client: &RezkaClient) -> Result<Value, Failure> {
    match command {
        Command::Login { .. } | Command::Restore { .. } | Command::Logout => Err(Failure::from(
            "Account commands are handled before dispatch".to_string(),
        )),
        Command::Catalog {
            category,
            filter,
            page,
        } => Ok(json!(
            client
                .fetch_catalog(category, filter.as_deref(), page)
                .await?
        )),
        Command::SetHiddenCountries { countries } => {
            client.set_hidden_countries(models::parse_country_list(&countries));
            Ok(Value::Null)
        }
        Command::Countries => Ok(json!(movo_core::client::countries::entries())),
        Command::Search { query, page } => Ok(json!(client.search_full(&query, page).await?)),
        Command::SearchSuggestions { query } => Ok(json!(client.search_suggestions(&query).await?)),
        Command::Home => Ok(json!(client.home().await?)),
        Command::SearchFilters => Ok(json!(client.search_filters().await?)),
        Command::Collections { page } => Ok(json!(client.fetch_collections(page).await?)),
        Command::Path { path, page } => Ok(json!(client.fetch_path(&path, page).await?)),
        Command::Details { url } => Ok(json!(client.fetch_details(&url).await?)),
        Command::Actor { url } => Ok(json!(client.fetch_actor(&url).await?)),
        Command::Comments { post_id, page } => {
            Ok(json!(client.fetch_comments(post_id, page).await?))
        }
        Command::Trailer { post_id } => Ok(json!(client.fetch_trailer(post_id).await?)),
        Command::Rate { post_id, rating } => {
            client.post_rating(post_id, rating).await?;
            Ok(Value::Null)
        }
        Command::LikeComment { id } => {
            client.like_comment(&id).await?;
            Ok(Value::Null)
        }
        Command::AccountData => Ok(json!(client.account_data().await?)),
        Command::ToggleScheduleWatched { id } => {
            client.toggle_schedule_watched(&id).await?;
            Ok(Value::Null)
        }
        Command::Episodes {
            post_id,
            translator_id,
            schedules,
        } => Ok(json!(
            client
                .fetch_episodes(post_id, translator_id, &schedules)
                .await?
        )),
        Command::MovieStream {
            post_id,
            translator,
        } => Ok(json!(
            client.fetch_movie_stream(post_id, &translator).await?
        )),
        Command::EpisodeStream {
            post_id,
            translator_id,
            season,
            episode,
        } => Ok(json!(
            client
                .fetch_episode_stream(post_id, translator_id, season, episode)
                .await?
        )),
        Command::FavoriteCategories => Ok(json!(client.fetch_favorites_categories().await?)),
        Command::Favorites { category_id, page } => {
            Ok(json!(client.fetch_favorites_page(category_id, page).await?))
        }
        Command::SetFavorite {
            url,
            post_id,
            category_id,
            favorite,
        } => {
            client
                .set_favorite(&url, post_id, category_id, favorite)
                .await?;
            Ok(Value::Null)
        }
        Command::History => Ok(json!(client.sync_history().await?)),
        Command::RemoveHistory { id } => {
            Ok(json!({"media_id": client.remove_history_with_media(&id).await?}))
        }
        Command::SetHistoryWatched { id, watched } => {
            client.set_history_watched(&id, watched).await?;
            Ok(Value::Null)
        }
        Command::SaveWatch {
            post_id,
            translator_id,
            season,
            episode,
        } => {
            client
                .save_watch(post_id, translator_id, season, episode)
                .await?;
            Ok(Value::Null)
        }
        Command::MarkWatched {
            url,
            post_id,
            season,
            episode,
        } => {
            client.mark_watched(&url, post_id, season, episode).await?;
            Ok(Value::Null)
        }
    }
}

/// Runs a command to its reply. A read sent under a request id ends early,
/// with an error nobody is waiting for, once the app drops it.
fn invoke(command: Command, request_id: Option<u64>) -> Result<Value, Failure> {
    let droppable = request_id.filter(|_| command.is_read());
    wait(run(command), droppable)
}

/// Waits for `work` to finish or, when it runs under `id`, for the app to
/// drop it, whichever comes first.
fn wait(
    work: impl Future<Output = Result<Value, Failure>>,
    id: Option<u64>,
) -> Result<Value, Failure> {
    let Some(id) = id else {
        return RUNTIME.block_on(work);
    };
    let dropped = track(id);
    let outcome = RUNTIME.block_on(async {
        tokio::select! {
            outcome = work => outcome,
            () = dropped.notified() => Err(Failure::from("The request was dropped".to_string())),
        }
    });
    reads().remove(&id);
    outcome
}

async fn run(command: Command) -> Result<Value, Failure> {
    match command {
        Command::Login { login, password } => {
            let client = CLIENT.write().await;
            client.check_stream_hosts();
            let user = client.login(&login, &password).await?;
            Ok(json!({"user": user, "secret": client.export_session()?}))
        }
        Command::Restore { secret } => {
            let client = CLIENT.write().await;
            client.check_stream_hosts();
            // The reply carries whether the session was turned down, so the
            // app only throws the stored secret away when it is worthless.
            client
                .import_session(&secret)
                .await
                .map(|user| json!(user))
                .map_err(|error| Failure {
                    message: error.error.message,
                    kind: error.error.kind,
                    session_rejected: error.is_rejected,
                })
        }
        Command::Logout => {
            CLIENT.write().await.logout().await?;
            Ok(Value::Null)
        }
        command @ (Command::SetFavorite { .. }
        | Command::RemoveHistory { .. }
        | Command::SetHistoryWatched { .. }
        | Command::Rate { .. }
        | Command::LikeComment { .. }
        | Command::ToggleScheduleWatched { .. }
        | Command::SaveWatch { .. }
        | Command::MarkWatched { .. }) => {
            let _mutation = MUTATIONS.lock().await;
            let client = CLIENT.read().await;
            invoke_read(command, &client).await
        }
        command => {
            let client = CLIENT.read().await;
            invoke_read(command, &client).await
        }
    }
}

/// Turns a panic into the same error reply a failed command produces.
///
/// A panic left to unwind out of an `extern "system"` function aborts the
/// process, so one malformed page from the provider would take the app down
/// instead of failing the request that asked for it.
///
/// This relies on the default `panic = "unwind"`. A release profile added to
/// the workspace that sets `panic = "abort"` turns every failure this
/// function catches into a process abort instead, so any such profile must
/// leave unwinding on for this crate.
fn caught(run: impl FnOnce() -> String) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
        .unwrap_or_else(|_| json!({"error": "The request stopped unexpectedly"}).to_string())
}

/// Reads a request: the command, and the id the app may later drop it under.
/// The id has a key of its own because several commands name a field `id`.
fn parse(request: &str) -> Result<(Command, Option<u64>), Failure> {
    let request: Value =
        serde_json::from_str(request).map_err(|error| Failure::from(error.to_string()))?;
    let request_id = request.get("request_id").and_then(Value::as_u64);
    let command =
        serde_json::from_value(request).map_err(|error| Failure::from(error.to_string()))?;
    Ok((command, request_id))
}

/// Drops the read the app sent under `id`: it answers at once, and whatever
/// it was waiting on is let go. A change, or a read that already answered, is
/// left alone.
#[no_mangle]
pub extern "system" fn Java_org_movo_app_core_NativeBridge_drop(
    _env: JNIEnv,
    _class: JClass,
    id: jlong,
) {
    // Nothing may unwind out of an `extern "system"` function; see `caught`.
    let _ = std::panic::catch_unwind(|| drop_read(id as u64));
}

#[no_mangle]
pub extern "system" fn Java_org_movo_app_core_NativeBridge_invoke(
    mut env: JNIEnv,
    _class: JClass,
    request: JString,
) -> jstring {
    let response = caught(|| {
        env.get_string(&request)
            .map_err(|error| Failure::from(error.to_string()))
            .and_then(|request| parse(&request.to_string_lossy()))
            .and_then(|(command, request_id)| invoke(command, request_id))
            .map(|data| json!({"data": data}))
            .unwrap_or_else(|failure| {
                json!({
                    "error": failure.message,
                    "code": failure.kind.code(),
                    "rejected": failure.session_rejected,
                })
            })
            .to_string()
    });
    // A failed allocation here must not unwind out of an `extern "system"` function either,
    // so the conversion is fallible all the way down instead of relying on `caught` above it.
    env.new_string(response)
        .or_else(|_| {
            env.new_string(
                r#"{"error":"The request stopped unexpectedly","code":"other","rejected":false}"#,
            )
        })
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An in-flight mutation holds the mutation lock and a read lock. Reads
    /// must still get through: some mutations poll the provider for seconds.
    #[test]
    fn a_running_mutation_does_not_block_reads() {
        RUNTIME.block_on(async {
            let mutation = MUTATIONS.lock().await;
            let held = CLIENT.read().await;

            let reader = tokio::time::timeout(Duration::from_millis(50), CLIENT.read()).await;
            assert!(reader.is_ok());

            let second = tokio::time::timeout(Duration::from_millis(50), MUTATIONS.lock()).await;
            assert!(second.is_err(), "mutations must stay serialized");

            drop(held);
            drop(mutation);
        });
    }

    /// A panic must not unwind past the JNI boundary: there it aborts.
    #[test]
    fn a_panicking_command_answers_with_an_error() {
        let response = caught(|| panic!("parser gave up"));

        assert!(response.contains("error"), "{response}");
    }

    #[test]
    fn client_state_allows_parallel_readers() {
        RUNTIME.block_on(async {
            let first = CLIENT.read().await;
            let second = tokio::time::timeout(Duration::from_millis(50), CLIENT.read()).await;
            assert!(second.is_ok());
            drop(first);
        });
    }

    /// Runs `work` under `id` on a thread of its own and hands back the
    /// outcome, or `None` if it was still waiting after a generous while.
    fn waited(
        work: impl Future<Output = Result<Value, Failure>> + Send + 'static,
        id: u64,
        before_drop: Duration,
    ) -> Option<Result<Value, Failure>> {
        let (sender, outcome) = std::sync::mpsc::channel();
        std::thread::spawn(move || sender.send(wait(work, Some(id))));
        std::thread::sleep(before_drop);
        drop_read(id);
        outcome.recv_timeout(Duration::from_secs(5)).ok()
    }

    #[test]
    fn a_request_id_rides_beside_the_command() {
        let Ok((command, id)) = parse(r#"{"request_id":7,"type":"logout"}"#) else {
            panic!("a request with an id did not parse");
        };
        assert!(matches!(command, Command::Logout));
        assert_eq!(id, Some(7));

        // A command with an `id` of its own keeps it.
        let Ok((command, id)) = parse(r#"{"request_id":8,"type":"like_comment","id":"c1"}"#) else {
            panic!("a command with an id of its own did not parse");
        };
        assert!(matches!(command, Command::LikeComment { id } if id == "c1"));
        assert_eq!(id, Some(8));

        let Ok((_, id)) = parse(r#"{"type":"home"}"#) else {
            panic!("a request without an id did not parse");
        };
        assert_eq!(id, None);
    }

    #[test]
    fn a_dropped_read_answers_at_once() {
        let outcome = waited(std::future::pending(), 9_001, Duration::from_millis(50));

        assert!(
            matches!(outcome, Some(Err(_))),
            "the dropped read is still waiting"
        );
        assert!(!READS.lock().unwrap().contains_key(&9_001));
    }

    #[test]
    fn a_drop_that_arrives_before_its_read_still_ends_it() {
        drop_read(9_002);

        let outcome = waited(std::future::pending(), 9_002, Duration::ZERO);

        assert!(
            matches!(outcome, Some(Err(_))),
            "the dropped read is still waiting"
        );
    }

    /// What a sign-out waits for: every read to let go of the client.
    #[test]
    fn a_dropped_read_lets_go_of_the_client() {
        let holding = async {
            let _client = CLIENT.read().await;
            std::future::pending::<()>().await;
            Ok(Value::Null)
        };

        assert!(waited(holding, 9_003, Duration::from_millis(50)).is_some());
        let write = RUNTIME.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), CLIENT.write())
                .await
                .is_ok()
        });
        assert!(write, "the dropped read still holds the client");
    }

    #[test]
    fn a_read_that_answers_leaves_nothing_behind() {
        assert!(matches!(
            wait(async { Ok(Value::Null) }, Some(9_004)),
            Ok(Value::Null)
        ));
        assert!(!READS.lock().unwrap().contains_key(&9_004));
    }

    #[test]
    fn only_reads_can_be_dropped() {
        assert!(Command::Home.is_read());
        assert!(Command::Details { url: String::new() }.is_read());
        assert!(!Command::Logout.is_read());
        assert!(!Command::Rate {
            post_id: 1,
            rating: 5
        }
        .is_read());
        assert!(!Command::SetHiddenCountries {
            countries: String::new()
        }
        .is_read());
    }

    /// A drop for a read that had already answered would otherwise stay for
    /// the life of the process.
    #[test]
    fn a_drop_nothing_came_for_is_forgotten() {
        let Some(long_ago) = Instant::now().checked_sub(EARLY_DROP_KEPT * 2) else {
            return;
        };
        READS.lock().unwrap().insert(
            9_005,
            Read {
                dropped: Arc::default(),
                early_since: Some(long_ago),
            },
        );

        assert!(!reads().contains_key(&9_005));
    }
}
