use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub name: String,
    pub ingredients: Vec<Ingredient>,
    pub description: String,
    pub liked: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Ingredient {
    pub name: String,
    pub quantity: Quantity,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Eq)]
pub enum Quantity {
    Count(usize),
    Tsp(usize),
    Milligram(usize),
}
