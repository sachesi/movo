/// Reverse-DNS name of the application. Also the base name of the installed desktop entry and
/// icon, which is how the shell finds an icon for the window.
pub const APP_ID: &str = "io.github.sachesi.Movo";

pub mod api;
pub mod app;
pub mod dialogs;
pub mod i18n;
pub mod playback;
#[cfg(feature = "test-provider")]
pub mod stall_watch;
pub mod state;
pub mod tray;
pub mod ui;
pub mod views;
