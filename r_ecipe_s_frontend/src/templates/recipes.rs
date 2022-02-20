use crate::auto_form_component::*;
use crate::util::markdown_to_html;
use r_ecipe_s_model::{Recipe, RecipeId, RecipeWithId, RecipesResponse};
use serde::{Deserialize, Serialize};
use sycamore::futures::ScopeSpawnFuture;
use sycamore::prelude::*;
use sycamore::rt::{JsCast, JsValue};
use sycamore::suspense::Suspense;
use web_sys::{Event, HtmlInputElement, HtmlTextAreaElement};

use anyhow::Error;

pub type Result<T> = std::result::Result<T, Error>;
pub async fn get_recipes_at_offset(offset: u32) -> Result<RecipesResponse> {
    let body = reqwasm::http::Request::get(&format!("/api/v1/recipes?offset={offset}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    serde_json::from_str::<RecipesResponse>(&body).map_err(|err| err.into())
}

#[component]
pub async fn RecipesPage<G: Html>(scope_ref: ScopeRef<'_>) -> View<G> {
    let raw_state = scope_ref.create_signal(RecipeAppState {
        selected: create_rc_signal(None),
        page: create_rc_signal(PageState {
            offset: 0,
            total_pages: 0,
        }),
        recipes: create_rc_signal(vec![]),
    });
    let selected = scope_ref.create_ref(raw_state.get().selected.clone());
    let recipes_response = get_recipes_at_offset(0).await.expect("err");
    let total_pages = recipes_response.total_pages;
    let page = PageState {
        offset: 0,
        total_pages,
    };
    raw_state.get().page.set(page);
    let recipes_data = recipes_response
        .recipes
        .into_iter()
        .map(|recipe| create_rc_signal(recipe.signal()))
        .collect::<Vec<_>>();

    raw_state.get().recipes.set(recipes_data);
    let selected_for_event = scope_ref.create_ref(selected.clone());
    let create_recipe = |_| {
        scope_ref.spawn_future(async {
            let mut vec = raw_state
                .get()
                .recipes
                .get()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let empty_recipe = Recipe {
                description: String::new(),
                ingredients: vec![],
                name: String::new(),
                liked: None,
            };
            let body = reqwasm::http::Request::put("/api/v1/recipes")
                .header("Content-Type", "application/json")
                .body(JsValue::from_str(
                    &serde_json::to_string(&empty_recipe).expect("failed to encode recipe as json"),
                ))
                .send()
                .await
                .expect("failed to get response from PUT recipes")
                .text()
                .await
                .expect("failed to get text from response body");
            let id = serde_json::from_str::<i64>(&body)
                .expect("failed to decode id and recipe from json");

            let new = create_rc_signal(
                RecipeWithId {
                    id: RecipeId { id },
                    data: empty_recipe,
                }
                .signal(),
            );

            vec.push(new.clone());
            raw_state.get().recipes.set(vec);

            let new_state = Some(SelectedState {
                recipe: new,
                editing: true,
            });
            selected_for_event.set(new_state);
        })
    };

    let recipes = scope_ref.create_ref(raw_state.get().recipes.clone());
    view! { scope_ref,
        div(class = "header") {
            span {"RecipeS – "}
            Suspense {
                fallback: view! {scope_ref, ""},
                LeftButton(raw_state)
            }
            PagePosition(raw_state)
            Suspense {
                fallback: view! {scope_ref, ""},
                RightButton(raw_state)
            }
        }

        Viewer(selected)
            Keyed(KeyedProps {
                iterable: recipes,
                view:  move  |ctx, recipe| {view! {ctx, // todo make context per recipe?
                    RecipeComponent((selected, &recipe))
                }},
                key: |recipe| recipe.get().as_ref().id.id,
            })
        div(class = "col-sm-6 col-md-4  unselected") {
            div(class = "plus-button", on:click = create_recipe, dangerously_set_inner_html="&nbsp;")
        }
    }
}
#[component]
pub fn PagePosition<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    app_state: &'a Signal<RecipeAppState>,
) -> View<G> {
    view! {scope_ref,  ({
        let PageState{ mut offset, mut total_pages } = *app_state.get().page.get();
        offset += 1;
        total_pages += 1;
        view! { scope_ref, (format!(" page {offset} of {total_pages} ")) }
    })}
}

#[component]
pub async fn RightButton<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    app_state: &'a Signal<RecipeAppState>,
) -> View<G> {
    let page = scope_ref.create_ref(app_state.get().page.clone());
    let recipes = scope_ref.create_ref(app_state.get().recipes.clone());
    let click_right = move |_: Event| {
        scope_ref.spawn_future(async {
            let current_offset = page.get().offset;
            let new_offset = current_offset + 1;

            let RecipesResponse {
                recipes: new_recipes,
                total_pages,
            } = get_recipes_at_offset(new_offset).await.expect("err");
            let recipes_data = new_recipes
                .into_iter()
                .map(|recipe| create_rc_signal(recipe.signal()))
                .collect::<Vec<_>>();

            recipes.set(recipes_data);

            page.set(PageState {
                offset: new_offset,
                total_pages,
            });
        });
    };
    view! { scope_ref, ({
        if page.get().at_last_page() {
            view! {scope_ref, ""}
        } else {
            view! {scope_ref, span(on:click=click_right) { "(Right)"} }
        }
    })
    }
}

