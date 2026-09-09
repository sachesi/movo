use jni::{
    objects::{JClass, JString},
    sys::jstring,
    JNIEnv,
};
use movo_core::client::{
    models::{self, CatalogCategory, Translator},
    RezkaClient,
};
use movo_core::error::{ClientError, ErrorKind};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::LazyLock;
use tokio::sync::RwLock;

static CLIENT: LazyLock<RwLock<RezkaClient>> = LazyLock::new(|| RwLock::new(RezkaClient::new()));

/// Serializes account mutations against each other without shutting readers
/// out. The mutations only need `&RezkaClient`; taking the write lock for them
/// blocked every catalog, search and details request for as long as the
/// mutation ran, and some of them poll the provider for several seconds.
static MUTATIONS: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));
static RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().expect("Android runtime"));

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
        Command::Login { .. } | Command::Restore { .. } | Command::Logout => unreachable!(),
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

fn invoke(command: Command) -> Result<Value, Failure> {
    RUNTIME.block_on(async move {
        match command {
            Command::Login { login, password } => {
                let client = CLIENT.write().await;
                let user = client.login(&login, &password).await?;
                Ok(json!({"user": user, "secret": client.export_session()?}))
            }
            Command::Restore { secret } => {
                // The reply carries whether the session was turned down, so the
                // app only throws the stored secret away when it is worthless.
                CLIENT
                    .write()
                    .await
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
    })
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

#[no_mangle]
pub extern "system" fn Java_org_movo_app_core_NativeBridge_invoke(
    mut env: JNIEnv,
    _class: JClass,
    request: JString,
) -> jstring {
    let response = caught(|| {
        env.get_string(&request)
            .map_err(|error| Failure::from(error.to_string()))
            .and_then(|request| {
                serde_json::from_str::<Command>(&request.to_string_lossy())
                    .map_err(|error| Failure::from(error.to_string()))
            })
            .and_then(invoke)
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
    use std::time::Duration;

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
}
