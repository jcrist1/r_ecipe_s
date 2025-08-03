use crate::db::CommitRequired;
use crate::error::{Error, Result};
use crate::model::*;
use futures::TryStreamExt;
use minilm::{EmbeddedStr, EmbeddedString};
use sqlx::PgExecutor;
use time::OffsetDateTime;

use crate::{db::DbAccess, model::Recipe, model::RecipeId, model::RecipeWithId};
use sqlx::types::Json;

const EMPTY_RECIPE_LIST: &[RecipeWithId] = &[];

const MAX_PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct RecipeAccess {
    db_access: DbAccess,
}

impl RecipeAccess {
    pub fn new(db_access: DbAccess) -> Self {
        Self { db_access }
    }

    pub(crate) async fn get_batch_for_encode(
        &self,
        batch_size: usize,
    ) -> Result<CommitRequired<'_, Vec<RecipeWithId>>> {
        let mut transaction = self.db_access.get_pool().begin().await?;
        let data_iter: Vec<RecipeWithId> = sqlx::query!(
            r#"
                SELECT
                    name,
                    ingredients as "ingredients: Json<Vec<Ingredient>>", 
                    description,
                    liked,
                    embedding::real[] as "embedding: Vec<f32>",
                    id as "id: RecipeId"
                FROM recipes
                WHERE embedding IS NULL
                ORDER BY id 
                LIMIT $1
            "#,
            batch_size as i64
        )
        .fetch(transaction.as_mut())
        .map_ok(|record| RecipeWithId {
            id: record.id,
            data: Recipe {
                name: record.name,
                ingredients: record.ingredients.0,
                description: record.description,
                liked: record.liked,
            },
        })
        .try_collect::<Vec<_>>()
        .await?;
        Ok(CommitRequired::new(transaction, data_iter))
    }

    pub(crate) async fn get_all(&self, page: i64, page_size: i64) -> Result<RecipesResponse> {
        if (page_size <= 0) || (page_size > MAX_PAGE_SIZE as i64) {
            return Err(Error::IncorrectPageSize(page_size));
        }
        let offset = page * page_size;
        let data = sqlx::query!(
            r#"
                SELECT
                    id as "id: RecipeId",
                    name,
                    ingredients as "ingredients: Json<Vec<Ingredient>>",
                    description,
                    liked
                FROM recipes
                ORDER BY updated DESC
                OFFSET $1
                LIMIT $2
            "#,
            offset,
            page_size
        )
        .fetch(self.db_access.get_pool())
        .map_ok(|record| RecipeWithId {
            id: record.id,
            data: Recipe {
                name: record.name,
                ingredients: record.ingredients.0,
                description: record.description,
                liked: record.liked,
            },
        })
        .try_collect::<Vec<_>>()
        .await?;
        let count = sqlx::query!(
            r#"
                SELECT GREATEST (((COUNT(id) - 1)::int8 / $1),  0::int8)::int8 as count  FROM recipes;
            "#,
            page_size
        )
        .fetch_one(self.db_access.get_pool())
        .await?
        .count
        .expect("SQL COUNT returned NULL, when counting recipes, which shouldn't be possible");

        Ok(RecipesResponse {
            recipes: data,
            total_pages: count,
        })
    }

    pub(crate) async fn update(
        &self,
        id: RecipeId,
        recipe: &Recipe,
        embedding: &EmbeddedString,
    ) -> Result<Option<RecipeId>> {
        sqlx::query!(
            r#"
                UPDATE recipes SET
                    name = $1,
                    ingredients = $2,
                    description = $3,
                    liked = $4,
                    embedding = $5::real[]::vector(384),
                    updated = $6
                where id = $7 RETURNING id as "id: RecipeId"
            "#,
            recipe.name,
            sqlx::types::Json(recipe.ingredients.clone()) as _,
            recipe.description,
            recipe.liked,
            embedding.embedding.as_slice(),
            OffsetDateTime::now_utc(),
            id.0,
        )
        .fetch_optional(self.db_access.get_pool())
        .await
        .map(|opt| opt.map(|record| record.id))
        .map_err(|err| err.into())
    }

    pub(crate) async fn delete(&self, id: RecipeId) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM recipes WHERE id = $1
                RETURNING id as "id: RecipeId";
            "#,
            id.0,
        )
        .fetch_optional(self.db_access.get_pool())
        .await?
        .and_then(|opt| (opt.id == id).then_some(()))
        .ok_or(Error::NotFoundId(id))
    }
    //
    pub(crate) async fn insert(&self, recipe: &Recipe) -> Result<RecipeId> {
        let now = OffsetDateTime::now_utc();
        let rec = sqlx::query!(
            r#"
                INSERT INTO recipes (
                    name,
                    ingredients,
                    description,
                    liked,
                    created,
                    updated
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $5
                ) RETURNING id as "id: RecipeId"
            "#,
            recipe.name,
            sqlx::types::Json(recipe.ingredients.clone()) as _,
            recipe.description,
            recipe.liked,
            now
        )
        .fetch_one(self.db_access.get_pool())
        .await?
        .id;
        Ok(rec)
    }

    async fn get_by_id_pool<'a, P: PgExecutor<'a>>(
        pool: P,
        id: RecipeId,
    ) -> Result<Option<RecipeWithId>> {
        let ret: Option<RecipeWithId> = sqlx::query!(
            r#"
                SELECT
                    id as "id: RecipeId",
                    name,
                    ingredients as "ingredients: Json<Vec<Ingredient>>",
                    description,
                    liked
                FROM recipes
                WHERE id = $1
            "#,
            id.0,
        )
        .fetch_optional(pool)
        .await?
        .map(|record| RecipeWithId {
            data: Recipe {
                name: record.name,
                ingredients: record.ingredients.0,
                description: record.description,
                liked: record.liked,
            },
            id: record.id,
        });
        Ok(ret)
    }

    pub(crate) async fn get_by_id(&self, id: RecipeId) -> Result<Option<RecipeWithId>> {
        Self::get_by_id_pool(self.db_access.get_pool(), id).await
    }

    pub(crate) async fn get_by_id_for_update(
        &self,
        id: RecipeId,
    ) -> Result<CommitRequired<'_, Option<RecipeWithId>>> {
        let mut transaction = self.db_access.get_pool().begin().await?;

        let ret: Option<RecipeWithId> = sqlx::query!(
            r#"
                SELECT
                    id as "id: RecipeId",
                    name,
                    ingredients as "ingredients: Json<Vec<Ingredient>>",
                    description,
                    liked
                FROM recipes
                WHERE id = $1
                FOR UPDATE
            "#,
            id.0
        )
        .fetch_optional(transaction.as_mut())
        .await?
        .map(|record| RecipeWithId {
            data: Recipe {
                name: record.name,
                ingredients: record.ingredients.0,
                description: record.description,
                liked: record.liked,
            },
            id: record.id,
        });

        Ok(CommitRequired::new(transaction, ret))
    }

    pub(crate) async fn search(&self, query: &str) -> Result<Vec<RecipeWithId>> {
        let result = sqlx::query!(
            r#"
                SELECT
                    id as "id: RecipeId",
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>",
                    description,
                    liked
                    FROM recipes
                WHERE 
                    (name @@@ $1 
                    OR description @@@ $1 
                    OR ingredient_names @@@ $1)
                ORDER BY paradedb.score(id) DESC;
            "#,
            query
        )
        .fetch(self.db_access.get_pool())
        .map_ok(|record| RecipeWithId {
            data: Recipe {
                name: record.name,
                ingredients: record.ingredients.0,
                description: record.description,
                liked: record.liked,
            },
            id: record.id,
        })
        .try_collect::<Vec<_>>()
        .await?;
        Ok(result)
    }
    pub(crate) async fn hybrid_search(
        &self,
        EmbeddedStr {
            text, embedding, ..
        }: EmbeddedStr<'_>,
    ) -> Result<Vec<RecipeWithId>> {
        let result = sqlx::query!(
            r#"
                SELECT
                    id as "id: RecipeId",
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>",
                    description,
                    liked
                    FROM recipes
                WHERE 
                    (name @@@ $1 
                    OR description @@@ $1 
                    OR ingredient_names @@@ $1) AND (embedding <-> $2::real[]::vector(384) > 0.2)
                ORDER BY (embedding <-> $2::real[]::vector(384)) * paradedb.score(id) DESC;
            "#,
            &text,
            embedding.as_slice()
        )
        .fetch(self.db_access.get_pool())
        .map_ok(|record| RecipeWithId {
            data: Recipe {
                name: record.name,
                ingredients: record.ingredients.0,
                description: record.description,
                liked: record.liked,
            },
            id: record.id,
        })
        .try_collect::<Vec<_>>()
        .await?;
        Ok(result)
    }
}

