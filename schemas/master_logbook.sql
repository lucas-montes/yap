CREATE TABLE IF NOT EXISTS files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    path VARCHAR(150) NOT NULL,
    branch VARCHAR(150) NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    timestamp INTEGER NOT NULL,
    branch VARCHAR(150) NOT NULL,
    path VARCHAR(150) NOT NULL,
    event VARCHAR(150) NOT NULL,
    UNIQUE (id)
);

