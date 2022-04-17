use std::fmt::Debug;
use thiserror::Error as ThisError;

use pulldown_cmark::{html, Options, Parser};

pub fn background(image_name: &str) -> String {
    format!("background-image: url(/static/{image_name})")
}
pub fn markdown_to_html(markdown_str: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown_str, options);

    // Write to String buffer.
    let mut html_output: String = String::with_capacity(markdown_str.len() * 2);
    html::push_html(&mut html_output, parser);
    html_output
}

pub fn recover_default_and_log_err<T: Default, E: Debug>(msg: &str, result: Result<T, E>) -> T {
    match result {
        Ok(t) => t,
        Err(err) => {
            web_sys::console::log_1(&format!("{msg}: {err:?}").into());
            T::default()
        }
    }
}

#[derive(Debug, ThisError)]
pub enum FrontErr {
    #[error("Error: {0}")]
    Message(String),
}
