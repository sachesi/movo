use crate::api::{guarded, Guarded};
use crate::i18n::{tr, trn};
use crate::state::AppState;
use crate::ui::paged_grid::PagedGrid;
use movo_core::client::models::{Collection, MediaItem};
use relm4::gtk::{self, prelude::*};
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;

pub struct CollectionsView {
    state: Rc<AppState>,
    grid: PagedGrid,
}

#[derive(Debug)]
pub enum CollectionsMsg {
    Reload,
    LoadNextPage,
    Open(MediaItem),
}

#[derive(Debug)]
pub enum CollectionsOutput {
    /// Title and provider path of the collection to open.
    OpenPath(String, String),
    AccountInvalidated,
    Warning(String),
}

#[derive(Debug)]
pub struct PageLoaded {
    page: usize,
    loaded: Guarded<Vec<Collection>>,
}

#[relm4::component(pub)]
impl Component for CollectionsView {
    type Init = Rc<AppState>;
    type Input = CollectionsMsg;
    type Output = CollectionsOutput;
    type CommandOutput = PageLoaded;

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
        let open_sender = sender.clone();
        let scroll_sender = sender.clone();
        let grid = PagedGrid::new(
            tr("Loading Collections"),
            "folder-videos-symbolic",
            tr("No Collections"),
            tr("Collections Did Not Load"),
            move |item| open_sender.input(CollectionsMsg::Open(item)),
            move || scroll_sender.input(CollectionsMsg::LoadNextPage),
        );

        let model = CollectionsView { state, grid };
        let content = model.grid.widget().clone();
        let widgets = view_output!();
        sender.input(CollectionsMsg::Reload);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            CollectionsMsg::Reload => {
                let page = self.grid.begin_reload();
                self.request_page(page, &sender);
            }
            CollectionsMsg::LoadNextPage => {
                if let Some(page) = self.grid.begin_next_page() {
                    self.request_page(page, &sender);
                }
            }
            CollectionsMsg::Open(item) => {
                let _ = sender.output(CollectionsOutput::OpenPath(item.title, item.url));
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
            let _ = sender.output(CollectionsOutput::AccountInvalidated);
            return;
        }
        let rows = loaded
            .result
            .map(|collections| collections.iter().map(as_media).collect());
        if let Some(message) = self.grid.apply(page, rows) {
            let _ = sender.output(CollectionsOutput::Warning(message));
        }
    }
}

impl CollectionsView {
    fn request_page(&mut self, page: usize, sender: &ComponentSender<Self>) {
        let client = self.state.client.clone();
        sender.oneshot_command(async move {
            let loaded = guarded(client, move |client| async move {
                client.fetch_collections(page).await
            })
            .await;
            PageLoaded { page, loaded }
        });
    }
}

/// Collections are shown with the same card as titles; the poster grid is the
/// only list widget the app needs.
fn as_media(collection: &Collection) -> MediaItem {
    MediaItem {
        id: 0,
        title: collection.title.clone(),
        orig_title: None,
        url: collection.url.clone(),
        poster_url: collection.image_url.clone(),
        year: None,
        category: None,
        rating: None,
        info: Some(trn("{} title", "{} titles", collection.count as u64)),
    }
}
