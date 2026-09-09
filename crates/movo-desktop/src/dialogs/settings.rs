use crate::i18n::{tr, trf, trn};
use crate::state::AppState;
use crate::ui::image;
use movo_core::client::models::parse_country_list;
use movo_core::storage::cache::ImageCache;
use movo_core::storage::settings::AppSettings;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{
    self,
    prelude::{ButtonExt, EditableExt, WidgetExt},
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Mutex;

/// The newest settings not yet on disk. Each save takes whatever is here
/// under the lock, so two rapid changes cannot finish out of order and leave
/// the older one persisted; the later save simply finds nothing to do.
static PENDING_SAVE: Mutex<Option<AppSettings>> = Mutex::new(None);

const QUALITIES: [&str; 5] = ["360p", "480p", "720p", "1080p", "1080p Ultra"];
const INITIAL_VIEWS: [&str; 7] = [
    "home",
    "catalog",
    "search",
    "collections",
    "notifications",
    "favorites",
    "history",
];

/// Present the settings dialog.
///
/// Every row saves as it changes, so nothing is lost if the window closes
/// before the dialog does. `on_changed` reports the saved settings so the
/// window can apply what it owns, such as the colour scheme.
pub fn present(
    parent: &impl IsA<gtk::Widget>,
    state: Rc<AppState>,
    on_changed: impl Fn(AppSettings) + 'static,
    on_account_changed: impl Fn() + 'static,
) {
    let dialog = adw::PreferencesDialog::builder()
        .title(tr("Settings"))
        .build();
    let page = adw::PreferencesPage::new();

    let on_changed = Rc::new(on_changed);
    let on_account_changed = Rc::new(on_account_changed);
    let save = {
        let state = state.clone();
        let on_changed = on_changed.clone();
        Rc::new(move |settings: AppSettings| {
            state.set_settings(settings.clone());
            on_changed(settings.clone());
            if let Ok(mut pending) = PENDING_SAVE.lock() {
                *pending = Some(settings);
            }
            relm4::spawn_blocking(|| {
                let Ok(mut pending) = PENDING_SAVE.lock() else {
                    return;
                };
                if let Some(settings) = pending.take() {
                    if let Err(error) = settings.save() {
                        log::warn!("Could not save settings: {error}");
                    }
                }
            });
        })
    };

    page.add(&account_group(
        parent,
        &dialog,
        state.clone(),
        on_account_changed,
    ));

    let settings = state.settings();

    let appearance = adw::PreferencesGroup::builder()
        .title(tr("Appearance"))
        .build();

    let theme = adw::ComboRow::builder()
        .title(tr("Theme"))
        .model(&gtk::StringList::new(&[
            tr("System"),
            tr("Light"),
            tr("Dark"),
        ]))
        .selected(match settings.theme.as_str() {
            "light" => 1,
            "dark" => 2,
            _ => 0,
        })
        .build();
    {
        let state = state.clone();
        let save = save.clone();
        theme.connect_selected_notify(move |row| {
            let mut settings = state.settings();
            settings.theme = match row.selected() {
                1 => "light",
                2 => "dark",
                _ => "system",
            }
            .to_string();
            save(settings);
        });
    }
    appearance.add(&theme);

    let initial_view = adw::ComboRow::builder()
        .title(tr("Initial Screen"))
        .model(&gtk::StringList::new(&[
            tr("Home"),
            tr("Catalog"),
            tr("Search"),
            tr("Collections"),
            tr("Notifications"),
            tr("Favorites"),
            tr("History"),
        ]))
        .selected(
            INITIAL_VIEWS
                .iter()
                .position(|view| *view == settings.initial_view)
                .unwrap_or(1) as u32,
        )
        .build();
    {
        let state = state.clone();
        let save = save.clone();
        initial_view.connect_selected_notify(move |row| {
            let mut settings = state.settings();
            settings.initial_view = INITIAL_VIEWS
                .get(row.selected() as usize)
                .unwrap_or(&"catalog")
                .to_string();
            save(settings);
        });
    }
    appearance.add(&initial_view);
    page.add(&appearance);

    let playback = adw::PreferencesGroup::builder()
        .title(tr("Playback"))
        .description(tr("Titles play in an external player, mpv by default."))
        .build();

    let quality = adw::ComboRow::builder()
        .title(tr("Default Quality"))
        .model(&gtk::StringList::new(&QUALITIES))
        .selected(
            QUALITIES
                .iter()
                .position(|value| *value == settings.default_quality)
                .unwrap_or(3) as u32,
        )
        .build();
    {
        let state = state.clone();
        let save = save.clone();
        quality.connect_selected_notify(move |row| {
            let mut settings = state.settings();
            settings.default_quality = QUALITIES
                .get(row.selected() as usize)
                .unwrap_or(&"1080p")
                .to_string();
            save(settings);
        });
    }
    playback.add(&quality);

    let ask_quality = adw::SwitchRow::builder()
        .title(tr("Ask Quality Before Playback"))
        .subtitle(tr("Show the quality picker every time playback starts"))
        .active(settings.ask_quality_before_play)
        .build();
    {
        let state = state.clone();
        let save = save.clone();
        ask_quality.connect_active_notify(move |row| {
            let mut settings = state.settings();
            settings.ask_quality_before_play = row.is_active();
            save(settings);
        });
    }
    playback.add(&ask_quality);

    let auto_next = adw::SwitchRow::builder()
        .title(tr("Auto-Next Episode"))
        .subtitle(tr("Continue with the next episode when one finishes"))
        .active(settings.auto_next_episode)
        .build();
    {
        let state = state.clone();
        let save = save.clone();
        auto_next.connect_active_notify(move |row| {
            let mut settings = state.settings();
            settings.auto_next_episode = row.is_active();
            save(settings);
        });
    }
    playback.add(&auto_next);

    let sort_voices = adw::SwitchRow::builder()
        .title(tr("Sort Voice-overs by Rating"))
        .subtitle(tr("Show the highest-rated provider voice-overs first"))
        .active(settings.sort_voices)
        .build();
    {
        let state = state.clone();
        let save = save.clone();
        sort_voices.connect_active_notify(move |row| {
            let mut settings = state.settings();
            settings.sort_voices = row.is_active();
            save(settings);
        });
    }
    playback.add(&sort_voices);

    for row in player_rows(state.clone(), save.clone()) {
        playback.add(&row);
    }
    page.add(&playback);

    let filters = adw::PreferencesGroup::builder()
        .title(tr("Listings"))
        .description(tr(
            "Titles from these countries are left out of the catalog, search and home. Use the names the site shows, separated by commas.",
        ))
        .build();
    let hidden_countries = adw::EntryRow::builder()
        .title(tr("Hidden Countries"))
        .text(settings.hidden_countries.join(", "))
        .build();
    {
        let state = state.clone();
        let save = save.clone();
        hidden_countries.connect_changed(move |row| {
            let mut settings = state.settings();
            settings.hidden_countries = parse_country_list(&row.text());
            state
                .client
                .set_hidden_countries(settings.hidden_countries.clone());
            save(settings);
        });
    }
    filters.add(&hidden_countries);
    page.add(&filters);

    let storage = adw::PreferencesGroup::builder().title(tr("Cache")).build();
    let clear_cache = adw::ActionRow::builder()
        .title(tr("Clear Image Cache"))
        .subtitle(tr("Removes saved covers and posters from disk"))
        .build();
    let clear_button = gtk::Button::builder()
        .label(tr("Clear"))
        .valign(gtk::Align::Center)
        .build();
    clear_button.add_css_class("destructive-action");
    clear_button.connect_clicked(|button| {
        button.set_sensitive(false);
        let button = button.clone();
        relm4::spawn_local(async move {
            let _ = relm4::spawn_blocking(ImageCache::clear).await;
            button.set_label(tr("Cleared"));
        });
    });
    clear_cache.add_suffix(&clear_button);
    storage.add(&clear_cache);
    page.add(&storage);

    dialog.add(&page);
    dialog.present(Some(parent));
}

/// The players offered by the picker, in the order they are listed. The empty
/// command is the custom entry, whose text is used instead.
const PLAYERS: [&str; 5] = ["mpv", "vlc", "ask", "default", ""];

/// Whether a command can be found, so the picker can warn before playback
/// fails with nothing but a toast.
fn on_path(command: &str) -> bool {
    let command = std::path::Path::new(command);
    if command.is_absolute() {
        return command.is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|directory| directory.join(command).is_file())
        })
        .unwrap_or(false)
}

