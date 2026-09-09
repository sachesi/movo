use super::Launched;
use crate::i18n::tr;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;

/// Hand the stream to the application the user picks from a list of
/// installed applications. The chosen application receives only the URL, so
/// the provider may refuse it, and it reports no progress back.
///
/// `GtkFileLauncher` would be the modern way to ask, but it asks the portal
/// about the URI, and a remote URI is matched by its scheme: the prompt then
/// lists web browsers and no media player at all. Asking by content type is
/// what puts the installed players in front of the user, so the list here is
/// built from `AppInfo::all_for_type` instead.
pub(super) async fn ask_application(window: &gtk::Window, url: &str) -> Result<Launched, String> {
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

    let apps = gtk::gio::AppInfo::all_for_type(&content_type);

    let (chosen, receiver) = relm4::channel::<Option<gtk::gio::AppInfo>>();

    let header = adw::HeaderBar::new();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);

    let dialog = adw::Dialog::builder()
        .title(tr("Open this stream with"))
        .content_width(360)
        .content_height(420)
        .child(&toolbar)
        .build();

    if apps.is_empty() {
        let status = adw::StatusPage::builder()
            .icon_name("dialog-question-symbolic")
            .title(tr("No Application Found"))
            .description(tr(
                "No application on this system can open this kind of stream.",
            ))
            .vexpand(true)
            .build();
        toolbar.set_content(Some(&status));
    } else {
        let list = crate::ui::activatable_list();
        for app in &apps {
            let row = adw::ActionRow::builder()
                .title(app.display_name())
                .activatable(true)
                .build();
            if let Some(icon) = app.icon() {
                row.add_prefix(&gtk::Image::from_gicon(&icon));
            }
            let chosen = chosen.clone();
            let dialog = dialog.clone();
            let picked = app.clone();
            row.connect_activated(move |_| {
                let _ = chosen.send(Some(picked.clone()));
                dialog.close();
            });
            list.append(&row);
        }
        let scrolled = gtk::ScrolledWindow::builder()
            .child(&list)
            .vexpand(true)
            .build();
        toolbar.set_content(Some(&scrolled));
    }

    // Row activation above sends the choice and then closes the dialog, which fires this
    // handler too; the second, empty send that follows is harmless because only the first
    // value pulled off the channel below is used.
    dialog.connect_closed(move |_| {
        let _ = chosen.send(None);
    });
    dialog.present(Some(window));

    // Closing the prompt without choosing is not a failure.
    let Some(Some(app)) = receiver.recv().await else {
        return Ok(Launched::Cancelled);
    };
    app.launch_uris(&[url], None::<&gtk::gio::AppLaunchContext>)
        .map_err(|error| error.to_string())?;
    Ok(Launched::Untracked(
        tr("This player does not report progress; the resume position is not updated.").to_string(),
    ))
}

pub(super) fn ask_quality(
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
