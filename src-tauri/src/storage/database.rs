use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::clipboard::item::ClipboardItem;
use crate::slots::SlotInfo;

const DEFAULT_HISTORY_LIMIT: u32 = 500;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(data_dir: PathBuf) -> SqliteResult<Self> {
        std::fs::create_dir_all(&data_dir).ok();
        let db_path = data_dir.join("clipslot.db");
        println!("[ClipSlot] Database: {}", db_path.display());

        let conn = Connection::open(&db_path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS clipboard_items (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                content_type TEXT NOT NULL DEFAULT 'text/plain',
                source_app TEXT,
                device_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                is_promoted INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_created_at ON clipboard_items(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_content_hash ON clipboard_items(content_hash);
            CREATE INDEX IF NOT EXISTS idx_is_promoted ON clipboard_items(is_promoted);

            CREATE TABLE IF NOT EXISTS app_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS slots (
                slot_number INTEGER PRIMARY KEY,
                item_id TEXT REFERENCES clipboard_items(id),
                name TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            ",
        )?;

        // Set default history limit if not present
        conn.execute(
            "INSERT OR IGNORE INTO app_config (key, value) VALUES ('history_limit', ?1)",
            params![DEFAULT_HISTORY_LIMIT.to_string()],
        )?;

        // Pre-populate 5 empty slots
        for i in 1..=5 {
            conn.execute(
                "INSERT OR IGNORE INTO slots (slot_number, name, updated_at) VALUES (?1, ?2, 0)",
                params![i, format!("Slot {}", i)],
            )?;
        }

        println!("[ClipSlot] Database migrations complete");
        Ok(())
    }

    /// Insert a clipboard item, skipping if the same content was captured in the last 2 seconds.
    /// Returns true if inserted, false if skipped as duplicate.
    pub fn insert_item(&self, item: &ClipboardItem) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();

        // Check for recent duplicate (same hash within last 2 seconds)
        let cutoff = item.created_at - 2000;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM clipboard_items WHERE content_hash = ?1 AND created_at > ?2)",
            params![item.content_hash, cutoff],
            |row| row.get(0),
        )?;

        if exists {
            return Ok(false);
        }

        conn.execute(
            "INSERT OR REPLACE INTO clipboard_items
             (id, content, content_hash, content_type, source_app, device_id, created_at, is_promoted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                item.id,
                item.content,
                item.content_hash,
                item.content_type,
                item.source_app,
                item.device_id,
                item.created_at,
                item.is_promoted as i32,
            ],
        )?;
        Ok(true)
    }

    pub fn get_history(&self, limit: u32, offset: u32) -> SqliteResult<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, content_hash, content_type, source_app, device_id, created_at, is_promoted
             FROM clipboard_items
             WHERE is_promoted = 0
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let items = stmt
            .query_map(params![limit, offset], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    content_hash: row.get(2)?,
                    content_type: row.get(3)?,
                    source_app: row.get(4)?,
                    device_id: row.get(5)?,
                    created_at: row.get(6)?,
                    is_promoted: row.get::<_, i32>(7)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    pub fn search(&self, query: &str) -> SqliteResult<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, content, content_hash, content_type, source_app, device_id, created_at, is_promoted
             FROM clipboard_items
             WHERE content LIKE ?1 AND is_promoted = 0
             ORDER BY created_at DESC
             LIMIT 100",
        )?;

        let items = stmt
            .query_map(params![pattern], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    content_hash: row.get(2)?,
                    content_type: row.get(3)?,
                    source_app: row.get(4)?,
                    device_id: row.get(5)?,
                    created_at: row.get(6)?,
                    is_promoted: row.get::<_, i32>(7)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    pub fn delete_item(&self, id: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn clear_history(&self) -> SqliteResult<u32> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "DELETE FROM clipboard_items WHERE is_promoted = 0",
            [],
        )?;
        Ok(rows as u32)
    }

    pub fn get_count(&self) -> SqliteResult<u32> {
        let conn = self.conn.lock().unwrap();
        let count: u32 =
            conn.query_row("SELECT COUNT(*) FROM clipboard_items WHERE is_promoted = 0", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn get_history_limit(&self) -> u32 {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM app_config WHERE key = 'history_limit'",
            [],
            |row| {
                let val: String = row.get(0)?;
                Ok(val.parse::<u32>().unwrap_or(DEFAULT_HISTORY_LIMIT))
            },
        )
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
    }

    // ── Slot Operations ──────────────────────────────────────────────────

    /// Save clipboard content to a slot. Creates a ClipboardItem if needed,
    /// marks it as promoted, and updates the slot to point to it.
    pub fn save_to_slot(&self, slot_number: u32, item: &ClipboardItem) -> SqliteResult<SlotInfo> {
        let conn = self.conn.lock().unwrap();

        // Insert or update the clipboard item (mark as promoted)
        conn.execute(
            "INSERT OR REPLACE INTO clipboard_items
             (id, content, content_hash, content_type, source_app, device_id, created_at, is_promoted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            params![
                item.id,
                item.content,
                item.content_hash,
                item.content_type,
                item.source_app,
                item.device_id,
                item.created_at,
            ],
        )?;

        // Update the slot
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE slots SET item_id = ?1, updated_at = ?2 WHERE slot_number = ?3",
            params![item.id, now, slot_number],
        )?;

        let name: String = conn.query_row(
            "SELECT name FROM slots WHERE slot_number = ?1",
            params![slot_number],
            |row| row.get(0),
        )?;

        let preview = if item.content.len() > 100 {
            Some(format!("{}...", &item.content[..100]))
        } else {
            Some(item.content.clone())
        };

        Ok(SlotInfo {
            slot_number,
            name,
            content: Some(item.content.clone()),
            content_preview: preview,
            updated_at: now,
            is_empty: false,
        })
    }

    pub fn get_slot(&self, slot_number: u32) -> SqliteResult<SlotInfo> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT s.slot_number, s.name, s.updated_at, c.content
             FROM slots s
             LEFT JOIN clipboard_items c ON s.item_id = c.id
             WHERE s.slot_number = ?1",
            params![slot_number],
            |row| {
                let content: Option<String> = row.get(3)?;
                let preview = content.as_ref().map(|c| {
                    if c.len() > 100 {
                        format!("{}...", &c[..100])
                    } else {
                        c.clone()
                    }
                });
                Ok(SlotInfo {
                    slot_number: row.get(0)?,
                    name: row.get(1)?,
                    content: content.clone(),
                    content_preview: preview,
                    updated_at: row.get(2)?,
                    is_empty: content.is_none(),
                })
            },
        )
    }

    pub fn get_all_slots(&self) -> SqliteResult<Vec<SlotInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.slot_number, s.name, s.updated_at, c.content
             FROM slots s
             LEFT JOIN clipboard_items c ON s.item_id = c.id
             ORDER BY s.slot_number ASC",
        )?;

        let slots = stmt
            .query_map([], |row| {
                let content: Option<String> = row.get(3)?;
                let preview = content.as_ref().map(|c| {
                    if c.len() > 100 {
                        format!("{}...", &c[..100])
                    } else {
                        c.clone()
                    }
                });
                Ok(SlotInfo {
                    slot_number: row.get(0)?,
                    name: row.get(1)?,
                    content: content.clone(),
                    content_preview: preview,
                    updated_at: row.get(2)?,
                    is_empty: content.is_none(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(slots)
    }

    pub fn clear_slot(&self, slot_number: u32) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE slots SET item_id = NULL, updated_at = 0 WHERE slot_number = ?1",
            params![slot_number],
        )?;
        Ok(rows > 0)
    }

    pub fn rename_slot(&self, slot_number: u32, name: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE slots SET name = ?1 WHERE slot_number = ?2",
            params![name, slot_number],
        )?;
        Ok(rows > 0)
    }

    // ── History Limit ───────────────────────────────────────────────────

    pub fn enforce_history_limit(&self) -> SqliteResult<u32> {
        let limit = self.get_history_limit();
        let count = self.get_count()?;

        if count <= limit {
            return Ok(0);
        }

        let excess = count - limit;
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "DELETE FROM clipboard_items WHERE id IN (
                SELECT id FROM clipboard_items
                WHERE is_promoted = 0
                ORDER BY created_at ASC
                LIMIT ?1
            )",
            params![excess],
        )?;

        if rows > 0 {
            println!("[ClipSlot] Expired {} old items (limit: {})", rows, limit);
        }

        Ok(rows as u32)
    }
}