/// The player picker and the custom-command entry it reveals.
fn player_rows(
    state: Rc<AppState>,
    save: Rc<impl Fn(AppSettings) + 'static>,
) -> Vec<adw::PreferencesRow> {
    let stored = state.settings().external_player.trim().to_string();
    let selected = PLAYERS
        .iter()
        .position(|player| !player.is_empty() && *player == stored)
        .unwrap_or(PLAYERS.len() - 1);

    let custom = adw::EntryRow::builder()
        .title(tr("Player Command"))
        .text(&stored)
        .visible(selected == PLAYERS.len() - 1)
        .build();

    let picker = adw::ComboRow::builder()
        .title(tr("Player"))
        .model(&gtk::StringList::new(&[
            tr("mpv"),
            tr("VLC"),
            tr("Ask Every Time"),
            tr("System Default"),
            tr("Custom Command"),
        ]))
        .selected(selected as u32)
        .build();

    let warn = {
        let picker = picker.clone();
        Rc::new(move |command: &str| {
            let handled_by_the_desktop = matches!(command, "" | "default" | "ask");
            let missing = !handled_by_the_desktop && !on_path(command);
            picker.set_subtitle(&if missing {
                trf("{} was not found on this system", &[&command])
            } else if command == "ask" {
                tr("The chosen application receives no headers, so playback may be refused.")
                    .to_string()
            } else if handled_by_the_desktop {
                tr("The system handler receives no headers, so playback may be refused.")
                    .to_string()
            } else {
                tr("Progress is only tracked with mpv.").to_string()
            });
        })
    };
    warn(&stored);

    {
        let state = state.clone();
        let save = save.clone();
        let custom = custom.clone();
        let warn = warn.clone();
        picker.connect_selected_notify(move |row| {
            let choice = PLAYERS.get(row.selected() as usize).copied().unwrap_or("");
            let is_custom = choice.is_empty();
            custom.set_visible(is_custom);

            let mut settings = state.settings();
            settings.external_player = if is_custom {
                custom.text().trim().to_string()
            } else {
                choice.to_string()
            };
            warn(&settings.external_player);
            save(settings);
        });
    }

    {
        let warn = warn.clone();
        custom.connect_changed(move |row| {
            let mut settings = state.settings();
            settings.external_player = row.text().trim().to_string();
            warn(&settings.external_player);
            save(settings);
        });
    }

    vec![picker.upcast(), custom.upcast()]
}

