use crate::api::{guarded, Guarded};
use crate::i18n::tr;
use crate::state::AppState;
use crate::ui::paged_grid::PagedGrid;
use movo_core::client::models::{MediaItem, SearchFilter};
use movo_core::storage::search_history::SearchHistory;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender};
use std::rc::Rc;
use std::time::Duration;

/// Typing pause before a query is sent to the provider.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Shortest query the provider returns useful suggestions for.
const MIN_SUGGESTION_LENGTH: usize = 2;

pub struct SearchView {
    state: Rc<AppState>,
    query: String,
    /// Identifies the newest keystroke so older debounced work can be dropped.
    query_id: u64,
    entry: gtk::SearchEntry,
    suggestions: gtk::FlowBox,
    recent: gtk::FlowBox,
    recent_section: gtk::Box,
    grid: PagedGrid,
}

#[derive(Debug)]
pub enum SearchMsg {
    QueryChanged(String),
    Search(String),
    LoadNextPage,
    Open(MediaItem),
    ShowFilters,
    ClearRecent,
}

#[derive(Debug)]
pub enum SearchOutput {
    Open(MediaItem),
    /// Title and provider path chosen in the advanced search dialog.
    OpenPath(String, String),
    AccountInvalidated,
    Warning(String),
}

#[derive(Debug)]
pub enum SearchCommand {
    Debounced {
        id: u64,
        query: String,
    },
    Suggestions {
        id: u64,
        values: Vec<String>,
    },
    Page {
        page: usize,
        loaded: Guarded<Vec<MediaItem>>,
    },
    Filters(Guarded<Vec<SearchFilter>>),
    Recent(Vec<String>),
}

#[relm4::component(pub)]
impl Component for SearchView {
    type Init = Rc<AppState>;
    type Input = SearchMsg;
    type Output = SearchOutput;
    type CommandOutput = SearchCommand;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_margin_start: 16,
            set_margin_end: 16,
            set_margin_top: 16,
            set_margin_bottom: 16,

