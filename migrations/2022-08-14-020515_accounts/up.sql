CREATE TABLE accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id TEXT NOT NULL,
    access_token TEXT NOT NULL,
    account_uuid TEXT NOT NULL,
    username TEXT NOT NULL
)