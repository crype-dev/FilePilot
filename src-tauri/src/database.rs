use crate::model::*;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database {
    pub conn: Connection,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 {
            return Err(invalid("Database version is newer than this application"));
        }
        conn.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS objects(kind TEXT NOT NULL, id TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(kind,id)); CREATE TABLE IF NOT EXISTS history(id TEXT PRIMARY KEY, time INTEGER NOT NULL, status TEXT NOT NULL, data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS suppression(path TEXT PRIMARY KEY, data TEXT NOT NULL); PRAGMA user_version=1; COMMIT;")?;
        Ok(Self { conn })
    }
    pub fn put<T: serde::Serialize>(&self, kind: &str, id: &str, item: &T) -> Result<()> {
        self.conn.execute("INSERT INTO objects VALUES(?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET data=excluded.data", params![kind,id,serde_json::to_string(item)?])?;
        Ok(())
    }
    pub fn list<T: serde::de::DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>> {
        let mut query = self
            .conn
            .prepare("SELECT data FROM objects WHERE kind=?1 ORDER BY id")?;
        let rows = query.query_map([kind], |r| r.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
    pub fn delete(&self, kind: &str, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM objects WHERE kind=?1 AND id=?2",
            params![kind, id],
        )?;
        Ok(())
    }
    pub fn settings(&self) -> Result<Settings> {
        Ok(self
            .list::<Settings>("settings")?
            .into_iter()
            .next()
            .unwrap_or_default())
    }
    pub fn record(&self, op: &Operation) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("INSERT INTO history VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET status=excluded.status,data=excluded.data", params![op.id,op.time as i64,op.status,serde_json::to_string(op)?])?;
        if ["completed", "undone"].contains(&op.status.as_str()) {
            let path = if op.status == "undone" {
                &op.source
            } else {
                &op.destination
            };
            tx.execute("INSERT INTO suppression VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET data=excluded.data", params![path.to_lowercase(),serde_json::to_string(&(op.size,op.modified))?])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn history(&self) -> Result<Vec<Operation>> {
        let mut query = self
            .conn
            .prepare("SELECT data FROM history ORDER BY time DESC, rowid DESC LIMIT 10000")?;
        let rows = query.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
    pub fn suppresses(&self, path: &str, size: u64, modified: u64) -> Result<bool> {
        use rusqlite::OptionalExtension;
        let data = self
            .conn
            .query_row(
                "SELECT data FROM suppression WHERE path=?1",
                [path.to_lowercase()],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        Ok(data
            .map(|s| serde_json::from_str::<(u64, u64)>(&s))
            .transpose()?
            == Some((size, modified)))
    }
    pub fn recover(&self) -> Result<bool> {
        let mut pending = self
            .conn
            .prepare("SELECT data FROM history WHERE status IN ('prepared','copying','undoing')")?;
        let ops: Vec<String> = pending
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let interrupted = !ops.is_empty();
        for data in ops {
            let mut op: Operation = serde_json::from_str(&data)?;
            op.status = "reconcile".into();
            op.error = Some(
                "Interrupted operation: inspect both paths. No file was removed during recovery."
                    .into(),
            );
            self.record(&op)?;
        }
        let ambiguous: i64 = self.conn.query_row(
            "SELECT count(*) FROM history WHERE status='reconcile'",
            [],
            |r| r.get(0),
        )?;
        Ok(interrupted || ambiguous > 0)
    }
    pub fn purge_history(&self, before: u64) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM history WHERE time < ?1 AND status IN ('completed','failed','undone','skipped')",[before as i64])?)
    }
}
