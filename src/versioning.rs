use libsql::{de, params, Connection, Database, Row, Rows};
use menva::get_env;
use serde::{Deserialize, Serialize};

use crate::{
    settings::Settings,
    turso::{Database as PlatformDb, DatabasesPlatform, TursoClient},
};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct File {
    id: u32,
    hash: String,
    created_at: String,
    updated_at: String,
    remotes: String,
    next: Option<u8>,
    previous: Option<u8>,
    path: String,
}

impl File {
    pub fn remotes(&self) -> Vec<&str> {
        self.remotes.split(",").collect()
    }
    async fn create(conn: &Connection, hash: &str, path: &str) -> Result<(), libsql::Error> {
        let r = conn
            .execute(
                "INSERT INTO files (hash, path) VALUES (?1, ?2)",
                params![hash, path],
            )
            .await?;
        println!("result for {path} {r} in create");
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

pub async fn run(settings: &Settings) -> i16 {
    get_db(settings.local.to_str().unwrap()).await;
    sync_remote(settings).await;
    0
}

async fn get_db(name: &str) {
    let db = Database::open(name).unwrap();
    let conn = db.connect().unwrap();
    let mut results = conn.query("SELECT * FROM files", ()).await.unwrap();
    let rows = RowsIter::new(&mut results);
    let files = rows
        .map(|r| de::from_row::<File>(&r).unwrap())
        .collect::<Vec<File>>()
        .iter()
        .map(|f| f.id)
        .collect::<Vec<u32>>();
    println!("{files:?}");
}

async fn get_remote_db(
    client: &TursoClient<DatabasesPlatform>,
    organization_name: &str,
    db_name: &str,
) -> Option<PlatformDb> {
    match client.retrieve(organization_name, db_name).await {
        Ok(db) => Some(db),
        Err(err) => {
            println!("remote db exists {err:?}");
            None
        }
    }
}

async fn sync_remote(settings: &Settings) {
    let (token, client) = (get_env("TOKEN"), TursoClient::new());
    let db_client = client.databases();
    let remote_db = match get_remote_db(&db_client, &settings.organization, &settings.remote).await
    {
        Some(db) => db,
        None => db_client
            .create(&settings.organization, &settings.remote, &settings.group)
            .await
            .unwrap(),
    };
    println!("{remote_db:?}");
    let db_url = "";
    // let db = Database::open_with_remote_sync(settings.local.to_str().unwrap(), db_url, token)
    //     .await
    //     .unwrap();
    // let conn = db.connect().unwrap();
    // match db.sync().await {
    //     Ok(r) => println!("{r:?}"),
    //     Err(err) => panic!("{err:?}"),
    // };
}
