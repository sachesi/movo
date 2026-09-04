use crate::api::{guarded, Guarded};
use crate::i18n::{tr, trf};
use crate::playback::mpv::{self, HistorySeed, LaunchRequest, PlaybackEvent, PlayerKind};
use crate::state::AppState;
use movo_core::client::models::{MediaDetails, StreamBundle, SubtitleTrack};
use movo_core::storage::history::WatchHistory;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{self, prelude::WidgetExt};
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;

/// What to play, as chosen on the details page.
#[derive(Debug, Clone)]
pub struct PlayRequest {
    pub details: MediaDetails,
    pub translator_id: i64,
    pub season: Option<i64>,
    pub episode: Option<i64>,
}

/// Starts external playback and follows it until the player exits.
///
/// The controller has no widgets of its own: it borrows the window to present
/// the quality picker and reports everything else as messages.
pub struct Launcher {
    state: Rc<AppState>,
    window: gtk::Window,
    current: Option<PlayRequest>,
}

#[derive(Debug)]
pub enum LauncherMsg {
    Play(Box<PlayRequest>),
}

#[derive(Debug)]
pub enum LauncherOutput {
    Notify(String),
    /// Progress was written, so history views are out of date.
    HistoryChanged,
    /// Playback finished and the next episode should start.
    PlayNext(Box<PlayRequest>),
    AccountInvalidated,
}

#[derive(Debug)]
pub enum LauncherCommand {
    Stream(Box<PlayRequest>, Box<Guarded<StreamBundle>>),
    Event(PlaybackEvent),
    /// The provider was told the title finished.
    Marked(Guarded<()>),
}

impl Component for Launcher {
    type Init = (Rc<AppState>, gtk::Window);
    type Input = LauncherMsg;
    type Output = LauncherOutput;
    type CommandOutput = LauncherCommand;
    type Root = gtk::Box;
    type Widgets = ();

    fn init_root() -> Self::Root {
        // Nothing is shown; the controller only drives an external player.
        let root = gtk::Box::default();
        root.set_visible(false);
        root
    }

    fn init(
        (state, window): Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        ComponentParts {
            model: Launcher {
                state,
                window,
                current: None,
            },
            widgets: (),
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        let LauncherMsg::Play(request) = message;

        // The provider refuses anonymous stream requests, so say so here
        // rather than surfacing its untranslated error.
        if self.state.user().is_none() {
            let _ = sender.output(LauncherOutput::Notify(
                tr("Sign in to play this title").to_string(),
            ));
            return;
        }

        self.current = Some((*request).clone());
        let client = self.state.client.clone();
        let details = request.details.clone();
        let translator_id = request.translator_id;
        let season = request.season;
        let episode = request.episode;

        sender.oneshot_command(async move {
            let loaded = guarded(client, move |client| async move {
                match (season, episode) {
                    (Some(season), Some(episode)) => {
                        client
                            .fetch_episode_stream(details.id, translator_id, season, episode)
                            .await
                    }
                    _ => match details
                        .translators
                        .iter()
                        .find(|translator| translator.id == translator_id)
                    {
                        Some(translator) => client.fetch_movie_stream(details.id, translator).await,
                        None => Err(trf("Voice-over {} was not found", &[&translator_id])),
                    },
                }
            })
            .await;
            LauncherCommand::Stream(request, Box::new(loaded))
        });
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            LauncherCommand::Stream(request, loaded) => {
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(LauncherOutput::AccountInvalidated);
                    return;
                }
                match loaded.result {
                    Ok(bundle) if bundle.streams.is_empty() => {
                        let _ = sender.output(LauncherOutput::Notify(
                            tr("No streams are available for the selected voice-over.").to_string(),
                        ));
                    }
                    Ok(bundle) => self.start(*request, bundle, &sender),
                    Err(error) => {
                        let _ = sender.output(LauncherOutput::Notify(error));
                    }
                }
            }
            LauncherCommand::Event(event) => match event {
                PlaybackEvent::Ended => {
                    let _ = sender.output(LauncherOutput::HistoryChanged);
                    self.mark_watched(&sender);
                    self.play_next(&sender);
                }
                PlaybackEvent::TrackingLost(message) => {
                    let _ = sender.output(LauncherOutput::HistoryChanged);
                    let _ = sender.output(LauncherOutput::Notify(message));
                }
                PlaybackEvent::Failed(message) => {
                    let _ = sender.output(LauncherOutput::Notify(message));
                }
            },
            LauncherCommand::Marked(marked) => {
                if let Err(error) = marked.result {
                    let _ = sender.output(LauncherOutput::Notify(error));
                }
            }
        }
    }
}

