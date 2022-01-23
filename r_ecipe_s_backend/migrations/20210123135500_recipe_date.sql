ALTER TABLE recipes
    ADD COLUMN created timestamptz,
    ADD COLUMN updated timestamptz;

UPDATE recipes SET 
created = NOW()
WHERE created IS NULL;


UPDATE recipes SET 
updated = NOW()
WHERE updated IS NULL;


ALTER TABLE recipes
    ALTER COLUMN created SET NOT NULL,
    ALTER COLUMN updated SET NOT NULL;


