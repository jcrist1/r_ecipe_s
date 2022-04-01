use crate::util::background;
use r_ecipe_s_model::{Ingredient, Quantity};
use r_ecipe_s_style::generated::*;
use sycamore::{prelude::*, rt::JsCast};
use tailwindcss_to_rust_macros::*;
use web_sys::{Event, HtmlInputElement, HtmlSelectElement};

pub trait DataToSignal {
    type SignalType;
    fn signal(&self) -> Self::SignalType;
    fn from_signal(signal_type: &Self::SignalType) -> Self;
}

impl DataToSignal for Quantity {
    type SignalType = RcSignal<Self>;
    fn signal(&self) -> Self::SignalType {
        create_rc_signal(*self)
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        *signal_type.get()
    }
}

const COUNT: &str = "count";
const GRAM: &str = "gram";
const TSP: &str = "tsp";

#[component]
fn QuantityComponent<G: Html>(scope_ref: ScopeRef, quantity: &RcSignal<Quantity>) -> View<G> {
    let quantity = scope_ref.create_ref(quantity.clone());
    view! { scope_ref,
        (match quantity.get().as_ref() {
            Quantity::Count(count) => format!("{count}"),
            Quantity::Tsp(count) => format!("{count} tsp."),
            Quantity::Gram(count) => format!("{count} g"),
        })
    }
}

#[component]
fn QuantityFormComponent<G: Html>(scope_ref: ScopeRef, quantity: &RcSignal<Quantity>) -> View<G> {
    let quantity = scope_ref.create_ref(quantity.clone());
    let selected_handler = move |event: Event| {
        let target = event
            .target()
            .expect("Failed to get event target for change event");
        let input_element = target
            .dyn_into::<HtmlSelectElement>()
            .expect("Failed to convert to input element");
        let value = input_element.value();
        let new = match value {
            some_str if some_str == COUNT => Quantity::Count(0),
            some_str if some_str == GRAM => Quantity::Gram(0),
            some_str if some_str == TSP => Quantity::Tsp(0),
            _ => panic!("invalid input value"),
        };
        quantity.set(new);
    };

    let quantity_handler = move |event: Event| {
        let input_value: usize = event
            .target()
            .expect("Failed to get even target for quantity change event")
            .dyn_into::<HtmlInputElement>()
            .expect("Failed to convert quantity change event target to input element")
            .value()
            .parse()
            .expect("Failed to parse int from input");

        let new_quantity = match *quantity.get() {
            Quantity::Count(_) => Quantity::Count(input_value),
            Quantity::Gram(_) => Quantity::Gram(input_value),
            Quantity::Tsp(_) => Quantity::Tsp(input_value),
        };
        quantity.set(new_quantity);
    };

    view! { scope_ref,
        select(
            class = DC![C.spc.p_1, C.siz.w_20],
            on:change = selected_handler
        ) {
            option(
                value = COUNT,
                selected=matches!(quantity.get().as_ref(), Quantity::Count(_))
            ) {(COUNT)}
            option(
                value = GRAM,
                selected= matches!(quantity.get().as_ref(), Quantity::Gram(_))
            ) {(GRAM)}
            option(
                value = TSP,
                selected= matches!(quantity.get().as_ref(), Quantity::Tsp(_))
            ) {(TSP)}
        }
        input(
            type="text",
            class = DC![C.spc.p_1, C.siz.w_14],
            name="spec",
            on:change = quantity_handler,
            value = get_quantity(quantity.get().as_ref())
        ) {}
    }
}

