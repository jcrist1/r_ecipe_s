use r_ecipe_s_frontend::recipes::RecipesPage;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;

fn main() {
    sycamore::render(|ctx| {
        view! { ctx,
            Suspense {
                fallback: view! {ctx, div {""} },
                RecipesPage()
            }
        }
    });
}
