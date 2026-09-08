use crate::api::{guarded_as, Guarded};
use crate::i18n::tr;
use crate::state::{account_of, Account, AppState};
use crate::ui::content::{ContentStack, ContentState};
use movo_core::client::models::{MediaItem, ServerHistoryEntry};
use movo_core::client::SyncedHistory;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::gtk::{self, prelude::WidgetExt};
use relm4::{Component, ComponentParts, ComponentSender, FactorySender};
use std::rc::Rc;

pub struct HistoryView {
    state: Rc<AppState>,
    account: Option<Account>,
    /// Set while the removal confirmation is up, so a second click on the row
    /// button does not stack a second dialog behind the first.
    confirming: bool,
    rows: FactoryVecDeque<HistoryRow>,
    content: ContentStack,
}

#[derive(Debug)]
pub enum HistoryMsg {
    Reload,
    Open(MediaItem),
    SetWatched(String, bool),
    Remove(String),
    ConfirmationClosed,
    Delete(String),
}

#[derive(Debug)]
pub enum HistoryOutput {
    Open(MediaItem),
    AccountInvalidated,
    Warning(String),
}

#[derive(Debug)]
pub enum HistoryCommand {
    Loaded(Guarded<SyncedHistory>),
    Changed(Guarded<()>),
}

#[relm4::component(pub)]
impl Component for HistoryView {
    type Init = Rc<AppState>;
    type Input = HistoryMsg;
    type Output = HistoryOutput;
    type CommandOutput = HistoryCommand;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            #[local_ref]
            content -> gtk::Stack {},
        }
    }

    fn init(
        state: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list = crate::ui::activatable_list();
        list.set_margin_start(18);
        list.set_margin_end(18);
        list.set_margin_top(12);
        list.set_margin_bottom(12);
        list.set_valign(gtk::Align::Start);

        let rows = FactoryVecDeque::builder()
            .launch(list.clone())
            .forward(sender.input_sender(), |output| output);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list)
            .build();

        let content = ContentStack::new(&scrolled, tr("History Did Not Load"));
        content.on_retry({
            let sender = sender.clone();
            move || sender.input(HistoryMsg::Reload)
        });

        let model = HistoryView {
            state,
            account: None,
            confirming: false,
            rows,
            content,
        };
        let content = model.content.widget.clone();
        let widgets = view_output!();
        sender.input(HistoryMsg::Reload);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            HistoryMsg::Reload => {
                if self.state.user().is_none() {
                    self.account = None;
                    self.rows.guard().clear();
                    self.content
                        .set(ContentState::SignedOut(tr("Sign In to See Your History")));
                    return;
                }
                self.content
                    .set(ContentState::Loading(tr("Loading History")));
                self.account = account_of(&self.state.client);
                let account = self.account.clone();
                let client = self.state.client.clone();
                sender.oneshot_command(async move {
                    HistoryCommand::Loaded(
                        guarded_as(client, account, |client| async move {
                            client.sync_history().await
                        })
                        .await,
                    )
                });
            }
            HistoryMsg::Open(item) => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(HistoryOutput::AccountInvalidated);
                    return;
                }
                let _ = sender.output(HistoryOutput::Open(item));
            }
            HistoryMsg::SetWatched(id, watched) => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(HistoryOutput::AccountInvalidated);
                    return;
                }
                let account = self.account.clone();
                let client = self.state.client.clone();
                sender.oneshot_command(async move {
                    HistoryCommand::Changed(
                        guarded_as(client, account, move |client| async move {
                            client.set_history_watched(&id, watched).await
                        })
                        .await,
                    )
                });
            }
            HistoryMsg::Remove(id) => {
                if self.confirming {
                    return;
                }
                let Some(window) = root
                    .root()
                    .and_then(|root| root.downcast::<gtk::Window>().ok())
                else {
                    return;
                };
                self.confirming = true;
                let dialog = adw::AlertDialog::builder()
                    .heading(tr("Remove From History?"))
                    .body(tr(
                        "This removes the title from your account history and this device.",
                    ))
                    .close_response("cancel")
                    .build();
                dialog.add_responses(&[("cancel", tr("Cancel")), ("remove", tr("Remove"))]);
                dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
                relm4::spawn_local(async move {
                    let response = dialog.choose_future(Some(&window)).await;
                    sender.input(HistoryMsg::ConfirmationClosed);
                    if response == "remove" {
                        sender.input(HistoryMsg::Delete(id));
                    }
                });
            }
            HistoryMsg::ConfirmationClosed => self.confirming = false,
            HistoryMsg::Delete(id) => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(HistoryOutput::AccountInvalidated);
                    return;
                }
                let account = self.account.clone();
                let client = self.state.client.clone();
                sender.oneshot_command(async move {
                    HistoryCommand::Changed(
                        guarded_as(client, account, move |client| async move {
                            client.remove_history(&id).await
                        })
                        .await,
                    )
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            HistoryCommand::Loaded(loaded) => {
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(HistoryOutput::AccountInvalidated);
                    return;
                }
                match loaded.result {
                    Ok(history) => self.render(history),
                    Err(error) => self.content.set(ContentState::Error(&error)),
                }
            }
            HistoryCommand::Changed(changed) => {
                if !self.state.accepts(&changed.account) {
                    let _ = sender.output(HistoryOutput::AccountInvalidated);
                    return;
                }
                match changed.result {
                    Ok(()) => sender.input(HistoryMsg::Reload),
                    Err(error) => {
                        let _ = sender.output(HistoryOutput::Warning(error));
                        sender.input(HistoryMsg::Reload);
                    }
                }
            }
        }
    }
}

