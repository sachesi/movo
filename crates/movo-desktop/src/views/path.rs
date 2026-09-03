use crate::api::{guarded, Guarded};
use crate::i18n::tr;
use crate::state::AppState;
use crate::ui::paged_grid::PagedGrid;
use movo_core::client::models::MediaItem;
use relm4::gtk::{self, prelude::*};
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;

/// Results for one provider path: a collection, a genre or a discovery link.
pub struct PathView {
    state: Rc<AppState>,
    path: String,
    grid: PagedGrid,
}

#[derive(Debug)]
pub enum PathMsg {
    LoadNextPage,
    Open(MediaItem),
}

#[derive(Debug)]
pub enum PathOutput {
    Open(MediaItem),
    AccountInvalidated,
    Warning(String),
}

#[derive(Debug)]
pub struct PageLoaded {
    page: usize,
    loaded: Guarded<Vec<MediaItem>>,
}

#[relm4::component(pub)]
impl Component for PathView {
    type Init = (Rc<AppState>, String);
    type Input = PathMsg;
    type Output = PathOutput;
    type CommandOutput = PageLoaded;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            #[local_ref]
            content -> gtk::Stack {},
        }
    }

    fn init(
        (state, path): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let open_sender = sender.clone();
        let scroll_sender = sender.clone();
        let grid = PagedGrid::new(
            tr("Loading"),
            "folder-videos-symbolic",
            tr("Nothing Found"),
            tr("This List Did Not Load"),
            move |item| open_sender.input(PathMsg::Open(item)),
            move || scroll_sender.input(PathMsg::LoadNextPage),
        );

        let mut model = PathView { state, path, grid };
        let content = model.grid.widget().clone();
        let widgets = view_output!();

        let page = model.grid.begin_reload();
        model.request_page(page, &sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PathMsg::LoadNextPage => {
                if let Some(page) = self.grid.begin_next_page() {
                    self.request_page(page, &sender);
                }
            }
            PathMsg::Open(item) => {
                let _ = sender.output(PathOutput::Open(item));
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let PageLoaded { page, loaded } = message;
        if !self.state.accepts(&loaded.account) {
            let _ = sender.output(PathOutput::AccountInvalidated);
            return;
        }
        if let Some(message) = self.grid.apply(page, loaded.result) {
            let _ = sender.output(PathOutput::Warning(message));
        }
    }
}

impl PathView {
    fn request_page(&mut self, page: usize, sender: &ComponentSender<Self>) {
        let client = self.state.client.clone();
        let path = self.path.clone();
        sender.oneshot_command(async move {
            let loaded = guarded(client, move |client| async move {
                client.fetch_path(&path, page).await
            })
            .await;
            PageLoaded { page, loaded }
        });
    }
}
