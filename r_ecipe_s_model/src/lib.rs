use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Recipe {
    pub name: String,
    pub ingredients: Vec<Ingredient>,
    pub description: String,
    pub liked: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipeWithId {
    pub id: RecipeId,
    pub data: Recipe,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipesResponse {
    pub recipes: Vec<RecipeWithId>,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct RecipeId {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Ingredient {
    pub name: String,
    pub quantity: Quantity,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Eq)]
pub enum Quantity {
    Count(usize),
    Tsp(usize),
    Gram(usize),
}

impl Default for Quantity {
    fn default() -> Self {
        Quantity::Count(0)
    }
}
