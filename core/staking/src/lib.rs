use anyhow::Result;
use chrono::Utc;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct StakingState {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProviderRecord {
    pub provider_id: String,
    pub stake: f64,
    pub rewards: f64,
    pub slashed: f64,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventRecord {
    pub event_type: String,
    pub amount: f64,
    pub memo: Option<String>,
    pub ts: String,
}

impl StakingState {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let db_path = base_dir.join("staking.db");
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::new(manager)?;
        {
            let conn = pool.get()?;
            init_schema(&conn)?;
        }
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn init_schema(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS providers (
            provider_id TEXT PRIMARY KEY,
            stake REAL NOT NULL DEFAULT 0,
            rewards REAL NOT NULL DEFAULT 0,
            slashed REAL NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            amount REAL NOT NULL,
            memo TEXT,
            ts TEXT NOT NULL
        );
    "#,
    )?;
    Ok(())
}

// core DB helpers (not public)
fn log_event_db(
    conn: &rusqlite::Connection,
    pid: &str,
    event_type: &str,
    amount: f64,
    memo: Option<String>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO events (provider_id, event_type, amount, memo, ts) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![pid, event_type, amount, memo, now()],
    )?;
    Ok(())
}

fn load_provider_db(conn: &rusqlite::Connection, pid: &str) -> Result<Option<ProviderRecord>> {
    let mut stmt = conn.prepare(
        "SELECT provider_id, stake, rewards, slashed, updated_at FROM providers WHERE provider_id = ?1",
    )?;
    let rec = stmt
        .query_map([pid], |row| {
            Ok(ProviderRecord {
                provider_id: row.get(0)?,
                stake: row.get(1)?,
                rewards: row.get(2)?,
                slashed: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .next()
        .transpose()?;
    Ok(rec)
}

fn upsert_provider_db(conn: &rusqlite::Connection, pid: &str, stake: f64) -> Result<ProviderRecord> {
    let ts = now();
    conn.execute(
        "INSERT INTO providers (provider_id, stake, rewards, slashed, updated_at)
         VALUES (?1, ?2, 0, 0, ?3)
         ON CONFLICT(provider_id) DO UPDATE SET updated_at=excluded.updated_at",
        params![pid, stake, ts],
    )?;
    log_event_db(conn, pid, "register", stake, None)?;
    Ok(ProviderRecord {
        provider_id: pid.to_string(),
        stake,
        rewards: 0.0,
        slashed: 0.0,
        updated_at: ts,
    })
}

// Public API
impl StakingState {
    pub fn register(&self, pid: &str, stake: f64) -> Result<ProviderRecord> {
        let conn = self.conn()?;
        let rec = upsert_provider_db(&conn, pid, stake)?;
        Ok(rec)
    }

    pub fn get_provider(&self, pid: &str) -> Result<Option<ProviderRecord>> {
        let conn = self.conn()?;
        load_provider_db(&conn, pid)
    }

    pub fn stake(&self, pid: &str, amount: f64) -> Result<ProviderRecord> {
        if amount <= 0.0 {
            anyhow::bail!("amount must be positive");
        }
        let conn = self.conn()?;
        let mut rec = match load_provider_db(&conn, pid)? {
            Some(r) => r,
            None => anyhow::bail!("provider not registered"),
        };
        rec.stake += amount;
        conn.execute(
            "UPDATE providers SET stake=?1, rewards=?2, slashed=?3, updated_at=?4 WHERE provider_id=?5",
            params![rec.stake, rec.rewards, rec.slashed, now(), pid],
        )?;
        log_event_db(&conn, pid, "stake", amount, None)?;
        Ok(rec)
    }

    pub fn reward(&self, pid: &str, amount: f64) -> Result<ProviderRecord> {
        if amount <= 0.0 {
            anyhow::bail!("amount must be positive");
        }
        let conn = self.conn()?;
        let mut rec = match load_provider_db(&conn, pid)? {
            Some(r) => r,
            None => anyhow::bail!("provider not registered"),
        };
        rec.rewards += amount;
        conn.execute(
            "UPDATE providers SET stake=?1, rewards=?2, slashed=?3, updated_at=?4 WHERE provider_id=?5",
            params![rec.stake, rec.rewards, rec.slashed, now(), pid],
        )?;
        log_event_db(&conn, pid, "reward", amount, None)?;
        Ok(rec)
    }

    pub fn slash(&self, pid: &str, amount: f64, reason: Option<String>) -> Result<ProviderRecord> {
        if amount <= 0.0 {
            anyhow::bail!("amount must be positive");
        }
        let conn = self.conn()?;
        let mut rec = match load_provider_db(&conn, pid)? {
            Some(r) => r,
            None => anyhow::bail!("provider not registered"),
        };
        let mut remaining = amount;
        if rec.stake >= remaining {
            rec.stake -= remaining;
            remaining = 0.0;
        } else {
            remaining -= rec.stake;
            rec.stake = 0.0;
        }
        if remaining > 0.0 {
            if rec.rewards >= remaining {
                rec.rewards -= remaining;
                remaining = 0.0;
            }
        }
        if remaining > 0.0 {
            anyhow::bail!("insufficient funds to slash");
        }
        rec.slashed += amount;
        conn.execute(
            "UPDATE providers SET stake=?1, rewards=?2, slashed=?3, updated_at=?4 WHERE provider_id=?5",
            params![rec.stake, rec.rewards, rec.slashed, now(), pid],
        )?;
        log_event_db(&conn, pid, "slash", amount, reason)?;
        Ok(rec)
    }

    pub fn withdraw(&self, pid: &str, amount: f64) -> Result<ProviderRecord> {
        if amount <= 0.0 {
            anyhow::bail!("amount must be positive");
        }
        let conn = self.conn()?;
        let mut rec = match load_provider_db(&conn, pid)? {
            Some(r) => r,
            None => anyhow::bail!("provider not registered"),
        };
        if rec.rewards < amount {
            anyhow::bail!("insufficient rewards to withdraw");
        }
        rec.rewards -= amount;
        conn.execute(
            "UPDATE providers SET stake=?1, rewards=?2, slashed=?3, updated_at=?4 WHERE provider_id=?5",
            params![rec.stake, rec.rewards, rec.slashed, now(), pid],
        )?;
        log_event_db(&conn, pid, "withdraw", amount, None)?;
        Ok(rec)
    }

    pub fn get_events(&self, pid: &str) -> Result<Vec<EventRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT event_type, amount, memo, ts FROM events WHERE provider_id=?1 ORDER BY id DESC LIMIT 100",
        )?;
        let rows = stmt
            .query_map([pid], |row| {
                Ok(EventRecord {
                    event_type: row.get(0)?,
                    amount: row.get(1)?,
                    memo: row.get(2)?,
                    ts: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}
