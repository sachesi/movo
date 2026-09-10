use crate::i18n::{tr, trf};
use crate::playback::launcher::{Launcher, LauncherMsg, LauncherOutput, PlayRequest};
use crate::state::AppState;
use crate::tray::{TrayEvent, TrayIcon};
use crate::views::catalog::{CatalogMsg, CatalogOutput, CatalogView};
use crate::views::collections::{CollectionsOutput, CollectionsView};
use crate::views::details::{DetailsMsg, DetailsOutput, DetailsView};
use crate::views::favorites::{FavoritesMsg, FavoritesOutput, FavoritesView};
use crate::views::history::{HistoryMsg, HistoryOutput, HistoryView};
use crate::views::home::{HomeMsg, HomeOutput, HomeView};
use crate::views::notifications::{NotificationsMsg, NotificationsOutput, NotificationsView};
use crate::views::path::{PathOutput, PathView};
use crate::views::search::{SearchOutput, SearchView};
use movo_core::client::models::MediaItem;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller};
use std::rc::Rc;
use std::{cell::Cell, sync::mpsc, time::Duration};

pub struct App {
    state: Rc<AppState>,
    home: Controller<HomeView>,
    // These views do not depend on the account; they are only held so their
    // components stay alive for as long as the window does.
    _catalog: Controller<CatalogView>,
    _search: Controller<SearchView>,
    _collections: Controller<CollectionsView>,
    notifications: Controller<NotificationsView>,
    favorites: Controller<FavoritesView>,
    history: Controller<HistoryView>,
    // Pages pushed on top of the shell, kept alive while they are shown.
    paths: Vec<Controller<PathView>>,
    details: Vec<Controller<DetailsView>>,
    launcher: Controller<Launcher>,
    navigation: adw::NavigationView,
    notifications_page: adw::ViewStackPage,
    toasts: adw::ToastOverlay,
}

#[derive(Debug)]
pub enum AppMsg {
    Open(MediaItem),
    Play(Box<PlayRequest>),
    OpenPath(String, String),
    AccountInvalidated,
    Notify(String),
    UnreadCount(usize),
    /// The visible tab changed; reload only what depends on the account.
    TabChanged(String),
    ShowSettings,
    ShowAbout,
    AccountChanged,
    /// The hidden countries changed; the listings on screen no longer match.
    FiltersChanged,
}

#[derive(Debug)]
pub enum AppCommand {
    SessionRestored(Result<Option<String>, String>),
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppCommand;

