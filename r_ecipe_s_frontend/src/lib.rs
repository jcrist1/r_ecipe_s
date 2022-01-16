mod error_pages;
mod templates;
mod util;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use perseus::{define_app, ErrorPages, Template};
use sycamore::view;

use lazy_static::lazy_static;

pub struct RecipeId {
    pub id: i64,
}

lazy_static! {
    pub static ref RECIPE_ID: Arc<Mutex<RecipeId>> = Arc::new(Mutex::new(RecipeId { id: 0 }));
}

define_app! {
    templates: [
        templates::index::get_template::<G>(),
        templates::recipes::get_template::<G>(),
        templates::recipe::get_template::<G>()
    ],
    error_pages: error_pages::get_error_pages(),
    static_aliases: {
        "/test.txt" => "static/test.txt",
        "/index.css" => "static/index.css"
    }
}
