use async_trait::async_trait;
use futures::StreamExt;
use libsql::Connection;

use std::path::PathBuf;

#[derive(Debug)]
pub struct File {
    pub id: u8,
    pub file: String,
}
#[allow(dead_code)]
impl File {
    pub fn new(file: String) -> Self {
        File { id: 0, file }
    }
    pub async fn get_all(connection: &Connection) -> Vec<String> {
        let query = format!("SELECT file FROM {table}", table = Self::table());
        connection
            .query(&query, ())
            .await
            .unwrap_or_else(|_| panic!("get_all {:?}", query))
            .into_stream()
            .map(|f| f.unwrap().get::<String>(0).unwrap())
            .collect()
            .await
    }
    pub async fn delete_many(paths: Vec<PathBuf>, connection: &Connection) -> u64 {
        let values = paths
            .iter()
            .map(|p| format!("'{}'", p.to_str().unwrap().to_owned()))
            .collect::<Vec<String>>()
            .join(",");
        let query = format!(
            "DELETE FROM {table} WHERE {fields} IN ({values});",
            table = Self::table(),
            fields = "file",
            values = values
        );
        connection
            .execute(&query, ())
            .await
            .unwrap_or_else(|_| panic!("delete_many {:?}", query));
        0
    }

    pub async fn insert_many(paths: &[PathBuf], connection: &Connection) -> u64 {
        let values = paths
            .iter()
            .map(|p| {
                format!(
                    "('{}')",
                    std::fs::canonicalize(p).unwrap().to_str().unwrap()
                )
            })
            .collect::<Vec<String>>()
            .join(",");
        let query = format!(
            "INSERT OR IGNORE INTO {table} ('{fields}') VALUES {values};",
            table = Self::table(),
            fields = "file",
            values = values
        );
        connection
            .execute(&query, ())
            .await
            .unwrap_or_else(|_| panic!("insert_many {:?}", query));
        0
    }
}

impl Manager for File {
    fn create_or_update_query(&self) -> String {
        format!(
            "
            INSERT INTO {table} ({fields})
            VALUES({values})
            ON CONFLICT({conflict_fields})
            DO UPDATE SET {update_fields};
        ",
            table = Self::table(),
            fields = "file",
            values = self.file,
            conflict_fields = "file",
            update_fields = "file"
        )
    }

    fn get_or_create_query(&self) -> String {
        format!(
            "INSERT OR IGNORE INTO {table} ({fields}) VALUES ('{values}');
            SELECT id, file FROM {table} WHERE {fields} = '{values}' LIMIT 1;
        ",
            table = Self::table(),
            fields = "file",
            values = self.file
        )
    }
}

#[derive(Debug)]
pub struct Word {
    pub id: u8,
    pub word: String,
}

impl Word {
    pub fn new(word: &str) -> Self {
        Word {
            id: 0,
            word: word.to_owned(),
        }
    }
}

impl Manager for Word {
    fn create_or_update_query(&self) -> String {
        format!(
            "
        INSERT INTO {table} ({fields})
        VALUES({values})
        ON CONFLICT({conflict_fields})
        DO UPDATE SET {update_fields};
        ",
            table = Self::table(),
            fields = "word",
            values = self.word,
            conflict_fields = "word",
            update_fields = "word",
        )
    }

    fn get_or_create_query(&self) -> String {
        format!(
            "INSERT OR IGNORE INTO {table} ({fields}) VALUES ('{values}');
            SELECT id, word FROM {table} WHERE {fields} = '{values}' LIMIT 1;
        ",
            table = Self::table(),
            fields = "word",
            values = self.word
        )
    }
}

#[derive(Debug)]
pub struct FileWordRelation {
    pub id: u8,
    pub word_id: u8,
    pub file_id: u8,
    pub word_count: u8,
}

impl FileWordRelation {
    pub fn new(word_id: u8, file_id: u8, word_count: u8) -> Self {
        FileWordRelation {
            id: 0,
            word_id,
            file_id,
            word_count,
        }
    }
}

#[async_trait]
impl Manager for FileWordRelation {
    fn create_or_update_query(&self) -> String {
        format!(
            "
            INSERT INTO {table} ({fields})
            VALUES ({word_id},{file_id},{word_count})
            ON CONFLICT(word_id, file_id)
            DO UPDATE SET word_count = {word_count};
            SELECT * FROM {table} WHERE word_id = {word_id} and file_id = {file_id} LIMIT 1;
            ",
            table = Self::table(),
            fields = "word_id, file_id, word_count",
            word_id = self.word_id,
            file_id = self.file_id,
            word_count = self.word_count
        )
    }

    fn get_or_create_query(&self) -> String {
        format!(
            "INSERT OR IGNORE INTO {table} ({fields}) VALUES ('{values}');
            SELECT {selected} FROM {table} WHERE {fields} = '{values}' and  LIMIT 1;
        ",
            table = Self::table(),
            fields = "",
            values = "",
            selected = "",
        )
    }
}

pub trait Manager
where
    Self: Send + Sync + Unpin + Sized,
{
    fn create_or_update_query(&self) -> String;

    async fn create_or_update(&self, connection: &Connection) {
        let query = self.create_or_update_query();
        connection
            .execute(&query, ())
            .await
            .unwrap_or_else(|_| panic!("create_or_update {:?}", query));
    }

    fn get_or_create_query(&self) -> String;

    async fn get_or_create(self, connection: &Connection) -> Self {
        let query = self.get_or_create_query();
        connection
            .execute(&query, ())
            .await
            .unwrap_or_else(|_| panic!("get_or_create {:?}", query));
        self
    }

    fn table() -> String {
        format!("{}s", Self::struct_to_snake_case())
    }

    fn struct_to_snake_case() -> String {
        let mut result = String::new();

        for (i, c) in Self::entity_name().chars().enumerate() {
            if c.is_ascii_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }

        result
    }

    fn entity_name() -> String {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .unwrap()
            .to_string()
    }
}
