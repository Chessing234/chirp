use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct List {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub content: String,
    pub completed: bool,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>,
    pub last_ping_at: Option<i64>,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Chirp");
        std::fs::create_dir_all(&data_dir).ok();

        let db_path = data_dir.join("chirp.db");
        let conn = Connection::open(db_path)?;

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

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

    // Lists
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

        List {
            id,
            name: name.to_string(),
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
        self.conn
            .execute("DELETE FROM tasks WHERE list_id = ?1", [id])
            .ok();
        self.conn
            .execute("DELETE FROM lists WHERE id = ?1", [id])
            .ok();
    }

    // Tasks
    pub fn get_tasks_by_list(&self, list_id: &str) -> Vec<Task> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, content, completed, due_at, ping_interval, last_ping_at
                 FROM tasks WHERE list_id = ?1 ORDER BY completed ASC, created_at DESC",
            )
            .unwrap();

        stmt.query_map([list_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                content: row.get(1)?,
                completed: row.get::<_, i32>(2)? != 0,
                due_at: row.get(3)?,
                ping_interval: row.get(4)?,
                last_ping_at: row.get(5)?,
            })
        })
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
    ) -> Task {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        self.conn
            .execute(
                "INSERT INTO tasks (id, list_id, content, completed, due_at, ping_interval, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7)",
                params![id, list_id, content, due_at, ping_interval, now, now],
            )
            .unwrap();

        Task {
            id,
            content: content.to_string(),
            completed: false,
            due_at,
            ping_interval,
            last_ping_at: None,
        }
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
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", [id])
            .ok();
    }

    /// Get all incomplete tasks that have a ping_interval set (across all lists).
    pub fn get_pingable_tasks(&self) -> Vec<Task> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, content, completed, due_at, ping_interval, last_ping_at
                 FROM tasks WHERE completed = 0 AND ping_interval IS NOT NULL",
            )
            .unwrap();

        stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                content: row.get(1)?,
                completed: row.get::<_, i32>(2)? != 0,
                due_at: row.get(3)?,
                ping_interval: row.get(4)?,
                last_ping_at: row.get(5)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn update_last_ping_at(&self, id: &str, timestamp: i64) {
        self.conn
            .execute(
                "UPDATE tasks SET last_ping_at = ?1 WHERE id = ?2",
                params![timestamp, id],
            )
            .ok();
    }
}
