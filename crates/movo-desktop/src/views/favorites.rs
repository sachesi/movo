use crate::api::{guarded_as, Guarded};
use crate::i18n::tr;
use crate::state::{account_of, Account, AppState};
use crate::ui::paged_grid::PagedGrid;
use movo_core::client::models::{FavoritesCollection, MediaItem};
use relm4::gtk::{self, prelude::*};
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;

pub struct FavoritesView {
    state: Rc<AppState>,
    account: Option<Account>,
    categories: gtk::DropDown,
    category_ids: Vec<Option<i64>>,
    selected: Option<i64>,
    grid: PagedGrid,
}

#[derive(Debug)]
pub enum FavoritesMsg {
    Reload,
    SelectCategory(u32),
    LoadNextPage,
    Open(MediaItem),
}

#[derive(Debug)]
pub enum FavoritesOutput {
    Open(MediaItem),
    AccountInvalidated,
    Warning(String),
}

#[derive(Debug)]
pub enum FavoritesCommand {
    Categories(Guarded<Vec<FavoritesCollection>>),
    Page {
        page: usize,
        loaded: Guarded<Vec<MediaItem>>,
    },
}

#[relm4::component(pub)]
impl Component for FavoritesView {
    type Init = Rc<AppState>;
    type Input = FavoritesMsg;
    type Output = FavoritesOutput;
    type CommandOutput = FavoritesCommand;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_margin_start: 16,
            set_margin_end: 16,
            set_margin_top: 12,
            set_margin_bottom: 12,

            #[local_ref]
            categories -> gtk::DropDown {
                set_halign: gtk::Align::Center,
                set_tooltip_text: Some(tr("Collection")),
                connect_selected_notify[sender] => move |dropdown| {
                    sender.input(FavoritesMsg::SelectCategory(dropdown.selected()));
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
        let categories = gtk::DropDown::from_strings(&[]);
        categories.set_visible(false);

        let open_sender = sender.clone();
        let scroll_sender = sender.clone();
        let grid = PagedGrid::new(
            tr("Loading Favorites"),
            "starred-symbolic",
            tr("No Favorites Yet"),
            tr("Favorites Did Not Load"),
            move |item| open_sender.input(FavoritesMsg::Open(item)),
            move || scroll_sender.input(FavoritesMsg::LoadNextPage),
        );

        let model = FavoritesView {
            state,
            account: None,
            categories: categories.clone(),
            category_ids: Vec::new(),
            selected: None,
            grid,
        };

        let content = model.grid.widget().clone();
        let widgets = view_output!();
        sender.input(FavoritesMsg::Reload);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            FavoritesMsg::Reload => {
                if self.state.user().is_none() {
                    self.account = None;
                    self.categories.set_visible(false);
                    self.grid
                        .set_signed_out(tr("Sign In to See Your Favorites"));
                    return;
                }
                self.account = account_of(&self.state.client);
                let account = self.account.clone();
                self.grid.set_loading_state();
                let client = self.state.client.clone();
                sender.oneshot_command(async move {
                    FavoritesCommand::Categories(
                        guarded_as(client, account, |client| async move {
                            client.fetch_favorites_categories().await
                        })
                        .await,
                    )
                });
            }
            FavoritesMsg::SelectCategory(index) => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(FavoritesOutput::AccountInvalidated);
                    return;
                }
                let selected = self.category_ids.get(index as usize).copied().flatten();
                if selected != self.selected {
                    self.selected = selected;
                    let page = self.grid.begin_reload();
                    self.request_page(page, &sender);
                }
            }
            FavoritesMsg::LoadNextPage => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(FavoritesOutput::AccountInvalidated);
                    return;
                }
                if let Some(page) = self.grid.begin_next_page() {
                    self.request_page(page, &sender);
                }
            }
            FavoritesMsg::Open(item) => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(FavoritesOutput::AccountInvalidated);
                    return;
                }
                let _ = sender.output(FavoritesOutput::Open(item));
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
            FavoritesCommand::Categories(loaded) => {
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(FavoritesOutput::AccountInvalidated);
                    return;
                }
                match loaded.result {
                    Ok(categories) => {
                        self.show_categories(&categories);
                        let page = self.grid.begin_reload();
                        self.request_page(page, &sender);
                    }
                    Err(error) => {
                        self.grid.reject(1, error.to_string());
                    }
                }
            }
            FavoritesCommand::Page { page, loaded } => {
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(FavoritesOutput::AccountInvalidated);
                    return;
                }
                if let Some(message) = self.grid.apply(page, loaded.result) {
                    let _ = sender.output(FavoritesOutput::Warning(message));
                }
            }
        }
    }
}

impl FavoritesView {
    fn show_categories(&mut self, categories: &[FavoritesCollection]) {
        let labels = categories.iter().map(category_label).collect::<Vec<_>>();
        let label_refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.category_ids = categories.iter().map(|category| category.id).collect();

        let selected = self
            .category_ids
            .iter()
            .position(|id| *id == self.selected)
            .unwrap_or(0);
        self.selected = self.category_ids.get(selected).copied().flatten();

        let guard = self.categories.freeze_notify();
        self.categories
            .set_model(Some(&gtk::StringList::new(&label_refs)));
        self.categories.set_selected(selected as u32);
        drop(guard);
        self.categories.set_visible(!labels.is_empty());
    }

    fn request_page(&mut self, page: usize, sender: &ComponentSender<Self>) {
        let client = self.state.client.clone();
        let category = self.selected;
        let account = self.account.clone();
        sender.oneshot_command(async move {
            let loaded = guarded_as(client, account, move |client| async move {
                client.fetch_favorites_page(category, page).await
            })
            .await;
            FavoritesCommand::Page { page, loaded }
        });
    }
}

fn category_label(category: &FavoritesCollection) -> String {
    if category.count == 0 {
        category.name.clone()
    } else {
        format!("{} ({})", category.name, category.count)
    }
}

#[cfg(test)]
mod tests {
    use super::category_label;
    use movo_core::client::models::FavoritesCollection;

    #[test]
    fn category_label_hides_a_zero_count() {
        let mut category = FavoritesCollection {
            id: Some(1),
            name: "Watch later".to_string(),
            url: String::new(),
            count: 0,
        };
        assert_eq!(category_label(&category), "Watch later");
        category.count = 12;
        assert_eq!(category_label(&category), "Watch later (12)");
    }
}
