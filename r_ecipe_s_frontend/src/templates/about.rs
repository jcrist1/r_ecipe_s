use perseus::*;
use r_ecipe_s_model::Recipe;
use serde::{Deserialize, Serialize};
use sycamore::flow::{Indexed, IndexedProps};
use sycamore::prelude::{component, view, Html, Keyed, KeyedProps, Signal, SsrNode, View};

#[perseus::template(AboutPage)]
#[component(AboutPage<G>)]
pub fn about_page(recipes: Vec<Recipe>) -> View<G> {
    let recipe_signal = Signal::new(recipes);
    view! {
            Indexed(IndexedProps{
                iterable: recipe_signal.handle(),
                template: |x| {
                    let ingredient_signal = Signal::new(x.ingredients);
                    view! {
        div(class = "pure-u-1 pure-u-md-1-2 pure-u-lg-1-4") {
            div(class = "l-box") {
                        p { (x.name) }
                        ul {
                            Indexed(IndexedProps {
                                iterable: ingredient_signal.handle(),
                                template: |x| {
                                    let b = x.name;
                                    let c = x.quantity;
                                    view! { li {(format!("{:?} {:}", c, b))}}
                                },
                            })
                        }
                        p { (x.description) }
                    }
        }
        }
                },
            })


    }
}

#[perseus::head]
pub fn head() -> View<SsrNode> {
    view! {
        title { "About Page | Perseus Example – Basic" }
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::new("about")
        .template(about_page)
        .request_state_fn(get_request_state)
        .head(head)
}

#[perseus::autoserde(request_state)]
pub async fn get_request_state(
    _path: String,
    _locale: String,
    _req: Request,
) -> RenderFnResultWithCause<Vec<Recipe>> {
    let body = ureq::get("http://localhost:8000/recipes")
        .call()
        .map_err(|err| perseus::GenericErrorWithCause {
            error: Box::new(err),
            cause: ErrorCause::Server(None),
        })?
        .into_string()
        .map_err(|err| perseus::GenericErrorWithCause {
            error: Box::new(err),
            cause: ErrorCause::Client(None),
        })?;
    serde_json::from_str::<Vec<Recipe>>(&body).map_err(|err| perseus::GenericErrorWithCause {
        error: Box::new(err),
        cause: ErrorCause::Client(None),
    })
}