impl Launcher {
    fn start(
        &mut self,
        request: PlayRequest,
        bundle: StreamBundle,
        sender: &ComponentSender<Self>,
    ) {
        let settings = self.state.settings();
        let Some(user) = self.state.user() else {
            let _ = sender.output(LauncherOutput::Notify(
                tr("Sign in to play this title").to_string(),
            ));
            return;
        };

        let default_index = default_stream_index(&bundle, &settings.default_quality);
        let user_id = user.user_id;
        let player = settings.external_player.trim().to_string();

        if settings.ask_quality_before_play && bundle.streams.len() > 1 {
            let sender = sender.clone();
            let window = self.window.clone();
            let chooser_window = window.clone();
            ask_quality(
                chooser_window,
                quality_labels(&bundle),
                default_index,
                move |index| {
                    launch(sender, &window, &request, &bundle, index, &user_id, player);
                },
            );
        } else {
            launch(
                sender.clone(),
                &self.window,
                &request,
                &bundle,
                default_index,
                &user_id,
                player,
            );
        }
    }

    fn mark_watched(&self, sender: &ComponentSender<Self>) {
        let Some(request) = self.current.clone() else {
            return;
        };
        let client = self.state.client.clone();
        sender.oneshot_command(async move {
            LauncherCommand::Marked(
                guarded(client, move |client| async move {
                    client
                        .mark_watched(
                            request.details.id,
                            request.translator_id,
                            request.season,
                            request.episode,
                        )
                        .await
                })
                .await,
            )
        });
    }

    /// Continue with the next episode of the current season, if there is one.
    fn play_next(&mut self, sender: &ComponentSender<Self>) {
        if !self.state.settings().auto_next_episode {
            return;
        }
        let Some(current) = self.current.clone() else {
            return;
        };
        let (Some(season_id), Some(episode_id)) = (current.season, current.episode) else {
            return;
        };
        let Some(season) = current
            .details
            .seasons
            .iter()
            .find(|season| season.id == season_id)
        else {
            return;
        };
        let Some(position) = season
            .episodes
            .iter()
            .position(|episode| episode.id == episode_id)
        else {
            return;
        };
        let Some(next) = season.episodes.get(position + 1) else {
            return;
        };

        let _ = sender.output(LauncherOutput::PlayNext(Box::new(PlayRequest {
            details: current.details.clone(),
            translator_id: current.translator_id,
            season: Some(season_id),
            episode: Some(next.id),
        })));
    }
}

fn launch(
    sender: ComponentSender<Launcher>,
    window: &gtk::Window,
    request: &PlayRequest,
    bundle: &StreamBundle,
    index: usize,
    user_id: &str,
    command: String,
) {
    let Some(url) = bundle
        .streams
        .get(index)
        .and_then(|stream| stream.best_url())
        .filter(|url| mpv::valid_media_url(url))
    else {
        let _ = sender.output(LauncherOutput::Notify(
            tr("The provider returned no playable stream for this quality.").to_string(),
        ));
        return;
    };

    let request = request.clone();
    let bundle = bundle.clone();
    let url = url.to_string();
    let user_id = user_id.to_string();
    let events = sender.command_sender().clone();
    let window = window.clone();

    relm4::spawn_local(async move {
        let launch_request = build_request(&request, &bundle, url, &user_id).await;
        match start_player(command, window, launch_request, events).await {
            Ok(()) => {}
            Err(error) => {
                let _ = sender.output(LauncherOutput::Notify(error));
            }
        }
    });
}

fn quality_labels(bundle: &StreamBundle) -> Vec<String> {
    bundle
        .streams
        .iter()
        .map(|stream| {
            if stream.is_premium {
                trf("Premium · {}", &[&stream.quality])
            } else {
                stream.quality.clone()
            }
        })
        .collect()
}

async fn build_request(
    request: &PlayRequest,
    bundle: &StreamBundle,
    url: String,
    user_id: &str,
) -> LaunchRequest {
    let lookup_user_id = user_id.to_string();
    let media_id = request.details.id;
    let season = request.season;
    let episode = request.episode;
    let previous = relm4::spawn_blocking(move || {
        WatchHistory::load(&lookup_user_id)
            .get_entry(media_id, season, episode)
            .cloned()
    })
    .await
    .ok()
    .flatten();

    LaunchRequest {
        url,
        subtitle: default_subtitle(&bundle.subtitles),
        title: match (request.season, request.episode) {
            (Some(season), Some(episode)) => {
                format!("{} (S{season}E{episode})", request.details.title)
            }
            _ => request.details.title.clone(),
        },
        user_agent: bundle.user_agent.clone(),
        referer: bundle.referer.clone(),
        start_secs: previous
            .as_ref()
            .map(|entry| entry.position_secs.max(0.0))
            .unwrap_or(0.0),
        duration_secs: previous
            .as_ref()
            .map(|entry| entry.duration_secs.max(0.0))
            .unwrap_or(0.0),
        history: HistorySeed {
            details: request.details.clone(),
            user_id: user_id.to_string(),
            translator_id: request.translator_id,
            season: request.season,
            episode: request.episode,
        },
    }
}

