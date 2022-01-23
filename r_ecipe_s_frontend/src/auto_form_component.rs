use r_ecipe_s_model::{Ingredient, Quantity};
use sycamore::{prelude::*, rt::JsCast};
use web_sys::{Event, HtmlInputElement, HtmlSelectElement};

pub(crate) trait DataToSignal {
    type SignalType;
    fn signal(&self) -> Self::SignalType;
    fn from_signal(signal_type: &Self::SignalType) -> Self;
}

pub(crate) trait DataToFormComponent {
    type DataType: DataToSignal<SignalType = Self>;
    fn form<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G>;
}

pub(crate) trait DataToComponent {
    type DataType: DataToSignal<SignalType = Self>;
    fn component<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G>;
}

impl DataToSignal for Quantity {
    type SignalType = Signal<Self>;
    fn signal(&self) -> Self::SignalType {
        Signal::new(*self)
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        *signal_type.get()
    }
}

const COUNT: &str = "count";
const GRAM: &str = "gram";
const TSP: &str = "tsp";

impl DataToComponent for Signal<Quantity>
where
    Signal<Quantity>: 'static,
{
    type DataType = Quantity;

    fn component<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let self_clone = self.clone();
        view! {
            (match self_clone.get().as_ref() {
                Quantity::Count(count) => format!("{count}"),
                Quantity::Tsp(count) => format!("{count} tsp."),
                Quantity::Gram(count) => format!("{count} g"),
            })
        }
    }
}

impl DataToFormComponent for Signal<Quantity>
where
    Signal<Quantity>: 'static,
{
    type DataType = Quantity;

    fn form<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let self_clone = self.clone();
        let selected_handler = cloned!((self_clone) => move |event: Event| {
            let target = event
                .target()
                .expect("Failed to get event target for change event");
            let input_element = target.dyn_into::<HtmlSelectElement>()
                .expect("Failed to convert to input element");
            let value = input_element.value();
            let new = match value {
                some_str if some_str == COUNT => Quantity::Count(0),
                some_str if some_str == GRAM => Quantity::Gram(0),
                some_str if some_str == TSP => Quantity::Tsp(0),
                _ => panic!("invalid input value"),
            };
            self_clone.set(new);

        });

        let quantity_handler = cloned!((self_clone) => move |event: Event| {
            let input_value: usize = event
                .target()
                .expect("Failed to get even target for quantity change event")
                .dyn_into::<HtmlInputElement>()
                .expect("Failed to convert quantity change event target to input element")
                .value()
                .parse()
                .expect("Failed to parse int from input");

            let new_quantity = match *self_clone.get() {
                Quantity::Count(_) => Quantity::Count(input_value),
                Quantity::Gram(_) => Quantity::Gram(input_value),
                Quantity::Tsp(_) => Quantity::Tsp(input_value),
            };
            self_clone.set(new_quantity);
        });
        let handle_count = self_clone.handle();
        let handle_gram = handle_count.clone();
        let handle_tsp = handle_count.clone();

        view! {
            select(on:change = selected_handler) {
                option(
                    value = COUNT,
                    selected=matches!(handle_count.get().as_ref(), Quantity::Count(_))
                ) {(COUNT)}
                option(
                    value = GRAM,
                    selected= matches!(handle_gram.get().as_ref(), Quantity::Gram(_))
                ) {(GRAM)}
                option(
                    value = TSP,
                    selected= matches!(handle_tsp.get().as_ref(), Quantity::Tsp(_))
                ) {(TSP)}
            }
            input(type="text", name="spec", on:change = quantity_handler, value=quantity(self_clone.get().as_ref())) {}
        }
    }
}

