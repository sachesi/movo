use crate::ui::content::{ContentStack, ContentState};
use crate::ui::poster_grid::{poster_grid, PosterGrid, PosterItem};
use movo_core::client::models::MediaItem;
use movo_core::error::ClientError;
use relm4::gtk::{self, prelude::*};

/// Distance from the bottom of the list at which the next page is requested.
const PREFETCH_MARGIN: f64 = 350.0;

/// A scrolling poster grid that loads more pages as it is scrolled.
///
/// Owns the paging state every browsing view needs: which page is in flight,
/// whether the provider has more to give, and what to show while the first
/// page is loading or when it comes back empty or failed.
pub struct PagedGrid {
    pub grid: PosterGrid,
    pub content: ContentStack,
    scrolled: gtk::ScrolledWindow,
    loading_title: &'static str,
    empty_icon: &'static str,
    empty_title: &'static str,
    loaded_page: usize,
    pending: Option<usize>,
    has_more: bool,
}

impl PagedGrid {
    pub fn new(
        loading_title: &'static str,
        empty_icon: &'static str,
        empty_title: &'static str,
        error_title: &'static str,
        on_activate: impl Fn(MediaItem) + 'static,
        on_scrolled_to_end: impl Fn() + 'static,
    ) -> Self {
        let grid = poster_grid(on_activate);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .hexpand(true)
            .child(&grid.view)
            .build();

        scrolled
            .vadjustment()
            .connect_value_changed(move |adjustment| {
                let bottom = adjustment.value() + adjustment.page_size();
                if adjustment.upper() > 0.0 && bottom >= adjustment.upper() - PREFETCH_MARGIN {
                    on_scrolled_to_end();
                }
            });

        let content = ContentStack::new(&scrolled, error_title);
        content.set(ContentState::Loading(loading_title));

        Self {
            grid,
            content,
            scrolled,
            loading_title,
            empty_icon,
            empty_title,
            loaded_page: 0,
            pending: None,
            has_more: true,
        }
    }

    pub fn widget(&self) -> &gtk::Stack {
        &self.content.widget
    }

    /// Start over from the first page, scrolled back to the top.
    pub fn begin_reload(&mut self) -> usize {
        self.loaded_page = 0;
        self.has_more = true;
        self.pending = Some(1);
        self.scrolled.vadjustment().set_value(0.0);
        self.content.set(ContentState::Loading(self.loading_title));
        1
    }

    /// The next page to request, or `None` if one is already in flight or the
    /// provider has nothing more to give.
    pub fn begin_next_page(&mut self) -> Option<usize> {
        if self.pending.is_some() || !self.has_more {
            return None;
        }
        let page = self.loaded_page + 1;
        self.pending = Some(page);
        Some(page)
    }

    /// Whether a completed request is still the one being awaited.
    fn is_pending(&self, page: usize) -> bool {
        self.pending == Some(page)
    }

    /// Applies a finished page request.
    ///
    /// A page the grid is no longer awaiting is dropped: it belongs to a
    /// query, category or sort order the user has already moved away from.
    /// Returns a message to surface as a notification when a later page
    /// failed, where a full-page error would hide results still on screen.
    pub fn apply(
        &mut self,
        page: usize,
        result: Result<Vec<MediaItem>, ClientError>,
    ) -> Option<String> {
        if !self.is_pending(page) {
            return None;
        }
        match result {
            Ok(items) => {
                self.accept(page, items);
                None
            }
            Err(error) => self.reject(page, error.to_string()),
        }
    }

    pub fn accept(&mut self, page: usize, items: Vec<MediaItem>) {
        self.pending = None;
        if page == 1 {
            self.grid.clear();
        }
        self.has_more = !items.is_empty();
        if !items.is_empty() {
            self.loaded_page = page;
            self.grid
                .extend_from_iter(items.into_iter().map(PosterItem::new));
        }
        if self.grid.is_empty() {
            self.content
                .set(ContentState::Empty(self.empty_icon, self.empty_title));
        } else {
            self.content.set(ContentState::Content);
        }
    }

    /// Report a failed page. Returns a message to surface as a notification
    /// when items are already on screen, where a full-page error would hide
    /// results the user can still use.
    pub fn reject(&mut self, page: usize, error: String) -> Option<String> {
        self.pending = None;
        self.has_more = false;
        if page == 1 {
            self.content.set(ContentState::Error(&error));
            None
        } else {
            Some(error)
        }
    }

    /// Stop paging and show a placeholder instead of results, for a view that
    /// has nothing to load yet: signed out, or waiting for a query.
    pub fn set_placeholder(&mut self, icon: &str, title: &str) {
        self.pending = None;
        self.has_more = false;
        self.grid.clear();
        self.content.set(ContentState::Empty(icon, title));
    }

    pub fn set_signed_out(&mut self, title: &str) {
        self.pending = None;
        self.has_more = false;
        self.grid.clear();
        self.content.set(ContentState::SignedOut(title));
    }

    /// Placeholder for a grid that has not started paging yet.
    pub fn set_placeholder_at_start(&self, icon: &str, title: &str) {
        self.content.set(ContentState::Empty(icon, title));
    }

    pub fn set_loading_state(&self) {
        self.content.set(ContentState::Loading(self.loading_title));
    }
}
