use crate::api::{guarded, Guarded};
use crate::i18n::tr;
use crate::state::AppState;
use crate::ui::paged_grid::PagedGrid;
use movo_core::client::models::{CatalogCategory, MediaItem};
use relm4::gtk::{self, prelude::*};
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;

pub struct CatalogView {
    state: Rc<AppState>,
    category: CatalogCategory,
    filter: &'static str,
    grid: PagedGrid,
}

#[derive(Debug)]
pub enum CatalogMsg {
    SelectCategory(u32),
    SelectFilter(u32),
    Reload,
    LoadNextPage,
    Open(MediaItem),
}

#[derive(Debug)]
pub enum CatalogOutput {
    Open(MediaItem),
    AccountInvalidated,
    /// A page beyond the first failed; the loaded items stay on screen.
    Warning(String),
}

#[derive(Debug)]
pub struct PageLoaded {
    page: usize,
    loaded: Guarded<Vec<MediaItem>>,
}

#[relm4::component(pub)]
impl Component for CatalogView {
    type Init = Rc<AppState>;
    type Input = CatalogMsg;
    type Output = CatalogOutput;
    type CommandOutput = PageLoaded;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_margin_start: 16,
            set_margin_end: 16,
            set_margin_top: 12,
            set_margin_bottom: 12,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_homogeneous: true,
                set_halign: gtk::Align::Center,

                #[local_ref]
                category_dropdown -> gtk::DropDown {
                    set_hexpand: true,
                    set_tooltip_text: Some(tr("Category")),
                    connect_selected_notify[sender] => move |dropdown| {
                        sender.input(CatalogMsg::SelectCategory(dropdown.selected()));
                    },
                },

                #[local_ref]
                filter_dropdown -> gtk::DropDown {
                    set_hexpand: true,
                    set_tooltip_text: Some(tr("Sort")),
                    connect_selected_notify[sender] => move |dropdown| {
                        sender.input(CatalogMsg::SelectFilter(dropdown.selected()));
                    },
                },
            },

            #[local_ref]
            content -> gtk::Stack {},
        }
    }

    fn init(
        state: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let category_dropdown = gtk::DropDown::from_strings(&[
            tr("All"),
            tr("Movies"),
            tr("Series"),
            tr("Cartoons"),
            tr("Anime"),
        ]);
        let filter_dropdown = gtk::DropDown::from_strings(&[
            tr("Popular"),
            tr("Watching Now"),
            tr("New Releases"),
            tr("Recently Added"),
        ]);

        let open_sender = sender.clone();
        let scroll_sender = sender.clone();
        let grid = PagedGrid::new(
            tr("Loading Catalog"),
            "folder-videos-symbolic",
            tr("The Catalog Is Empty"),
            tr("The Catalog Did Not Load"),
            move |item| open_sender.input(CatalogMsg::Open(item)),
            move || scroll_sender.input(CatalogMsg::LoadNextPage),
        );

        let model = CatalogView {
            state,
            category: CatalogCategory::All,
            filter: "popular",
            grid,
        };

        let content = model.grid.widget().clone();
        let widgets = view_output!();
        sender.input(CatalogMsg::Reload);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            CatalogMsg::SelectCategory(index) => {
                self.category = category_at(index);
                sender.input(CatalogMsg::Reload);
            }
            CatalogMsg::SelectFilter(index) => {
                self.filter = filter_at(index);
                sender.input(CatalogMsg::Reload);
            }
            CatalogMsg::Reload => {
                let page = self.grid.begin_reload();
                self.request_page(page, &sender);
            }
            CatalogMsg::LoadNextPage => {
                if let Some(page) = self.grid.begin_next_page() {
                    self.request_page(page, &sender);
                }
            }
            CatalogMsg::Open(item) => {
                let _ = sender.output(CatalogOutput::Open(item));
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
            let _ = sender.output(CatalogOutput::AccountInvalidated);
            return;
        }

        if let Some(message) = self.grid.apply(page, loaded.result) {
            let _ = sender.output(CatalogOutput::Warning(message));
        }
    }
}

impl CatalogView {
    fn request_page(&mut self, page: usize, sender: &ComponentSender<Self>) {
        let client = self.state.client.clone();
        let category = self.category;
        let filter = self.filter;
        sender.oneshot_command(async move {
            let loaded = guarded(client, move |client| async move {
                client.fetch_catalog(category, Some(filter), page).await
            })
            .await;
            PageLoaded { page, loaded }
        });
    }
}

fn category_at(index: u32) -> CatalogCategory {
    match index {
        1 => CatalogCategory::Films,
        2 => CatalogCategory::Series,
        3 => CatalogCategory::Cartoons,
        4 => CatalogCategory::Animation,
        _ => CatalogCategory::All,
    }
}

fn filter_at(index: u32) -> &'static str {
    match index {
        1 => "watching",
        2 => "new",
        3 => "last",
        _ => "popular",
    }
}

#[cfg(test)]
mod tests {
    use super::{category_at, filter_at};
    use movo_core::client::models::CatalogCategory;

    #[test]
    fn dropdown_indexes_map_to_provider_values() {
        assert_eq!(category_at(0), CatalogCategory::All);
        assert_eq!(category_at(4), CatalogCategory::Animation);
        assert_eq!(category_at(99), CatalogCategory::All);
        assert_eq!(filter_at(0), "popular");
        assert_eq!(filter_at(3), "last");
        assert_eq!(filter_at(99), "popular");
    }
}
