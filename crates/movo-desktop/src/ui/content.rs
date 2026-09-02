use crate::i18n::tr;
use relm4::adw;
use relm4::gtk::{self, prelude::*};

/// What a view is currently showing.
pub enum ContentState<'a> {
    Loading(&'a str),
    /// Icon name and title for a successful but empty result.
    Empty(&'a str, &'a str),
    /// The view needs an account; the placeholder offers a way to sign in.
    SignedOut(&'a str),
    /// The message from the failed request, never a generic replacement.
    Error(&'a str),
    Content,
}

/// The loading / empty / error / content scaffold shared by every view.
pub struct ContentStack {
    pub widget: gtk::Stack,
    status: adw::StatusPage,
    spinner: adw::StatusPage,
    sign_in: gtk::Button,
    /// Names what failed, so the error page says more than that something did.
    error_title: &'static str,
}

impl ContentStack {
    pub fn new(content: &impl IsA<gtk::Widget>, error_title: &'static str) -> Self {
        let spinner = adw::StatusPage::builder().vexpand(true).build();
        spinner.set_paintable(Some(&adw::SpinnerPaintable::new(Some(&spinner))));

        let status = adw::StatusPage::builder().vexpand(true).build();

        let sign_in = gtk::Button::builder()
            .label(tr("Sign In"))
            .action_name("win.sign-in")
            .halign(gtk::Align::Center)
            .visible(false)
            .build();
        sign_in.add_css_class("suggested-action");
        sign_in.add_css_class("pill");
        status.set_child(Some(&sign_in));

        let widget = gtk::Stack::builder()
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        widget.add_named(&spinner, Some("loading"));
        widget.add_named(&status, Some("status"));
        widget.add_named(content, Some("content"));

        Self {
            widget,
            status,
            spinner,
            sign_in,
            error_title,
        }
    }

    pub fn set(&self, state: ContentState<'_>) {
        match state {
            ContentState::Loading(title) => {
                self.spinner.set_title(title);
                self.widget.set_visible_child_name("loading");
            }
            ContentState::Empty(icon, title) => {
                self.sign_in.set_visible(false);
                self.status.set_icon_name(Some(icon));
                self.status.set_title(title);
                self.status.set_description(None);
                self.widget.set_visible_child_name("status");
            }
            ContentState::SignedOut(title) => {
                self.sign_in.set_visible(true);
                self.status
                    .set_icon_name(Some("system-lock-screen-symbolic"));
                self.status.set_title(title);
                self.status.set_description(None);
                self.widget.set_visible_child_name("status");
            }
            ContentState::Error(message) => {
                self.sign_in.set_visible(false);
                self.status.set_icon_name(Some("dialog-error-symbolic"));
                self.status.set_title(self.error_title);
                self.status.set_description(Some(message));
                self.widget.set_visible_child_name("status");
            }
            ContentState::Content => self.widget.set_visible_child_name("content"),
        }
    }
}
