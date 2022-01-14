use perseus::*;
use r_ecipe_s_model::Recipe;
use serde::{Deserialize, Serialize};
use sycamore::flow::{Indexed, IndexedProps};
use sycamore::prelude::*;

#[perseus::template(AboutPage)]
#[component(AboutPage<G>)]
pub fn about_page() -> View<G> {
    let recipe_signal = Signal::new(vec![]);
    if G::IS_BROWSER {
        // Spawn a `Future` on this thread to fetch the data (`spawn_local` is re-exported from `wasm-bindgen-futures`)
        // Don't worry, this doesn't need to be sent to JavaScript for execution
        //
        // We want to access the `message` `Signal`, so we'll clone it in (and then we need `move` because this has to be `'static`)
        perseus::spawn_local(cloned!(recipe_signal => async move {
                // This interface may seem weird, that's because it wraps the browser's Fetch API
                // We request from a local path here because of CORS restrictions (see the book)
                let body = reqwasm::http::Request::get("/api/v1/recipes")
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();

        let recipes = serde_json::from_str::<Vec<Recipe>>(&body).map_err(|err| perseus::GenericErrorWithCause {
            error: Box::new(err),
            cause: ErrorCause::Client(None),
        }).expect("err");
                recipe_signal.set(recipes);
            }));
    }
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
                        div(dangerously_set_inner_html=&x.description)
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
    Template::new("about").template(about_page).head(head)
}
