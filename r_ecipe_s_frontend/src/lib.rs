mod error_pages;
mod templates;
use perseus::{define_app, ErrorPages, Template};
use sycamore::view;

define_app! {
    templates: [
        templates::index::get_template::<G>(),
        templates::about::get_template::<G>()
    ],
    error_pages: error_pages::get_error_pages(),
    static_aliases: {
        "/test.txt" => "static/test.txt",
        "/index.css" => "static/index.css"
    }
}
