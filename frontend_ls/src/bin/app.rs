use either::Either;
use frontend_ls::{AsyncMutex, EncodeOnDemand, Error};
use frontend_ls::{EncodeResponse, MiniLmWorkereComm};
use gloo_worker::reactor::ReactorBridge;

use std::time::Duration;

use leptos::*;
use logging::{log, warn};
use r_ecipe_s_frontend::api::*;
use r_ecipe_s_frontend::form_component_ls::*;
use r_ecipe_s_model::Recipe;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    warn!("Starting");
    mount_to_body(|| {
        view! {
            <App />
        }
    })
}

#[component]
fn DivViewWIthText(text: String) -> impl IntoView {
    text
}

type MiniLmRead = ReadSignal<Option<Rc<AsyncMutex<ReactorBridge<EncodeOnDemand>>>>>;
type MiniLmWrite = WriteSignal<Option<Rc<AsyncMutex<ReactorBridge<EncodeOnDemand>>>>>;

async fn get_embedding(minilm: Option<MiniLmRead>, input: String) -> Option<Vec<f32>> {
    let bridge = minilm?.get_untracked()?;
    let mut guard = bridge.lock().await;
    guard
        .as_mut()
        .send_input(MiniLmWorkereComm::TextInput(input));
    let EncodeResponse(output) = guard.as_mut().next().await?;
    Some(output)
}

fn spawn_minilm(host: &str) -> Rc<AsyncMutex<ReactorBridge<EncodeOnDemand>>> {
    log!("Starting web worker");
    let bridge = EncodeOnDemand::spawner().spawn("/worker.js");
    bridge.send_input(MiniLmWorkereComm::ModelPath(host.to_string()));
    Rc::new(AsyncMutex::new(bridge))
}