fn get_quantity(qnty: &Quantity) -> usize {
    match qnty {
        Quantity::Count(n) => *n,
        Quantity::Tsp(n) => *n,
        Quantity::Gram(n) => *n,
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IngredientSignal {
    name: RcSignal<String>,
    quantity: RcSignal<Quantity>,
}

impl Default for IngredientSignal {
    fn default() -> Self {
        Self {
            name: create_rc_signal(String::default()),
            quantity: create_rc_signal(Quantity::default()),
        }
    }
}

impl DataToSignal for Ingredient {
    type SignalType = IngredientSignal;
    fn signal<'a>(&self) -> Self::SignalType {
        let Ingredient { name, quantity } = self;
        IngredientSignal {
            name: create_rc_signal(name.to_string()),
            quantity: create_rc_signal(*quantity),
        }
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        let name = signal_type.name.get().to_string();
        let quantity = DataToSignal::from_signal(&signal_type.quantity);
        Ingredient { name, quantity }
    }
}

#[component]
pub fn IngredientComponent<G: Html>(
    scope_ref: ScopeRef,
    ingredient_with_id: &RcSignal<(usize, IngredientSignal)>,
) -> View<G> {
    let ingredient = &scope_ref
        .create_ref(ingredient_with_id.get().clone())
        .as_ref()
        .1;
    let ingredient = scope_ref.create_ref(ingredient.clone());
    view! { scope_ref,
        span {QuantityComponent(&ingredient.quantity)}
        span {" "}
        span {(ingredient.name.to_string())}
    }
}

#[component]
pub fn IngredientFormComponent<G: Html>(
    scope_ref: ScopeRef,
    ingredient: &RcSignal<(usize, IngredientSignal)>,
) -> View<G> {
    let ingredient_with_id = scope_ref.create_ref(ingredient.clone());
    let name_handler = move |event: Event| {
        let (id, ingredient) = &*ingredient_with_id.get();
        let name_signal = ingredient.name.clone();
        let input_value: String = event
            .target()
            .expect("Failed to get even target for ingredient name change event")
            .dyn_into::<HtmlInputElement>()
            .expect("Failed to convert ingredient name change event target to input element")
            .value();

        name_signal.set(input_value);
        let quantity_signal = ingredient.quantity.clone();
        ingredient_with_id.set((
            *id,
            IngredientSignal {
                name: name_signal,
                quantity: quantity_signal,
            },
        ));
    };
    let ingredient = &scope_ref.create_ref(ingredient_with_id.get()).as_ref().1;
    let quantity = scope_ref.create_ref(ingredient.quantity.clone());
    let name = scope_ref.create_ref(ingredient.name.clone());
    view! { scope_ref,
            QuantityFormComponent(quantity)
            input(
                type = "text",
                class = DC![C.spc.p_1, C.siz.w_48],
                name = "name",
                on:change = name_handler,
                value = (name.get().to_string())
            )
    }
}

impl<T, SignalT> DataToSignal for Vec<T>
where
    T: DataToSignal<SignalType = SignalT>,
{
    type SignalType = Vec<RcSignal<(usize, SignalT)>>;
    fn signal(&self) -> Self::SignalType {
        self.iter()
            .enumerate()
            .map(|(i, t)| (i, t.signal()))
            .map(create_rc_signal)
            .collect()
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        signal_type
            .iter()
            .map(|rc| DataToSignal::from_signal(&rc.get().1))
            .collect()
    }
}

#[component]
pub fn IngredientsFormComponent<G: Html>(
    scope_ref: ScopeRef,
    ingredients: &RcSignal<Vec<RcSignal<(usize, IngredientSignal)>>>,
) -> View<G> {
    let ingredients = scope_ref.create_ref(ingredients.clone());
    let add_handler = move |_| {
        let size = ingredients.get().len();
        let new_t = Ingredient::default();
        let new = create_rc_signal((size, new_t.signal()));
        let new_vec = ingredients
            .get()
            .iter()
            .cloned()
            .chain(std::iter::once(new))
            .collect::<Vec<_>>();
        ingredients.set(new_vec);
    };

    view! { scope_ref,

        Keyed(KeyedProps {
            iterable: ingredients,
            view: |ctx, data| {
                let idx = data.get().0;
                let ingredients_clone = ingredients.clone();
                let remove_handler =  move |_| {
                        let new = ingredients_clone.get()
                            .iter()
                            .filter(|rc| rc.get().0 != idx)
                            .map( |rc| rc.get().1.clone())
                            .enumerate()
                            .map(create_rc_signal)
                            .collect();
                        ingredients_clone.set(new);
                    };
                view! { ctx,
                    div(class = DC![C.lay.flex,C.fg.gap_1, C.spc.p_2, C.siz.h_14]) {
                        IngredientFormComponent(&data)
                        div(class = DC![C.spc.pt_2]) {
                            div(
                                class = DC![ C.siz.h_6, C.siz.w_6, C.bg.bg_no_repeat, C.bg.bg_contain],
                                on:click = remove_handler,
                                style = background("dash-circle.svg")
                            )
                        }
                    }
                }
            },
            key: |rc| rc.get().0,
        })
        div(class = DC![C.spc.p_2]) {
            div(
                class = DC![C.siz.h_6, C.siz.w_6, C.bg.bg_no_repeat, C.bg.bg_contain],
                on:click=add_handler,
                style = background("plus-circle.svg")
            ) {}
        }
    }
}

#[component]
pub fn IngredientsComponent<G: Html>(
    scope_ref: ScopeRef,
    ingredients: &RcSignal<Vec<RcSignal<(usize, IngredientSignal)>>>,
) -> View<G> {
    let ingredients = scope_ref.create_ref(ingredients.clone());
    view! { scope_ref,
        Keyed(KeyedProps {
            iterable: ingredients,
            view: |ctx, data| {
                view! {ctx,
                    div {IngredientComponent(&data)}
                }
            },
            key: |data| data.get().0

        })
    }
}
