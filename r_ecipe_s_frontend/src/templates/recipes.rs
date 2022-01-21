use std::pin::Pin;

use crate::util::markdown_to_html;
use crate::RECIPE_ID;
use perseus::*;
use r_ecipe_s_model::{Ingredient, Quantity, Recipe};
use serde::{Deserialize, Serialize};
use sycamore::flow::{Indexed, IndexedProps};
use sycamore::prelude::*;
use sycamore::rt::{JsCast, JsValue};
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
        perseus::spawn_local(cloned!((recipe_signal) => async move {
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
            })
                .expect("err")
                .into_iter()
                .map(|recipe| Signal::new(recipe))
                .collect::<Vec<_>>();
            recipe_signal.set(recipes);
        }));
    }
    let cloned_state = selected.clone();

    let off_class = cloned!((selected) => move || if selected.get().as_ref().is_some() {
        "shown"
    } else {
        "hidden"
    });

    let handle_close_off = cloned!((selected) => move |_| {
        selected.set(None);
    });

    let create_recipe = cloned!((recipe_signal, selected) => move |_| {
        perseus::spawn_local(cloned!((selected, recipe_signal) => async move {
            let empty_recipe = Recipe {
                description: String::new(),
                ingredients: vec![],
                name: String::new(),
                liked: None,

            };
            let body = reqwasm::http::Request::put("/api/v1/recipes")
                .header("Content-Type", "application/json")
                .body(JsValue::from_str(&serde_json::to_string(&empty_recipe).expect("failed to encode recipe as json")))
                .send()
                .await
                .expect("failed to get response from PUT recipes")
                .text()
                .await
                .expect("failed to get text from response body");
            let id = serde_json::from_str::<i64>(&body).expect("failed to decode id and recipe from json");

            let mut vec = recipe_signal.get().to_vec();// push((id, new_recipe));
            vec.push(Signal::new((id, empty_recipe)));
            recipe_signal.set(vec);

            let new_state = SelectedState {
                id: Signal::new(RecipeId{ id}),
                editing: Signal::new(true),

            };
            selected.set(Some(new_state));
        })

        );

    });

    view! {
        div(class = "header") {"RecipeS"}
        div(class = format!("de-selector {}", off_class()), on:click= handle_close_off) { br { }}
        Indexed(cloned!((recipe_signal) =>  IndexedProps {
            iterable: recipe_signal.handle(),
            template: move |signal| {
                let cloned_cloned_state = cloned_state.clone();
                view! {RecipePage((cloned_cloned_state, signal))}
            },
        }))
        div(class = "pure-u-1 pure-u-md-1-2 pure-u-lg-1-4 unselected", ) { // todo: on:click = new recipe
            div(class = "plus-button", on:click = create_recipe, dangerously_set_inner_html="&nbsp;")
        }


    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Object<T> {
    obj: T,
}
#[component(RecipePage<G>)]
fn recipe_component(
    (selected_state_signal, recipe): (Signal<Option<SelectedState>>, Signal<(i64, Recipe)>),
) -> View<G> {
    let id = recipe.get().0;
    let recipe_data = recipe.get().1.clone();
    let handle_click = cloned!((selected_state_signal) => move |_| {
        match selected_state_signal.get().as_ref() {
            Some(selected_state) => {
                if selected_state.id.get().id != id {
                    set_recipe_id(&selected_state_signal, id);
                }
            }
            None =>
                set_recipe_id(&selected_state_signal, id),
        }

    });
    let ingredient_signal = Signal::new(recipe_data.ingredients);
    let class = cloned!((selected_state_signal) =>  move || {
        let default = "pure-u-1 pure-u-md-1-2 pure-u-lg-1-4 unselected";
        match selected_state_signal.get().as_ref() {
            Some(SelectedState { id: recipe_id, .. }) => {
                if recipe_id.get().id == id {
                    "pure-u-1 pure-u-md-1-1 pure-u-lg-1-2 selected"
                } else {
                    default
                }
            }
            None => default,
        }
    });

    let x_class = cloned!((selected_state_signal) => move || match selected_state_signal.get().as_ref() {
        Some(SelectedState {id: recipe_id, ..}) =>

            if id == recipe_id.get().id {
                "shown"
            } else {
                "hidden"
            },
                None =>"hidden",
    }
    );

    let handle_close_x = cloned!((selected_state_signal) => move |_| {
        selected_state_signal.set(None);
    });

    let edit = cloned!((selected_state_signal) => move |_| {
        match selected_state_signal.get().as_ref() {
            Some(selected_state) =>
                selected_state.editing.set(true),
            None => (),
        };
    });

    let name_clone = recipe_data.name.clone();

    let form_class_gen = |state_signal: &Signal<Option<SelectedState>>, id: i64| {
        let cloned_state = state_signal.clone();
        if instance_is_editing(cloned_state.get().as_ref(), id) {
            "shown"
        } else {
            "hidden"
        }
    };

    let display_class_gen = |state_signal: &Signal<Option<SelectedState>>, id: i64| {
        let cloned_state = state_signal.clone();
        if instance_is_editing(cloned_state.get().as_ref(), id) {
            "hidden"
        } else {
            "shown"
        }
    };
    let description_clone = recipe_data.description.clone();
    let signal_clone = selected_state_signal.clone();
    let signal_clone_1 = selected_state_signal.clone();
    let signal_clone_2 = selected_state_signal.clone();
    let signal_clone_3 = selected_state_signal.clone();
    let new_ingredient = cloned!((ingredient_signal) => move |_| {
        let mut ingredients = ingredient_signal.get().to_vec();
        ingredients.push(Ingredient {
            name: "".to_string(),
            quantity: Quantity::Count(0)
        });
        ingredient_signal.set(ingredients);
    });
    let set_name = cloned!((recipe) => |event: web_sys::Event| {
        let target = event.target().expect("failed to get title form target");
        let input: HtmlInputElement = target.dyn_into::<HtmlInputElement>().expect("Failed to convert to input element");
        let recipe_data = Recipe {name: input.value(), ..recipe.get().0.clone()};
        recipe.set((id, recipe_data));
        panic!("Value: {:?}", input.value());
    });
    view! {
        div(class = format!("{} recipe-tile", class())){
            div(class = format!("{} close-button", x_class()), on:click=handle_close_x) {

            }
            div( on:dblclick=edit, on:click=handle_click,) {
                div(class = format!("recipe-title {}", display_class_gen(&selected_state_signal, id))) { (recipe_data.name.clone()) }
                div(class =  format!("recipe-title {}", form_class_gen(&signal_clone_1, id))) {
                    form(class="pure-form") {
                        fieldset {
                            input(type="text", value=name_clone, on:change = set_name)

                        }
                    }
                }
                div(class = "l-box recipe-ingredients") {
                    p(style = "font-weight: 600;") {"Ingredients"}
                    ul {
                        Indexed(IndexedProps {
                            iterable: ingredient_signal.handle(),
                            template: cloned!((signal_clone) => move |x| {
                                view! { IngredientComponent((signal_clone.clone(), id, x)) }
                            }),
                        })
                        li (class =  format!("plus-button {}", form_class_gen(&signal_clone_3, id)), on:click = new_ingredient)
                    }
                    p(style = "font-weight: 600;") {"Instructions"}
                    div(class = format!("recipe-description {}", display_class_gen(&signal_clone, id)), dangerously_set_inner_html = &markdown_to_html(&recipe_data.description.clone())) {}
                    div(class = format!("recipe-description {}", form_class_gen(&signal_clone_2, id))) {
                        form(class = "pure-form") {
                            fieldset {
                                textarea(style = "width: 100%; height: 500pt;") {(description_clone )}
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component(IngredientComponent<G>)]
fn ingredient_component(
    (selected_state_signal, id, ingredient): (Signal<Option<SelectedState>>, i64, Ingredient),
) -> View<G> {
    let form_class = cloned!((selected_state_signal) => move  || {
        if instance_is_editing(selected_state_signal.get().as_ref(), id) {
            "shown"
        } else {
            "hidden"
        }
    });

    let entry_class = cloned!((selected_state_signal) => move  || {
        if instance_is_editing(selected_state_signal.get().as_ref(), id) {
            "hidden"
        } else {
            "shown"
        }
    });
    let b = ingredient.name;
    let c = ingredient.quantity;
    let b_clone = b.clone();
    view! {
        li(class = entry_class()) {(format!("{:?} {:}", c, b))}
        li(class = form_class()) {

            form(class="pure-form") {
                fieldset {
                    input(type="text", valu = b_clone)

                }
            }
        }
    }
}

fn instance_is_editing(selected_state: &Option<SelectedState>, id: i64) -> bool {
    match selected_state {
        Some(selected_state) => *selected_state.editing.get() && (selected_state.id.get().id == id),
        _ => false,
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