#[component]
fn NavBar(
    offset: i64,
    origin: ReadSignal<String>,
    set_edit: WriteSignal<EditModal>,
    get_page_action: Action<i64, (i64, Result<RecipesResponse, Error>)>,
    minilm: MiniLmRead,
    set_minilm: MiniLmWrite,
    set_ai_pref: WriteSignal<bool>,
    api_key: Signal<Option<String>>,
    set_api_key: WriteSignal<Option<String>>,
) -> impl IntoView {
    let spawn_minilm = move || set_minilm.set(Some(spawn_minilm(&origin.get_untracked())));

    let (_, right_disabled): (View, &str) = match get_page_action.value().get_untracked() {
        Some((offset, Ok(RecipesResponse { total_pages, .. }))) => (
            view! { <DivViewWIthText  text = format!("of {total_pages}")/>},
            if offset >= total_pages {
                log!("{offset} total: {total_pages}");
                "btn-disabled"
            } else {
                ""
            },
        ),
        _ => (
            if get_page_action.pending().get() {
                view! { <Pending />}
            } else {
                view! { <DivViewWIthText text = {String::new()}/>}
            },
            "btn-disabled",
        ),
    };

    let left_disabled = match get_page_action.value().get_untracked() {
        Some((0, _)) => "btn-disabled",
        _ => "",
    };
    let search_action = create_action(move |query: &String| {
        let (query, _) = create_signal(query.to_owned());
        async move {
            let query = query.get_untracked();
            let vector = get_embedding(Some(minilm), query.clone()).await;
            let x = search(
                &query,
                vector.as_ref().map(<Vec<f32> as AsRef<[f32]>>::as_ref),
            )
            .await
            .map_err(|err| Error::Msg(format!("We got an Error {err}")));
            log!("Search result: {x:?}");
            x
        }
    });
    let (searching, set_searching) = create_signal(false);

    let (api_input, set_api_input) = create_signal(false);

    let search_view = move || {
        if searching.get() {
            if search_action.pending().get() {
                view! { <Pending />}
            } else if let Some(value) = search_action.value().get() {
                view! {
                    <ErrorBoundary
                        fallback = move | errs| view!{
                            <div>
                                "BLOPP"
                                {move || errs.get().into_iter().map(
                                        move |err| view!{  {format!("{err:?}")} }
                                ).collect::<Vec<_>>()}
                                <Error />
                            </div>
                        }
                    >
                        {value.map(|boop| {
                            boop.results.into_iter().map(|x| {
                                let RecipeWithId {id, data: recipe } = x.recipe;
                                let title = recipe.name.clone();
                                view!{ <li class="tabindex-0" on:click=move |_| {
                                    set_edit.set(EditModal {
                                        state: Some((id, false,  Either::Left(recipe.clone()))),
                                    });
                                    set_searching.set(false);
                                }>{title}</li>}
                            })
                            .collect_view()

                        })}
                    </ErrorBoundary>
                }
            } else {
                view! { <DivViewWIthText text = {"".into()} />}
            }
        } else {
            view! { <DivViewWIthText text = {"".into()}/>}
        }
    };

    let show = move || {
        if searching.get() {
            "visible"
        } else {
            "collapse"
        }
    };
    view! {
        <div class = "navbar bg-base-100">
            <div class="flex-none">
                <details class="dropdown z-30">
                    <summary class="btn btn-square btn-ghost btn-sm">
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="inline-block w-5 h-5 stroke-current"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path></svg>
                    </summary>
                    <ul class="dropdown-content bg-base-100 border border-base-content rounded-box w-52 shadow-md shadow-base-300 ">
                    <li>
                        <button class="btn-sm" on:click = move |_| set_api_input.update(|api_input| *api_input = !*api_input)>
                            Set API Key
                        </button>
                        {
                            move|| { api_input.get().then(move || {
                                view! {<input class = "input input-sm input-bordered w-48 ml-2" on:input= move |event| {
                                        let api_key = event_target_value(&event);
                                        set_api_key.set(Some(api_key))
                                    } value = {api_key.get_untracked().unwrap_or_default()}/>
                                }
                            })}
                        }
                    </li>
                    <li>

            {
                move || minilm.get().map(|_| {
                    view! {
                        <button class="btn-sm" on:click=move |_| {
                            set_minilm.set(None);
                            set_ai_pref.set(false)
                        }>
                            Use AI search X
                        </button>
                    }
                })
                .unwrap_or_else(|| {
                    view! {
                        <button class="btn-sm" on:click=move |_| {
                            set_ai_pref.set(true);
                            spawn_minilm()
                        }>
                            Use AI search O
                        </button>
                    }
                })
            }
                        </li>
                    </ul>
                </details>
            </div>
            <div class = "flex-1" />
            <div class = "flex-1">
                <div class="join">
                    <button class={format!("join-item btn btn-sm {left_disabled}")} on:click = {move |_| {
                        let new_offset = offset - 1;
                        get_page_action.dispatch(new_offset);
                    }}>"«"</button>
                    <button class="join-item btn btn-sm">Page {offset + 1} {}</button>
                    <button class={format!("join-item btn btn-sm {right_disabled}")} on:click = {move |_| {
                        let new_offset = offset + 1;
                        log!("Offset {new_offset}");
                        get_page_action.dispatch(new_offset);
                    }}>"»"</button>
                </div>
            </div>
            <div class = "flex-1" />
            <div class = "flex-none">
                <div class="dropdown" >
                    <label>
                        <div class = "form-control">
                            <input tabindex = "0" type="text" placeholder="Search" class="input input-sm input-bordered w-24 md:w-auto" on:input = move |event| {
                                let query = event_target_value(&event);
                                set_searching.set(true);
                                search_action.dispatch(query);
                            } on:keydown = move |ev| {
                                if ev.key().as_str() == "Escape" {
                                    set_searching.set(false);
                                }
                            }/>
                        </div>
                    </label>
                    <div class= {show} >
                        <ul tabindex="0" class="dropdown-content z-30 menu p-2 shadow bg-base-100 rounded-box w-52" >
                        {search_view}
                        </ul>
                    </div>
                </div>
            </div>
        </div>

    }
}
use std::rc::Rc;

use futures::StreamExt;
use gloo_worker::Spawnable;
use leptos_use::storage::use_local_storage;