fn account_group(
    parent: &impl IsA<gtk::Widget>,
    dialog: &adw::PreferencesDialog,
    state: Rc<AppState>,
    on_account_changed: Rc<impl Fn() + 'static>,
) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(tr("HDRezka Account"))
        .description(tr("Sign in to sync favorites, history and premium quality"))
        .build();

    let row = adw::ActionRow::builder().title(tr("Profile")).build();
    let icon = gtk::Image::from_icon_name("avatar-default-symbolic");
    let avatar = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Cover)
        .width_request(56)
        .height_request(56)
        .visible(false)
        .build();
    avatar.add_css_class("card");
    row.add_prefix(&icon);
    row.add_prefix(&avatar);

    let button = gtk::Button::builder().valign(gtk::Align::Center).build();
    row.add_suffix(&button);
    group.add(&row);

    let avatar_token: image::ImageToken = Rc::new(Cell::new(0));
    let show_account = {
        let state = state.clone();
        let row = row.clone();
        let button = button.clone();
        let icon = icon.clone();
        let avatar = avatar.clone();
        Rc::new(move || match state.user() {
            Some(user) => {
                row.set_title(&user.username);
                let mut details = vec![format!("ID: {}", user.user_id)];
                if let Some(email) = user.email.as_deref().filter(|email| !email.is_empty()) {
                    details.push(email.to_string());
                }
                details.push(match user.premium_days {
                    Some(days) => trn("Premium: {} day", "Premium: {} days", days as u64),
                    None if user.is_vip => tr("VIP / Premium").to_string(),
                    None => tr("Standard").to_string(),
                });
                row.set_subtitle(&details.join(" • "));

                image::load(&avatar, user.avatar_url.as_deref(), 56, 56, &avatar_token);
                avatar.set_visible(user.avatar_url.is_some());
                icon.set_visible(user.avatar_url.is_none());

                button.set_label(tr("Sign Out"));
                button.remove_css_class("suggested-action");
                button.add_css_class("destructive-action");
            }
            None => {
                avatar.set_visible(false);
                icon.set_visible(true);
                row.set_title(tr("Not signed in"));
                row.set_subtitle(tr("Sign in to sync favorites and unlock premium quality"));
                button.set_label(tr("Sign In"));
                button.remove_css_class("destructive-action");
                button.add_css_class("suggested-action");
            }
        })
    };
    show_account();

    let parent = parent.as_ref().clone();
    let dialog = dialog.clone();
    button.connect_clicked(move |_| {
        let state = state.clone();
        let show_account = show_account.clone();
        let on_account_changed = on_account_changed.clone();
        let parent = parent.clone();
        let dialog = dialog.clone();

        if state.user().is_some() {
            relm4::spawn_local(async move {
                let confirm = adw::AlertDialog::builder()
                    .heading(tr("Sign out?"))
                    .body(tr("Local session data will be removed from this device."))
                    .close_response("cancel")
                    .build();
                confirm.add_responses(&[("cancel", tr("Cancel")), ("logout", tr("Sign Out"))]);
                confirm.set_response_appearance("logout", adw::ResponseAppearance::Destructive);
                if confirm.choose_future(Some(&dialog)).await != "logout" {
                    return;
                }

                let client = state.client.clone();
                let result = relm4::spawn(async move { client.logout().await })
                    .await
                    .unwrap_or_else(|_| {
                        Err(tr("Signing out stopped unexpectedly").to_string().into())
                    });
                show_account();
                on_account_changed();
                if let Err(error) = result {
                    adw::AlertDialog::builder()
                        .heading(tr("Signed out, but the stored session was not removed"))
                        .body(error.to_string())
                        .build()
                        .present(Some(&dialog));
                }
            });
        } else {
            crate::dialogs::login::present(&parent, state, move |_| {
                show_account();
                on_account_changed();
            });
        }
    });

    group
}
