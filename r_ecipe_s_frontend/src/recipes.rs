use crate::form_component::*;
use crate::search_bar::SearchBar;
use crate::util::{background, markdown_to_html, recover_default_and_log_err, FrontErr};
use r_ecipe_s_model::{serde_json, Recipe, RecipeWithId, RecipesResponse};
use r_ecipe_s_style::generated::*;
use serde::{Deserialize, Serialize};
use sycamore::futures::ScopeSpawnFuture;
use sycamore::prelude::*;
use sycamore::rt::{JsCast, JsValue};
use sycamore::suspense::Suspense;
use tailwindcss_to_rust_macros::*;
use web_sys::{Event, EventTarget, HtmlInputElement, HtmlTextAreaElement};

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

fn header() -> String {
    format!(
        "{}",
        DC![
            C.typ.text_lg,
            C.spc.pr_3,
            C.spc.pl_3,
            C.spc.pt_2,
            C.spc.pb_2,
            C.bg.bg_amber_200,
            C.bor.rounded_t_lg,
            C.siz.h_12
        ]
    )
}
fn tile_background() -> String {
    format!(
        "{}",
        DC![C.bg.bg_amber_50, C.bor.rounded_lg, C.fil.drop_shadow_xl]
    )
}

#[component]
pub async fn RecipesPage<G: Html>(scope_ref: ScopeRef<'_>) -> View<G> {
    let raw_state = scope_ref.create_signal(RecipeAppState {
        modal_view: create_rc_signal(None),
        page: create_rc_signal(PageState {
            offset: 0,
            total_pages: 0,
        }),
        recipes: create_rc_signal(vec![]),
        authentication_status: create_rc_signal(AuthenticationToken("".to_string())),
    });
    let modal_view = scope_ref.create_ref(raw_state.get().modal_view.clone());
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
    let selected_for_event = scope_ref.create_ref(modal_view.clone());
    let create_recipe = |_| {
        scope_ref.spawn_future(async {
            let empty_recipe = Recipe {
                description: String::new(),
                ingredients: vec![],
                name: String::new(),
                liked: None,
            };
            let authentication_signal =
                scope_ref.create_ref(raw_state.get().authentication_status.clone());
            let AuthenticationToken(token) = &*authentication_signal.get();
            let body = reqwasm::http::Request::put("/api/v1/recipes")
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {token}"))
                .body(JsValue::from_str(
                    &serde_json::to_string(&empty_recipe).expect("failed to encode recipe as json"),
                ))
                .send()
                .await
                .expect("failed to get response from PUT recipes")
                .text()
                .await
                .expect("failed to get text from response body");
            let _id = serde_json::from_str::<i64>(&body)
                .expect("failed to decode id and recipe from json");

            let RecipesResponse {
                recipes,
                total_pages,
            } = get_recipes_at_offset(0)
                .await
                .expect("Failed to get recipes");
            raw_state.get().page.set(PageState {
                offset: 0,
                total_pages,
            });
            let recipes = recipes
                .into_iter()
                .map(|recipe| create_rc_signal(recipe.signal()))
                .collect::<Vec<_>>();
            let new = recipes
                .get(0)
                .expect("Shouldn't have empty first recipe")
                .clone();
            raw_state.get().recipes.set(recipes);

            let new_state = Some(ModalView::Recipe(SelectedState {
                recipe: new,
                editing: true,
                changed: create_rc_signal(false),
            }));
            selected_for_event.set(new_state);
        })
    };

    let recipes = scope_ref.create_ref(raw_state.get().recipes.clone());
    view! { scope_ref,
        div(class = DC![C.siz.h_fit, C.siz.min_h_screen, C.siz.w_screen]) {
            div(class = DC![C.lay.flex]) {
                div(class = DC![C.siz.w_32, C.siz.h_24, C.bg.bg_contain, C.bg.bg_no_repeat, C.spc.mt_6, C.spc.ml_6], style = background("ferris-chef.svg"))
                div(class = DC![C.fg.flex_auto])
                ({
                    let modal_view = raw_state.get().modal_view.clone();
                    let authentication_signal = raw_state.get().authentication_status.clone();
                    let authenticate = move |_| {
                        modal_view.set(Some(ModalView::Authenticate(authentication_signal.clone())));
                    };
                    view! {scope_ref,
                        div(
                            class = DC![C.siz.w_8, C.siz.h_8, C.bg.bg_contain, C.bg.bg_no_repeat, C.spc.mt_6, C.spc.mr_6],
                            style = background("lock.svg"),
                            on:click = authenticate
                        ) {}
                    }
                })
            }
            div(class = DC![
                C.spc.p_6, C.lay.sticky, C.lay.top_2,
                C.lay.z_10,
                M![M.two_xl, C.siz.w_1_of_2],
                M![M.xl, C.siz.w_1_of_2],
                M![M.lg, C.siz.w_full],
                M![M.md, C.siz.w_full],
                M![M.sm, C.siz.w_full],
            ]) {
                div(class = DC![
                    C.lay.flex, C.fg.flex_wrap, C.fg.content_center, C.spc.p_5, C.bg.bg_amber_50, C.bor.rounded_lg,
                    C.fil.drop_shadow_xl
                ]) {
                    div(class = DC![C.fg.flex_auto, C.typ.text_2xl, C.spc.mb_2]) {"RecipeS"}
                    Suspense {
                        fallback: view! {scope_ref, ""},
                        LeftButton(raw_state)
                    }
                    PagePosition(raw_state)
                    Suspense {
                        fallback: view! {scope_ref, ""},
                        RightButton(raw_state)
                    }
                    SearchBar((
                         scope_ref.create_ref(raw_state.get().authentication_status.clone()),
                         scope_ref.create_ref(raw_state.get().modal_view.clone())
                    ))
                }
                Modal((modal_view, scope_ref.create_ref(raw_state.get().authentication_status.clone())))
            }

            div(class = DC![
                C.lay.absolute,
                C.lay.top_72,
                C.lay.grid,
                C.fg.grid_cols_1,
                C.fg.gap_6,
                C.spc.p_5,
                M![M.two_xl, C.fg.grid_cols_3],
                M![M.xl, C.fg.grid_cols_3],
                M![M.lg, C.fg.grid_cols_2],
            ]) {
                Keyed(KeyedProps {
                    iterable: recipes,
                    view:  move  |ctx, recipe| {view! {ctx, // todo make context per recipe?
                        RecipeComponent((modal_view.clone(), recipe.clone()))
                    }},
                    key: |recipe| recipe.get().as_ref().id,
                })
                div(class = DC![
                    C.lay.grid, C.fg.grid_cols_1, C.fg.place_items_center, C.fg.content_center, C.bg.bg_amber_50,
                    C.bor.rounded_lg, C.fil.drop_shadow_xl, C.siz.w_16, C.siz.h_16
                ]) {
                    div(
                        class = DC![C.bg.bg_no_repeat, C.bg.bg_cover, C.siz.h_6, C.siz.w_6, C.fil.drop_shadow_xl],
                        on:click = create_recipe,
                        style = background("plus-circle.svg")
                    )
                }
            }
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
        view! { scope_ref, div(class = DC![C.spc.mb_2, C.fg.flex_auto, C.typ.text_2xl]) {(format!(" page {offset} of {total_pages} "))}}
    })}
}

