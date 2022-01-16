use crate::util::markdown_to_html;
use crate::RECIPE_ID;
use perseus::*;
use r_ecipe_s_model::Recipe;
use serde::{Deserialize, Serialize};
use sycamore::flow::{Indexed, IndexedProps};
use sycamore::prelude::*;
use web_sys::{Event, HtmlInputElement, KeyboardEvent};

#[perseus::template(RecipesPage)]
#[component(RecipesPage<G>)]
pub fn recipes_page() -> View<G> {
    let raw_state = RecipeAppStateRaw {
        selected: None,
        page: PageState { offset: 0 },
    };
    let RecipeAppState { selected, .. } = raw_state.signal();
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

            let recipes = serde_json::from_str::<Vec<(i64, Recipe)>>(&body).map_err(|err| perseus::GenericErrorWithCause {
                error: Box::new(err),
                cause: ErrorCause::Client(None),
            }).expect("err");
            recipe_signal.set(recipes);
        }));
    }
    let cloned_state = selected.clone();

    view! {
        p {
            (format!("SELECTED: {:?}", selected.get()))
        }
        Indexed(IndexedProps{
            iterable: recipe_signal.handle(),
            template: move |(id, recipe_data)| {
                let cloned_cloned_state = cloned_state.clone();
                view! {RecipePage((cloned_cloned_state, id, recipe_data))}
            },
        })


    }
}

#[component(RecipePage<G>)]
fn recipe_component(
    (selected_state_signal, id, x): (Signal<Option<SelectedState>>, i64, Recipe),
) -> View<G> {
    let handle_click = cloned!((selected_state_signal) => move |_| {
        set_recipe_id(&selected_state_signal, id);
    });
    let ingredient_signal = Signal::new(x.ingredients);
    let class = cloned!((selected_state_signal) =>  move || match selected_state_signal.get().as_ref() {
        Some(SelectedState { id: recipe_id, .. }) => {
            if recipe_id.get().id == id {
                "pure-u-1-lg"
            } else {
                "pure-u-1 pure-u-md-1-2 pure-u-lg-1-4"
            }
        }
        None => "pure-u-1 pure-u-md-1-2 pure-u-lg-1-4",
    });

    let x_class = cloned!((selected_state_signal) => move || match selected_state_signal.get().as_ref() {
        Some(SelectedState { id: recipe_id, .. }) => {
            if recipe_id.get().id == id {
                "shown"
            } else {
                "hidden"
            }
        }
        None => "hidden",
    });

    let handle_close = cloned!((selected_state_signal) => move |_| {
        selected_state_signal.set(None);
    });

    view! {
        div(class = class() ){
            div(class = "l-box recipe-tile", on:click=handle_click) {
                div(class = x_class(), on:click=handle_close) {
                    p {
                        " "
                    }
                }
                p() { (x.name) }
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
                div(dangerously_set_inner_html = &markdown_to_html(&x.description))
            }
        }
    }
}

fn set_recipe_id(selected_state: &Signal<Option<SelectedState>>, id: i64) {
    match selected_state.get().as_ref() {
        None => {
            let state = SelectedState {
                id: Signal::new(RecipeId { id }),
                editing: Signal::new(false),
            };
            selected_state.set(Some(state));
        }
        Some(SelectedState { id: id_signal, .. }) => {
            id_signal.set(RecipeId { id });
        }
    }
    let new_state = selected_state.get().as_ref().clone();
    selected_state.set(new_state);
}

#[perseus::head]
pub fn head() -> View<SsrNode> {
    view! {
        title { "Recipes"}
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::new("recipes")
        //        .request_state_fn(get_request_state)
        .template(recipes_page)
        .head(head)
}

#[perseus::autoserde(request_state)]
pub async fn get_request_state(
    _path: String,
    _local: String,
    _req: Request,
) -> RenderFnResultWithCause<RecipeAppStateRaw> {
    // todo serialize and deserialize state to storage.
    RecipeAppStateRaw {
        selected: None,
        page: PageState { offset: 0 },
    }
    .to_ok()
}

trait IntoOk
where
    Self: Sized,
{
    fn to_ok<ErrType>(self) -> Result<Self, ErrType>;
}
impl<T: Sized> IntoOk for T {
    fn to_ok<ErrType>(self) -> Result<T, ErrType> {
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RecipeAppStateRaw {
    selected: Option<SelectedStateRaw>,
    page: PageState,
}
impl RecipeAppStateRaw {
    pub fn signal(&self) -> RecipeAppState {
        RecipeAppState {
            selected: Signal::new(self.selected.map(|state| state.signal())),
            page: Signal::new(self.page),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecipeAppState {
    selected: Signal<Option<SelectedState>>,
    page: Signal<PageState>,
}
impl RecipeAppState {
    pub fn raw(&self) -> RecipeAppStateRaw {
        RecipeAppStateRaw {
            selected: self
                .selected
                .get()
                .as_ref()
                .clone()
                .map(|state| state.raw())
                .as_ref()
                .cloned(),
            page: *self.page.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SelectedStateRaw {
    id: RecipeId,
    editing: bool,
}

impl SelectedStateRaw {
    pub fn signal(&self) -> SelectedState {
        SelectedState {
            id: Signal::new(self.id),
            editing: Signal::new(self.editing),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectedState {
    id: Signal<RecipeId>,
    editing: Signal<bool>,
}

impl SelectedState {
    pub fn raw(&self) -> SelectedStateRaw {
        SelectedStateRaw {
            id: *self.id.get(),
            editing: *self.editing.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageState {
    offset: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RecipeId {
    id: i64,
}
