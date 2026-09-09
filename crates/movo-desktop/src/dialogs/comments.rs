use crate::api::guarded_as;
use crate::i18n::{tr, trf};
use crate::state::{Account, AppState};
use movo_core::client::models::{Comment, CommentsPage};
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, WidgetExt},
};
use std::cell::Cell;
use std::rc::Rc;

/// Show the comment thread for a title, one page at a time.
pub fn present(
    parent: &impl IsA<gtk::Widget>,
    state: Rc<AppState>,
    post_id: i64,
    account: Option<Account>,
    on_error: impl Fn(String) + 'static,
) {
    let dialog = adw::Dialog::builder()
        .title(tr("Comments"))
        .content_width(620)
        .content_height(680)
        .build();

    let list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(18)
        .margin_end(18)
        .margin_top(18)
        .margin_bottom(18)
        .build();

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();

    let previous = gtk::Button::with_label(tr("Previous"));
    let next = gtk::Button::with_label(tr("Next"));
    let position = gtk::Label::new(None);
    position.add_css_class("dim-label");

    let pager = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .halign(gtk::Align::Center)
        .margin_bottom(12)
        .build();
    pager.append(&previous);
    pager.append(&position);
    pager.append(&next);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scrolled));
    toolbar.add_bottom_bar(&pager);
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(parent));

    let on_error = Rc::new(on_error);
    let page = Rc::new(Cell::new(1usize));
    let pages = Rc::new(Cell::new(1usize));

    let widgets = Pager {
        post_id,
        list,
        previous: previous.clone(),
        next: next.clone(),
        position,
        page: page.clone(),
        pages: pages.clone(),
    };

    {
        let state = state.clone();
        let account = account.clone();
        let widgets = widgets.clone();
        let on_error = on_error.clone();
        previous.connect_clicked(move |_| {
            let target = widgets.page.get().saturating_sub(1).max(1);
            load(
                state.clone(),
                account.clone(),
                widgets.clone(),
                target,
                on_error.clone(),
            );
        });
    }
    {
        let state = state.clone();
        let account = account.clone();
        let widgets = widgets.clone();
        let on_error = on_error.clone();
        next.connect_clicked(move |_| {
            let target = (widgets.page.get() + 1).min(widgets.pages.get().max(1));
            load(
                state.clone(),
                account.clone(),
                widgets.clone(),
                target,
                on_error.clone(),
            );
        });
    }

    load(state, account, widgets, 1, on_error);
}

/// The comment list and its pager, cloned into the page callbacks.
#[derive(Clone)]
struct Pager {
    post_id: i64,
    list: gtk::Box,
    previous: gtk::Button,
    next: gtk::Button,
    position: gtk::Label,
    page: Rc<Cell<usize>>,
    pages: Rc<Cell<usize>>,
}

fn load<F: Fn(String) + 'static>(
    state: Rc<AppState>,
    account: Option<Account>,
    widgets: Pager,
    page: usize,
    on_error: Rc<F>,
) {
    widgets.previous.set_sensitive(false);
    widgets.next.set_sensitive(false);

    let client = state.client.clone();
    let account_state = state.clone();
    let post_id = widgets.post_id;
    relm4::spawn_local(async move {
        let loaded = guarded_as(client, account.clone(), move |client| async move {
            client.fetch_comments(post_id, page).await
        })
        .await;
        if !account_state.accepts(&account) {
            widgets.previous.set_sensitive(widgets.page.get() > 1);
            widgets
                .next
                .set_sensitive(widgets.page.get() < widgets.pages.get());
            on_error(tr("The account changed while comments were loading").to_string());
            return;
        }
        match loaded.result {
            Ok(comments) => {
                render(&widgets.list, &comments, state, on_error);
                widgets.page.set(comments.page);
                widgets.pages.set(comments.total_pages.max(1));
                widgets.position.set_label(&trf(
                    "Page {0} of {1}",
                    &[&comments.page, &widgets.pages.get()],
                ));
                widgets.previous.set_sensitive(comments.page > 1);
                widgets
                    .next
                    .set_sensitive(comments.page < widgets.pages.get());
            }
            Err(error) => {
                widgets.previous.set_sensitive(widgets.page.get() > 1);
                widgets
                    .next
                    .set_sensitive(widgets.page.get() < widgets.pages.get());
                on_error(error);
            }
        }
    });
}

fn render<F: Fn(String) + 'static>(
    list: &gtk::Box,
    comments: &CommentsPage,
    state: Rc<AppState>,
    on_error: Rc<F>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for comment in &comments.items {
        list.append(&comment_card(comment, state.clone(), on_error.clone()));
    }
}

fn comment_card<F: Fn(String) + 'static>(
    comment: &Comment,
    state: Rc<AppState>,
    on_error: Rc<F>,
) -> gtk::Widget {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        // Replies keep the provider's nesting depth.
        .margin_start((comment.indent.min(5) * 24) as i32)
        .build();
    card.add_css_class("card");
    card.set_margin_top(6);

    let header = gtk::Label::builder()
        .label(format!("{} · {}", comment.username, comment.date))
        .xalign(0.0)
        .margin_start(12)
        .margin_top(6)
        .build();
    header.add_css_class("heading");
    card.append(&header);

    let text = gtk::Label::builder()
        .label(&comment.text)
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .margin_start(12)
        .margin_end(12)
        .build();

    if comment.has_spoiler {
        // Spoilers stay hidden until the reader asks for them.
        let reveal = gtk::Button::with_label(tr("Show Spoiler"));
        reveal.add_css_class("flat");
        reveal.set_halign(gtk::Align::Start);
        reveal.set_margin_start(12);
        text.set_visible(false);
        let text_for_reveal = text.clone();
        reveal.connect_clicked(move |button| {
            text_for_reveal.set_visible(true);
            button.set_visible(false);
        });
        card.append(&reveal);
    }
    card.append(&text);

    let like = gtk::Button::builder()
        .label(format!("♥ {}", comment.likes))
        .halign(gtk::Align::Start)
        .margin_start(12)
        .margin_bottom(6)
        .build();
    like.add_css_class("flat");
    like.set_sensitive(state.user().is_some() && !comment.is_liked);

    let id = comment.id.clone();
    let likes = comment.likes;
    let account = state.account();
    like.connect_clicked(move |button| {
        button.set_sensitive(false);
        let client = state.client.clone();
        let id = id.clone();
        let button = button.clone();
        let on_error = on_error.clone();
        let state_for_result = state.clone();
        let account = account.clone();
        relm4::spawn_local(async move {
            let loaded = guarded_as(client, account.clone(), move |client| async move {
                client.like_comment(&id).await
            })
            .await;
            if !state_for_result.accepts(&account) {
                button.set_sensitive(true);
                on_error(tr("The account changed while the comment was being liked").to_string());
                return;
            }
            match loaded.result {
                Ok(()) => button.set_label(&format!("♥ {}", likes + 1)),
                Err(error) => {
                    button.set_sensitive(true);
                    on_error(error);
                }
            }
        });
    });
    card.append(&like);

    card.upcast()
}
