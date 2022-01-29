use crate::auto_form_component::*;
use crate::util::markdown_to_html;
use perseus::*;
use r_ecipe_s_model::{Recipe, RecipeId, RecipeWithId};
use serde::{Deserialize, Serialize};
use sycamore::flow::{Indexed, IndexedProps};
use sycamore::prelude::*;
use sycamore::rt::{JsCast, JsValue};
use web_sys::{Event, HtmlInputElement, HtmlTextAreaElement};

async fn get_recipes_at_offset(
    offset: u32,
) -> Result<Vec<RecipeWithId>, perseus::GenericErrorWithCause> {
    let body = reqwasm::http::Request::get(&format!("/api/v1/recipes?offset={offset}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    serde_json::from_str::<Vec<RecipeWithId>>(&body).map_err(|err| perseus::GenericErrorWithCause {
        error: Box::new(err),
        cause: ErrorCause::Client(None),
    })
}

#[perseus::template(RecipesPage)]
#[component(RecipesPage<G>)]
pub fn recipes_page() -> View<G> {
    let raw_state = RecipeAppState {
        selected: Signal::new(None),
        page: Signal::new(PageState { offset: 0 }),
    };
    let selected = raw_state.selected;
    let page = raw_state.page;
    let recipes_signal = Signal::new(vec![]);
    if G::IS_BROWSER {
        perseus::spawn_local(cloned!((recipes_signal) => async move {
            let recipes = get_recipes_at_offset(0).
                await
                .expect("err")
                    .into_iter()
                    .map(|recipe| recipe.signal())
                    .collect::<Vec<_>>();

            recipes_signal.set(recipes);
        }));
    }
    let create_recipe = cloned!((recipes_signal, selected) => move |_| {
        let mut vec = recipes_signal.get().iter().cloned().collect::<Vec<_>>();
        perseus::spawn_local(
            cloned!((selected, recipes_signal) => async move {
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

                let new = RecipeWithId { id: RecipeId { id }, data: empty_recipe}.signal();

                vec.push(new.clone());
                recipes_signal.set(vec);

                let new_state = SelectedState {
                    recipe: new,
                    editing: true,

                };
                selected.set(Some(new_state));
            })

        );

    });

    let selected_for_recipes = selected.clone();
    let recipes_for_left = recipes_signal.clone();
    let recipes_for_right = recipes_signal.clone();
    let page_right = page.clone();
    view! {
        div(class = "header") {
            span {"RecipeS – "}
            (cloned!((page, recipes_for_left) => left_button(page, recipes_for_left)))
            (cloned!((page_right, recipes_for_right) => right_button(page_right, recipes_for_right)))
        }

            (cloned!((selected) => viewer(selected)))
            (
                {
                    ""
                }
            )
            Indexed(IndexedProps {
                iterable: recipes_signal.handle(),
                template:  cloned!((selected_for_recipes) => move |recipe| {
                    recipe_component((selected_for_recipes.clone(), recipe))
                }),
            })
            div(class = "col-sm-6 col-md-4  unselected", ) {
                div(class = "plus-button", on:click = create_recipe, dangerously_set_inner_html="&nbsp;")
            }
    }
}

pub fn right_button<G: sycamore::generic_node::GenericNode + perseus::Html>(
    selected: Signal<PageState>,
    recipes_signal: Signal<Vec<Signal<RecipeSignal>>>,
) -> View<G> {
    let click_right = cloned!((selected, recipes_signal) => move |_: Event| {
        let current_offset = selected.get().offset;
        let new_offset = current_offset + 1;
        if G::IS_BROWSER {
            perseus::spawn_local(cloned!((selected, recipes_signal) => async move {

                let recipes = get_recipes_at_offset(new_offset).
                    await
                    .expect("err")
                        .into_iter()
                        .map(|recipe| recipe.signal())
                        .collect::<Vec<_>>();

                recipes_signal.set(recipes);

                selected.set(PageState{
                    offset: new_offset
                });

            }));
        }
    });
    if G::IS_BROWSER {
        web_sys::console::log_1(&"HELP0".into());
    }
    view! {
        span(on:click=click_right) { "(Right)"}
    }
}

pub fn left_button<G: sycamore::generic_node::GenericNode + perseus::Html>(
    selected: Signal<PageState>,
    recipes_signal: Signal<Vec<Signal<RecipeSignal>>>,
) -> View<G> {
    let click_left = cloned!((selected, recipes_signal) => move |_: Event| {
        let current_offset = selected.get().offset;
        let new_offset = current_offset - 1;
        if G::IS_BROWSER {
            perseus::spawn_local(cloned!((selected, recipes_signal) => async move {

                let recipes = get_recipes_at_offset(new_offset).
                    await
                    .expect("err")
                        .into_iter()
                        .map(|recipe| recipe.signal())
                        .collect::<Vec<_>>();

                recipes_signal.set(recipes);

                selected.set(PageState{
                    offset: new_offset
                });

            }));
        }
    });
    if selected.get().offset > 0 {
        view! {
        span(on:click=click_left) { "(Left)"}
         }
    } else {
        view! {
            span {}
        }
    }
}

