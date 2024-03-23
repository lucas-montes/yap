CREATE TABLE IF NOT EXISTS files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER DEFAULT CURRENT_TIMESTAMP,
    path VARCHAR(150) NOT NULL DEFAULT "",
    branch_id INTEGER NOT NULL,
    remote VARCHAR(150) NOT NULL DEFAULT "",
    branch_id INTEGER NOT NULL,
    UNIQUE (id),
    FOREIGN KEY (branch_id) REFERENCES branches(id),
);

CREATE TABLE IF NOT EXISTS authors (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    uuid VARCHAR(150) NOT NULL,
    name VARCHAR(150) NOT NULL,
    email VARCHAR(150) NOT NULL DEFAULT "",
    UNIQUE (id),
);

CREATE TABLE IF NOT EXISTS branches (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    name VARCHAR(150) NOT NULL,
    description TEXT NOT NULL DEFAULT "",
    status VARCHAR(150) NOT NULL DEFAULT "Active",
    author_id INTEGER NOT NULL,
    UNIQUE (id),
    FOREIGN KEY (author_id) REFERENCES authors(id),
    UNIQUE (name),
);

CREATE TABLE IF NOT EXISTS commits ( 
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    git_commit VARCHAR(150),
    message TEXT,
    file_from_id INTEGER NOT NULL,
    file_to_id INTEGER NOT NULL,
    diff_id INTEGER,
    branch_id INTEGER NOT NULL,
    author_id INTEGER NOT NULL,
    UNIQUE (id),
    FOREIGN KEY (author_id) REFERENCES authors(id),
    FOREIGN KEY (branch_id) REFERENCES branches(id),
    FOREIGN KEY (file_from_id) REFERENCES files(id),
    FOREIGN KEY (file_to_id) REFERENCES files(id),
    FOREIGN KEY (diff_id) REFERENCES diffs(id),
);

CREATE TABLE IF NOT EXISTS diffs ( 
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER DEFAULT CURRENT_TIMESTAMP,
    git_commit VARCHAR(150) ,
    result_path VARCHAR(150) NOT NULL DEFAULT "",
    script VARCHAR(150) NOT NULL DEFAULT "",
    result BLOB,
    techniques: TEXT NOT NULL,
    file_from_id INTEGER NOT NULL,
    file_to_id INTEGER NOT NULL,
    author_id INTEGER NOT NULL,
    branch_id INTEGER NOT NULL,
    UNIQUE (id),
    FOREIGN KEY (author_id) REFERENCES authors(id),
    FOREIGN KEY (branch_id) REFERENCES branches(id),
    FOREIGN KEY (file_from_id) REFERENCES files(id),
    FOREIGN KEY (file_to_id) REFERENCES files(id),
);

CREATE TRIGGER update_files_timestamp
AFTER UPDATE ON files
FOR EACH ROW
BEGIN
    UPDATE files SET updated_at = CURRENT_INTEGER WHERE id = NEW.id;
END;

