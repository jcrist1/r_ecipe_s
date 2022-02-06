mod auto_form_component;
mod error_pages;
mod templates;
mod util;

use perseus::define_app;

define_app! {
    templates: [
        templates::index::get_template::<G>(),
        templates::recipes::get_template::<G>()
    ],
    error_pages: error_pages::get_error_pages(),
    static_aliases: {
        "/test.txt" => "static/test.txt",
        "/index.css" => "static/index.css",
        "/x_circle.svg" => "static/x-circle.svg",
        "/plus_circle.svg" => "static/plus-circle.svg"
    }
}
