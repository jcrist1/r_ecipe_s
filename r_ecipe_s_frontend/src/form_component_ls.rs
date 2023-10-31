use leptos::logging::log;
use leptos::*;
use r_ecipe_s_model::{Ingredient, Quantity, Recipe, COUNT, GRAM, MATCHERS, ML, TSP};
use web_sys::Event;

use std::num::ParseIntError;
use std::str::FromStr;

use crate::util::markdown_to_html;

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum QuantityError {
    #[error("Error reading number for quantity: {0}")]
    ParseNumber(#[from] ParseIntError),
    #[error("Invalid quantiyt type provided: {0}")]
    InvalidType(String),
}

type QtyRes<T> = std::result::Result<T, QuantityError>;
type QuantityRes = QtyRes<Quantity>;

#[component]
pub fn Quantity<S: SignalWith<Value = Quantity> + 'static>(quantity: S) -> impl IntoView {
    let formatted_quantity = move || {
        quantity.with(|quantity| match quantity {
            Quantity::Count(count) => format!("{count}"),
            Quantity::Tsp(count) => format!("{count} tsp."),
            Quantity::Gram(count) => format!("{count} g"),
            Quantity::Ml(count) => format!("{count} ml"),
        })
    };

    view! {
        {move || formatted_quantity()}
    }
}

//
pub fn quantity_from_symbol(symbol: &str, current_num: QtyRes<usize>) -> QuantityRes {
    let current_num = current_num?;
    match symbol {
        COUNT => Ok(Quantity::Count(current_num)),
        GRAM => Ok(Quantity::Gram(current_num)),
        TSP => Ok(Quantity::Tsp(current_num)),
        ML => Ok(Quantity::Ml(current_num)),
        invalid => Err(QuantityError::InvalidType(invalid.to_string())),
    }
}

pub fn quantity_handler(ev: &Event) -> QtyRes<usize> {
    let value = event_target_value(ev);
    let number = usize::from_str(&value)?;
    Ok(number)
}

