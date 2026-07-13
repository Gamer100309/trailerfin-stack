use std::sync::Arc;
use redb::{TableDefinition};
use crate::caching::redb_database::RedbDatabase;

#[derive(Debug)]
pub struct TmdbToImdbCache {
    redb: Arc<RedbDatabase>,
}

const TMDB_TO_IMDB_TABLE: TableDefinition<&str, &str> = TableDefinition::new("tmdb_to_imdb");

impl TmdbToImdbCache {
    pub fn new(redb: Arc<RedbDatabase>) -> anyhow::Result<Self> {
        let txn = redb.db().begin_write()?;
        _ = txn.open_table(TMDB_TO_IMDB_TABLE).expect("Failed to open TMDB to IMDB table.");
        txn.commit().expect("Failed to commit transaction.");
        Ok(Self { redb })
    }

    pub fn try_get_imdb_id(&self, tmdb_id: &str) -> anyhow::Result<Option<String>> {
        let txn = self.redb.db().begin_read()?;
        let table = txn.open_table(TMDB_TO_IMDB_TABLE)?;
        let value = table.get(tmdb_id)?;
        Ok(value.map(|v| v.value().to_owned()))
    }

    pub fn add(&self, tmdb_id: &str, imdb_id: &str) -> anyhow::Result<()> {
        let txn = self.redb.db().begin_write()?;
        {
            let mut table = txn.open_table(TMDB_TO_IMDB_TABLE)?;
            table.insert(tmdb_id, imdb_id)?;
        }
        txn.commit()?;
        Ok(())
    }
}