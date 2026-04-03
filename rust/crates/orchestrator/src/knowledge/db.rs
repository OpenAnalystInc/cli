//! SQLite database for the OpenAnalyst CLI.
//!
//! Schema:
//! - cli_credentials: provider API keys — 3rd persistence layer alongside .env + credentials.json
//! - kb_queries: every query with intent, timestamp, session context
//! - kb_feedback: user rating + correction for each query
//! - kb_learnings: positive learnings extracted from feedback (reusable)
//! - kb_wrong_learnings: mistakes to avoid (anti-patterns)
//!
//! The database is at .openanalyst/knowledge.db — created on first use.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::intent::Intent;

/// The learning database connection.
pub struct LearningDb {
    conn: Connection,
}

/// A stored query record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRecord {
    pub id: i64,
    pub query: String,
    pub intent: String,
    pub kb_results_count: i32,
    pub response_preview: String,
    pub created_at: String,
    pub session_id: String,
}

/// A stored learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    pub id: i64,
    pub category: String,
    pub intent: String,
    pub insight: String,
    pub confidence: f64,
    pub use_count: i32,
    pub created_at: String,
    pub source_query_id: i64,
}

/// A stored mistake to avoid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrongLearning {
    pub id: i64,
    pub category: String,
    pub intent: String,
    pub mistake: String,
    pub correction: String,
    pub created_at: String,
    pub source_query_id: i64,
}

/// User feedback for a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackRating {
    /// User was satisfied (thumbs up).
    Positive,
    /// User was not satisfied (thumbs down).
    Negative,
    /// User provided a correction.
    Corrected,
}