#[component]
fn App() -> impl IntoView {
    let (ai_pref, set_ai_pref, _) = use_local_storage("use_ai", false);
    let (api_key, set_api_key, _) = use_local_storage::<Option<String>, _>("api_key", None);
    let (edit, edit_set) = create_signal(EditModal { state: None });
    let window = web_sys::window().expect("Must be in a windowed i.e. browser setting (You'r not trying to run this in a wasm runtime are you?)");
    let location = window.location();
    let origin = location
        .origin()
        .expect("Must have an origin for this to work");
    let (minilm, set_minilm) =
        create_signal::<Option<_>>(ai_pref.get_untracked().then(|| spawn_minilm(&origin)));
    provide_context(minilm);
    let (origin, _) = create_signal(origin);

    let get_page_action = create_action(move |offset| {
        let offset = *offset;
        async move {
            log!("Getting a page");
            (
                offset,
                get_recipes_at_offset(offset)
                    .await
                    .map_err(|err| Error::Msg(format!("{err}"))),
            )
        }
    });

    let offset = 0;
    get_page_action.dispatch(offset);
    view! {

    <div class = "bg-base-100 w-full h-screen overflow-auto md:overflow-x-hidden">
        {move || {
            let offset = match get_page_action.value().get_untracked() {
                Some((new_offset, _)) => new_offset,
                _ => offset,
            };
            edit.get().state.map(|(id, editing,  state)| {
                view! {
                    <div class = "fixed z-[999] h-screen w-full grid grid-cols-1 place-items-center">
                        <div class="modal-box max-w-lg center m-0  p-2 w-full sm:w-7/8 sm:m-4 sm:p-4">
                            <RecipeView offset  refresh_action = get_page_action id recipe_state=state editing = edit_set edit_flag = editing api_key/>
                        </div>
                        <div class="h-screen modal-backdrop">
                            <button on:click = move |_| edit_set.update(|s| {
                                s.state = None;
                                get_page_action.dispatch(offset);
                            }) >close</button>
                        </div>
                    </div>
                }
            })
        }}

        <TopBar/>
        <div class = "ml-5 mr-5 mt-3">
            {move || {
                let page = get_page_action.value().get();
                page.map(|(offset, page)|{ view! {
                    <NavBar offset origin  get_page_action set_edit = edit_set  minilm set_minilm set_ai_pref set_api_key api_key/>
                    <ErrorRecipes offset = offset refresh_action = get_page_action page edit_modal = edit_set api_key/>
                }})
            }}
        </div>
    </div>
        }
}
#[component]
fn TopBar() -> impl IntoView {
    view! {
        <div class="h-24 flex class border border-b-1 border-base-content shadow-md shadow-base-300 z-50">
            <div class="h-24 flex-1 pt-5 pl-5">
                <span class="prose"><h1>RecipeS</h1></span>
            </div>
            <div class="h-24 flex-none pt-5 pr-5">
                <img class="h-4/5" src="/static/ferris-chef.svg" />
            </div>
        </div>
    }
}

#[component]
fn ErrorRecipes<B: Clone + 'static>(
    offset: i64,
    refresh_action: Action<i64, B>,
    page: Result<RecipesResponse, Error>,
    edit_modal: WriteSignal<EditModal>,
    api_key: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <ErrorBoundary
            fallback = move | errs| {
               for err in errs.get() {
                   warn!("Error in recipes: {err:?}");
               }
               view! { "Error loading recipes page. Try reloading"}
            }
        >
        {move || page.clone().map(|resp| view! { <Recipes offset refresh_action recipes = {resp.recipes} edit_modal api_key/>})}

        </ErrorBoundary>
    }
}

use futures_timer::Delay;
use r_ecipe_s_model::RecipeWithId;
use r_ecipe_s_model::RecipesResponse;