#[component]
pub async fn LeftButton<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    app_state: &'a Signal<RecipeAppState>,
) -> View<G> {
    let page = scope_ref.create_ref(app_state.get().page.clone());
    let recipes = scope_ref.create_ref(app_state.get().recipes.clone());

    let click_left = |_: Event| {
        scope_ref.spawn_future(async {
            let current_offset = page.get().offset;
            let new_offset = current_offset - 1;

            let RecipesResponse {
                recipes: new_recipes,
                total_pages,
            } = get_recipes_at_offset(new_offset).await.expect("err");
            let recipes_data = new_recipes
                .into_iter()
                .map(|recipe| create_rc_signal(recipe.signal()))
                .collect::<Vec<_>>();

            recipes.set(recipes_data);

            page.set(PageState {
                offset: new_offset,
                total_pages,
            });
        });
    };
    view! { scope_ref,  ({
        if page.get().offset > 0 {
            view! { scope_ref,
            span(on:click=click_left) { "(Left)"}
            }
        } else {
            view! { scope_ref,
            span {}
            }
        }
    })
    }
}

#[component]
pub async fn Viewer<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    selected: &'a Signal<Option<SelectedState>>,
) -> View<G> {
    let close_recipe = move || {
        move |_: Event| {
            scope_ref.spawn_future(async {
                let recipe_option: Option<RecipeWithId> =
                    selected.get().as_ref().as_ref().map(|selected_state| {
                        DataToSignal::from_signal(selected_state.recipe.get().as_ref())
                    });

                let recipe =
                    recipe_option.expect("Failed to get current recipe from viewer. This is a bug");
                let recipe_id = recipe.id.id;
                let resp = reqwasm::http::Request::post(&format!("/api/v1/recipes/{recipe_id}"))
                    .header("Content-Type", "application/json")
                    .body(JsValue::from_str(
                        &serde_json::to_string(&recipe.data)
                            .expect("failed to encode recipe as json"),
                    ))
                    .send()
                    .await
                    .expect("failed to get response from POST recipes/{{id}}");

                let body = resp
                    .text()
                    .await
                    .expect("failed to get text from response body");
                serde_json::from_str::<RecipeId>(&body)
                    .expect("failed to decode id and recipe from json");
                selected.set(None);
            });
        }
    };

    let edit_recipe = move || {
        move |_: Event| {
            let mut selected_state = selected
                .get()
                .as_ref()
                .clone()
                .expect("We shouldn't be able to edit a recipe if it isn't open");
            selected_state.editing = true;
            selected.set(Some(selected_state));
        }
    };

    view! {scope_ref, ({
        let selected_ref = selected.get();
        match selected_ref.as_ref() {
            Some(selected_state) => {
                let recipe = scope_ref.create_ref(selected_state.recipe.clone());
                let recipe_id = recipe.get().id.id;
                let recipe = scope_ref.create_ref(recipe.clone());
                let editing = scope_ref.create_ref(selected_state.editing);
                view! { scope_ref,
                div(on:dblclick = edit_recipe()) {
                    div(class = "recipe-tile selected", id = format!("recipe-{:?}", recipe_id)) {
                        div(class = "close-button", on:click=close_recipe()) {}
                        (if *editing {
                            view! {scope_ref, RecipeDataFormComponent(recipe) }
                        } else {
                            view! {scope_ref, RecipeDataComponent(recipe)}
                        })
                    }
                }
                div(class = "de-selector", on:click=close_recipe()) { br }
                }
            }
            None => {
                view! { scope_ref, "" }
            }
        }
    })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeSignal {
    id: RecipeId,
    name: RcSignal<String>,
    ingredients: RcSignal<Vec<RcSignal<(usize, IngredientSignal)>>>,
    description: RcSignal<String>,
}

impl DataToSignal for RecipeWithId {
    type SignalType = RecipeSignal;
    fn signal(&self) -> Self::SignalType {
        RecipeSignal {
            id: self.id,
            name: create_rc_signal(self.data.name.to_string()),
            ingredients: create_rc_signal(self.data.ingredients.signal()),
            description: create_rc_signal(self.data.description.to_string()),
        }
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        let recipe_signal = signal_type;
        let name = recipe_signal.name.get().to_string();
        let description = recipe_signal.description.get().to_string();
        let ingredients = DataToSignal::from_signal(recipe_signal.ingredients.get().as_ref());
        RecipeWithId {
            id: signal_type.id,
            data: Recipe {
                name,
                ingredients,
                description,
                liked: None,
            },
        }
    }
}

#[component]
fn RecipeDataFormComponent<G: Html>(
    scope_ref: ScopeRef,
    recipe: &RcSignal<RecipeSignal>,
) -> View<G> {
    let recipe = scope_ref.create_ref(recipe.clone());
    let ingredients = scope_ref.create_ref(recipe.get().ingredients.clone());
    let name = scope_ref.create_ref(recipe.get().name.clone());
    let description = scope_ref.create_ref(recipe.get().description.clone());
    let set_name = move |event: Event| {
        let input_value = event
            .target()
            .expect("Failed to get even target for name change event")
            .dyn_into::<HtmlInputElement>()
            .expect("Failed to convert name change event target to input element")
            .value();
        name.set(input_value);
    };
    let set_description = move |event: Event| {
        let input_value = event
            .target()
            .expect("Failed to get even target for name change event")
            .dyn_into::<HtmlTextAreaElement>()
            .expect("Failed to convert name change event target to input element")
            .value();
        description.set(input_value);
    };
    view! { scope_ref,
    div(class = "recipe-title") {
        input(type="text", value=name.get(), on:change = set_name)
    }
    div(class = "recipe-body") {
        p(style = "font-weight: 600;") {"Ingredients"}
        IngredientsFormComponent(ingredients)
            p(style = "font-weight: 600;") {"Directions"}
        div(class = "recipe-description") {
            textarea(style = "width: 100%; height: 500pt;", on:change = set_description) {(description.get())}
        }
    }
    }
}

#[component]
pub fn RecipeDataComponent<G: Html>(
    scope_ref: ScopeRef,
    recipe: &RcSignal<RecipeSignal>,
) -> View<G> {
    let recipe = scope_ref.create_ref(recipe.clone());
    let ingredients = scope_ref.create_ref(recipe.get().ingredients.clone());
    let name = scope_ref.create_ref(recipe.get().name.clone());
    let description = scope_ref.create_ref(recipe.get().description.clone());
    view! { scope_ref,
    div(class = "recipe-title") {
        span {(format!("{} ", name.get()))}
    }
    div(class = "recipe-body") {
        p(style = "font-weight: 600;") {"Ingredients"}
        IngredientsComponent(ingredients)

            p(style = "font-weight: 600;") {"Directions"}
        div(class = "recipe-description", dangerously_set_inner_html = &markdown_to_html(&description.get()))
    }
    }
}

#[component]
pub fn RecipeComponent<G: Html>(
    scope_ref: ScopeRef,
    (selected_signal, recipe): (&RcSignal<Option<SelectedState>>, &RcSignal<RecipeSignal>),
) -> View<G> {
    let recipe_id = recipe.get().id.id;
    let recipe = scope_ref.create_ref(recipe.clone());
    let selected_signal = scope_ref.create_ref(selected_signal.clone());

    view! { scope_ref,
    div(
        class = "col-sm-6 col-md-4",
        on:click = move |_: Event| {
            web_sys::console::log_1(&format!("{:?}  Help", selected_signal.get().as_ref()).into());
            let new_signal = Some(
                SelectedState {
                    recipe: recipe.clone(),
                    editing: false
                }
                );
            selected_signal.set(new_signal);
        }
       ) {
        div(
            class = "recipe-tile unselected",
            id = format!("recipe-{:?}", recipe_id)
           ) {
            RecipeDataComponent(recipe)
        }
    }
    }
}

trait IntoOk
where
    Self: Sized,
{
    fn to_ok<ErrType>(self) -> std::result::Result<Self, ErrType>;
}
impl<T: Sized> IntoOk for T {
    fn to_ok<ErrType>(self) -> std::result::Result<T, ErrType> {
        Ok(self)
    }
}

#[derive(Debug, Clone)]
pub struct RecipeAppState {
    pub selected: RcSignal<Option<SelectedState>>,
    pub page: RcSignal<PageState>,
    pub recipes: RcSignal<Vec<RcSignal<RecipeSignal>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedState {
    pub recipe: RcSignal<RecipeSignal>,
    pub editing: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageState {
    pub offset: u32,
    pub total_pages: i64,
}

impl PageState {
    fn at_last_page(&self) -> bool {
        self.offset == self.total_pages as u32
    }
}
