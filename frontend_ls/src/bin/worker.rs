use frontend_ls::{EncodeOnDemand, Error};
use gloo_worker::Registrable;

fn main() {
    console_error_panic_hook::set_once();

    EncodeOnDemand::registrar().register();
}
