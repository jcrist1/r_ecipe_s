-- Add migration script here
ALTER TABLE recipes DROP COLUMN embedding;
CREATE OR REPLACE FUNCTION extract_ingredient_names(ingredients json)
RETURNS text AS $$
BEGIN
  RETURN array_to_string(
    ARRAY(
      SELECT json_array_elements(ingredients) ->> 'name'
    ), 
    ' '
  );
END;
$$ LANGUAGE plpgsql IMMUTABLE;

ALTER TABLE recipes 
  ADD COLUMN embedding vector(384),
ADD COLUMN ingredient_names TEXT GENERATED ALWAYS AS (
  extract_ingredient_names(ingredients)
) STORED;

  -- ADD COLUMN ingredient_text TEXT GENERATED ALWAYS AS concat(json_array_elements(ingredients)->'name'::text, ' ');
CREATE INDEX ON recipes USING vchordrq (embedding vector_l2_ops) WITH (options = $$
residual_quantization = true
[build.internal]
lists = []
$$);
SELECT ingredient_names FROM recipes;
CREATE INDEX search_idx ON recipes
USING bm25 (id, description, name, ingredient_names)
WITH (key_field='id');
