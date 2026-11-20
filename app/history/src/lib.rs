//! SQLite history store with visits, branch links and day grouping (P4-02).
//!
//! Stores visited pages, Trail branches (opener links), and "came back" flags.
//! Private windows must never record visits here (EDGE_CASES §6).

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Visit {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub host: String,
    pub timestamp: i64,
    pub opener_id: Option<i64>,
    pub came_back: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DayGroup {
    pub label: String,       // "Today", "Yesterday", or "2026-11-18"
    pub day_start_secs: i64, // Local or UTC day boundary in unix seconds
    pub visits: Vec<Visit>,
}

#[derive(Debug)]
pub enum HistoryError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryError::Sqlite(e) => write!(f, "SQLite error: {e}"),
            HistoryError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for HistoryError {}

impl From<rusqlite::Error> for HistoryError {
    fn from(e: rusqlite::Error) -> Self {
        HistoryError::Sqlite(e)
    }
}

impl From<std::io::Error> for HistoryError {
    fn from(e: std::io::Error) -> Self {
        HistoryError::Io(e)
    }
}

#[derive(Clone)]
pub struct HistoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for HistoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryStore").finish()
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn extract_host(url: &str) -> String {
    let s = url.trim();
    let rest = if let Some(stripped) = s.strip_prefix("https://") {
        stripped
    } else if let Some(stripped) = s.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = s.strip_prefix("odyssey://") {
        stripped
    } else {
        s
    };
    rest.split(['/', ':', '?', '#'])
        .next()
        .unwrap_or(url)
        .to_lowercase()
}

impl HistoryStore {
    /// Open or create SQLite history database file at `path`.
    pub fn open(path: &Path) -> Result<Self, HistoryError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// In-memory store (for testing or ephemeral use).
    pub fn open_in_memory() -> Result<Self, HistoryError> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, HistoryError> {
        // WAL mode for performance and concurrent read safety
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS visits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                host TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                opener_id INTEGER,
                came_back INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_visits_timestamp ON visits(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_visits_url ON visits(url);
            CREATE INDEX IF NOT EXISTS idx_visits_opener ON visits(opener_id);
            ",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Record a visit.
    ///
    /// If this URL was previously visited, mark this visit as `came_back = true`
    /// (draws a brighter star in the Trail / History side panel, DESIGN.md §9).
    pub fn record(
        &self,
        url: &str,
        title: &str,
        opener_id: Option<i64>,
    ) -> Result<i64, HistoryError> {
        let conn = self.conn.lock().unwrap();
        let host = extract_host(url);
        let safe_title = if title.trim().is_empty() {
            &host
        } else {
            title.trim()
        };
        let now = current_timestamp();

        // Check if previously visited
        let prev_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM visits WHERE url = ?1",
            params![url],
            |row| row.get(0),
        )?;
        let came_back = prev_count > 0;

        conn.execute(
            "INSERT INTO visits (url, title, host, timestamp, opener_id, came_back)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![url, safe_title, host, now, opener_id, came_back as i32],
        )?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    /// Query visits within a day range [start_timestamp, end_timestamp).
    pub fn query_range(&self, start: i64, end: i64) -> Result<Vec<Visit>, HistoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, host, timestamp, opener_id, came_back
             FROM visits
             WHERE timestamp >= ?1 AND timestamp < ?2
             ORDER BY timestamp DESC, id DESC",
        )?;

        let rows = stmt.query_map(params![start, end], |row| {
            Ok(Visit {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                host: row.get(3)?,
                timestamp: row.get(4)?,
                opener_id: row.get(5)?,
                came_back: row.get::<_, i32>(6)? != 0,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Query recent visits up to `limit`.
    pub fn query_recent(&self, limit: usize) -> Result<Vec<Visit>, HistoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, host, timestamp, opener_id, came_back
             FROM visits
             ORDER BY timestamp DESC, id DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(Visit {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                host: row.get(3)?,
                timestamp: row.get(4)?,
                opener_id: row.get(5)?,
                came_back: row.get::<_, i32>(6)? != 0,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Search visits by title, URL or host (case-insensitive substring match).
    pub fn search(&self, query: &str) -> Result<Vec<Visit>, HistoryError> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query.trim().to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT id, url, title, host, timestamp, opener_id, came_back
             FROM visits
             WHERE LOWER(title) LIKE ?1 OR LOWER(url) LIKE ?1 OR LOWER(host) LIKE ?1
             ORDER BY timestamp DESC, id DESC
             LIMIT 100",
        )?;

        let rows = stmt.query_map(params![pattern], |row| {
            Ok(Visit {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                host: row.get(3)?,
                timestamp: row.get(4)?,
                opener_id: row.get(5)?,
                came_back: row.get::<_, i32>(6)? != 0,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Update title of a recorded visit (e.g. when page title loads after navigation).
    pub fn update_title(&self, id: i64, title: &str) -> Result<(), HistoryError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE visits SET title = ?1 WHERE id = ?2",
            params![trimmed, id],
        )?;
        Ok(())
    }

    /// Query visits grouped into day sections ("Today", "Yesterday", or formatted dates).
    pub fn query_day_groups(&self) -> Result<Vec<DayGroup>, HistoryError> {
        let visits = self.query_recent(500)?;
        Ok(Self::group_visits(visits))
    }

    /// Group any slice or vector of visits into day sections.
    pub fn group_visits(visits: Vec<Visit>) -> Vec<DayGroup> {
        if visits.is_empty() {
            return Vec::new();
        }

        let now = current_timestamp();
        // Day length in seconds
        const DAY_SECS: i64 = 86400;
        let today_start = (now / DAY_SECS) * DAY_SECS;
        let yesterday_start = today_start - DAY_SECS;

        let mut groups: Vec<DayGroup> = Vec::new();

        for v in visits {
            let day_start = (v.timestamp / DAY_SECS) * DAY_SECS;
            let label = if day_start >= today_start {
                "Today".to_string()
            } else if day_start >= yesterday_start {
                "Yesterday".to_string()
            } else {
                format_day_label(day_start)
            };

            if let Some(last) = groups.last_mut().filter(|g| g.day_start_secs == day_start) {
                last.visits.push(v);
                continue;
            }

            groups.push(DayGroup {
                label,
                day_start_secs: day_start,
                visits: vec![v],
            });
        }

        groups
    }

    /// Forget / remove one visit. Re-parents any children to grandparent so branches don't orphan.
    pub fn forget(&self, id: i64) -> Result<(), HistoryError> {
        let conn = self.conn.lock().unwrap();

        // Find parent of the visit to be removed
        // A root visit has a NULL opener_id: read it as Option<i64> inside the row, or the
        // whole forget fails on every top-level visit.
        let parent_opener: Option<i64> = conn
            .query_row(
                "SELECT opener_id FROM visits WHERE id = ?1",
                params![id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();

        // Re-attach children
        conn.execute(
            "UPDATE visits SET opener_id = ?1 WHERE opener_id = ?2",
            params![parent_opener, id],
        )?;

        // Delete the visit
        conn.execute("DELETE FROM visits WHERE id = ?1", params![id])?;

        Ok(())
    }

    /// Clear all visits within one day boundary [day_start_secs, day_start_secs + 86400).
    pub fn clear_day(&self, day_start_secs: i64) -> Result<(), HistoryError> {
        let conn = self.conn.lock().unwrap();
        const DAY_SECS: i64 = 86400;
        conn.execute(
            "DELETE FROM visits WHERE timestamp >= ?1 AND timestamp < ?2",
            params![day_start_secs, day_start_secs + DAY_SECS],
        )?;
        Ok(())
    }

    /// Clear all history.
    pub fn clear_all(&self) -> Result<(), HistoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM visits", [])?;
        Ok(())
    }
}

fn format_day_label(secs: i64) -> String {
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{years:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_query_recent() {
        let store = HistoryStore::open_in_memory().unwrap();
        let id1 = store
            .record("https://example.com/page1", "Page 1", None)
            .unwrap();
        let id2 = store
            .record("https://example.com/page2", "Page 2", Some(id1))
            .unwrap();

        let recent = store.query_recent(10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, id2);
        assert_eq!(recent[0].opener_id, Some(id1));
        assert!(!recent[0].came_back);
        assert_eq!(recent[1].id, id1);
        assert!(!recent[1].came_back);
    }

    #[test]
    fn detects_came_back_on_revisit() {
        let store = HistoryStore::open_in_memory().unwrap();
        let _id1 = store
            .record("https://example.com/star", "Star essay", None)
            .unwrap();
        let id2 = store
            .record("https://example.com/other", "Other", None)
            .unwrap();
        let id3 = store
            .record("https://example.com/star", "Star essay", Some(id2))
            .unwrap();

        let v3 = store.query_recent(1).unwrap().into_iter().next().unwrap();
        assert_eq!(v3.id, id3);
        assert!(v3.came_back, "Re-visited URL must have came_back = true");
    }

    #[test]
    fn search_case_insensitive() {
        let store = HistoryStore::open_in_memory().unwrap();
        store
            .record("https://northstar.example/essay", "Reading the sky", None)
            .unwrap();
        store
            .record("https://tides.example/table", "Tide tables 2026", None)
            .unwrap();

        let s1 = store.search("reading").unwrap();
        assert_eq!(s1.len(), 1);
        assert_eq!(s1[0].title, "Reading the sky");

        let s2 = store.search("NORTHSTAR").unwrap();
        assert_eq!(s2.len(), 1);
        assert_eq!(s2[0].host, "northstar.example");

        let s3 = store.search("nonexistent").unwrap();
        assert!(s3.is_empty());
    }

    #[test]
    fn forget_reparents_branches() {
        let store = HistoryStore::open_in_memory().unwrap();
        let id1 = store.record("https://a.com", "A", None).unwrap();
        let id2 = store.record("https://b.com", "B", Some(id1)).unwrap();
        let id3 = store.record("https://c.com", "C", Some(id2)).unwrap();

        // Forget middle node id2 (B)
        store.forget(id2).unwrap();

        let recent = store.query_recent(10).unwrap();
        assert_eq!(recent.len(), 2);

        let c = recent.iter().find(|v| v.id == id3).unwrap();
        assert_eq!(
            c.opener_id,
            Some(id1),
            "Child should re-attach to grandparent"
        );
    }

    #[test]
    fn forgetting_a_root_visit_works_and_reparents_children() {
        let store = HistoryStore::open_in_memory().unwrap();
        let root = store.record("https://root.example", "Root", None).unwrap();
        let child = store
            .record("https://child.example", "Child", Some(root))
            .unwrap();
        store.forget(root).unwrap();
        let left = store.query_recent(10).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, child);
        assert_eq!(left[0].opener_id, None);
    }

    #[test]
    fn query_day_groups_and_clear_day() {
        let store = HistoryStore::open_in_memory().unwrap();
        let _id1 = store.record("https://today.com", "Today", None).unwrap();

        let groups = store.query_day_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label, "Today");
        assert_eq!(groups[0].visits.len(), 1);

        let day_start = groups[0].day_start_secs;
        store.clear_day(day_start).unwrap();

        let after = store.query_recent(10).unwrap();
        assert!(after.is_empty(), "Cleared day should remove its visits");
    }

    #[test]
    fn clear_all_clears_database() {
        let store = HistoryStore::open_in_memory().unwrap();
        store.record("https://a.com", "A", None).unwrap();
        store.record("https://b.com", "B", None).unwrap();

        assert_eq!(store.query_recent(10).unwrap().len(), 2);
        store.clear_all().unwrap();
        assert!(store.query_recent(10).unwrap().is_empty());
    }

    #[test]
    fn update_title_updates_recorded_visit() {
        let store = HistoryStore::open_in_memory().unwrap();
        let id = store
            .record("https://example.com", "example.com", None)
            .unwrap();
        store.update_title(id, "Real Example Title").unwrap();

        let visits = store.query_recent(1).unwrap();
        assert_eq!(visits[0].title, "Real Example Title");
    }
}
