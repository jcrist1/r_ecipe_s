ALTER TABLE recipes
    ADD COLUMN searchable boolean;

UPDATE recipes
    SET searchable = false;

ALTER TABLE recipes
    ALTER COLUMN searchable SET NOT NULL;
