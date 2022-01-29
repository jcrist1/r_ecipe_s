
use perseus::{
    http::header::{HeaderMap, HeaderName},
    Html, RenderFnResultWithCause, SsrNode, Template,
};
use serde::{Deserialize, Serialize};
use sycamore::prelude::{component, view, View};

#[derive(Serialize, Deserialize, Debug)]
pub struct IndexPageProps {
    pub greeting: String,
}

#[perseus::template(IndexPage)]
#[component(IndexPage<G>)]
pub fn index_page(props: IndexPageProps) -> View<G> {
    view! {
        div {
        p {(props.greeting)}
        a(href = "recipes", id = "") { "recipes!" }
        }
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::new("index")
        .build_state_fn(get_build_props)
        .template(index_page)
        .head(head)
        .set_headers_fn(set_headers)
}

#[perseus::head]
pub fn head(_props: IndexPageProps) -> View<SsrNode> {
    view! {
        title { "About recipes" }
    }
}

#[perseus::autoserde(build_state)]
pub async fn get_build_props(
    _path: String,
    _locale: String,
) -> RenderFnResultWithCause<IndexPageProps> {
    Ok(IndexPageProps {
        greeting: r#"In addition to a really tortured attempt to shoehorn the 
            rust \"rs\" somewhere, this is also actually a place where I will
            try and store some personal reicpes"#
            .to_string(),
    })
}

#[perseus::autoserde(set_headers)]
pub fn set_headers(props: Option<IndexPageProps>) -> HeaderMap {
    let mut map = HeaderMap::new();
    map.insert(
        HeaderName::from_lowercase(b"x-greeting").unwrap(),
        props.unwrap().greeting.parse().unwrap(),
    );
    map
}
