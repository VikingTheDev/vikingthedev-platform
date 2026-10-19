CREATE TABLE users (
    id uuid PRIMARY KEY,
    -- The API validates more (no control or invisible characters); this guards the length.
    display_name text NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 64),
    created_at timestamptz NOT NULL
);