            adw::Clamp {
                set_maximum_size: 700,
                set_tightening_threshold: 400,
                set_margin_start: 12,
                set_margin_end: 12,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,

                    #[local_ref]
                    entry -> gtk::SearchEntry {
                        set_placeholder_text: Some(tr("Search movies, series, cartoons...")),
                        set_hexpand: true,
                        connect_search_changed[sender] => move |entry| {
                            sender.input(SearchMsg::QueryChanged(entry.text().to_string()));
                        },
                        connect_activate[sender] => move |entry| {
                            sender.input(SearchMsg::Search(entry.text().to_string()));
                        },
                    },

                    gtk::Button {
                        set_label: tr("Filters"),
                        set_tooltip_text: Some(tr("Advanced Search")),
                        set_valign: gtk::Align::Center,
                        connect_clicked => SearchMsg::ShowFilters,
                    },
                },
            },

            #[local_ref]
            suggestions -> gtk::FlowBox {
                set_selection_mode: gtk::SelectionMode::None,
                set_max_children_per_line: 8,
                set_halign: gtk::Align::Center,
                set_row_spacing: 8,
                set_column_spacing: 8,
                set_visible: false,
            },

            #[local_ref]
            recent_section -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_visible: false,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,

                    gtk::Label {
                        set_label: tr("Recent Searches"),
                        set_halign: gtk::Align::Start,
                        set_hexpand: true,
                        add_css_class: "heading",
                    },

                    gtk::Button {
                        set_label: tr("Clear"),
                        add_css_class: "flat",
                        connect_clicked => SearchMsg::ClearRecent,
                    },
                },

                #[local_ref]
                recent -> gtk::FlowBox {
                    set_selection_mode: gtk::SelectionMode::None,
                    set_max_children_per_line: 8,
                    set_row_spacing: 8,
                    set_column_spacing: 8,
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
        let entry = gtk::SearchEntry::new();
        let suggestions = gtk::FlowBox::new();
        let recent = gtk::FlowBox::new();
        let recent_section = gtk::Box::new(gtk::Orientation::Vertical, 8);

        let open_sender = sender.clone();
        let scroll_sender = sender.clone();
        let grid = PagedGrid::new(
            tr("Searching"),
            "system-search-symbolic",
            tr("Nothing Found"),
            tr("Search Did Not Finish"),
            move |item| open_sender.input(SearchMsg::Open(item)),
            move || scroll_sender.input(SearchMsg::LoadNextPage),
        );

        let model = SearchView {
            state,
            query: String::new(),
            query_id: 0,
            entry: entry.clone(),
            suggestions: suggestions.clone(),
            recent: recent.clone(),
            recent_section: recent_section.clone(),
            grid,
        };

        let content = model.grid.widget().clone();
        let widgets = view_output!();

        model
            .grid
            .set_placeholder_at_start("system-search-symbolic", tr("Search for Something"));
        model.load_recent(&sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SearchMsg::QueryChanged(query) => {
                self.query_id = self.query_id.wrapping_add(1);
                let id = self.query_id;
                sender.oneshot_command(async move {
                    tokio::time::sleep(DEBOUNCE).await;
                    SearchCommand::Debounced { id, query }
                });
            }
            SearchMsg::Search(query) => {
                self.query_id = self.query_id.wrapping_add(1);
                self.start_search(query, &sender);
            }
            SearchMsg::LoadNextPage => {
                if let Some(page) = self.grid.begin_next_page() {
                    self.request_page(page, &sender);
                }
            }
            SearchMsg::Open(item) => {
                let _ = sender.output(SearchOutput::Open(item));
            }
            SearchMsg::ShowFilters => {
                let client = self.state.client.clone();
                sender.oneshot_command(async move {
                    SearchCommand::Filters(
                        guarded(
                            client,
                            |client| async move { client.search_filters().await },
                        )
                        .await,
                    )
                });
            }
            SearchMsg::ClearRecent => {
                if let Some(user) = self.state.user() {
                    if let Err(error) = SearchHistory::clear_for(&user.user_id) {
                        let _ = sender.output(SearchOutput::Warning(error));
                    }
                }
                self.show_recent(&[]);
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            SearchCommand::Debounced { id, query } => {
                if id == self.query_id {
                    self.start_search(query, &sender);
                }
            }
            SearchCommand::Suggestions { id, values } => {
                if id == self.query_id {
                    self.show_suggestions(&values, &sender);
                }
            }
            SearchCommand::Page { page, loaded } => {
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(SearchOutput::AccountInvalidated);
                    return;
                }
                if let Some(message) = self.grid.apply(page, loaded.result) {
                    let _ = sender.output(SearchOutput::Warning(message));
                }
            }
            SearchCommand::Filters(loaded) => match loaded.result {
                Ok(filters) => show_filters(root, filters, &sender),
                Err(error) => {
                    let _ = sender.output(SearchOutput::Warning(error));
                }
            },
            SearchCommand::Recent(values) => self.show_recent(&values),
        }
    }
}

impl SearchView {
    fn start_search(&mut self, query: String, sender: &ComponentSender<Self>) {
        let query = query.trim().to_string();
        if query == self.query {
            return;
        }
        self.query = query.clone();

        if self.query.is_empty() {
            self.suggestions.set_visible(false);
            self.grid
                .set_placeholder("system-search-symbolic", tr("Search for Something"));
            self.load_recent(sender);
            return;
        }

        self.recent_section.set_visible(false);
        self.record_query(&query);

        if query.chars().count() >= MIN_SUGGESTION_LENGTH {
            let client = self.state.client.clone();
            let id = self.query_id;
            let suggestion_query = query.clone();
            sender.oneshot_command(async move {
                let loaded = guarded(client, move |client| async move {
                    client.search_suggestions(&suggestion_query).await
                })
                .await;
                SearchCommand::Suggestions {
                    id,
                    values: loaded.result.unwrap_or_default(),
                }
            });
        } else {
            self.suggestions.set_visible(false);
        }

        let page = self.grid.begin_reload();
        self.request_page(page, sender);
    }

    fn request_page(&mut self, page: usize, sender: &ComponentSender<Self>) {
        let client = self.state.client.clone();
        let query = self.query.clone();
        sender.oneshot_command(async move {
            let loaded = guarded(client, move |client| async move {
                client.search_full(&query, page).await
            })
            .await;
            SearchCommand::Page { page, loaded }
        });
    }

    fn record_query(&self, query: &str) {
        let Some(user) = self.state.user() else {
            return;
        };
        let query = query.to_string();
        relm4::spawn(async move {
            let _ = SearchHistory::add_for(&user.user_id, &query);
        });
    }

