use meilisearch_sdk::document::Document;
use serde::{Deserialize, Serialize};
pub use serde_json;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Recipe {
    pub name: String,
    pub ingredients: Vec<Ingredient>,
    pub description: String,
    pub liked: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipeWithId {
    pub id: i64,
    pub data: Recipe,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipesResponse {
    pub recipes: Vec<RecipeWithId>,
    pub total_pages: i64,
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

impl Document for RecipeWithId {
    type UIDType = i64;

    fn get_uid(&self) -> &i64 {
        &self.id
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    // todo: limit + offset
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub recipe: RecipeWithId,
}