#[component]
pub fn RecipeView<B: Clone + 'static>(
    id: i64,
    offset: i64,
    refresh_action: Action<i64, B>,
    recipe_state: Either<Recipe, (RecipeReadState, RecipeWriteState)>,
    editing: WriteSignal<EditModal>,
    edit_flag: bool,
    api_key: Signal<Option<String>>,
) -> impl IntoView {
    let (read_toggle, set_toggle) = create_signal(edit_flag);
    let minilm = use_context::<ReadSignal<Option<Rc<AsyncMutex<ReactorBridge<EncodeOnDemand>>>>>>();
    let (read_state, write_state) = match recipe_state {
        Either::Left(recipe) => {
            let (read_state, write_state) = RecipeState::state();
            write_state.set(recipe);
            (read_state, write_state)
        }
        Either::Right(state) => state,
    };

    let save_action = create_action(move |(id, recipe): &(i64, Recipe)| {
        let id = *id;
        let recipe = recipe.clone();
        let api_key = api_key.get_untracked();
        async move {
            let api_key = api_key.as_ref().map(AsRef::as_ref);
            let text = format!("{}\n{}", recipe.name, recipe.description);
            let embedding = get_embedding(minilm, text).await;
            let recipe = Recipe {
                embedding,
                ..recipe.clone()
            };
            // todo: remove delay
            Delay::new(Duration::from_secs(1)).await;

            update_recipe(id, &recipe, api_key).await
        }
    });

    let save_pending = save_action.pending();
    let button_message = move || {
        if save_pending.get() {
            view! {
                <button class = "btn btn-primary btn-xs">
                    <div class = "loading loading-infinity loading-secondary" />
                </button>
            }
        } else if read_toggle.get() {
            view! {
                <button class = "btn btn-primary btn-xs" on:click = move |_| {
                    let recipe = read_state.get_data_untracked();
                    save_action.dispatch((id, recipe));
                    set_toggle.set(false)
                }>
                    <div>"submit"</div>
                </button>
            }
        } else {
            view! {
                <button class = "btn btn-primary btn-xs" on:click = move |_| {
                    set_toggle.set(true)
                }>
                    <div>"edit"</div>
                </button>
            }
        }
    };

    let close_action = move || {
        log!("Dispatching");
        refresh_action.dispatch(offset);
        editing.update(|u| u.state = None);
    };
    let form = move |read_state, write_state| {
        view! {
            <RecipeForm read_state write_state />
        }
    };
    let wait = move || {
        view! {
            <Pending />
        }
    };
    let view_recipe = move |read_state| {
        view! {
            <Recipe read_state focus = {true} on:dblclick = move |_| close_action()/>
        }
    };
    view! {
        <div class = "grid grid-cols-1 gap-4 w-full  mx-auto">
            <div>
                {
                    move || {
                        let r_state = read_state;
                        let w_state = write_state;
                        view! {
                            {move|| read_toggle.get().then(move || form(r_state, w_state))}
                            {move|| (!read_toggle.get()).then(move || view! {
                                {move|| save_pending.get().then(wait)}
                                {move ||(!save_pending.get()).then(|| view_recipe(r_state))}
                            })}
                        }
                    }
               }
            </div>
            {button_message}
        </div>
    }
}

#[component]
fn Pending() -> impl IntoView {
    view! {
        <div class = "loading loading-infinity loading-secondary" />
    }
}

#[derive(Debug, Clone)]
struct EditModal {
    state: Option<(
        i64,
        bool,
        Either<Recipe, (RecipeReadState, RecipeWriteState)>,
    )>,
}

async fn put_recipe_action(api_key: Option<&str>) -> Result<RecipeWithId, Error> {
    let empty_recipe = Recipe {
        name: "".into(),
        ingredients: vec![],
        description: "".into(),
        liked: None,
        embedding: None,
    };
    // todo: remove delay
    Delay::new(Duration::from_secs(1)).await;
    put_recipe(&empty_recipe, api_key)
        .await
        .map_err(|err| Error::Msg(format!("{err}")))
        .map(|id| RecipeWithId {
            id,
            data: empty_recipe,
        })
}

#[component]
fn Recipes<B: Clone + 'static>(
    recipes: Vec<RecipeWithId>,
    offset: i64,
    refresh_action: Action<i64, B>,
    edit_modal: WriteSignal<EditModal>,
    api_key: Signal<Option<String>>,
) -> impl IntoView {
    let (recipes, set_recipes) = create_signal(
        recipes
            .into_iter()
            .map(move |RecipeWithId { id, data: recipe }| {
                let (read_state, write_state) = RecipeState::state();
                write_state.set(recipe);
                (id, read_state, write_state)
            })
            .collect::<Vec<_>>(),
    );

    view! {
        <div class = "grid md:grid-cols-2 gap-4 lg:grid-cols-3 sm:grid-cols-1">
            <For
                each = move || {recipes.get()}
                key = move |(idx, _, _)| *idx
                children = move | (id, read_state, write_state)| {
                    let click_action = move || {
                        log!("Time to expand");
                        edit_modal.update(|modal| modal.state = Some((id, false, Either::Right((read_state, write_state)))));
                    };
                    view! {
                        <div class = "relative">
                            <Recipe read_state on:dblclick = move |_| click_action() focus = {false}/>
                            <Delete id offset refresh_action set_recipes api_key/>
                        </div>
                    }
                }
            />
            <CreateButton edit_modal api_key/>
        </div>
    }
}

