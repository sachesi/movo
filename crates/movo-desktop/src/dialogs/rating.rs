use crate::api::guarded_as;
use crate::i18n::tr;
use crate::state::{Account, AppState};
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{self, prelude::WidgetExt};
use std::rc::Rc;

/// Ask for a 1-10 rating and post it.
///
/// `on_done` reports the outcome so the page can show a toast either way.
pub fn present(
    parent: &impl IsA<gtk::Widget>,
    state: Rc<AppState>,
    post_id: i64,
    account: Option<Account>,
    on_done: impl Fn(Result<u8, String>) + 'static,
) {
    let dialog = adw::AlertDialog::builder()
        .heading(tr("Rate This Title"))
        .body(tr("The provider accepts whole numbers from 1 to 10."))
        .close_response("cancel")
        .build();
    dialog.add_responses(&[("cancel", tr("Cancel")), ("rate", tr("Rate"))]);
    dialog.set_response_appearance("rate", adw::ResponseAppearance::Suggested);

    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 1.0, 10.0, 1.0);
    scale.set_value(8.0);
    scale.set_draw_value(true);
    scale.set_width_request(260);
    scale.set_margin_top(12);
    for value in 1..=10 {
        scale.add_mark(value as f64, gtk::PositionType::Bottom, None);
    }
    dialog.set_extra_child(Some(&scale));

    let parent = parent.as_ref().clone();
    relm4::spawn_local(async move {
        if dialog.choose_future(Some(&parent)).await != "rate" {
            return;
        }
        let rating = scale.value().round().clamp(1.0, 10.0) as u8;
        let client = state.client.clone();
        let loaded = guarded_as(client, account.clone(), move |client| async move {
            client.post_rating(post_id, rating).await
        })
        .await;
        if !state.accepts(&account) {
            on_done(Err(
                tr("The account changed before the rating was sent").to_string()
            ));
        } else {
            on_done(loaded.result.map(|()| rating));
        }
    });
}
