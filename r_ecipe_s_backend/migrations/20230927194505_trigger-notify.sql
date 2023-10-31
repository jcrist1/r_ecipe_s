-- Add migration script here
CREATE OR REPLACE FUNCTION notify_recipe_updated()
  RETURNS trigger AS $$
DECLARE
BEGIN
  PERFORM pg_notify(
    CAST('search_index' AS text),
    NEW.id::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER notify_recipe_updated
  AFTER UPDATE ON recipes 
  FOR EACH ROW
  WHEN (NEW.searchable = false)
  EXECUTE PROCEDURE notify_recipe_updated();
