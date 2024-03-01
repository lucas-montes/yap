use core::panic;

use libsql::{de, params, Connection, Database, Row, Rows};
use menva::{get_env, read_env_file};
use serde::{Deserialize, Serialize};

pub async fn run() -> i16 {
    get_db().await;
    0
}

struct RowsIter<'a> {
    rows: &'a mut Rows,
}

impl<'a> RowsIter<'a> {
    fn new(rows: &'a mut Rows) -> Self {
        Self { rows }
    }
}

impl<'a> Iterator for RowsIter<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        match self.rows.next() {
            Ok(row) => row,
            Err(err) => panic!("some error in the iterator getting the next row: {err:?}"),
        }
    }
}
//impl Iterator for Rows {
//    type Item = Row;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        match self.inner.next() {
//            Ok(row) => row,
//            Err(err) => panic!("some error in the iterator getting the next row: {err:?}"),
//        }
//    }
//}
//

#[derive(Debug, Serialize, Deserialize, Clone)]
struct File {
    hash: String,
    created_at: String,
    updated_at: String,
    next: Option<u8>,
    previous: Option<u8>,
    name: String,
}

impl File {
    async fn create(conn: &Connection, hash: &str, path: &str) -> Result<(), libsql::Error> {
        conn.execute(
            "INSERT INTO files (hash, path) VALUES (?1, ?2)",
            params![hash, path],
        )
        .await?;
        Ok(())
    }

    async fn get_by_id(conn: &Connection, id: u32) -> Option<File> {
        let mut result = conn
            .query("SELECT * FROM files WHERE id = ?1", params![id])
            .await
            .unwrap();

        result
            .next()
            .unwrap()
            .and_then(|r| de::from_row(&r).expect("no row found in get by id"))
    }

    async fn update(conn: &Connection, id: u32, new_path: &str) -> Result<(), libsql::Error> {
        conn.execute(
            "UPDATE files SET path = ?1 WHERE id = ?2",
            params![new_path, id],
        )
        .await?;
        Ok(())
    }

    async fn delete_by_id(conn: &Connection, id: u32) -> Result<(), libsql::Error> {
        conn.execute("DELETE FROM Files WHERE id = ?1", params![id])
            .await?;
        Ok(())
    }
}

async fn get_db() {
    let db = Database::open("test.db").unwrap();
    let conn = db.connect().unwrap();
    let mut results = conn.query("SELECT * FROM files", ()).await.unwrap();
    let rows = RowsIter::new(&mut results);
    let files = rows
        .map(|r| de::from_row::<File>(&r).unwrap())
        .collect::<Vec<File>>();
    println!("{files:?}");
    File::create(&conn, "hash4", "path4").await;
    File::create(&conn, "hash5", "path5").await;
}

async fn sync_remote() {
    let db_url = "test.db";
    let db_remote = "test";
    let auth_token = get_env("TOKEN");
    println!("{auth_token}");
    let db = Database::open_with_remote_sync(db_url, db_remote, auth_token)
        .await
        .unwrap();
    let conn = db.connect().unwrap();
    match db.sync().await {
        Ok(r) => println!("{r:?}"),
        Err(err) => panic!("{err:?}"),
    };
    let mut results = conn.query("SELECT * FROM files", ()).await.unwrap();
    let rows = RowsIter::new(&mut results);
    let files = rows
        .map(|r| de::from_row::<File>(&r).unwrap())
        .collect::<Vec<File>>();
    println!("{files:?}");
}
