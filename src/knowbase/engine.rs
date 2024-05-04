use libsql::Connection;

use super::file_handlers::get_word_count;
use super::models::{File, FileWordRelation, Manager, Word};
use std::path::Path;

#[allow(dead_code)]
pub async fn tf(filepath: &Path, connection: &Connection) {
    let word_count = get_word_count(filepath);
    let file = File::new(filepath.to_str().unwrap().to_string())
        .get_or_create(connection)
        .await;

    for (key, value) in word_count.iter() {
        let word = Word::new(key).get_or_create(connection).await;
        let _ = FileWordRelation::new(word.id, file.id, *value)
            .create_or_update(connection)
            .await;
    }
}
