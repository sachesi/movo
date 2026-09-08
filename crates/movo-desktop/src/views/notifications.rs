use crate::api::{guarded_as, Guarded};
use crate::i18n::tr;
use crate::state::{account_of, Account, AppState};
use crate::ui::content::{ContentStack, ContentState};
use movo_core::client::models::{AccountData, NotificationGroup};
use movo_core::storage::seen_notifications::SeenNotifications;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;

pub struct NotificationsView {
    state: Rc<AppState>,
    account: Option<Account>,
    groups: gtk::Box,
    content: ContentStack,
    seen: SeenNotifications,
    listed: Vec<String>,
}

#[derive(Debug)]
pub enum NotificationsMsg {
    Reload,
    Open(String, String),
}

#[derive(Debug)]
pub enum NotificationsOutput {
    /// Title and details URL of the episode to open.
    Open(String, String),
    AccountInvalidated,
    /// How many listed notifications the account has not opened yet.
    UnreadCount(usize),
}

#[relm4::component(pub)]
impl Component for NotificationsView {
    type Init = Rc<AppState>;
    type Input = NotificationsMsg;
    type Output = NotificationsOutput;
    type CommandOutput = (SeenNotifications, Guarded<AccountData>);

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
        let groups = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_start(18)
            .margin_end(18)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&groups)
            .build();

        let content = ContentStack::new(&scrolled, tr("Notifications Did Not Load"));

        let model = NotificationsView {
            state,
            account: None,
            groups,
            content,
            seen: SeenNotifications::default(),
            listed: Vec::new(),
        };
        let content = model.content.widget.clone();
        let widgets = view_output!();
        sender.input(NotificationsMsg::Reload);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            NotificationsMsg::Reload => {
                let Some(user) = self.state.user() else {
                    self.account = None;
                    self.clear();
                    self.content
                        .set(ContentState::SignedOut(tr("Sign In to See Notifications")));
                    let _ = sender.output(NotificationsOutput::UnreadCount(0));
                    return;
                };
                self.account = account_of(&self.state.client);
                let account = self.account.clone();
                self.content
                    .set(ContentState::Loading(tr("Loading Notifications")));
                let client = self.state.client.clone();
                let user_id = user.user_id.clone();
                sender.oneshot_command(async move {
                    let seen = relm4::spawn_blocking(move || SeenNotifications::load(&user_id))
                        .await
                        .unwrap_or_default();
                    let data = guarded_as(client, account, |client| async move {
                        client.account_data().await
                    })
                    .await;
                    (seen, data)
                });
            }
            NotificationsMsg::Open(title, url) => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(NotificationsOutput::AccountInvalidated);
                    return;
                }
                if let Some(user) = self.state.user() {
                    let listed = self.listed.clone();
                    self.seen.urls.insert(url.clone());
                    self.seen.urls.retain(|seen_url| listed.contains(seen_url));
                    let _ = sender.output(NotificationsOutput::UnreadCount(self.unread()));

                    let user_id = user.user_id.clone();
                    let seen_url = url.clone();
                    relm4::spawn_blocking(move || {
                        if let Err(error) =
                            SeenNotifications::mark_seen_for(&user_id, &seen_url, &listed)
                        {
                            log::warn!("Could not record the opened notification: {error}");
                        }
                    });
                }
                let _ = sender.output(NotificationsOutput::Open(title, url));
            }
        }
    }

    fn update_cmd(
        &mut self,
        (seen, message): Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if !self.state.accepts(&message.account) {
            let _ = sender.output(NotificationsOutput::AccountInvalidated);
            return;
        }
        self.seen = seen;
        match message.result {
            Ok(data) => {
                self.render(data.notifications, &sender);
                let _ = sender.output(NotificationsOutput::UnreadCount(self.unread()));
            }
            Err(error) => self.content.set(ContentState::Error(&error)),
        }
    }
}

impl NotificationsView {
    fn clear(&mut self) {
        while let Some(child) = self.groups.first_child() {
            self.groups.remove(&child);
        }
        self.listed.clear();
    }

    fn unread(&self) -> usize {
        self.listed
            .iter()
            .filter(|url| !self.seen.contains(url))
            .count()
    }

    fn render(&mut self, groups: Vec<NotificationGroup>, sender: &ComponentSender<Self>) {
        self.clear();
        for group in groups.iter().filter(|group| !group.items.is_empty()) {
            let date = gtk::Label::builder()
                .label(&group.date)
                .halign(gtk::Align::Start)
                .margin_start(12)
                .build();
            date.add_css_class("heading");
            date.add_css_class("accent");

            let list = crate::ui::activatable_list();

            for item in &group.items {
                self.listed.push(item.url.clone());
                let row = adw::ActionRow::builder()
                    .use_markup(false)
                    .title(&item.title)
                    .subtitle(&item.info)
                    .activatable(true)
                    .build();
                row.add_prefix(&gtk::Image::from_icon_name(
                    "preferences-system-notifications-symbolic",
                ));
                if !self.seen.contains(&item.url) {
                    let unread = gtk::Image::from_icon_name("media-record-symbolic");
                    unread.add_css_class("accent");
                    unread.set_tooltip_text(Some(tr("Not opened yet")));
                    row.add_suffix(&unread);
                }
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

                let sender = sender.clone();
                let title = item.title.clone();
                let url = item.url.clone();
                row.connect_activated(move |_| {
                    sender.input(NotificationsMsg::Open(title.clone(), url.clone()));
                });
                list.append(&row);
            }

            self.groups.append(&date);
            self.groups.append(&list);
        }

        if self.listed.is_empty() {
            self.content.set(ContentState::Empty(
                "preferences-system-notifications-symbolic",
                tr("No Notifications"),
            ));
        } else {
            self.content.set(ContentState::Content);
        }
    }
}
