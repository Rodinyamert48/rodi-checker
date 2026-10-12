use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::Storage;
use crate::error::{CoreError, Result};
use crate::files::{FileEntry, HashAlgorithm, ScanOptions};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Baseline {
    pub id: i64,
    pub name: String,
    pub root_path: String,
    pub algorithm: String,
    /// Newline separated exclude patterns.
    pub excludes: String,
    pub created_at: String,
    pub file_count: i64,
    pub total_size: i64,
}

impl Baseline {
    pub fn scan_options(&self) -> Result<ScanOptions> {
        Ok(ScanOptions {
            algorithm: self.algorithm.parse::<HashAlgorithm>()?,
            excludes: self
                .excludes
                .lines()
                .map(str::to_owned)
                .filter(|l| !l.trim().is_empty())
                .collect(),
        })
    }
}

#[derive(FromRow)]
struct EntryRow {
    path: String,
    size: i64,
    modified_at: Option<DateTime<Utc>>,
    hash: String,
}

const BASELINE_SQL: &str =
    "SELECT b.id, b.name, b.root_path, b.algorithm, b.excludes, b.created_at,
        COUNT(e.path) AS file_count, COALESCE(SUM(e.size), 0) AS total_size
     FROM file_baselines b LEFT JOIN file_entries e ON e.baseline_id = b.id";

impl Storage {
    /// Saves a baseline, replacing any baseline with the same name.
    pub async fn save_baseline(
        &self,
        name: &str,
        root: &str,
        options: &ScanOptions,
        entries: &[FileEntry],
    ) -> Result<Baseline> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Invalid("baseline name is empty".into()));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM file_baselines WHERE name = ?")
            .bind(name)
            .execute(&mut *tx)
            .await?;
        let id = sqlx::query(
            "INSERT INTO file_baselines (name, root_path, algorithm, excludes) VALUES (?, ?, ?, ?)",
        )
        .bind(name)
        .bind(root)
        .bind(options.algorithm.as_str())
        .bind(options.excludes.join("\n"))
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
        for entry in entries {
            sqlx::query("INSERT INTO file_entries (baseline_id, path, size, modified_at, hash) VALUES (?, ?, ?, ?, ?)")
                .bind(id)
                .bind(&entry.path)
                .bind(entry.size as i64)
                .bind(entry.modified_at)
                .bind(&entry.hash)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        self.baseline(id).await
    }

    pub async fn baseline(&self, id: i64) -> Result<Baseline> {
        sqlx::query_as(&format!("{BASELINE_SQL} WHERE b.id = ? GROUP BY b.id"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("baseline {id}")))
    }

    pub async fn baseline_by_name(&self, name: &str) -> Result<Baseline> {
        sqlx::query_as(&format!("{BASELINE_SQL} WHERE b.name = ? GROUP BY b.id"))
            .bind(name.trim())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("baseline {name:?}")))
    }

    pub async fn baselines(&self) -> Result<Vec<Baseline>> {
        Ok(
            sqlx::query_as(&format!("{BASELINE_SQL} GROUP BY b.id ORDER BY b.name"))
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn baseline_entries(&self, id: i64) -> Result<Vec<FileEntry>> {
        let rows: Vec<EntryRow> =
            sqlx::query_as("SELECT path, size, modified_at, hash FROM file_entries WHERE baseline_id = ? ORDER BY path")
                .bind(id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .map(|row| FileEntry {
                path: row.path,
                size: row.size as u64,
                modified_at: row.modified_at,
                hash: row.hash,
            })
            .collect())
    }

    pub async fn delete_baseline(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM file_baselines WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u64) -> FileEntry {
        FileEntry {
            path: path.into(),
            size,
            modified_at: Some(Utc::now()),
            hash: format!("h-{path}"),
        }
    }

    #[tokio::test]
    async fn baseline_round_trip_and_replace() {
        let storage = Storage::in_memory().await.unwrap();
        let options = ScanOptions {
            algorithm: HashAlgorithm::Blake3,
            excludes: vec!["*.log".into(), "target".into()],
        };
        let saved = storage
            .save_baseline(
                "proje",
                "C:\\proje",
                &options,
                &[entry("b.txt", 5), entry("a.txt", 3)],
            )
            .await
            .unwrap();
        assert_eq!((saved.file_count, saved.total_size), (2, 8));
        assert_eq!(saved.scan_options().unwrap(), options);

        let entries = storage.baseline_entries(saved.id).await.unwrap();
        assert_eq!(
            entries.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
            ["a.txt", "b.txt"]
        );
        assert!(entries[0].modified_at.is_some());

        let replaced = storage
            .save_baseline("proje", "C:\\proje", &options, &[entry("c.txt", 1)])
            .await
            .unwrap();
        assert_eq!(storage.baselines().await.unwrap().len(), 1);
        assert_eq!(
            storage.baseline_by_name(" proje ").await.unwrap().id,
            replaced.id
        );
        assert_eq!(replaced.file_count, 1);

        storage.delete_baseline(replaced.id).await.unwrap();
        assert!(storage
            .baseline_entries(replaced.id)
            .await
            .unwrap()
            .is_empty());
        assert!(matches!(
            storage.baseline_by_name("proje").await,
            Err(CoreError::NotFound(_))
        ));
    }
}