    view! {
        adw::ApplicationWindow {
            set_title: Some("Movo"),
            // Names the installed hicolor icon; the shell has nothing else to go on for the
            // window, the switcher or the dock.
            set_icon_name: Some(crate::APP_ID),
            set_default_size: (1280, 820),
            // Matches the bottom-bar breakpoint below: the switcher bar and
            // its rows are the narrowest content the shell is laid out for.
            set_width_request: 360,
            set_height_request: 294,

            #[local_ref]
            toasts -> adw::ToastOverlay {
                #[local_ref]
                navigation -> adw::NavigationView {
                    add = &adw::NavigationPage {
                        set_title: "Movo",
                        set_tag: Some("shell"),

                        adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                #[wrap(Some)]
                                set_title_widget = &gtk::Box {
                                    set_halign: gtk::Align::Center,

                                    #[name = "header_switcher"]
                                    adw::ViewSwitcher {
                                        set_policy: adw::ViewSwitcherPolicy::Wide,
                                        set_stack: Some(&stack),
                                    },

                                    // Below the breakpoint the seven destinations move to
                                    // the bottom bar; the header falls back to a plain
                                    // title instead of a switcher with nowhere to shrink.
                                    #[name = "header_title"]
                                    adw::WindowTitle {
                                        set_title: "Movo",
                                        set_visible: false,
                                    },
                                },

                                #[name = "menu_button"]
                                pack_end = &gtk::MenuButton {
                                    set_icon_name: "open-menu-symbolic",
                                    set_tooltip_text: Some(tr("Main Menu")),
                                    set_primary: true,
                                    set_menu_model: Some(&primary_menu()),
                                },
                            },

                            #[wrap(Some)]
                            set_content = &stack.clone() -> adw::ViewStack {
                                connect_visible_child_name_notify[sender] => move |stack| {
                                    if let Some(name) = stack.visible_child_name() {
                                        sender.input(AppMsg::TabChanged(name.to_string()));
                                    }
                                },
                            },

                            #[name = "switcher_bar"]
                            add_bottom_bar = &adw::ViewSwitcherBar {
                                set_stack: Some(&stack),
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let state = AppState::new();
        apply_theme(&state.settings().theme);

        // A window action lets any signed-out placeholder offer a way in
        // without every view having to route a message back here.
        let actions = relm4::gtk::gio::SimpleActionGroup::new();
        let sign_in = relm4::gtk::gio::SimpleAction::new("sign-in", None);
        {
            let sender = sender.clone();
            sign_in.connect_activate(move |_, _| sender.input(AppMsg::ShowSettings));
        }
        actions.add_action(&sign_in);
        root.insert_action_group("win", Some(&actions));

        // The primary menu and its accelerators need actions on the application itself,
        // since the menu is built once and is not tied to a particular window.
        let application = relm4::main_application();
        let preferences = relm4::gtk::gio::SimpleAction::new("preferences", None);
        {
            let sender = sender.clone();
            preferences.connect_activate(move |_, _| sender.input(AppMsg::ShowSettings));
        }
        application.add_action(&preferences);

        let about = relm4::gtk::gio::SimpleAction::new("about", None);
        {
            let sender = sender.clone();
            about.connect_activate(move |_, _| sender.input(AppMsg::ShowAbout));
        }
        application.add_action(&about);

        let shortcuts = relm4::gtk::gio::SimpleAction::new("shortcuts", None);
        {
            let root = root.clone();
            shortcuts.connect_activate(move |_, _| show_shortcuts(&root));
        }
        application.add_action(&shortcuts);

        let quit = relm4::gtk::gio::SimpleAction::new("quit", None);
        quit.connect_activate(|_, _| relm4::main_application().quit());
        application.add_action(&quit);

        application.set_accels_for_action("app.preferences", &["<Control>comma"]);
        application.set_accels_for_action("app.shortcuts", &["<Control>question"]);
        application.set_accels_for_action("app.quit", &["<Control>q"]);

        let home =
            HomeView::builder()
                .launch(state.clone())
                .forward(sender.input_sender(), |output| match output {
                    HomeOutput::Open(item) => AppMsg::Open(item),
                    HomeOutput::OpenPath(title, path) => AppMsg::OpenPath(title, path),
                    HomeOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                });
        let catalog =
            CatalogView::builder()
                .launch(state.clone())
                .forward(sender.input_sender(), |output| match output {
                    CatalogOutput::Open(item) => AppMsg::Open(item),
                    CatalogOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                    CatalogOutput::Warning(message) => AppMsg::Notify(message),
                });
        let search =
            SearchView::builder()
                .launch(state.clone())
                .forward(sender.input_sender(), |output| match output {
                    SearchOutput::Open(item) => AppMsg::Open(item),
                    SearchOutput::OpenPath(title, path) => AppMsg::OpenPath(title, path),
                    SearchOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                    SearchOutput::Warning(message) => AppMsg::Notify(message),
                });
        let collections = CollectionsView::builder().launch(state.clone()).forward(
            sender.input_sender(),
            |output| match output {
                CollectionsOutput::OpenPath(title, path) => AppMsg::OpenPath(title, path),
                CollectionsOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                CollectionsOutput::Warning(message) => AppMsg::Notify(message),
            },
        );
        let notifications = NotificationsView::builder().launch(state.clone()).forward(
            sender.input_sender(),
            |output| match output {
                NotificationsOutput::Open(title, url) => {
                    AppMsg::Open(notification_media(title, url))
                }
                NotificationsOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                NotificationsOutput::UnreadCount(count) => AppMsg::UnreadCount(count),
            },
        );
        let favorites = FavoritesView::builder().launch(state.clone()).forward(
            sender.input_sender(),
            |output| match output {
                FavoritesOutput::Open(item) => AppMsg::Open(item),
                FavoritesOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                FavoritesOutput::Warning(message) => AppMsg::Notify(message),
            },
        );
        let history =
            HistoryView::builder()
                .launch(state.clone())
                .forward(sender.input_sender(), |output| match output {
                    HistoryOutput::Open(item) => AppMsg::Open(item),
                    HistoryOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                    HistoryOutput::Warning(message) => AppMsg::Notify(message),
                });

        let stack = adw::ViewStack::new();
        add_page(
            &stack,
            home.widget(),
            "home",
            tr("Home"),
            "user-home-symbolic",
        );
        add_page(
            &stack,
            catalog.widget(),
            "catalog",
            tr("Catalog"),
            "folder-videos-symbolic",
        );
        add_page(
            &stack,
            search.widget(),
            "search",
            tr("Search"),
            "system-search-symbolic",
        );
        add_page(
            &stack,
            collections.widget(),
            "collections",
            tr("Collections"),
            "view-grid-symbolic",
        );
        let notifications_page = add_page(
            &stack,
            notifications.widget(),
            "notifications",
            tr("Notifications"),
            "preferences-system-notifications-symbolic",
        );
        add_page(
            &stack,
            favorites.widget(),
            "favorites",
            tr("Favorites"),
            "starred-symbolic",
        );
        add_page(
            &stack,
            history.widget(),
            "history",
            tr("History"),
            "document-open-recent-symbolic",
        );

        let initial = state.settings().initial_view;
        if stack.child_by_name(&initial).is_some() {
            stack.set_visible_child_name(&initial);
        }

        let toasts = adw::ToastOverlay::new();
        let navigation = adw::NavigationView::new();

        let launcher = Launcher::builder()
            .launch((state.clone(), root.clone().upcast()))
            .forward(sender.input_sender(), |output| match output {
                LauncherOutput::Notify(message) => AppMsg::Notify(message),
                LauncherOutput::HistoryChanged => AppMsg::AccountChanged,
                LauncherOutput::PlayNext(request) => AppMsg::Play(request),
                LauncherOutput::AccountInvalidated => AppMsg::AccountInvalidated,
            });

        let client = state.client.clone();
        sender.oneshot_command(async move {
            let restored = relm4::spawn(async move {
                client
                    .restore_session()
                    .await
                    .map(|user| user.map(|user| user.username))
                    .map_err(|error| error.message)
            })
            .await
            .unwrap_or_else(|_| Err(tr("Restoring the session stopped unexpectedly").to_string()));
            AppCommand::SessionRestored(restored)
        });

        let model = App {
            state,
            home,
            _catalog: catalog,
            _search: search,
            _collections: collections,
            notifications,
            favorites,
            history,
            paths: Vec::new(),
            details: Vec::new(),
            launcher,
            navigation: navigation.clone(),
            notifications_page,
            toasts: toasts.clone(),
        };
        let widgets = view_output!();

        let tray_available = Rc::new(Cell::new(false));
        let (tray_events, events) = mpsc::channel();
        let tray = TrayIcon::new(tray_events, tr("Open Movo").into(), tr("Quit").into());
        relm4::spawn(tray.run());

        {
            let root = root.clone();
            let application = application.clone();
            let tray_available = tray_available.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
                while let Ok(event) = events.try_recv() {
                    match event {
                        TrayEvent::Available => tray_available.set(true),
                        TrayEvent::Unavailable => tray_available.set(false),
                        TrayEvent::Show => root.present(),
                        TrayEvent::Quit => application.quit(),
                    }
                }
                gtk::glib::ControlFlow::Continue
            });
        }

        root.connect_close_request(move |window| {
            if tray_available.get() {
                window.set_visible(false);
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });

        // The tooltip is not read by a screen reader; the icon-only button needs its own label.
        widgets
            .menu_button
            .update_property(&[gtk::accessible::Property::Label(tr("Main Menu"))]);

        // Seven destinations do not fit a header switcher on a narrow window,
        // so hand them to the bottom bar below the breakpoint.
        if let Ok(condition) = adw::BreakpointCondition::parse("max-width: 720sp") {
            let breakpoint = adw::Breakpoint::new(condition);
            breakpoint.add_setter(&widgets.header_switcher, "visible", Some(&false.into()));
            breakpoint.add_setter(&widgets.header_title, "visible", Some(&true.into()));
            breakpoint.add_setter(&widgets.switcher_bar, "reveal", Some(&true.into()));
            root.add_breakpoint(breakpoint);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AppMsg::Open(item) => self.push_details(item, &sender),
            AppMsg::Play(request) => self.launcher.emit(LauncherMsg::Play(request)),
            AppMsg::OpenPath(title, path) => self.push_path(title, path, &sender),
            AppMsg::AccountInvalidated => {
                self.notify(tr("Your session has ended. Sign in again to continue."))
            }
            AppMsg::Notify(message) => self.notify(&message),
            AppMsg::UnreadCount(count) => {
                self.notifications_page.set_badge_number(count as u32);
                self.notifications_page.set_needs_attention(count > 0);
            }
            AppMsg::TabChanged(name) => self.reload_tab(&name),
            AppMsg::ShowSettings => self.show_settings(root, &sender),
            AppMsg::ShowAbout => show_about(root),
            AppMsg::AccountChanged => self.reload_account_views(),
            AppMsg::FiltersChanged => {
                self.home.emit(HomeMsg::Reload);
                self._catalog.emit(CatalogMsg::Reload);
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppCommand::SessionRestored(Ok(Some(username))) => {
                self.notify(&trf("Signed in as {}", &[&username]));
                self.reload_account_views();
            }
            AppCommand::SessionRestored(Ok(None)) => {}
            AppCommand::SessionRestored(Err(error)) => self.notify(&error),
        }
    }
}

impl App {
    fn notify(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }

    /// Reload the views whose contents belong to the signed-in account.
    fn reload_account_views(&self) {
        self.favorites.emit(FavoritesMsg::Reload);
        self.history.emit(HistoryMsg::Reload);
        self.notifications.emit(NotificationsMsg::Reload);
        for details in &self.details {
            details.emit(DetailsMsg::Reload);
        }
    }

    /// A tab keeps what it loaded; only account-scoped tabs are refreshed, and
    /// only when they are actually shown.
    fn reload_tab(&self, name: &str) {
        match name {
            "home" => self.home.emit(HomeMsg::Reload),
            "favorites" => self.favorites.emit(FavoritesMsg::Reload),
            "history" => self.history.emit(HistoryMsg::Reload),
            "notifications" => self.notifications.emit(NotificationsMsg::Reload),
            _ => {}
        }
    }

    fn show_settings(&self, root: &adw::ApplicationWindow, sender: &ComponentSender<Self>) {
        let account_sender = sender.clone();
        let filters_sender = sender.clone();
        let hidden_countries = std::cell::RefCell::new(self.state.settings().hidden_countries);
        crate::dialogs::settings::present(
            root,
            self.state.clone(),
            move |settings| {
                apply_theme(&settings.theme);
                if *hidden_countries.borrow() != settings.hidden_countries {
                    hidden_countries.replace(settings.hidden_countries);
                    filters_sender.input(AppMsg::FiltersChanged);
                }
            },
            move || account_sender.input(AppMsg::AccountChanged),
        );
    }

    fn push_details(&mut self, item: MediaItem, sender: &ComponentSender<Self>) {
        let view = DetailsView::builder()
            .launch((self.state.clone(), item.url.clone()))
            .forward(sender.input_sender(), |output| match output {
                DetailsOutput::Play(details, translator_id, season, episode) => {
                    AppMsg::Play(Box::new(PlayRequest {
                        details: *details,
                        translator_id,
                        season,
                        episode,
                    }))
                }
                DetailsOutput::Open(item) => AppMsg::Open(item),
                DetailsOutput::OpenPath(title, path) => AppMsg::OpenPath(title, path),
                DetailsOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                DetailsOutput::Notify(message) => AppMsg::Notify(message),
            });

        self.push_page(&item.title, view.widget());
        self.details.push(view);
    }

    /// Wrap a view in a navigation page with its own header and show it.
    fn push_page(&self, title: &str, child: &impl IsA<gtk::Widget>) {
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(child));

        let page = adw::NavigationPage::builder()
            .title(title)
            .child(&toolbar)
            .build();
        self.navigation.push(&page);
    }

    fn push_path(&mut self, title: String, path: String, sender: &ComponentSender<Self>) {
        let view = PathView::builder()
            .launch((self.state.clone(), path))
            .forward(sender.input_sender(), |output| match output {
                PathOutput::Open(item) => AppMsg::Open(item),
                PathOutput::AccountInvalidated => AppMsg::AccountInvalidated,
                PathOutput::Warning(message) => AppMsg::Notify(message),
            });

        self.push_page(&title, view.widget());
        self.paths.push(view);
    }
}

fn add_page(
    stack: &adw::ViewStack,
    child: &impl IsA<gtk::Widget>,
    name: &str,
    title: &str,
    icon: &str,
) -> adw::ViewStackPage {
    let page = stack.add_titled_with_icon(child, Some(name), title, icon);
    page.set_title(Some(title));
    page
}

/// Notifications carry a title and a details URL, which is all the details
/// page needs to open one.
fn notification_media(title: String, url: String) -> MediaItem {
    MediaItem {
        id: 0,
        title,
        orig_title: None,
        url,
        poster_url: None,
        year: None,
        category: None,
        rating: None,
        info: None,
    }
}

fn primary_menu() -> relm4::gtk::gio::Menu {
    let menu = relm4::gtk::gio::Menu::new();
    menu.append(Some(tr("Preferences")), Some("app.preferences"));
    menu.append(Some(tr("Keyboard Shortcuts")), Some("app.shortcuts"));
    menu.append(Some(tr("About Movo")), Some("app.about"));
    menu.append(Some(tr("Quit")), Some("app.quit"));
    menu
}

fn show_shortcuts(root: &adw::ApplicationWindow) {
    let dialog = adw::ShortcutsDialog::new();
    let section = adw::ShortcutsSection::new(None);
    section.add(adw::ShortcutsItem::from_action(
        tr("Preferences"),
        "app.preferences",
    ));
    section.add(adw::ShortcutsItem::from_action(
        tr("Keyboard Shortcuts"),
        "app.shortcuts",
    ));
    section.add(adw::ShortcutsItem::from_action(tr("Quit"), "app.quit"));
    dialog.add(section);
    dialog.present(Some(root));
}

fn show_about(root: &adw::ApplicationWindow) {
    adw::AboutDialog::builder()
        .application_name("Movo")
        .application_icon(crate::APP_ID)
        .developer_name("sachesi")
        .version(env!("CARGO_PKG_VERSION"))
        .website("https://github.com/sachesi/movo")
        .license_type(gtk::License::Gpl30)
        .build()
        .present(Some(root));
}

fn apply_theme(theme: &str) {
    let scheme = match theme {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}
