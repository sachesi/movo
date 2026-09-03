/// Reverse-DNS name of the application. Also the base name of the installed desktop entry and
/// icon, which is how the shell finds an icon for the window.
pub const APP_ID: &str = "org.gnome.Movo";

pub mod api;
pub mod app;
pub mod dialogs;
pub mod i18n;
pub mod playback;
pub mod state;
pub mod ui;
pub mod views;
