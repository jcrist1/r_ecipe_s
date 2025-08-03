-- Add migration script here
DROP TRIGGER notify_recipe_updated ON recipes;
ALTER TABLE recipes DROP COLUMN searchable;