#[component]
fn CreateButton(
    edit_modal: WriteSignal<EditModal>,
    api_key: Signal<Option<String>>,
) -> impl IntoView {
    let create = create_action(move |_: &()| async move {
        let api_key = api_key.get_untracked();
        let api_key = api_key.as_ref();
        put_recipe_action(api_key.map(AsRef::as_ref)).await
    });

    let create_button_view = move || {
        if create.pending().get() {
            view! {
                <button class = "btn btn-sm btn-primary">
                    <Pending />
                </button>
            }
        } else {
            match create.value().get() {
                Some(Ok(RecipeWithId { id, .. })) => {
                    let (read_state, write_state) = RecipeState::state();
                    edit_modal.update(|edit| {
                        edit.state = Some((id, true, Either::Right((read_state, write_state))))
                    });
                    view! {
                        <button class = "btn btn-sm btn-primary">
                            <Create />
                        </button>
                    }
                }
                Some(Err(_)) => view! {
                    <button class = "btn btn-sm btn-primary">
                        <Error />
                    </button>
                },
                None => view! {
                    <button class = "btn btn-sm btn-primary" on:click=move |_| create.dispatch(()) >
                        <Create />
                    </button>
                },
            }
        }
    };

    view! {
            {create_button_view}
    }
}

#[component]
fn Create() -> impl IntoView {
    view! {
        <div >
            Create
        </div>

    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeleteStates {
    AwaitingInput,
    Confirming,
    Pending,
    Deleted,
}

use std::future::Future;
#[component]
fn Delete<B: Clone + 'static>(
    id: i64,
    offset: i64,
    refresh_action: Action<i64, B>,
    set_recipes: WriteSignal<Vec<(i64, RecipeReadState, RecipeWriteState)>>,
    api_key: Signal<Option<String>>,
) -> impl IntoView {
    use DeleteStates as Ds;
    let (confirming, set_confirming) = create_signal(Ds::AwaitingInput);

    let delete_action = create_action(move |id| {
        let id = *id;
        async move {
            let api_key = api_key.get_untracked();
            let api_key = api_key.as_ref().map(AsRef::as_ref);
            Delay::new(Duration::from_millis(1234)).await;
            delete_recipe(id, api_key)
                .await
                .map_err(|err| Error::Msg(format!("{err}")))
        }
    });
    let delete_view = move || {
        if delete_action.pending().get() {
            view! {
                <Pending />
            }
        } else if confirming.get() == Ds::Confirming {
            view! {
                <ConfirmDelete on:click= move |_| {
                    set_confirming.set(Ds::Pending);
                    delete_action.dispatch(id);
                }/>
            }
        } else {
            match delete_action.value().get() {
                Some(Err(err)) => {
                    warn!("failed to delete {err}");
                    view! { <Error /> }
                }
                Some(Ok(_)) => {
                    set_confirming.set(Ds::Deleted);
                    refresh_action.dispatch(offset);
                    set_recipes
                        .update(|recipes| recipes.retain(|(recipe_id, _, _)| *recipe_id != id));
                    view! {
                        <DeleteButton />
                    }
                }
                None => view! {
                    <DeleteButton on:click = move |_| set_confirming.set(Ds::Confirming)/>
                },
            }
        }
    };

    view! {
        {move || (confirming.get() == Ds::Confirming).then(move || view!{ <div class = "absolute top-0 right-0 h-full w-full z-10" on:click = move |_| set_confirming.set(Ds::AwaitingInput)/>})}
        <div class = "absolute top-2 right-2 z-20">
            <button class = "btn btn-xs btn-primary">
                {delete_view}
            </button>
        </div>
    }
}

#[component]
fn Error() -> impl IntoView {
    view! { <div> "Error"</div>}
}

#[component]
fn ConfirmDelete() -> impl IntoView {
    view! {
        <div >
            "Confirm Delete?"
        </div>
    }
}

#[component]
fn DeleteButton() -> impl IntoView {
    view! {
        <div class="w-4">
            <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M 18 18 L 6 6 M 18 6 L 6 18" /></svg>
        </div>
    }
}