fn button_class(scope_ref: ScopeRef) -> &str {
    scope_ref.create_ref(format!(
        "{}",
        DC![
            C.fg.flex_none,
            C.siz.h_8,
            C.siz.w_8,
            C.bg.bg_no_repeat,
            C.bg.bg_contain,
            C.spc.mb_2,
            C.spc.ml_1
        ]
    ))
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
    let button_class = button_class(scope_ref);
    view! { scope_ref, ({
        if page.get().at_last_page() {
            view! {scope_ref, div(class = button_class) {}}
        } else {
            view! {
                scope_ref,
                div(
                    class = button_class,
                    style = background("chevron-right.svg"),
                    on:click=click_right
                ) {}
            }
        }
    })}
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
    let button_class = button_class(scope_ref);
    view! { scope_ref,  ({
        if page.get().offset > 0 {
            view! { scope_ref,
            div(
                class = button_class,
                style = background("chevron-left.svg"),
                on:click=click_left
            )
            }
        } else {
            view! { scope_ref,
            div(class = button_class) {}
            }
        }
    })}
}

#[component]
pub async fn Modal<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    modal_view_and_authentication: (
        &'a Signal<Option<ModalView>>,
        &'a RcSignal<AuthenticationToken>,
    ),
) -> View<G> {
    let (modal_view, authentication_signal) = modal_view_and_authentication;
    let close_modal = move || {
        move |_: Event| {
            // let modal_view_clone = modal_view.clone();
            let authentication_signal = authentication_signal.clone();
            scope_ref.spawn_future(async move {
                let res = if let Some(view) = modal_view.get().as_ref() {
                    view.on_close(authentication_signal).await
                } else {
                    Ok(())
                };
                recover_default_and_log_err("Failed to save recipe: ", res);
                modal_view.set(None);
            });
        }
    };
    view! { scope_ref, (modal_view.get().as_ref().clone().map(|modal_view| {
        view! { scope_ref,

            div(
                class = DC![
                    C.lay.fixed, C.lay.top_0, C.siz.w_full, C.siz.min_h_full, C.siz.h_screen,
                   C.lay.z_30
                ]
            ) {
                div(class = DC![C.lay.flex, C.fg.place_items_center, C.siz.w_full, C.siz.max_w_full]) {
                    div(class = DC![C.fg.flex_auto, C.fg.basis_0])
                    div(class = DC![C.lay.relative, C.siz.max_w_screen_md, C.fg.basis_5_of_6, C.siz.max_h_screen, C.siz.h_auto, C.fg.flex_auto ]) {
                        div(
                            class = DC![
                                C.lay.absolute, C.lay.top_3, C.lay.right_3, C.siz.h_6, C.siz.w_6, C.bg.bg_cover,
                                C.bg.bg_no_repeat, C.lay.z_40
                            ],
                            on:click=close_modal(),
                            style = background("x-circle.svg")
                        ) {}
                        (
                            match modal_view.clone() {
                                ModalView::Recipe(selected_state) => {
                                    let reff = scope_ref.create_signal(selected_state);
                                    view! {scope_ref, RecipeModal(reff)}
                                }
                                ModalView::Authenticate(authentication_signal) => view! { scope_ref, AuthenticationModal(authentication_signal.clone())}
                            }
                        )
                    }
                    div(class = DC![C.fg.flex_auto, C.fg.basis_0])
                }
                div(class = DC![C.lay.absolute, C.lay.fixed, C.lay.top_0, C.siz.w_full, C.siz.h_full, C.lay.z_20], on:click = close_modal()) { br }
            }
        }
    }).unwrap_or_else(|| view! { scope_ref, "" }))}
}