impl LearningDb {
    /// Open or create the learning database.
    pub fn open() -> SqlResult<Self> {
        let db_dir = std::path::Path::new(".openanalyst");
        let _ = std::fs::create_dir_all(db_dir);
        let db_path = db_dir.join("knowledge.db");
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open with a specific path (for testing).
    pub fn open_at(path: &std::path::Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Create all tables and indexes.
    fn init_schema(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            -- ── Credentials (3rd persistence layer) ──
            CREATE TABLE IF NOT EXISTS cli_credentials (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_name TEXT NOT NULL UNIQUE,
                env_var       TEXT NOT NULL,
                api_key       TEXT NOT NULL,
                auth_method   TEXT NOT NULL DEFAULT 'api_key',
                oauth_refresh TEXT DEFAULT '',
                oauth_expires INTEGER DEFAULT 0,
                updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS kb_queries (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                query         TEXT NOT NULL,
                intent        TEXT NOT NULL,
                kb_results_count INTEGER DEFAULT 0,
                response_preview TEXT DEFAULT '',
                session_id    TEXT DEFAULT '',
                created_at    TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS kb_feedback (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                query_id      INTEGER NOT NULL REFERENCES kb_queries(id),
                rating        TEXT NOT NULL CHECK(rating IN ('positive', 'negative', 'corrected')),
                user_comment  TEXT DEFAULT '',
                correction    TEXT DEFAULT '',
                created_at    TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS kb_learnings (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                category      TEXT NOT NULL,
                intent        TEXT NOT NULL,
                insight       TEXT NOT NULL,
                confidence    REAL DEFAULT 1.0,
                use_count     INTEGER DEFAULT 0,
                source_query_id INTEGER REFERENCES kb_queries(id),
                created_at    TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS kb_wrong_learnings (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                category      TEXT NOT NULL,
                intent        TEXT NOT NULL,
                mistake       TEXT NOT NULL,
                correction    TEXT NOT NULL,
                source_query_id INTEGER REFERENCES kb_queries(id),
                created_at    TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- ── Knowledge cache (local results for instant replay) ──
            CREATE TABLE IF NOT EXISTS kb_cache (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                query_hash    TEXT NOT NULL UNIQUE,
                query_text    TEXT NOT NULL,
                intent        TEXT NOT NULL,
                response_json TEXT NOT NULL,
                answer_text   TEXT DEFAULT '',
                file_path     TEXT DEFAULT '',
                hit_count     INTEGER DEFAULT 0,
                created_at    TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at    TEXT
            );

            -- Indexes
            CREATE UNIQUE INDEX IF NOT EXISTS idx_cache_hash ON kb_cache(query_hash);
            CREATE INDEX IF NOT EXISTS idx_cache_expires ON kb_cache(expires_at);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_creds_provider ON cli_credentials(provider_name);
            CREATE INDEX IF NOT EXISTS idx_creds_env_var ON cli_credentials(env_var);
            CREATE INDEX IF NOT EXISTS idx_queries_intent ON kb_queries(intent);
            CREATE INDEX IF NOT EXISTS idx_queries_created ON kb_queries(created_at);
            CREATE INDEX IF NOT EXISTS idx_queries_session ON kb_queries(session_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_query ON kb_feedback(query_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_rating ON kb_feedback(rating);
            CREATE INDEX IF NOT EXISTS idx_learnings_intent ON kb_learnings(intent);
            CREATE INDEX IF NOT EXISTS idx_learnings_category ON kb_learnings(category);
            CREATE INDEX IF NOT EXISTS idx_learnings_confidence ON kb_learnings(confidence DESC);
            CREATE INDEX IF NOT EXISTS idx_wrong_intent ON kb_wrong_learnings(intent);
            CREATE INDEX IF NOT EXISTS idx_wrong_category ON kb_wrong_learnings(category);
            "
        )
    }

    // ── Credential operations (3rd persistence layer) ──

    /// Save or update a provider credential.
    pub fn save_credential(
        &self,
        provider_name: &str,
        env_var: &str,
        api_key: &str,
        auth_method: &str,
        oauth_refresh: Option<&str>,
        oauth_expires: Option<i64>,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO cli_credentials (provider_name, env_var, api_key, auth_method, oauth_refresh, oauth_expires, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
             ON CONFLICT(provider_name) DO UPDATE SET
               env_var = excluded.env_var,
               api_key = excluded.api_key,
               auth_method = excluded.auth_method,
               oauth_refresh = excluded.oauth_refresh,
               oauth_expires = excluded.oauth_expires,
               updated_at = datetime('now')",
            params![
                provider_name,
                env_var,
                api_key,
                auth_method,
                oauth_refresh.unwrap_or(""),
                oauth_expires.unwrap_or(0),
            ],
        )?;
        Ok(())
    }

    /// Load a credential by provider name.
    pub fn load_credential(&self, provider_name: &str) -> SqlResult<Option<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT env_var, api_key, auth_method FROM cli_credentials WHERE provider_name = ?1"
        )?;
        let mut rows = stmt.query_map(params![provider_name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;
        match rows.next() {
            Some(Ok(row)) => Ok(Some(row)),
            _ => Ok(None),
        }
    }

    /// Load all credentials (for startup env loading).
    pub fn load_all_credentials(&self) -> SqlResult<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider_name, env_var, api_key FROM cli_credentials ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Delete a credential by provider name.
    pub fn delete_credential(&self, provider_name: &str) -> SqlResult<bool> {
        let count = self.conn.execute(
            "DELETE FROM cli_credentials WHERE provider_name = ?1",
            params![provider_name],
        )?;
        Ok(count > 0)
    }

    // ── Write operations ──

    /// Record a query and return its ID.
    pub fn record_query(
        &self,
        query: &str,
        intent: Intent,
        kb_results_count: i32,
        response_preview: &str,
        session_id: &str,
    ) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO kb_queries (query, intent, kb_results_count, response_preview, session_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![query, intent.label(), kb_results_count, response_preview, session_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record user feedback for a query.
    pub fn record_feedback(
        &self,
        query_id: i64,
        rating: FeedbackRating,
        comment: &str,
        correction: &str,
    ) -> SqlResult<()> {
        let rating_str = match rating {
            FeedbackRating::Positive => "positive",
            FeedbackRating::Negative => "negative",
            FeedbackRating::Corrected => "corrected",
        };
        self.conn.execute(
            "INSERT INTO kb_feedback (query_id, rating, user_comment, correction) VALUES (?1, ?2, ?3, ?4)",
            params![query_id, rating_str, comment, correction],
        )?;
        Ok(())
    }

    /// Store a positive learning.
    pub fn add_learning(
        &self,
        category: &str,
        intent: Intent,
        insight: &str,
        confidence: f64,
        source_query_id: i64,
    ) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO kb_learnings (category, intent, insight, confidence, source_query_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![category, intent.label(), insight, confidence, source_query_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Store a mistake/wrong learning to avoid.
    pub fn add_wrong_learning(
        &self,
        category: &str,
        intent: Intent,
        mistake: &str,
        correction: &str,
        source_query_id: i64,
    ) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO kb_wrong_learnings (category, intent, mistake, correction, source_query_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![category, intent.label(), mistake, correction, source_query_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Increment use_count for a learning (tracks how often it's been applied).
    pub fn mark_learning_used(&self, learning_id: i64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE kb_learnings SET use_count = use_count + 1 WHERE id = ?1",
            params![learning_id],
        )?;
        Ok(())
    }

    // ── Read operations (for AI context injection) ──

    /// Get relevant learnings for a given intent (ordered by confidence, most useful first).
    pub fn get_learnings_for_intent(&self, intent: Intent, limit: usize) -> SqlResult<Vec<Learning>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, intent, insight, confidence, use_count, source_query_id, created_at
             FROM kb_learnings
             WHERE intent = ?1
             ORDER BY confidence DESC, use_count DESC
             LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![intent.label(), limit as i64], |row| {
            Ok(Learning {
                id: row.get(0)?,
                category: row.get(1)?,
                intent: row.get(2)?,
                insight: row.get(3)?,
                confidence: row.get(4)?,
                use_count: row.get(5)?,
                source_query_id: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    /// Get mistakes to avoid for a given intent.
    pub fn get_wrong_learnings_for_intent(&self, intent: Intent, limit: usize) -> SqlResult<Vec<WrongLearning>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, intent, mistake, correction, source_query_id, created_at
             FROM kb_wrong_learnings
             WHERE intent = ?1
             ORDER BY created_at DESC
             LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![intent.label(), limit as i64], |row| {
            Ok(WrongLearning {
                id: row.get(0)?,
                category: row.get(1)?,
                intent: row.get(2)?,
                mistake: row.get(3)?,
                correction: row.get(4)?,
                source_query_id: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// Get ALL learnings (for global context — top N most confident).
    pub fn get_top_learnings(&self, limit: usize) -> SqlResult<Vec<Learning>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, intent, insight, confidence, use_count, source_query_id, created_at
             FROM kb_learnings
             ORDER BY confidence DESC, use_count DESC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(Learning {
                id: row.get(0)?,
                category: row.get(1)?,
                intent: row.get(2)?,
                insight: row.get(3)?,
                confidence: row.get(4)?,
                use_count: row.get(5)?,
                source_query_id: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    /// Get recent query history for session context.
    pub fn get_recent_queries(&self, session_id: &str, limit: usize) -> SqlResult<Vec<QueryRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, query, intent, kb_results_count, response_preview, created_at, session_id
             FROM kb_queries
             WHERE session_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![session_id, limit as i64], |row| {
            Ok(QueryRecord {
                id: row.get(0)?,
                query: row.get(1)?,
                intent: row.get(2)?,
                kb_results_count: row.get(3)?,
                response_preview: row.get(4)?,
                created_at: row.get(5)?,
                session_id: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// Get database statistics for /knowledge status.
    pub fn stats(&self) -> SqlResult<DbStats> {
        let query_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM kb_queries", [], |r| r.get(0))?;
        let learning_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM kb_learnings", [], |r| r.get(0))?;
        let wrong_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM kb_wrong_learnings", [], |r| r.get(0))?;
        let positive_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM kb_feedback WHERE rating = 'positive'", [], |r| r.get(0))?;
        let negative_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM kb_feedback WHERE rating = 'negative'", [], |r| r.get(0))?;
        Ok(DbStats {
            total_queries: query_count,
            total_learnings: learning_count,
            total_wrong_learnings: wrong_count,
            positive_feedback: positive_count,
            negative_feedback: negative_count,
        })
    }

    /// Build context string for injection into the knowledge query prompt.
    /// This is what makes the system self-learning.
    pub fn build_learning_context(&self, intent: Intent) -> String {
        let mut context = String::new();

        // Get relevant positive learnings
        if let Ok(learnings) = self.get_learnings_for_intent(intent, 5) {
            if !learnings.is_empty() {
                context.push_str("\n<past-learnings>\n");
                for l in &learnings {
                    context.push_str(&format!("- [{}] {}\n", l.category, l.insight));
                }
                context.push_str("</past-learnings>\n");
            }
        }

        // Get mistakes to avoid
        if let Ok(wrongs) = self.get_wrong_learnings_for_intent(intent, 3) {
            if !wrongs.is_empty() {
                context.push_str("\n<avoid-these-mistakes>\n");
                for w in &wrongs {
                    context.push_str(&format!("- WRONG: {} → CORRECT: {}\n", w.mistake, w.correction));
                }
                context.push_str("</avoid-these-mistakes>\n");
            }
        }

        // Get top global learnings
        if let Ok(top) = self.get_top_learnings(3) {
            if !top.is_empty() && context.is_empty() {
                context.push_str("\n<general-learnings>\n");
                for l in &top {
                    context.push_str(&format!("- {}\n", l.insight));
                }
                context.push_str("</general-learnings>\n");
            }
        }

        context
    }
}

/// Database statistics.
#[derive(Debug, Clone)]
pub struct DbStats {
    pub total_queries: i64,
    pub total_learnings: i64,
    pub total_wrong_learnings: i64,
    pub positive_feedback: i64,
    pub negative_feedback: i64,
}

impl std::fmt::Display for DbStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Knowledge DB: {} queries, {} learnings, {} mistakes, {} 👍 / {} 👎",
            self.total_queries, self.total_learnings, self.total_wrong_learnings,
            self.positive_feedback, self.negative_feedback
        )
    }
}

/// A cached knowledge base result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    pub query_hash: String,
    pub query_text: String,
    pub intent: String,
    pub response_json: String,
    pub answer_text: String,
    pub file_path: String,
    pub hit_count: i32,
    pub created_at: String,
}

impl LearningDb {
    // ── Cache operations ──

    /// Look up a cached result by query hash. Returns None if not found or expired.
    /// Bumps hit_count on cache hit.
    pub fn cache_lookup(&self, query_hash: &str) -> SqlResult<Option<CachedResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT query_hash, query_text, intent, response_json, answer_text, file_path, hit_count, created_at, expires_at
             FROM kb_cache WHERE query_hash = ?1"
        )?;
        let result = stmt.query_row(params![query_hash], |row| {
            let expires_at: Option<String> = row.get(8)?;
            Ok((CachedResult {
                query_hash: row.get(0)?,
                query_text: row.get(1)?,
                intent: row.get(2)?,
                response_json: row.get(3)?,
                answer_text: row.get(4)?,
                file_path: row.get(5)?,
                hit_count: row.get(6)?,
                created_at: row.get(7)?,
            }, expires_at))
        });

        match result {
            Ok((cached, expires_at)) => {
                // Check expiry
                if let Some(exp) = expires_at {
                    if exp < chrono_now_str() {
                        // Expired — delete and return None
                        let _ = self.conn.execute("DELETE FROM kb_cache WHERE query_hash = ?1", params![query_hash]);
                        return Ok(None);
                    }
                }
                // Bump hit count
                let _ = self.conn.execute(
                    "UPDATE kb_cache SET hit_count = hit_count + 1 WHERE query_hash = ?1",
                    params![query_hash],
                );
                Ok(Some(cached))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Store a result in the cache. Also writes a .md file to .openanalyst/knowledge/.
    pub fn cache_store(
        &self,
        query_hash: &str,
        query_text: &str,
        intent: &str,
        response_json: &str,
        answer_text: &str,
        ttl_days: i64,
    ) -> SqlResult<()> {
        // Write .md cache file
        let cache_dir = std::path::Path::new(".openanalyst").join("knowledge");
        let _ = std::fs::create_dir_all(&cache_dir);
        let file_path = cache_dir.join(format!("{query_hash}.md"));
        let md_content = format!(
            "---\nquery: \"{}\"\nintent: {}\ncached_at: {}\n---\n\n## Answer\n\n{}\n",
            query_text.replace('"', "\\\""),
            intent,
            chrono_now_str(),
            answer_text,
        );
        let _ = std::fs::write(&file_path, &md_content);

        let file_path_str = file_path.to_string_lossy().to_string();

        self.conn.execute(
            &format!(
                "INSERT INTO kb_cache (query_hash, query_text, intent, response_json, answer_text, file_path, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, {})
                 ON CONFLICT(query_hash) DO UPDATE SET
                   response_json = excluded.response_json,
                   answer_text = excluded.answer_text,
                   file_path = excluded.file_path,
                   hit_count = kb_cache.hit_count + 1",
                if ttl_days > 0 { format!("datetime('now', '+{ttl_days} days')") } else { "NULL".to_string() }
            ),
            params![query_hash, query_text, intent, response_json, answer_text, file_path_str],
        )?;
        Ok(())
    }
}

/// Simple timestamp string (no chrono dependency).
fn chrono_now_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // ISO-ish format for SQLite comparison
    let secs = now.as_secs();
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;
    // Simplified year calc
    let mut y = 1970u64;
    let mut rem = days;
    loop {
        let dy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if rem < dy { break; }
        rem -= dy;
        y += 1;
    }
    let months = [31, if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 1u64;
    for dm in &months {
        if rem < *dm { break; }
        rem -= dm;
        mo += 1;
    }
    let d = rem + 1;
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02}")
}

/// Compute SHA256 hash of a normalized query for cache lookup.
pub fn normalize_query_hash(query: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let normalized = query.trim().to_lowercase();
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_db() -> LearningDb {
        LearningDb::open_at(&PathBuf::from(":memory:")).unwrap()
    }

    #[test]
    fn creates_schema() {
        let db = test_db();
        let stats = db.stats().unwrap();
        assert_eq!(stats.total_queries, 0);
    }

    #[test]
    fn records_and_retrieves_queries() {
        let db = test_db();
        let id = db.record_query("what is a funnel?", Intent::Factual, 3, "A funnel is...", "sess-1").unwrap();
        assert!(id > 0);
        let queries = db.get_recent_queries("sess-1", 10).unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].query, "what is a funnel?");
    }

    #[test]
    fn stores_and_retrieves_learnings() {
        let db = test_db();
        let qid = db.record_query("test", Intent::Factual, 0, "", "").unwrap();
        db.add_learning("marketing", Intent::Factual, "Users prefer short answers for factual queries", 0.9, qid).unwrap();
        db.add_learning("marketing", Intent::Factual, "Always cite the source module", 0.8, qid).unwrap();

        let learnings = db.get_learnings_for_intent(Intent::Factual, 10).unwrap();
        assert_eq!(learnings.len(), 2);
        assert!(learnings[0].confidence >= learnings[1].confidence); // ordered by confidence
    }

    #[test]
    fn stores_and_retrieves_wrong_learnings() {
        let db = test_db();
        let qid = db.record_query("test", Intent::Procedural, 0, "", "").unwrap();
        db.add_wrong_learning("process", Intent::Procedural, "Listed steps without prerequisites", "Always list prerequisites first", qid).unwrap();

        let wrongs = db.get_wrong_learnings_for_intent(Intent::Procedural, 10).unwrap();
        assert_eq!(wrongs.len(), 1);
        assert_eq!(wrongs[0].mistake, "Listed steps without prerequisites");
    }

    #[test]
    fn builds_learning_context() {
        let db = test_db();
        let qid = db.record_query("test", Intent::Strategic, 0, "", "").unwrap();
        db.add_learning("strategy", Intent::Strategic, "User prefers actionable recommendations", 0.95, qid).unwrap();
        db.add_wrong_learning("strategy", Intent::Strategic, "Too abstract", "Give specific next steps", qid).unwrap();

        let context = db.build_learning_context(Intent::Strategic);
        assert!(context.contains("actionable recommendations"));
        assert!(context.contains("Too abstract"));
        assert!(context.contains("Give specific next steps"));
    }

    #[test]
    fn feedback_recording() {
        let db = test_db();
        let qid = db.record_query("test", Intent::General, 0, "", "").unwrap();
        db.record_feedback(qid, FeedbackRating::Positive, "Great answer!", "").unwrap();
        db.record_feedback(qid, FeedbackRating::Negative, "Too vague", "").unwrap();
        let stats = db.stats().unwrap();
        assert_eq!(stats.positive_feedback, 1);
        assert_eq!(stats.negative_feedback, 1);
    }
}