pub fn viewer<G: sycamore::generic_node::GenericNode + perseus::Html>(
    selected: Signal<Option<SelectedState>>,
) -> View<G> {
    let close_recipe = cloned!((selected) => move || {
        cloned!((selected) => move |_: Event| {
            let recipe_option: Option<RecipeWithId> = selected
                .get()
                .as_ref()
                .as_ref()
                .map(|selected_state| {
                    DataToSignal::from_signal(&selected_state.recipe)
                });

            let recipe = recipe_option.expect("Failed to get current recipe from viewer. This is a bug");
            let recipe_id = recipe.id.id;
            if G::IS_BROWSER {
                perseus::spawn_local(async move  {
                    let resp = reqwasm::http::Request::post(&format!("/api/v1/recipes/{recipe_id}"))
                        .header("Content-Type", "application/json")
                        .body(JsValue::from_str(&serde_json::to_string(&recipe.data).expect("failed to encode recipe as json")))
                        .send()
                        .await
                        .expect("failed to get response from POST recipes/{{id}}");

                    let body = resp
                        .text()
                        .await
                        .expect("failed to get text from response body");
                    serde_json::from_str::<RecipeId>(&body).expect("failed to decode id and recipe from json");
                });
            }
            selected.set(None);
        })
    });

    let edit_recipe = cloned!((selected) => move || {
        cloned!((selected) => move |_: Event| {
            let mut selected_state = selected
                .get()
                .as_ref().clone().expect("We shouldn't be able to edit a recipe if it isn't open");
            selected_state.editing = true;
            selected.set(Some(selected_state));
        })
    });

    cloned!((selected) => match selected.get().as_ref().clone() {

        Some(SelectedState { recipe, editing }) => {
            let recipe_id = recipe.get().id.id;
            cloned!((recipe) => view! {
                div(on:dblclick = edit_recipe()) {
                    div(class = "recipe-tile selected", id = format!("recipe-{:?}", recipe_id)) {
                        div(class = "close-button", on:click=close_recipe()) {}
                            (if editing {
                                recipe.form()
                            } else {
                                recipe.component()
                            })
                    }
                }
                div(class = "de-selector", on:click=close_recipe()) { br }
            })
        }
        None =>  {
            view! { "" }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeSignal {
    id: RecipeId,
    name: Signal<String>,
    ingredients: Signal<Vec<(usize, Signal<IngredientSignal>)>>,
    description: Signal<String>,
}

impl DataToSignal for RecipeWithId {
    type SignalType = Signal<RecipeSignal>;
    fn signal(&self) -> Self::SignalType {
        Signal::new(RecipeSignal {
            id: self.id,
            name: Signal::new(self.data.name.to_string()),
            ingredients: self.data.ingredients.signal(),
            description: Signal::new(self.data.description.to_string()),
        })
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        let recipe_signal = signal_type;
        let name = recipe_signal.get().name.get().to_string();
        let description = recipe_signal.get().description.get().to_string();
        let ingredients = DataToSignal::from_signal(&recipe_signal.get().ingredients);
        RecipeWithId {
            id: signal_type.get().id,
            data: Recipe {
                name,
                ingredients,
                description,
                liked: None,
            },
        }
    }
}

impl DataToFormComponent for Signal<RecipeSignal> {
    type DataType = RecipeWithId;

    fn form<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let ingredients = self.get().ingredients.clone();
        let name = self.get().name.clone();
        let description = self.get().description.clone();
        let set_name = cloned!((name) => move |event: Event| {
            let input_value = event
                .target()
                .expect("Failed to get even target for name change event")
                .dyn_into::<HtmlInputElement>()
                .expect("Failed to convert name change event target to input element")
                .value();
            name.set(input_value);
        });
        let set_description = cloned!((description) => move |event: Event| {
            let input_value  = event
                .target()
                .expect("Failed to get even target for name change event")
                .dyn_into::<HtmlTextAreaElement>()
                .expect("Failed to convert name change event target to input element")
                .value();
            description.set(input_value);
        });
        view! {
            div(class = "recipe-title") {
                input(type="text", value=name.get(), on:change = set_name)
            }
            div(class = "recipe-body") {
                p(style = "font-weight: 600;") {"Ingredients"}
                (ingredients.form())
                p(style = "font-weight: 600;") {"Directions"}
                div(class = "recipe-description") {
                    textarea(style = "width: 100%; height: 500pt;", on:change = set_description) {(description.get())}
                }
            }
        }
    }
}

impl DataToComponent for Signal<RecipeSignal> {
    type DataType = RecipeWithId;

    fn component<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let ingredients = self.get().ingredients.clone();
        let name = self.get().name.clone();
        let description = self.get().description.clone();
        view! {
            div(class = "recipe-title") {
                span {(format!("{} ", name.get()))}
            }
            div(class = "recipe-body") {
                p(style = "font-weight: 600;") {"Ingredients"}
                (ingredients.component())

                    p(style = "font-weight: 600;") {"Directions"}
                div(class = "recipe-description", dangerously_set_inner_html = &markdown_to_html(&description.get()))
            }
        }
    }
}

fn recipe_component<G: sycamore::generic_node::GenericNode + perseus::Html>(
    (selected_signal, recipe): (Signal<Option<SelectedState>>, Signal<RecipeSignal>),
) -> View<G> {
    let recipe_id = recipe.get().id.id;

    cloned!((selected_signal, recipe) => view! {
        div(
            class = "col-sm-6 col-md-4",
            on:click = cloned!((selected_signal, recipe) => move |_: Event| {
                let new_signal = Some(
                    SelectedState {
                        recipe: recipe.clone(),
                        editing: false
                    }
                );
                selected_signal.set(new_signal);
            })
        ) {
            div(
                class = "recipe-tile unselected",
                id = format!("recipe-{:?}", recipe_id)
            ) {
                (recipe.component())
            }
        }
    })
}

#[perseus::head]
pub fn head() -> View<SsrNode> {
    view! {
        title { "Recipes"}
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::new("recipes").template(recipes_page).head(head)
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

#[derive(Debug, Clone)]
pub struct RecipeAppState {
    selected: Signal<Option<SelectedState>>,
    page: Signal<PageState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedState {
    recipe: Signal<RecipeSignal>,
    editing: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageState {
    offset: u32,
}
