use pulldown_cmark::{html, Options, Parser};

pub fn markdown_to_html(markdown_str: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown_str, options);

    // Write to String buffer.
    let mut html_output: String = String::with_capacity(markdown_str.len() * 2);
    html::push_html(&mut html_output, parser);
    html_output
}