#[component]
pub fn QuantityForm<F: Fn(Quantity) + 'static>(
    #[prop()] initial_quantity: Quantity,
    #[prop()] set_quantity_val: F,
    set_quantity: WriteSignal<Quantity>,
) -> impl IntoView {
    let (quant_type, set_type) = create_signal(initial_quantity.label().to_string());
    let (value, set_value) = create_signal(Ok(initial_quantity.value()));
    let select_handler = move |ev: Event| {
        set_type.set(event_target_value(&ev));
    };
    let quantity_handler = move |ev| {
        set_value.set(quantity_handler(&ev));
    };

    let initial_value = initial_quantity.value();
    let quantity = move || {
        let quantity = quantity_from_symbol(quant_type.get().as_str(), value.get());
        match quantity {
            Ok(quantity) => {
                set_quantity.set(quantity);
                log!("Blop");
                set_quantity_val(quantity);
                log!("Blip");
                Ok(())
            }
            Err(err) => Err(err),
        }
    };
    let quantity = quantity.into_signal();
    // let validator = ||

    let matchers = MATCHERS.into_iter().collect::<Vec<_>>();

    view! {
        <ErrorBoundary fallback = | errs| {
            let err_str = errs.get().into_iter().map(|(_, err)| view! {
                    {format!("{err}")}
                }).collect::<String>();

            view! {
                <div class = "input input-xs input-error bg-base-300 tooltip join-item tooltip-error tooltip-right border-error w-1/6" data-tip = {err_str}>
                    Error
                </div>
            }

        }>
            {quantity}
        </ErrorBoundary>
        <input
            type = "text"
            class = "input input-xs input-bordered input-primary py-0 px-1 bg-base-300 w-1/5 max-w-xs join-item"
            name = "spec"
            on:input = quantity_handler
            value = initial_value
        />
        <select
            class = "select select-bordered select-primary py-0 px-1 select-xs bg-base-300 w-2/5 max-w-xs join-item"
            on:input = select_handler
        >
        {
            matchers.into_iter().map(move |(label, matcher)|
                if matcher(&initial_quantity) {
                    view! {<option value = label selected>{label}</option> }
                } else {
                    view! {<option value = label>{label}</option> }
                }

            )
            .collect::<Vec<_>>()
        }
        </select>
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Editing(bool);

#[component]
fn Ingredient(ingredient: ReadSignal<Ingredient>) -> impl IntoView {
    let name = create_memo(move |_| ingredient.get().name);
    let quantity = create_memo(move |_| ingredient.get().quantity);

    view! {
        <li>
            <Quantity quantity = quantity/>" "{ name }
        </li>
    }
}
#[component]
fn IngredientForm(
    #[prop()] ingredient: Ingredient,
    set_ingredient: WriteSignal<Ingredient>,
) -> impl IntoView {
    let Ingredient { name, quantity } = ingredient;
    let (_, set_quantity) = create_signal(quantity);
    let text_input = move |ev: Event| {
        let name = event_target_value(&ev);
        set_ingredient.update(|ingr| ingr.name = name);
    };
    let quantity_adjust = move |quantity| {
        set_ingredient.update(|ingr| {
            ingr.quantity = quantity;
        });
    };

    view! {
        <QuantityForm initial_quantity = quantity set_quantity_val = quantity_adjust set_quantity />
        <input class = "bg-base-300 input input-bordered input-primary input-xs w-fit py-0 px-1 join-item" on:input = text_input value = {name}/>
    }
}

pub type IndexedIngredientState = (i32, (ReadSignal<Ingredient>, WriteSignal<Ingredient>));
#[component]
pub fn Ingredients(ingredients: ReadSignal<Vec<IndexedIngredientState>>) -> impl IntoView {
    view! {

        <ul>
            <For
                each = move || ingredients.get()
                key = |(idx, (_, _))| *idx
                children = move | (_, (get_ingredient, _))| {
                    view! {
                        <Ingredient ingredient = get_ingredient />
                    }
                }
            />
        </ul>
    }
}

#[component]
pub fn IngredientsForm(
    ingredients_data: ReadSignal<Vec<IndexedIngredientState>>,
    ingredients: WriteSignal<Vec<IndexedIngredientState>>,
) -> impl IntoView {
    let mut idx = 0;
    view! {
        <div class = "grid grid-cols-1 gap-2">
            <For
                each = move || ingredients_data.get()
                key = |data| data.0
                children = move | (idx, (get_ingredient, set_ingredient))| {
                    view! {
                    <div class = "join w-full mx-auto" >
                        <IngredientForm ingredient = get_ingredient.get_untracked() set_ingredient />
                        <button
                            class = "btn btn-circle btn-primary btn-xs join-item"
                            on:click = move |_| {
                                log!("Time to close {}", idx);
                                ingredients.update(|ingredients| {
                                    ingredients.retain(|(idx_2, _)| *idx_2 != idx)
                                })
                            }
                        >
                           <svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M 18 12 L 6 12" /></svg>
                        </button>
                    </div>
                    }
                }
            />
            <div class = "grid grid-cols-9 place-content-center">
                <div class = "col-start-4 col-span-3">
                    <button
                        class = "btn btn-primary btn-xs btn-wide w-full"
                        on:click = move |_| {
                            ingredients.update(move |ingredients| {
                                let signals = create_signal( Ingredient {
                                    name: "".into(),
                                    quantity: Quantity::Count(0),
                                });
                                {
                                    ingredients.push((idx, signals))
                                }
                            });
                            idx += 1
                        }
                    >
                       <svg xmlns="http://www.w3.org/2000/svg" height = "100%" className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 18 L12 6 M 18 12 L 6 12" /></svg>
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RecipeWriteState {
    title: WriteSignal<String>,
    ingredients: WriteSignal<Vec<IndexedIngredientState>>,
    description: WriteSignal<String>,
}
impl RecipeWriteState {
    pub fn set(
        &self,

        Recipe {
            name,
            description,
            ingredients,
            ..
        }: Recipe,
    ) {
        self.title.set(name);
        self.description.set(description);
        let ingredients = ingredients
            .into_iter()
            .enumerate()
            .map(|(idx, ingredient)| (idx as i32, create_signal(ingredient)))
            .collect::<Vec<_>>();
        self.ingredients.set(ingredients);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RecipeReadState {
    pub title: ReadSignal<String>,
    pub ingredients: ReadSignal<Vec<IndexedIngredientState>>,
    description: ReadSignal<String>,
}

impl RecipeReadState {
    pub fn get_data_untracked(&self) -> Recipe {
        log!("Self: {self:#?}");
        let RecipeReadState {
            title,
            ingredients,
            description,
        } = *self;
        log!("title: {title:#?}");
        let title = title.get_untracked();
        log!("Got");
        let ingredients = ingredients.with_untracked(|ingredients| {
            ingredients
                .iter()
                .map(|(_, (read_ingredient, _))| read_ingredient.get())
                .collect::<Vec<_>>()
        });
        let description = description.get_untracked();
        Recipe {
            name: title,
            ingredients,
            description,
            liked: None,
            embedding: None,
        }
    }

    pub fn get_data(&self) -> Recipe {
        log!("Self: {self:#?}");
        let RecipeReadState {
            title,
            ingredients,
            description,
        } = *self;
        log!("title: {title:#?}");
        let title = title.get();
        log!("Got");
        let ingredients = ingredients.with(|ingredients| {
            ingredients
                .iter()
                .map(|(_, (read_ingredient, _))| read_ingredient.get())
                .collect::<Vec<_>>()
        });
        let description = description.get();
        Recipe {
            name: title,
            ingredients,
            description,
            liked: None,
            embedding: None,
        }
    }
}

pub struct RecipeState;

impl RecipeState {
    pub fn state() -> (RecipeReadState, RecipeWriteState) {
        let (get_ingredients, set_ingredients) =
            create_signal(Vec::<IndexedIngredientState>::new());

        let (get_title, set_title) = create_signal(String::new());
        let (get_description, set_description) = create_signal(String::new());
        let read_state = RecipeReadState {
            title: get_title,
            ingredients: get_ingredients,
            description: get_description,
        };

        let write_state = RecipeWriteState {
            title: set_title,
            ingredients: set_ingredients,
            description: set_description,
        };

        (read_state, write_state)
    }
}

#[component]
pub fn Recipe(read_state: RecipeReadState, focus: bool) -> impl IntoView {
    let RecipeReadState {
        title: get_title,
        ingredients: get_ingredients,
        description: get_description,
        ..
    } = read_state;
    let prose_class = if focus { "prose-md" } else { "prose-xs" };
    let max_height = if focus { "" } else { "max-h-60" };
    let class = format!(
        "card {prose_class} bg-base-100 border border-base-content shadow-md shadow-base-300"
    );
    let card_body_class = format!("card-body {max_height}");

    let text_overflow = if focus { "" } else { "overflow-hidden" };
    let overflow = format!("relative h-full {text_overflow}");

    view! {
        <div class = {class}>
            <div class = {card_body_class}>
                <div class = {overflow}>
                    <h2 class = "card-title">{move || get_title.get()}</h2>
                    <Ingredients ingredients = get_ingredients />
                    <div inner_html =
                        { move || markdown_to_html(&get_description.get()) } >
                    </div>
                    {(!focus).then(|| view!{
                        <div class="bg-base-100 w-full pointer-events-none bottom-0 flex absolute bottom-0 h-20 [mask-image:linear-gradient(transparent,#000000)]" />
                    })}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn RecipeForm(read_state: RecipeReadState, write_state: RecipeWriteState) -> impl IntoView {
    let RecipeWriteState {
        title: set_title,
        ingredients: set_ingredients,
        description: set_description,
    } = write_state;
    let set_title = move |ev: Event| {
        let title = event_target_value(&ev);
        set_title.set(title);
    };

    let set_description = move |ev: Event| {
        let description = event_target_value(&ev);
        set_description.set(description);
    };
    let RecipeReadState {
        title,
        description: description_data,
        ingredients,
        ..
    } = read_state;
    view! {
        <div class = "card w-full bg-base-100 border border-base-content shadow-md shadow-base-300">
            <div class = "card-body">
                <div class="form-control">
                    <label class="label">
                        <span class="label-text">Title</span>
                    </label>
                    <div>
                        <input class = "input input-bordered input-primary bg-base-300 w-full" on:input = set_title value={title.get_untracked()} />
                    </div>
                    <label class="label">
                        <span class="label-text">Ingredients</span>
                    </label>
                    <div>
                        <IngredientsForm ingredients_data = ingredients ingredients = set_ingredients />
                    </div>
                    <label class="label">
                        <span class="label-text">Description</span>
                    </label>
                    <div>
                        <textarea
                            class = "textarea text-area-primary bg-base-300 textarea-bordered w-full h-500"
                            on:input = set_description
                        >

                            {description_data}
                        </textarea>
                    </div>
                </div>
            </div>
        </div>
    }
}
