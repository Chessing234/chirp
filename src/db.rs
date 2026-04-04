use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use uuid::Uuid;

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Chirp")
}

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct List {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Task {
    pub id: String,
    pub content: String,
    pub completed: bool,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>,
    pub last_ping_at: Option<i64>,
    pub priority: Option<u8>,
    pub recurrence: Option<String>,
    pub sort_order: i64,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir).ok();

        let db_path = dir.join("chirp.db");
        let conn = Connection::open(db_path)?;

        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS lists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT DEFAULT '#4a9f6e',
                reminder TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                list_id TEXT NOT NULL,
                content TEXT NOT NULL,
                completed INTEGER DEFAULT 0,
                due_at INTEGER,
                ping_interval INTEGER,
                last_ping_at INTEGER,
                parent_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (list_id) REFERENCES lists(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Migrations for new columns (safe to re-run)
        conn.execute("ALTER TABLE tasks ADD COLUMN priority INTEGER", []).ok();
        conn.execute("ALTER TABLE tasks ADD COLUMN recurrence TEXT", []).ok();
        conn.execute("ALTER TABLE tasks ADD COLUMN sort_order INTEGER", []).ok();
        conn.execute(
            "UPDATE tasks SET sort_order = -(created_at / 1000) WHERE sort_order IS NULL",
            [],
        ).ok();

        let db = Self { conn };
        db.ensure_default_list();
        Ok(db)
    }

    fn ensure_default_list(&self) {
        let count: i32 = self
            .conn
            .query_row("SELECT COUNT(*) FROM lists", [], |row| row.get(0))
            .unwrap_or(0);

        if count == 0 {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().timestamp_millis();
            self.conn
                .execute(
                    "INSERT INTO lists (id, name, color, created_at, updated_at) VALUES (?1, 'Inbox', '#4a9f6e', ?2, ?3)",
                    params![id, now, now],
                )
                .ok();
        }
    }

    fn read_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            content: row.get(1)?,
            completed: row.get::<_, i32>(2)? != 0,
            due_at: row.get(3)?,
            ping_interval: row.get(4)?,
            last_ping_at: row.get(5)?,
            priority: row.get::<_, Option<i32>>(6)?.map(|p| p as u8),
            recurrence: row.get(7)?,
            sort_order: row.get::<_, Option<i64>>(8)?.unwrap_or(0),
        })
    }

    const TASK_COLS: &'static str =
        "id, content, completed, due_at, ping_interval, last_ping_at, priority, recurrence, sort_order";

    // === Lists ===

    pub fn get_all_lists(&self) -> Vec<List> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM lists ORDER BY created_at")
            .unwrap();

        stmt.query_map([], |row| {
            Ok(List {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn create_list(&self, name: &str) -> List {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        self.conn
            .execute(
                "INSERT INTO lists (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, now, now],
            )
            .unwrap();

        List { id, name: name.to_string() }
    }

    pub fn find_or_create_list(&self, name: &str) -> String {
        if let Ok(id) = self.conn.query_row(
            "SELECT id FROM lists WHERE name = ?1 COLLATE NOCASE",
            [name],
            |row| row.get::<_, String>(0),
        ) {
            id
        } else {
            self.create_list(name).id
        }
    }

    pub fn rename_list(&self, id: &str, name: &str) {
        let now = Utc::now().timestamp_millis();
        self.conn
            .execute(
                "UPDATE lists SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now, id],
            )
            .ok();
    }

    pub fn delete_list(&self, id: &str) {
        self.conn.execute("DELETE FROM tasks WHERE list_id = ?1", [id]).ok();
        self.conn.execute("DELETE FROM lists WHERE id = ?1", [id]).ok();
    }

    // === Tasks ===

    fn next_sort_order(&self, list_id: &str) -> i64 {
        self.conn
            .query_row(
                "SELECT COALESCE(MIN(sort_order), 0) - 1 FROM tasks WHERE list_id = ?1 AND completed = 0",
                [list_id],
                |row| row.get(0),
            )
            .unwrap_or(-1)
    }

    pub fn get_tasks_by_list(&self, list_id: &str) -> Vec<Task> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE list_id = ?1 ORDER BY completed ASC, COALESCE(priority, 4) ASC, sort_order ASC",
            Self::TASK_COLS
        );
        let mut stmt = self.conn.prepare(&sql).unwrap();

        stmt.query_map([list_id], Self::read_task)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    pub fn create_task(
        &self,
        list_id: &str,
        content: &str,
        due_at: Option<i64>,
        ping_interval: Option<i64>,
        priority: Option<u8>,
        recurrence: Option<&str>,
    ) -> Task {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        let sort_order = self.next_sort_order(list_id);
        self.conn
            .execute(
                "INSERT INTO tasks (id, list_id, content, completed, due_at, ping_interval, priority, recurrence, sort_order, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![id, list_id, content, due_at, ping_interval,
                        priority.map(|p| p as i32), recurrence, sort_order, now, now],
            )
            .unwrap();

        Task {
            id,
            content: content.to_string(),
            completed: false,
            due_at,
            ping_interval,
            last_ping_at: None,
            priority,
            recurrence: recurrence.map(|s| s.to_string()),
            sort_order,
        }
    }

    pub fn update_task(
        &self,
        id: &str,
        content: &str,
        due_at: Option<i64>,
        ping_interval: Option<i64>,
        priority: Option<u8>,
        recurrence: Option<&str>,
    ) {
        let now = Utc::now().timestamp_millis();
        self.conn
            .execute(
                "UPDATE tasks SET content=?1, due_at=?2, ping_interval=?3, priority=?4, recurrence=?5, last_ping_at=NULL, updated_at=?6 WHERE id=?7",
                params![content, due_at, ping_interval, priority.map(|p| p as i32), recurrence, now, id],
            )
            .ok();
    }

    pub fn toggle_task(&self, id: &str) {
        let now = Utc::now().timestamp_millis();
        self.conn
            .execute(
                "UPDATE tasks SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END, updated_at = ?1 WHERE id = ?2",
                params![now, id],
            )
            .ok();
    }

    pub fn delete_task(&self, id: &str) {
        self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id]).ok();
    }

    pub fn swap_sort_order(&self, id_a: &str, id_b: &str) {
        let get = |id: &str| -> i64 {
            self.conn
                .query_row("SELECT COALESCE(sort_order,0) FROM tasks WHERE id=?1", [id], |r| r.get(0))
                .unwrap_or(0)
        };
        let (a, b) = (get(id_a), get(id_b));
        self.conn.execute("UPDATE tasks SET sort_order=?1 WHERE id=?2", params![b, id_a]).ok();
        self.conn.execute("UPDATE tasks SET sort_order=?1 WHERE id=?2", params![a, id_b]).ok();
    }

    pub fn snooze_task(&self, id: &str) {
        let now = Utc::now().timestamp_millis();
        self.conn
            .execute("UPDATE tasks SET last_ping_at=?1 WHERE id=?2", params![now, id])
            .ok();
    }

    pub fn get_pingable_tasks(&self) -> Vec<Task> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE completed=0 AND ping_interval IS NOT NULL",
            Self::TASK_COLS
        );
        let mut stmt = self.conn.prepare(&sql).unwrap();

        stmt.query_map([], Self::read_task)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    pub fn update_last_ping_at(&self, id: &str, timestamp: i64) {
        self.conn
            .execute("UPDATE tasks SET last_ping_at=?1 WHERE id=?2", params![timestamp, id])
            .ok();
    }

    pub fn get_task_list_id(&self, task_id: &str) -> Option<String> {
        self.conn
            .query_row("SELECT list_id FROM tasks WHERE id=?1", [task_id], |r| r.get(0))
            .ok()
    }
}
