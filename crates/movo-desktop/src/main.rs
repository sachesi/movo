use movo::app::App;
use movo::APP_ID;
use relm4::RelmApp;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    movo::i18n::init();

    // Network fetches, image decoding and history writes all run off the main
    // thread; a single worker would serialize them behind each other.
    relm4::RELM_THREADS.set(4).ok();

    RelmApp::new(APP_ID).run::<App>(());
}