async fn start_player(
    command: String,
    window: gtk::Window,
    request: LaunchRequest,
    events: relm4::Sender<LauncherCommand>,
) -> Result<(), String> {
    let playback_events = ForwardEvents(events);

    match mpv::player_kind(&command) {
        PlayerKind::Mpv => {
            let socket = relm4::spawn_blocking(mpv::socket_path)
                .await
                .unwrap_or_default();
            match socket {
                Some(socket) => {
                    let spawn_request = request.clone();
                    let spawn_socket = socket.clone();
                    let spawn_command = command.clone();
                    relm4::spawn_blocking(move || {
                        mpv::spawn_mpv(&spawn_command, &spawn_request, &spawn_socket)
                    })
                    .await
                    .map_err(|_| tr("Starting MPV stopped unexpectedly").to_string())?
                    .map_err(|error| trf("Could not start {}: {}", &[&command, &error]))?;

                    relm4::spawn(mpv::monitor(
                        socket,
                        request.history.clone(),
                        request.start_secs,
                        request.duration_secs,
                        playback_events.into_sender(),
                    ));
                    Ok(())
                }
                None => {
                    let spawn_request = request.clone();
                    let spawn_command = command.clone();
                    relm4::spawn_blocking(move || {
                        mpv::spawn_mpv_untracked(&spawn_command, &spawn_request)
                    })
                    .await
                    .map_err(|_| tr("Starting MPV stopped unexpectedly").to_string())?
                    .map_err(|error| trf("Could not start {}: {}", &[&command, &error]))?;
                    Err(tr(
                        "MPV opened, but the XDG runtime directory is unavailable; progress is not tracked.",
                    )
                    .to_string())
                }
            }
        }
        PlayerKind::Vlc => {
            let spawn_request = request.clone();
            let spawn_command = command.clone();
            relm4::spawn_blocking(move || mpv::spawn_vlc(&spawn_command, &spawn_request))
                .await
                .map_err(|_| tr("Starting VLC stopped unexpectedly").to_string())?
                .map_err(|error| trf("Could not start {}: {}", &[&command, &error]))?;
            Err(tr("VLC does not report progress; history is not updated.").to_string())
        }
        PlayerKind::Other => {
            let url = request.url.clone();
            let spawn_command = command.clone();
            relm4::spawn_blocking(move || {
                let mut process = std::process::Command::new(spawn_command);
                process.arg(url);
                mpv::spawn_reaped(process)
            })
            .await
            .map_err(|_| tr("Starting the player stopped unexpectedly").to_string())?
            .map_err(|error| trf("Could not start {}: {}", &[&command, &error]))?;
            Err(tr("This player does not report progress; history is not updated.").to_string())
        }
        PlayerKind::Ask => ask_application(&window, &request.url).await,
        PlayerKind::Default => {
            // No configured player: hand the stream to the desktop's default
            // handler. It receives no headers, so the provider may refuse it.
            gtk::gio::AppInfo::launch_default_for_uri(
                &request.url,
                None::<&gtk::gio::AppLaunchContext>,
            )
            .map_err(|error| error.to_string())?;
            Err(tr("Opened with the system player; set mpv in Settings for progress.").to_string())
        }
    }
}