impl HistoryView {
    fn render(&mut self, history: SyncedHistory) {
        let mut rows = self.rows.guard();
        rows.clear();
        if history.entries.is_empty() {
            drop(rows);
            self.content.set(ContentState::Empty(
                "document-open-recent-symbolic",
                tr("History Is Empty"),
            ));
            return;
        }
        for entry in history.entries {
            let progress = progress_of(&entry, &history.local);
            rows.push_back((entry, progress));
        }
        drop(rows);
        self.content.set(ContentState::Content);
    }
}

/// Local playback progress for a server history entry, if this device has any.
fn progress_of(
    entry: &ServerHistoryEntry,
    local: &movo_core::storage::history::WatchHistory,
) -> f64 {
    let Some(media_id) = entry.media_id() else {
        return 0.0;
    };
    local
        .entries
        .iter()
        .filter(|local| local.media_id == media_id)
        .max_by_key(|local| local.updated_at)
        .map(|local| local.progress_fraction())
        .unwrap_or(0.0)
}

pub struct HistoryRow {
    entry: ServerHistoryEntry,
    progress: f64,
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for HistoryRow {
    type Init = (ServerHistoryEntry, f64);
    type Input = ();
    type Output = HistoryMsg;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_use_markup: false,
            set_title: &self.entry.title,
            set_subtitle: &subtitle(&self.entry),
            set_activatable: true,
            set_opacity: if self.entry.is_watched { 0.65 } else { 1.0 },

            add_suffix = &gtk::ProgressBar {
                set_fraction: self.progress,
                set_width_request: 60,
                set_valign: gtk::Align::Center,
                set_visible: !self.entry.is_watched && self.progress > 0.0,
            },

            add_suffix = &gtk::Button {
                set_icon_name: if self.entry.is_watched {
                    "object-select-symbolic"
                } else {
                    "checkbox-symbolic"
                },
                set_tooltip_text: Some(tr("Toggle Watched Status")),
                set_has_frame: false,
                set_valign: gtk::Align::Center,
                connect_clicked[sender, id = self.entry.id.clone(), watched = self.entry.is_watched] => move |button| {
                    button.set_sensitive(false);
                    sender.output(HistoryMsg::SetWatched(id.clone(), !watched)).ok();
                },
            },

            add_suffix = &gtk::Button {
                set_icon_name: "user-trash-symbolic",
                set_tooltip_text: Some(tr("Remove from History")),
                set_has_frame: false,
                set_valign: gtk::Align::Center,
                connect_clicked[sender, id = self.entry.id.clone()] => move |_| {
                    sender.output(HistoryMsg::Remove(id.clone())).ok();
                },
            },

            connect_activated[sender, item = as_media(&self.entry)] => move |_| {
                sender.output(HistoryMsg::Open(item.clone())).ok();
            },
        }
    }

    fn init_model(
        (entry, progress): Self::Init,
        _index: &relm4::factory::DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { entry, progress }
    }
}

fn subtitle(entry: &ServerHistoryEntry) -> String {
    [
        entry.info.as_deref(),
        entry.additional_info.as_deref(),
        entry.date.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" • ")
}

fn as_media(entry: &ServerHistoryEntry) -> MediaItem {
    MediaItem {
        id: entry.media_id().unwrap_or_default(),
        title: entry.title.clone(),
        orig_title: None,
        url: entry.url.clone(),
        poster_url: entry.poster_url.clone(),
        year: None,
        category: None,
        rating: None,
        info: entry.info.clone(),
    }
}
