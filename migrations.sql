CREATE TABLE files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    hash VARCHAR(48) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    remotes TEXT NOT NULL,
    next INTEGER,
    previous INTEGER,
    path VARCHAR(150) NOT NULL,
    size INTEGER,
    UNIQUE (hash),
    UNIQUE (id),
    FOREIGN KEY (next) REFERENCES files_changes(id),
    FOREIGN KEY (previous) REFERENCES files_changes(id)
);

CREATE TRIGGER update_files_timestamp
AFTER UPDATE ON files
FOR EACH ROW
BEGIN
    UPDATE files SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
