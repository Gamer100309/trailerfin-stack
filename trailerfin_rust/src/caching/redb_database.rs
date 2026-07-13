use std::path::Path;
use redb::Database;

#[derive(Debug)]
pub struct RedbDatabase {
    db: Database,
}

impl RedbDatabase {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let db = if !path.exists() {
            Database::create(path).expect("Failed to create new database file.")
        } else {
            let db = Database::open(path).expect("Failed to open existing database file.");
            db
        };

        Ok(Self { db })
    }

    pub fn db(&self) -> &Database {
        &self.db
    }
}
