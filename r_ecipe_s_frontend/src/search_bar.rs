use r_ecipe_s_model::{RecipeWithId, SearchQuery, SearchResponse, SearchResult};
use r_ecipe_s_style::generated::{C, M};
use sycamore::futures::ScopeSpawnFuture;
use sycamore::prelude::*;
use sycamore::rt::JsCast;
use tailwindcss_to_rust_macros::*;
use web_sys::{Event, EventTarget, HtmlInputElement};

use crate::{
    form_component::DataToSignal,
    recipes::{AuthenticationToken, ModalView, SelectedState},
    util::{recover_default_and_log_err, FrontErr},
};
#[component]
pub fn SearchBar<'a, G: Html>(
    scope_ref: ScopeRef<'a>,
    search_bar_data: (
        &'a RcSignal<AuthenticationToken>,
        &'a RcSignal<Option<ModalView>>,
    ),
) -> View<G> {
    let (authentication_token, modal_view) = search_bar_data;
    let search_result_signal = scope_ref.create_signal::<Vec<RecipeWithId>>(vec![]);
    let authentication_token = scope_ref.create_ref(authentication_token.clone());
    let run_query = move |event: Event| async move {
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
        if input_value.is_empty() {
            search_result_signal.set(vec![])
        } else {
            let token = authentication_token.get().0.to_string();
            let body =
                reqwasm::http::Request::get(&format!("/api/v1/recipes/search?query={input_value}"))
                    .header("Authorization", &format!("Bearer {token}"))
                    .send()
                    .await
                    .map_err(|err| FrontErr::Message(format!("Error executing query: {err}")))?
                    .text()
                    .await
                    .map_err(|err| {
                        FrontErr::Message(format!("Error getting text from query response: {err}"))
                    })?;

            let search_response = r_ecipe_s_model::serde_json::from_str::<SearchResponse>(&body)
                .map_err(|err| {
                    FrontErr::Message(format!("Failed to parse recipe search response: {err:?}"))
                })?;
            search_result_signal.set(
                search_response
                    .results
                    .into_iter()
                    .map(|result| result.recipe)
                    .collect(),
            );
        }

        Ok(()) as Result<(), FrontErr>
    };
    let query = move |event| {
        scope_ref.spawn_future(async move {
            recover_default_and_log_err("failure in search", run_query(event).await);
        })
    };
    view! {
        scope_ref,
        div {
            input(
                type="text",
                class = DC![C.bor.rounded_lg, C.spc.ml_2],
                on:input= query,
            )

            div(
                class = DC![C.bg.bg_amber_100, C.spc.ml_2, C.fil.drop_shadow_xl, C.bor.rounded_lg, C.lay.sticky, C.lay.top_6, C.lay.z_10]
            ) {
                Keyed(KeyedProps {
                    iterable: search_result_signal,
                    view:  move  |ctx, recipe| {
                        let recipe_clone = recipe.clone();
                        view! {ctx, // todo make context per recipe?
                        div(
                            class = DC![
                                C.spc.p_2, C.bg.bg_none,
                                C.lay.z_10, C.siz.h_full,
                                M![M.focus, C.bor.border_amber_300, C.bor.border_2]
                            ],
                            on:click = move |_| {
                                search_result_signal.set(vec![]);

                                modal_view.set(Some(ModalView::Recipe(SelectedState {
                                    recipe: create_rc_signal(recipe.signal()),
                                    editing: false,
                                    changed: create_rc_signal(false),
                                })));
                            }
                        ) {
                            (recipe_clone.data.name)
                        }
                    }},
                    key: |recipe| recipe.id,
                })
            }
        }
    }
}
