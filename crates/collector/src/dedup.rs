//! Dedup-cache: blake3(url) -> ferdig prosessert NewsEntry som JSON.
//! Artikler som allerede er oppsummert sendes aldri til Claude igjen.

use anyhow::Result;
use redb::{Database, TableDefinition};
use schema::NewsEntry;
use std::path::Path;

// v2: NewsItem fikk detail_no — nytt tabellnavn så alt re-oppsummeres én gang
const SUMMARIES: TableDefinition<&str, &str> = TableDefinition::new("summaries_v2");
// Småting med hash-basert gjenbruk (surf-analyse o.l.)
const META: TableDefinition<&str, &str> = TableDefinition::new("meta");

pub struct Cache {
    db: Database,
}

impl Cache {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Database::create(path)?;
        let tx = db.begin_write()?;
        tx.open_table(SUMMARIES)?;
        tx.open_table(META)?;
        tx.commit()?;
        Ok(Self { db })
    }

    fn hash(url: &str) -> String {
        blake3::hash(url.as_bytes()).to_hex().to_string()
    }

    pub fn get(&self, url: &str) -> Option<NewsEntry> {
        let tx = self.db.begin_read().ok()?;
        let table = tx.open_table(SUMMARIES).ok()?;
        let key = Self::hash(url);
        let value = table.get(key.as_str()).ok()??;
        serde_json::from_str(value.value()).ok()
    }

    pub fn put(&self, entries: &[NewsEntry]) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(SUMMARIES)?;
            for entry in entries {
                let key = Self::hash(&entry.item.source_url);
                let value = serde_json::to_string(entry)?;
                table.insert(key.as_str(), value.as_str())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Option<String> {
        let tx = self.db.begin_read().ok()?;
        let table = tx.open_table(META).ok()?;
        let value = table.get(key).ok()??;
        Some(value.value().to_string())
    }

    pub fn put_meta(&self, key: &str, value: &str) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(META)?;
            table.insert(key, value)?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::{Category, NewsItem};

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join("capegent-dedup-test");
        let _ = std::fs::remove_dir_all(&dir);
        let cache = Cache::open(&dir.join("test.redb")).unwrap();

        let entry = NewsEntry {
            item: NewsItem {
                headline_no: "Testoverskrift".into(),
                summary_no: "Et sammendrag.".into(),
                detail_no: "Et lengre sammendrag.".into(),
                category: Category::Other,
                urgency: 3,
                source_url: "https://example.com/artikkel".into(),
            },
            source: "test".into(),
            published_at: None,
        };

        assert!(cache.get(&entry.item.source_url).is_none());
        cache.put(std::slice::from_ref(&entry)).unwrap();
        let fetched = cache.get(&entry.item.source_url).unwrap();
        assert_eq!(fetched.item.headline_no, "Testoverskrift");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
