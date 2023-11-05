use frontend_ls::DownloadInBackground;
use gloo_worker::Registrable;
use leptos::logging::log;

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    log!("Starting background downloader");

    DownloadInBackground::registrar().register();
}
