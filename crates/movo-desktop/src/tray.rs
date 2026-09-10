use crate::APP_ID;
use ksni::TrayMethods;
use std::sync::mpsc::Sender;

const ICON_NAME: &str = "io.github.sachesi.Movo-symbolic";

#[derive(Clone, Copy, Debug)]
pub enum TrayEvent {
    Available,
    Unavailable,
    Show,
    Quit,
}

pub struct TrayIcon {
    events: Sender<TrayEvent>,
    open_label: String,
    quit_label: String,
}

impl TrayIcon {
    pub fn new(events: Sender<TrayEvent>, open_label: String, quit_label: String) -> Self {
        Self {
            events,
            open_label,
            quit_label,
        }
    }

    pub async fn run(self) {
        let events = self.events.clone();
        match self.spawn().await {
            Ok(_handle) => {
                let _ = events.send(TrayEvent::Available);
                std::future::pending::<()>().await;
            }
            Err(error) => {
                log::warn!("System tray is unavailable: {error}");
                let _ = events.send(TrayEvent::Unavailable);
            }
        }
    }
}

impl ksni::Tray for TrayIcon {
    fn id(&self) -> String {
        APP_ID.into()
    }

    fn title(&self) -> String {
        "Movo".into()
    }

    fn icon_name(&self) -> String {
        ICON_NAME.into()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.events.send(TrayEvent::Show);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;

        vec![
            StandardItem {
                label: self.open_label.clone(),
                icon_name: "window-new-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(TrayEvent::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: self.quit_label.clone(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn watcher_online(&self) {
        let _ = self.events.send(TrayEvent::Available);
    }

    fn watcher_offline(&self, _reason: ksni::OfflineReason) -> bool {
        let _ = self.events.send(TrayEvent::Unavailable);
        true
    }
}
