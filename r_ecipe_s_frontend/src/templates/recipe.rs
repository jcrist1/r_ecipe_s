use perseus::*;
use r_ecipe_s_model::{Ingredient, Recipe};
use serde::{Deserialize, Serialize};
use sycamore::flow::{Indexed, IndexedProps};
use sycamore::prelude::*;

#[perseus::template(RecipePage)]
#[component(RecipePage<G>)]
pub fn recipe_page(id: i64) -> View<G> {
    let recipe_signal = Signal::new(Recipe {
        description: format!(""),
        name: format!(""),
        ingredients: vec![],
        liked: None,
    });
    let recipe_signal_2 = recipe_signal.clone();
    let ingredient_signal = Signal::new(vec![]);
    let ingredient_signal_2 = ingredient_signal.clone();
    if G::IS_BROWSER {
        // Spawn a `Future` on this thread to fetch the data (`spawn_local` is re-exported from `wasm-bindgen-futures`)
        // Don't worry, this doesn't need to be sent to JavaScript for execution
        //
        // We want to access the `message` `Signal`, so we'll clone it in (and then we need `move` because this has to be `'static`)
        perseus::spawn_local(cloned!(recipe_signal => async move  {
            // This interface may seem weird, that's because it wraps the browser's Fetch API
            // We request from a local path here because of CORS restrictions (see the book)
            let body = reqwasm::http::Request::get(&format!("/api/v1/recipes/{}", id))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

            let recipe = serde_json::from_str::<Option<Recipe>>(&body).map_err(|err| perseus::GenericErrorWithCause {
                error: Box::new(err),
                cause: ErrorCause::Client(None),
            }).expect("err").unwrap();
            let ingredients = recipe.ingredients.clone();

            ingredient_signal.set(ingredients);
            recipe_signal.set(recipe);
        }));
    }
    view! {
    div(class = "pure-u-1 pure-u-md-1-2 pure-u-lg-1-2") {
        div(class = "l-box") {
                    p { (recipe_signal_2.get().name) }
                    ul {
                        Indexed(IndexedProps {
                            iterable: ingredient_signal_2.handle(),
                            template: |x| {
                                let b = x.name;
                                let c = x.quantity;
                                view! { li {(format!("{:?} {:}", c, b))}}
                            },
                        })
                    }
                    div(dangerously_set_inner_html=&recipe_signal.get().description)
                }
    }
    }
}

#[perseus::head]
pub fn head() -> View<SsrNode> {
    view! {
        title { "About Page | Perseus Example – Basic" }
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::new("recipe").template(recipe_page).head(head)
}