impl RecipeAccess {
    // pub fn new(db_access: &Arc<DbAccess>) -> Self {
    //     RecipeAccess {
    //         db_access: Arc::clone(db_access),
    //     }
    // }
}

#[cfg(test)]
mod test {
    use itertools::Itertools;
    use sqlx::PgPool;

    use crate::db::DbAccess;
    use crate::model::{Ingredient, Quantity, Recipe};

    use super::RecipeAccess;

    #[sqlx::test]
    async fn test_query(pool: PgPool) {
        let data = Recipe {
            name: "Fried Chicken".into(),
            ingredients: vec![Ingredient {
                name: "Chicken".into(),
                quantity: Quantity::Count(1),
            }],
            description: "Fry the chicken".into(),
            liked: None,
        };

        let recipe_access = RecipeAccess {
            db_access: DbAccess::for_test(pool),
        };

        let id = recipe_access
            .insert(&data)
            .await
            .expect("failed to insert data");
        let uncommitted_data = recipe_access
            .get_batch_for_encode(20)
            .await
            .expect("Failed to get batch for encode");
        let for_embed = uncommitted_data
            .commit_and_get_data()
            .await
            .expect("failed to commit");
        let recipe = recipe_access
            .get_by_id(id)
            .await
            .expect("Failed to get single recipe");

        assert_eq!(
            vec![id],
            for_embed.into_iter().map(|data| data.id).collect_vec()
        );

        assert_eq!(Some(data), recipe.map(|data| data.data))
    }
}
