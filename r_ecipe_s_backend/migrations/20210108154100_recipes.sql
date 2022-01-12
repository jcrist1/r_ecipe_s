CREATE TABLE IF NOT EXISTS recipes
(
    id          BIGSERIAL PRIMARY KEY,
    ingredients JSON    NOT NULL,
    description TEXT    NOT NULL,
    liked       BOOLEAN
);
