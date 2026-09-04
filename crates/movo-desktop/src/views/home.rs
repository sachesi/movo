use crate::api::{guarded, Guarded};
use crate::i18n::tr;
use crate::state::AppState;
use crate::ui::content::{ContentStack, ContentState};
use crate::ui::poster_grid::{poster_row, PosterItem, PosterRow};
use movo_core::client::models::{HomeSection, MediaItem};
use relm4::gtk::{self, prelude::*};
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;

pub struct HomeView {
    state: Rc<AppState>,
    sections: gtk::Box,
    content: ContentStack,
    /// Kept alive with their widgets: a dropped view model empties its rail.
    rows: Vec<PosterRow>,
}

#[derive(Debug)]
pub enum HomeMsg {
    Reload,
    Open(MediaItem),
    /// A section's "View All" was activated, naming its provider path.
    OpenSection(String),
}

#[derive(Debug)]
pub enum HomeOutput {
    Open(MediaItem),
    /// Title and provider path of the section to open in full.
    OpenPath(String, String),
    AccountInvalidated,
}

#[relm4::component(pub)]
impl Component for HomeView {
    type Init = Rc<AppState>;
    type Input = HomeMsg;
    type Output = HomeOutput;
    type CommandOutput = Guarded<Vec<HomeSection>>;

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
        let sections = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&sections)
            .build();

        let content = ContentStack::new(&scrolled, tr("Home Did Not Load"));
        content.on_retry({
            let sender = sender.clone();
            move || sender.input(HomeMsg::Reload)
        });

        let model = HomeView {
            state,
            sections,
            content,
            rows: Vec::new(),
        };

        let content = model.content.widget.clone();
        let widgets = view_output!();
        sender.input(HomeMsg::Reload);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            HomeMsg::Reload => {
                self.content.set(ContentState::Loading(tr("Loading Home")));
                let client = self.state.client.clone();
                sender.oneshot_command(async move {
                    guarded(client, |client| async move { client.home().await }).await
                });
            }
            HomeMsg::Open(item) => {
                let _ = sender.output(HomeOutput::Open(item));
            }
            HomeMsg::OpenSection(id) => {
                if let Some(path) = section_path(&id) {
                    let _ = sender.output(HomeOutput::OpenPath(
                        section_title(&id).to_string(),
                        path.to_string(),
                    ));
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if !self.state.accepts(&message.account) {
            let _ = sender.output(HomeOutput::AccountInvalidated);
            return;
        }

        match message.result {
            Ok(sections) => self.render(sections, &sender),
            Err(error) => self.content.set(ContentState::Error(&error)),
        }
    }
}

impl HomeView {
    fn render(&mut self, sections: Vec<HomeSection>, sender: &ComponentSender<Self>) {
        while let Some(child) = self.sections.first_child() {
            self.sections.remove(&child);
        }
        self.rows.clear();

        for section in sections
            .into_iter()
            .filter(|section| !section.items.is_empty())
        {
            let title = gtk::Label::builder()
                .label(section_title(&section.id))
                .halign(gtk::Align::Start)
                .hexpand(true)
                .build();
            title.add_css_class("title-2");

            let view_all = gtk::Button::builder()
                .label(tr("View All"))
                .valign(gtk::Align::Center)
                .visible(section_path(&section.id).is_some())
                .build();
            view_all.add_css_class("flat");
            {
                let sender = sender.clone();
                let id = section.id.clone();
                view_all.connect_clicked(move |_| {
                    sender.input(HomeMsg::OpenSection(id.clone()));
                });
            }

            let header = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .margin_start(18)
                .margin_end(18)
                .build();
            header.append(&title);
            header.append(&view_all);

            let open_sender = sender.clone();
            let mut row = poster_row(move |item| open_sender.input(HomeMsg::Open(item)));
            row.extend_from_iter(section.items.into_iter().map(PosterItem::new));

            let scrolled = gtk::ScrolledWindow::builder()
                .vscrollbar_policy(gtk::PolicyType::Never)
                .margin_start(18)
                .margin_end(18)
                .child(&row.view)
                .build();

            self.sections.append(&header);
            self.sections.append(&scrolled);
            self.rows.push(row);
        }

        if self.rows.is_empty() {
            self.content.set(ContentState::Empty(
                "user-home-symbolic",
                tr("Home Is Empty"),
            ));
        } else {
            self.content.set(ContentState::Content);
        }
    }
}

fn section_title(id: &str) -> &'static str {
    match id {
        "hot" => tr("Hot Now"),
        "new" => tr("New Releases"),
        "watching" => tr("Watching Now"),
        "popular" => tr("Popular"),
        "awaiting" => tr("Coming Soon"),
        _ => tr("Catalog"),
    }
}

/// The listing page behind a section, mirroring the paths `home()` fetches.
/// "hot" comes from an AJAX slider and has no page of its own.
fn section_path(id: &str) -> Option<&'static str> {
    match id {
        "new" => Some("new"),
        "watching" => Some("new?filter=watching"),
        "popular" => Some("new?filter=popular"),
        "awaiting" => Some("announce"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{section_path, section_title};

    #[test]
    fn provider_home_sections_have_stable_titles() {
        assert_eq!(section_title("hot"), "Hot Now");
        assert_eq!(section_title("awaiting"), "Coming Soon");
        assert_eq!(section_title("unknown"), "Catalog");
    }

    #[test]
    fn only_sections_with_a_listing_page_can_be_opened_in_full() {
        assert_eq!(section_path("watching"), Some("new?filter=watching"));
        assert_eq!(section_path("awaiting"), Some("announce"));
        assert_eq!(section_path("hot"), None);
    }
}
