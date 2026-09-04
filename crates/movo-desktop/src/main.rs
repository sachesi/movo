use movo::app::App;
use movo::APP_ID;
use relm4::RelmApp;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    movo::i18n::init();

    // Network fetches, image decoding and history writes all run off the main
    // thread; a single worker would serialize them behind each other.
    relm4::RELM_THREADS.set(4).ok();

    let app = RelmApp::new(APP_ID);
    // Adwaita rounds `gridview > child` but leaves `listview > row` square;
    // the home rails are list views, so give their rows the grid's shape.
    relm4::set_global_css("listview.poster-rail > row { border-radius: 9px; padding: 3px; }");
    app.run::<App>(());
}
