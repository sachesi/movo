pub mod content;
pub mod image;
pub mod paged_grid;
pub mod poster_grid;

use relm4::gtk::{self, prelude::*};

/// The boxed list the views use for rows outside a preferences group.
pub fn activatable_list() -> gtk::ListBox {
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .build();
    list.add_css_class("boxed-list");
    list
}
