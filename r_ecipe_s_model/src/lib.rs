use meilisearch_sdk::document::Document;
use serde::{Deserialize, Serialize};
pub use serde_json;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Recipe {
    pub name: String,
    pub ingredients: Vec<Ingredient>,
    pub description: String,
    pub liked: Option<bool>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RecipeWithId {
    pub id: i64,
    pub data: Recipe,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
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
    Ml(usize),
}
pub const COUNT: &str = "count";
pub const TSP: &str = "tsp";
pub const GRAM: &str = "gram";
pub const ML: &str = "ml";

fn matches_gram(quantity: &Quantity) -> bool {
    matches!(quantity, Quantity::Gram(_))
}

fn matches_count(quantity: &Quantity) -> bool {
    matches!(quantity, Quantity::Count(_))
}

fn matches_tsp(quantity: &Quantity) -> bool {
    matches!(quantity, Quantity::Tsp(_))
}

fn matches_ml(quantity: &Quantity) -> bool {
    matches!(quantity, Quantity::Ml(_))
}

pub const MATCHERS: [(&str, for<'a> fn(&'a Quantity) -> bool); 4] = [
    (COUNT, matches_count),
    (TSP, matches_tsp),
    (GRAM, matches_gram),
    (ML, matches_ml),
];

impl Quantity {
    pub fn label(&self) -> &'static str {
        match self {
            Quantity::Count(_) => COUNT,
            Quantity::Tsp(_) => TSP,
            Quantity::Gram(_) => GRAM,
            Quantity::Ml(_) => ML,
        }
    }

    pub fn value(&self) -> usize {
        match *self {
            Quantity::Count(count) => count,
            Quantity::Tsp(tsp) => tsp,
            Quantity::Gram(gram) => gram,
            Quantity::Ml(ml) => ml,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub recipe: RecipeWithId,
}
