use crate::i18n::tr;
use crate::state::AppState;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{
    self,
    prelude::{ButtonExt, EditableExt, WidgetExt},
};
use std::rc::Rc;

/// Ask for credentials and sign in.
///
/// `on_signed_in` runs with the account name once the provider has confirmed
/// the session.
pub fn present(
    parent: &impl IsA<gtk::Widget>,
    state: Rc<AppState>,
    on_signed_in: impl Fn(String) + 'static,
) {
    let dialog = adw::PreferencesDialog::builder()
        .title(tr("Sign In to HDRezka"))
        .build();

    let group = adw::PreferencesGroup::builder()
        .title(tr("Account"))
        .description(tr(
            "Sign in with your HDRezka account to sync favorites and history.",
        ))
        .build();

    let login = adw::EntryRow::builder()
        .title(tr("Username or Email"))
        .build();
    let password = adw::PasswordEntryRow::builder()
        .title(tr("Password"))
        .build();
    group.add(&login);
    group.add(&password);

    let error = gtk::Label::builder().wrap(true).visible(false).build();
    error.add_css_class("error");

    let spinner = adw::Spinner::builder()
        .visible(false)
        .halign(gtk::Align::Center)
        .build();

    let submit = gtk::Button::builder()
        .label(tr("Sign In"))
        .halign(gtk::Align::Center)
        .margin_top(12)
        .build();
    submit.add_css_class("suggested-action");
    submit.add_css_class("pill");

    let actions = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .halign(gtk::Align::Center)
        .build();
    actions.append(&error);
    actions.append(&spinner);
    actions.append(&submit);

    let page = adw::PreferencesPage::new();
    page.add(&group);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(18)
        .margin_end(18)
        .margin_bottom(18)
        .build();
    content.append(&page);
    content.append(&actions);

    dialog.set_child(Some(&content));
    dialog.set_default_widget(Some(&submit));

    // Enter in either field submits, like any other login form.
    for row in [
        login.clone().upcast::<adw::PreferencesRow>(),
        password.clone().upcast::<adw::PreferencesRow>(),
    ] {
        let submit = submit.clone();
        if let Some(entry) = row.downcast_ref::<adw::EntryRow>() {
            entry.connect_entry_activated(move |_| submit.emit_clicked());
        } else if let Some(entry) = row.downcast_ref::<adw::PasswordEntryRow>() {
            entry.connect_entry_activated(move |_| submit.emit_clicked());
        }
    }

    let on_signed_in = Rc::new(on_signed_in);
    let dialog_for_submit = dialog.clone();
    submit.connect_clicked(move |button| {
        let name = login.text().to_string();
        let secret = password.text().to_string();
        if name.trim().is_empty() || secret.is_empty() {
            error.set_label(tr("Enter your username and password"));
            error.set_visible(true);
            return;
        }

        button.set_sensitive(false);
        error.set_visible(false);
        spinner.set_visible(true);

        let client = state.client.clone();
        let dialog = dialog_for_submit.clone();
        let button = button.clone();
        let error = error.clone();
        let spinner = spinner.clone();
        let on_signed_in = on_signed_in.clone();
        relm4::spawn_local(async move {
            let result = relm4::spawn(async move { client.login(&name, &secret).await })
                .await
                .unwrap_or_else(|_| Err(tr("Signing in stopped unexpectedly").to_string()));

            spinner.set_visible(false);
            button.set_sensitive(true);
            match result {
                Ok(user) => {
                    on_signed_in(user.username);
                    dialog.close();
                }
                Err(message) => {
                    error.set_label(&message);
                    error.set_visible(true);
                }
            }
        });
    });

    dialog.present(Some(parent));
}
