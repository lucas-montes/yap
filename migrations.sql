CREATE TABLE files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    hash VARCHAR(48) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    remotes TEXT NOT NULL,
    next INTEGER,
    previous INTEGER,
    path VARCHAR(150) NOT NULL,
    UNIQUE (hash),
    UNIQUE (id),
    FOREIGN KEY (next) REFERENCES files_changes(id),
    FOREIGN KEY (previous) REFERENCES files_changes(id)
);

CREATE TABLE files_changes (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    next INTEGER,
    current INTEGER,
    path VARCHAR(150) NOT NULL,
    UNIQUE (id),
    FOREIGN KEY (current) REFERENCES files(id),
    FOREIGN KEY (next) REFERENCES files(id)
);

CREATE TRIGGER update_timestamp
AFTER UPDATE ON (Files, FilesChanges)
FOR EACH ROW
BEGIN
    UPDATE Files SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