/// Hand the stream to the application the user picks from the desktop's
/// "Open With" prompt. The chosen application receives only the URL, so the
/// provider may refuse it and nothing is written to history.
///
/// `GtkFileLauncher` would be the modern way to ask, but it asks the portal
/// about the URI, and a remote URI is matched by its scheme: the prompt then
/// lists web browsers and no media player at all. Asking by content type is
/// what puts the installed players in front of the user, and only the
/// deprecated dialog can do that.
async fn ask_application(window: &gtk::Window, url: &str) -> Result<(), String> {
    let path = reqwest::Url::parse(url)
        .ok()
        .map(|parsed| parsed.path().to_string())
        .unwrap_or_default();
    let (guessed, uncertain) = gtk::gio::content_type_guess(Some(&path), None);
    let content_type = if uncertain || !guessed.starts_with("video/") {
        "video/mp4".to_string()
    } else {
        guessed.to_string()
    };

    #[allow(deprecated)]
    let dialog = gtk::AppChooserDialog::for_content_type(
        Some(window),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        &content_type,
    );
    #[allow(deprecated)]
    dialog.set_heading(tr("Open this stream with"));

    let (chosen, receiver) = relm4::channel::<Option<gtk::gio::AppInfo>>();
    #[allow(deprecated)]
    dialog.connect_response(move |dialog, response| {
        let picked = (response == gtk::ResponseType::Ok)
            .then(|| dialog.app_info())
            .flatten();
        let _ = chosen.send(picked);
        dialog.destroy();
    });
    dialog.present();

    // Closing the prompt without choosing is not a failure.
    let Some(Some(app)) = receiver.recv().await else {
        return Ok(());
    };
    app.launch_uris(&[url], None::<&gtk::gio::AppLaunchContext>)
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Adapts playback events into launcher commands.
struct ForwardEvents(relm4::Sender<LauncherCommand>);

impl ForwardEvents {
    fn into_sender(self) -> relm4::Sender<PlaybackEvent> {
        let (sender, receiver) = relm4::channel::<PlaybackEvent>();
        let commands = self.0;
        relm4::spawn(async move {
            while let Some(event) = receiver.recv().await {
                if commands.send(LauncherCommand::Event(event)).is_err() {
                    break;
                }
            }
        });
        sender
    }
}

fn default_subtitle(tracks: &[SubtitleTrack]) -> Option<SubtitleTrack> {
    tracks
        .iter()
        .find(|track| track.is_default)
        .or_else(|| tracks.first())
        .cloned()
}

/// Pick the stream that matches the preferred quality, falling back to 1080p
/// and then to a partial match before giving up on the first entry.
fn default_stream_index(bundle: &StreamBundle, preferred: &str) -> usize {
    bundle
        .streams
        .iter()
        .position(|stream| stream.quality == preferred)
        .or_else(|| {
            bundle
                .streams
                .iter()
                .position(|stream| stream.quality == "1080p")
        })
        .or_else(|| {
            bundle
                .streams
                .iter()
                .position(|stream| stream.quality.contains(preferred))
        })
        .unwrap_or(0)
}

fn ask_quality(
    window: gtk::Window,
    names: Vec<String>,
    default_index: usize,
    on_chosen: impl FnOnce(usize) + 'static,
) {
    let dropdown =
        gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
    dropdown.set_selected(default_index as u32);

    let dialog = adw::AlertDialog::builder()
        .heading(tr("Choose Quality"))
        .close_response("cancel")
        .extra_child(&dropdown)
        .build();
    dialog.add_responses(&[("cancel", tr("Cancel")), ("play", tr("Watch"))]);
    dialog.set_response_appearance("play", adw::ResponseAppearance::Suggested);

    relm4::spawn_local(async move {
        if dialog.choose_future(Some(&window)).await == "play" {
            on_chosen(dropdown.selected() as usize);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{default_stream_index, default_subtitle};
    use movo_core::client::models::{StreamBundle, StreamEntry, SubtitleTrack};

    fn bundle(qualities: &[(&str, bool)]) -> StreamBundle {
        StreamBundle {
            id: 1,
            translator_id: 1,
            season: None,
            episode: None,
            streams: qualities
                .iter()
                .map(|(quality, is_premium)| StreamEntry {
                    quality: quality.to_string(),
                    is_premium: *is_premium,
                    urls: vec!["https://cdn.example/stream.m3u8".to_string()],
                })
                .collect(),
            subtitles: Vec::new(),
            storyboard_url: None,
            storyboard: Vec::new(),
            user_agent: String::new(),
            referer: String::new(),
        }
    }

    #[test]
    fn default_quality_prefers_an_exact_match() {
        let streams = bundle(&[("1080p Ultra", true), ("1080p", false), ("720p", false)]);
        assert_eq!(default_stream_index(&streams, "1080p"), 1);
        assert_eq!(default_stream_index(&streams, "720p"), 2);
    }

    #[test]
    fn unknown_quality_falls_back_to_1080p() {
        let streams = bundle(&[("480p", false), ("1080p", false)]);
        assert_eq!(default_stream_index(&streams, "2160p"), 1);
    }

    #[test]
    fn subtitles_prefer_the_provider_default() {
        let track = |code: &str, is_default: bool| SubtitleTrack {
            code: code.to_string(),
            title: code.to_string(),
            url: format!("https://cdn.example/{code}.vtt"),
            language_code: None,
            is_default,
        };
        let tracks = vec![track("en", false), track("ru", true)];
        assert_eq!(default_subtitle(&tracks).unwrap().code, "ru");
        assert!(default_subtitle(&[]).is_none());
    }
}
