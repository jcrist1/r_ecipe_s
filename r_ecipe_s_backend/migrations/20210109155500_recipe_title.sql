ALTER TABLE recipes ADD COLUMN name TEXT NOT NULL;

INSERT INTO recipes (
    name,
    description, 
    ingredients
) VALUES (
    'omelette',
    'break the eggs into a bowl\n add salt and pepper to taset\n fry', 
    '[{"name": "eggs", "quantity": {"Count": 2}}]'
);