    fn load_recent(&self, sender: &ComponentSender<Self>) {
        let Some(user) = self.state.user() else {
            self.recent_section.set_visible(false);
            return;
        };
        sender.oneshot_command(async move {
            let values = relm4::spawn_blocking(move || SearchHistory::load(&user.user_id).values)
                .await
                .unwrap_or_default();
            SearchCommand::Recent(values)
        });
    }

    fn show_recent(&self, values: &[String]) {
        clear(&self.recent);
        for value in values {
            let button = gtk::Button::with_label(value);
            button.add_css_class("pill");
            let entry = self.entry.clone();
            let value = value.clone();
            button.connect_clicked(move |_| entry.set_text(&value));
            self.recent.append(&button);
        }
        self.recent_section
            .set_visible(!values.is_empty() && self.query.is_empty());
    }

    fn show_suggestions(&self, values: &[String], sender: &ComponentSender<Self>) {
        clear(&self.suggestions);
        for value in values {
            let button = gtk::Button::with_label(value);
            button.add_css_class("pill");
            let entry = self.entry.clone();
            let sender = sender.clone();
            let value = value.clone();
            button.connect_clicked(move |_| {
                entry.set_text(&value);
                sender.input(SearchMsg::Search(value.clone()));
            });
            self.suggestions.append(&button);
        }
        self.suggestions.set_visible(!values.is_empty());
    }
}

fn clear(container: &gtk::FlowBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn show_filters(root: &gtk::Box, filters: Vec<SearchFilter>, sender: &ComponentSender<SearchView>) {
    let dialog = adw::PreferencesDialog::builder()
        .title(tr("Advanced Search"))
        .build();
    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::new();

    let category_names = filters
        .iter()
        .map(|filter| filter.name.as_str())
        .collect::<Vec<_>>();
    let category = adw::ComboRow::builder()
        .title(tr("Category"))
        .model(&gtk::StringList::new(&category_names))
        .build();
    let genre = adw::ComboRow::builder().title(tr("Genre")).build();
    let year = adw::ComboRow::builder().title(tr("Year")).build();
    group.add(&category);
    group.add(&genre);
    group.add(&year);

    let filters = Rc::new(filters);
    let show_options = {
        let filters = filters.clone();
        let genre = genre.clone();
        let year = year.clone();
        move |selected: u32| {
            let Some(filter) = filters.get(selected as usize) else {
                return;
            };
            let genres = filter
                .genres
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>();
            let years = filter
                .years
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>();
            genre.set_model(Some(&gtk::StringList::new(&genres)));
            year.set_model(Some(&gtk::StringList::new(&years)));
            genre.set_selected(0);
            year.set_selected(0);
        }
    };
    show_options(0);
    let show_options = Rc::new(show_options);
    let on_category = show_options.clone();
    category.connect_selected_notify(move |row| on_category(row.selected()));

    let search = gtk::Button::builder()
        .label(tr("Search"))
        .halign(gtk::Align::End)
        .build();
    search.add_css_class("suggested-action");
    search.add_css_class("pill");

    let sender = sender.clone();
    let dialog_for_search = dialog.clone();
    search.connect_clicked(move |_| {
        let Some(filter) = filters.get(category.selected() as usize) else {
            return;
        };
        let Some(selected_genre) = filter.genres.get(genre.selected() as usize) else {
            return;
        };
        let selected_year = filter
            .years
            .get(year.selected() as usize)
            .map(|year| year.url.as_str())
            .unwrap_or_default();
        let _ = sender.output(SearchOutput::OpenPath(
            filter.name.clone(),
            search_filter_path(&selected_genre.url, selected_year),
        ));
        dialog_for_search.close();
    });

    let action = adw::ActionRow::new();
    action.add_suffix(&search);
    group.add(&action);
    page.add(&group);
    dialog.add(&page);
    dialog.present(Some(root));
}

fn search_filter_path(genre: &str, year: &str) -> String {
    match year {
        "-1" => genre.strip_suffix("/best").unwrap_or(genre).to_string(),
        "0" | "" => genre.to_string(),
        _ => format!("{genre}{year}"),
    }
}

#[cfg(test)]
mod tests {
    use super::search_filter_path;

    #[test]
    fn filter_path_matches_provider_contract() {
        assert_eq!(search_filter_path("films/best", "-1"), "films");
        assert_eq!(search_filter_path("films/best", "0"), "films/best");
        assert_eq!(search_filter_path("films/best", "2025"), "films/best2025");
    }
}
