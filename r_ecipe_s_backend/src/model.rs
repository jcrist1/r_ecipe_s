use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[cfg(feature = "backend")]
use itertools::Itertools;
#[cfg(feature = "backend")]
use minilm::{EmbeddedString, MiniLM};

#[cfg_attr(feature = "backend", derive(sqlx::Type))]
#[derive(
    Default, Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
#[cfg_attr(feature = "backend", sqlx(transparent))]
pub struct RecipeId(pub(crate) i64);

impl Display for RecipeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Recipe {
    pub name: String,
    #[cfg_attr(feature = "backend", sqlx(json))]
    pub ingredients: Vec<Ingredient>,
    pub description: String,
    pub liked: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct VectorSearch;
#[cfg(feature = "backend")]
impl Recipe {
    pub(crate) fn encode(&self, model: &MiniLM) -> Result<EmbeddedString, minilm::Error> {
        let text = format!(
            "{name}\n{ingredients}\n{description}",
            name = self.name,
            ingredients = self.ingredients.iter().map(|ing| &ing.name).join(" "),
            description = self.description
        );
        model.embed_string(text)
    }
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RecipeWithId {
    #[cfg_attr(feature = "backend", sqlx(flatten))]
    #[serde(flatten)]
    pub data: Recipe,
    pub id: RecipeId,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    // todo: limit + offset
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<RecipeWithId>,
}