fn quantity(qnty: &Quantity) -> usize {
    match qnty {
        Quantity::Count(n) => *n,
        Quantity::Tsp(n) => *n,
        Quantity::Gram(n) => *n,
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct IngredientSignal {
    name: Signal<String>,
    quantity: Signal<Quantity>,
}

impl DataToSignal for Ingredient {
    type SignalType = Signal<IngredientSignal>;
    fn signal(&self) -> Self::SignalType {
        let Ingredient { name, quantity } = self;
        Signal::new(IngredientSignal {
            name: Signal::new(name.to_string()),
            quantity: quantity.signal(),
        })
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        let name = signal_type.get().name.get().to_string();
        let quantity = DataToSignal::from_signal(&signal_type.get().quantity);
        Ingredient { name, quantity }
    }
}

impl DataToComponent for Signal<IngredientSignal> {
    type DataType = Ingredient;

    fn component<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let self_clone = self.clone();
        let self_clone_2 = self.clone();

        view! {
            span {(self_clone_2.get().quantity.component())}
            span {" "}
            span {(self_clone.get().name.get().to_string())}
        }
    }
}

impl DataToFormComponent for Signal<IngredientSignal> {
    type DataType = Ingredient;

    fn form<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let self_clone = self.clone();
        let self_clone_2 = self.clone();
        let name_handler = cloned!((self_clone) => move |event: Event| {
            let name_signal = self_clone.get().name.clone();
            let input_value: String = event
                .target()
                .expect("Failed to get even target for ingredient name change event")
                .dyn_into::<HtmlInputElement>()
                .expect("Failed to convert ingredient name change event target to input element")
                .value();

            name_signal.set(input_value);
            let quantity_signal = self_clone.get().quantity.clone();
            self_clone.set(IngredientSignal{
                name: name_signal,
                quantity: quantity_signal
            });
        });
        view! {
            (self_clone_2.get().quantity.form())
            input(
                type="text",
                name = "name",
                on:change = name_handler,
                value = (self_clone.get().name.get().to_string())
            )
        }
    }
}

impl<T, SignalT> DataToSignal for Vec<T>
where
    T: DataToSignal<SignalType = SignalT>,
    SignalT: 'static,
{
    type SignalType = Signal<Vec<(usize, SignalT)>>;
    fn signal(&self) -> Self::SignalType {
        Signal::new(
            self.iter()
                .enumerate()
                .map(|(i, t)| (i, t.signal()))
                .collect(),
        )
    }

    fn from_signal(signal_type: &Self::SignalType) -> Self {
        signal_type
            .get()
            .iter()
            .map(|(i, signal_t)| DataToSignal::from_signal(signal_t))
            .collect()
    }
}

impl<T, SignalT> DataToFormComponent for Signal<Vec<(usize, SignalT)>>
where
    SignalT: DataToFormComponent<DataType = T> + PartialEq + Clone + 'static,
    T: DataToSignal<SignalType = SignalT> + PartialEq + Default + Clone,
{
    type DataType = Vec<T>;

    fn form<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let self_clone = self.clone();
        let add_handler = cloned!((self_clone) => move |_|  {
            let size = self_clone.get().len();
            let new_t = <T as Default>::default();
            let new = new_t.signal();
            let new_vec = self_clone
                .get()
                .iter()
                .cloned()
                .chain(std::iter::once((size, new)))
                .collect::<Vec<_>>();
            self_clone.set(new_vec);
        });

        let self_for_mutation = self.clone();
        view! {

            Indexed(IndexedProps {
                iterable: self_clone.handle(),
                template: move |(idx, signal_t)| {
                    let remove_handler = cloned!((self_for_mutation) => move |_| {
                            let new = self_for_mutation
                                .get()
                                .iter()
                                .filter(|(i, _)| *i != idx)
                                .map(|(_, signal_t)| signal_t)
                                .cloned()
                                .enumerate()
                                .collect();
                            self_for_mutation.set(new);
                        });
                    view! {
                        div {(signal_t.form())}
                        div(on:click = remove_handler){ "-" }
                    }
                },
            })
            div {
                div(class = "plus-button", on:click=add_handler) {}
            }
        }
    }
}

impl<T, SignalT> DataToComponent for Signal<Vec<(usize, SignalT)>>
where
    SignalT: DataToComponent<DataType = T> + PartialEq + Clone + 'static,
    T: DataToSignal<SignalType = SignalT> + PartialEq + Default + Clone,
{
    type DataType = Vec<T>;

    fn component<G: sycamore::generic_node::GenericNode + perseus::Html>(&self) -> View<G> {
        let self_clone = self.clone();
        view! {
            Indexed(IndexedProps {
                iterable: self_clone.handle(),
                template: move |(_, signal_t)| {
                    view! {div {(signal_t.component()) } }
                },
            })
        }
    }
}
