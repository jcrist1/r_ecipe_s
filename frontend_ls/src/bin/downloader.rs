use frontend_ls::DownloadInBackground;
use gloo_worker::Registrable;

fn main() {
    console_error_panic_hook::set_once();
    DownloadInBackground::registrar().register();
}
