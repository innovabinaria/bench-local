CREATE TABLE IF NOT EXISTS items (
  id   INT PRIMARY KEY,
  name TEXT NOT NULL
);

INSERT INTO items (id, name)
VALUES (1, 'Hello from Postgres')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;

