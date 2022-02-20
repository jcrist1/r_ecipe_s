use r_ecipe_s_frontend::templates::recipes::RecipesPage;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;

fn main() {
    println!("BOOYAH");

    sycamore::render(|ctx| {
        view! { ctx,
            Suspense {
                fallback: view! {ctx, div {""} },
                RecipesPage()
            }
        }
    });
}