#[component]
pub async fn RecipeModal<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    selected_state_signal: &'a Signal<SelectedState>,
) -> View<G> {
    view! { scope_ref, ( {
        let recipe = scope_ref.create_ref(selected_state_signal.get().recipe.clone());
        let recipe = scope_ref.create_ref(recipe.clone());
        let editing = scope_ref.create_ref(selected_state_signal.get().editing);
        let edit_recipe = move || {
            move |_: Event| {
                let mut selected_state = selected_state_signal.get().as_ref().clone();
                selected_state.editing = true;
                selected_state.changed.set(true);
                selected_state_signal.set(selected_state);
            }
        };
        view! { scope_ref,
            div(class = DC![C.spc.pb_5, C.siz.max_h_screen]) {
                div(
                    class = DC![
                        &tile_background(), C.lay.relative, C.lay.z_30, C.spc.m_1, C.spc.pb_5, C.siz.max_h_full
                    ],
                    // id = format!("recipe-{:?}", recipe_id),
                    on:dblclick = edit_recipe(),
                ) {
                    (if *editing {
                        view! {scope_ref, RecipeDataFormComponent(recipe) }
                    } else {
                        view! {scope_ref, RecipeDataComponent(recipe)}
                    })
                }
            }
        }
    })}
}

#[component]
pub async fn AuthenticationModal<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    authentication_signal: RcSignal<AuthenticationToken>,
) -> View<G> {
    let authentication_signal = scope_ref.create_ref(authentication_signal);
    let set_token = move |event: Event| -> std::result::Result<(), FrontErr> {
        let input_value: Option<EventTarget> = event.target();
        let input_value = input_value
            .ok_or_else(|| {
                FrontErr::Message("Failed to get even target for token change event".into())
            })?
            .dyn_into::<HtmlInputElement>()
            .map_err(|err| {
                FrontErr::Message(format!(
                    "Failed to convert token change event target to input element: {err:?}"
                ))
            })?
            .value();
        authentication_signal.set(AuthenticationToken(input_value));
        Ok(())
    };
    view! { scope_ref, ( {
        let set_token = move |event: Event| -> std::result::Result<(), FrontErr> {
            let input_value: Option<EventTarget> = event.target();
            let input_value = input_value
                .ok_or_else(|| {
                    FrontErr::Message("Failed to get even target for token change event".into())
                })?
                .dyn_into::<HtmlInputElement>()
                .map_err(|err| {
                    FrontErr::Message(format!(
                        "Failed to convert token change event target to input element: {err:?}"
                    ))
                })?
                .value();
            authentication_signal.set(AuthenticationToken(input_value));
            Ok(())
        };
        let set_token = move |event: Event| {
            recover_default_and_log_err("failed to set authentication token", set_token(event))
        };
        view! { scope_ref,
            div(
                class = DC![
                    &tile_background(), C.lay.relative, C.lay.z_30, C.spc.m_1,
                ],
            ) {
                div(class = DC![C.spc.p_3]) {
                    div(class = DC![C.spc.m_3]) {"API Key"}
                    input(
                        type="text",
                        class = DC![C.spc.p_1, C.siz.w_4_of_5, C.spc.m_3],
                        name="spec",
                        on:input = set_token,
                        value = authentication_signal.clone().get().0
                    ) {}
                }
            }
        }
    })}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeSignal {
    id: i64,
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
    div(class = header()) {
        input(class = DC![C.spc.p_1], type="text", value=name.get(), on:input = set_name)
    }
    div(
        class = DC![C.spc.p_6, C.siz.max_h_screen, C.lay.overflow_scroll]
    ) {
        p(class = DC![C.typ.text_xl, C.typ.text_gray_600])  {"Ingredients"}
        IngredientsFormComponent(ingredients)
            p(class = DC![C.typ.text_xl, C.typ.text_gray_600]) {"Directions"}
        div {
            textarea(class = DC![C.siz.w_full, C.siz.h_60], on:input = set_description) {(description.get())}
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
    let name = scope_ref.create_ref(recipe.get().name.clone());
    view! { scope_ref,
        div(class = header()) {
            p {(format!("{}", name.get()))}
        }
        div(class = DC![C.spc.p_3, C.spc.mb_5, C.siz.max_h_screen, C.lay.overflow_scroll]) {
            RecipeBody(recipe)
        }
    }
}

#[component]
pub fn RecipeBody<G: Html>(scope_ref: ScopeRef, recipe: &RcSignal<RecipeSignal>) -> View<G> {
    let ingredients = scope_ref.create_ref(recipe.get().ingredients.clone());
    let description = scope_ref.create_ref(recipe.get().description.clone());
    view! { scope_ref,
        p(class = DC![C.typ.text_xl, C.typ.text_gray_600]) {"Ingredients"}
        div(class = DC![C.spc.p_3]) {
            IngredientsComponent(ingredients)
        }
        p(class = DC![C.typ.text_xl, C.typ.text_gray_600]) {"Directions"}
        div(class = DC![C.pro.prose, C.typ.whitespace_normal,  C.siz.max_h_full, C.siz.max_w_full, C.spc.p_3, C.spc.mb_5], dangerously_set_inner_html = &markdown_to_html(&description.get()))
    }
}

#[component]
pub fn RecipeComponent<G: Html>(
    scope_ref: ScopeRef,
    (modal_view, recipe): (RcSignal<Option<ModalView>>, RcSignal<RecipeSignal>),
) -> View<G> {
    let recipe_id = recipe.get().id;
    let recipe = scope_ref.create_ref(recipe);

    let name = scope_ref.create_ref(recipe.get().name.clone());
    view! { scope_ref,
        div(
            on:click = move |_: Event| {
                let new_signal = Some(
                    ModalView::Recipe(SelectedState {
                        recipe: recipe.clone(),
                        editing: false,
                        changed: create_rc_signal(false)
                    }
                    ));
                modal_view.set(new_signal);
            }
           ) {
            div(
                class = DC![&tile_background(), C.siz.max_h_80, C.typ.truncate, C.lay.relative],
                id = format!("recipe-{:?}", recipe_id)
               ) {

                div(class = header()) {
                    p {(format!("{}", name.get()))}
                }
                div(class = DC![C.spc.p_3, C.spc.mb_5, C.siz.max_h_screen]) {
                    RecipeBody(recipe)
                }
                div(class = DC![
                    C.lay.absolute, C.siz.h_2_of_3, C.siz.w_full, C.lay.bottom_0, C.bor.rounded_t_lg, C.bg.bg_gradient_to_t,
                    C.bg.from_amber_50
                ])
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
pub struct AuthenticationToken(pub String);

#[derive(Debug, Clone)]
pub enum ModalView {
    Recipe(SelectedState),
    Authenticate(RcSignal<AuthenticationToken>),
}

impl ModalView {
    pub async fn on_close(
        &self,
        authentication_signal: RcSignal<AuthenticationToken>,
    ) -> Result<()> {
        match self {
            Self::Recipe(selected_state) => {
                selected_state
                    .save(authentication_signal.get().as_ref())
                    .await
            }
            Self::Authenticate(_) => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecipeAppState {
    pub modal_view: RcSignal<Option<ModalView>>,
    pub page: RcSignal<PageState>,
    pub recipes: RcSignal<Vec<RcSignal<RecipeSignal>>>,
    pub authentication_status: RcSignal<AuthenticationToken>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedState {
    pub recipe: RcSignal<RecipeSignal>,
    pub editing: bool,
    pub changed: RcSignal<bool>,
}

impl SelectedState {
    pub async fn save(&self, AuthenticationToken(token): &AuthenticationToken) -> Result<()> {
        let recipe = self.recipe.get();
        //.expect("Failed to get current recipe from viewer. This is a bug");
        let recipe_id = recipe.id;
        if *self.changed.get() {
            let resp = reqwasm::http::Request::post(&format!("/api/v1/recipes/{recipe_id}"))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {token}"))
                .body(JsValue::from_str(&serde_json::to_string(
                    &RecipeWithId::from_signal(self.recipe.get().as_ref()).data,
                )?))
                .send()
                // .expect("failed to get response from POST recipes/:id")
                .await?;
            // .expect("failed to get response from POST recipes/:id}");

            let body = resp.text().await?;
            serde_json::from_str::<i64>(&body)?;
            self.changed.set(false);
        }

        let e: Result<()> = Ok(());
        e
    }
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
