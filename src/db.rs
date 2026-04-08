use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use uuid::Uuid;

pub fn data_dir() -> PathBuf {
    // macOS: ~/Library/Application Support/Chirp
    // Linux: ~/.local/share/Chirp (or $XDG_DATA_HOME/Chirp)
    // Windows: %APPDATA%\Chirp
    #[cfg(target_os = "windows")]
    let base = dirs::config_dir(); // %APPDATA% on Windows

    #[cfg(not(target_os = "windows"))]
    let base = dirs::data_dir();

    base.unwrap_or_else(|| PathBuf::from(".")).join("Chirp")
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
    pub list_id: String,
    pub content: String,
    pub completed: bool,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>,
    pub last_ping_at: Option<i64>,
    pub priority: Option<u8>,
    pub recurrence: Option<String>,
    pub sort_order: i64,
    pub note: Option<String>,
}

impl Database {
    fn init(conn: Connection) -> Result<Self, rusqlite::Error> {
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS lists (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, color TEXT DEFAULT '#4a9f6e',
                reminder TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
            )", [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY, list_id TEXT NOT NULL, content TEXT NOT NULL,
                completed INTEGER DEFAULT 0, due_at INTEGER, ping_interval INTEGER,
                last_ping_at INTEGER, parent_id TEXT,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
                FOREIGN KEY (list_id) REFERENCES lists(id) ON DELETE CASCADE
            )", [],
        )?;

        // Migrations (safe to re-run)
        conn.execute("ALTER TABLE tasks ADD COLUMN priority INTEGER", []).ok();
        conn.execute("ALTER TABLE tasks ADD COLUMN recurrence TEXT", []).ok();
        conn.execute("ALTER TABLE tasks ADD COLUMN sort_order INTEGER", []).ok();
        conn.execute("ALTER TABLE tasks ADD COLUMN note TEXT", []).ok();
        conn.execute("UPDATE tasks SET sort_order = -(created_at / 1000) WHERE sort_order IS NULL", []).ok();

        let db = Self { conn };
        db.ensure_default_list();
        Ok(db)
    }

    pub fn new() -> Result<Self, rusqlite::Error> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir).ok();
        Self::init(Connection::open(dir.join("chirp.db"))?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        Self::init(Connection::open_in_memory()?)
    }

    fn ensure_default_list(&self) {
        let count: i32 = self.conn
            .query_row("SELECT COUNT(*) FROM lists", [], |row| row.get(0))
            .unwrap_or(0);
        if count == 0 {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().timestamp_millis();
            self.conn.execute(
                "INSERT INTO lists (id, name, color, created_at, updated_at) VALUES (?1, 'Inbox', '#4a9f6e', ?2, ?3)",
                params![id, now, now],
            ).ok();
        }
    }

    const TASK_COLS: &'static str =
        "id, list_id, content, completed, due_at, ping_interval, last_ping_at, priority, recurrence, sort_order, note";

    fn read_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            list_id: row.get(1)?,
            content: row.get(2)?,
            completed: row.get::<_, i32>(3)? != 0,
            due_at: row.get(4)?,
            ping_interval: row.get(5)?,
            last_ping_at: row.get(6)?,
            priority: row.get::<_, Option<i32>>(7)?.map(|p| p as u8),
            recurrence: row.get(8)?,
            sort_order: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
            note: row.get(10)?,
        })
    }

    // === Lists ===

    pub fn get_all_lists(&self) -> Vec<List> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM lists ORDER BY created_at").unwrap();
        stmt.query_map([], |row| Ok(List { id: row.get(0)?, name: row.get(1)? }))
            .unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn create_list(&self, name: &str) -> List {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        self.conn.execute(
            "INSERT INTO lists (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, now, now],
        ).unwrap();
        List { id, name: name.to_string() }
    }

    pub fn find_or_create_list(&self, name: &str) -> String {
        if let Ok(id) = self.conn.query_row(
            "SELECT id FROM lists WHERE name = ?1 COLLATE NOCASE", [name], |row| row.get::<_, String>(0),
        ) { id } else { self.create_list(name).id }
    }

    pub fn rename_list(&self, id: &str, name: &str) {
        let now = Utc::now().timestamp_millis();
        self.conn.execute("UPDATE lists SET name=?1, updated_at=?2 WHERE id=?3", params![name, now, id]).ok();
    }

    pub fn delete_list(&self, id: &str) {
        self.conn.execute("DELETE FROM tasks WHERE list_id=?1", [id]).ok();
        self.conn.execute("DELETE FROM lists WHERE id=?1", [id]).ok();
    }

    // === Tasks ===

    fn next_sort_order(&self, list_id: &str) -> i64 {
        self.conn.query_row(
            "SELECT COALESCE(MIN(sort_order),0)-1 FROM tasks WHERE list_id=?1 AND completed=0",
            [list_id], |row| row.get(0),
        ).unwrap_or(-1)
    }

    pub fn get_all_tasks(&self) -> Vec<Task> {
        let sql = format!(
            "SELECT {} FROM tasks ORDER BY list_id, completed ASC, COALESCE(priority,4) ASC, sort_order ASC",
            Self::TASK_COLS
        );
        let mut stmt = self.conn.prepare(&sql).unwrap();
        stmt.query_map([], Self::read_task).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_tasks_by_list(&self, list_id: &str) -> Vec<Task> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE list_id=?1 ORDER BY completed ASC, COALESCE(priority,4) ASC, sort_order ASC",
            Self::TASK_COLS
        );
        let mut stmt = self.conn.prepare(&sql).unwrap();
        stmt.query_map([list_id], Self::read_task).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_today_tasks(&self, end_of_today_ms: i64) -> Vec<Task> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE due_at IS NOT NULL AND due_at <= ?1 ORDER BY completed ASC, COALESCE(priority,4) ASC, due_at ASC",
            Self::TASK_COLS
        );
        let mut stmt = self.conn.prepare(&sql).unwrap();
        stmt.query_map([end_of_today_ms], Self::read_task).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn count_due_before(&self, end_ms: i64) -> usize {
        self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE completed=0 AND due_at IS NOT NULL AND due_at <= ?1",
            [end_ms], |r| r.get(0),
        ).unwrap_or(0)
    }

    pub fn next_upcoming_task(&self, now_ms: i64) -> Option<(String, i64)> {
        self.conn.query_row(
            "SELECT content, due_at FROM tasks WHERE completed=0 AND due_at IS NOT NULL AND due_at >= ?1 ORDER BY due_at ASC LIMIT 1",
            [now_ms], |r| Ok((r.get(0)?, r.get(1)?)),
        ).ok()
    }

    pub fn create_task(
        &self, list_id: &str, content: &str, due_at: Option<i64>,
        ping_interval: Option<i64>, priority: Option<u8>, recurrence: Option<&str>,
    ) -> Task {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        let sort_order = self.next_sort_order(list_id);
        self.conn.execute(
            "INSERT INTO tasks (id, list_id, content, completed, due_at, ping_interval, priority, recurrence, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, list_id, content, due_at, ping_interval, priority.map(|p| p as i32), recurrence, sort_order, now, now],
        ).unwrap();
        Task {
            id, list_id: list_id.to_string(), content: content.to_string(),
            completed: false, due_at, ping_interval, last_ping_at: None,
            priority, recurrence: recurrence.map(|s| s.to_string()), sort_order, note: None,
        }
    }

    pub fn update_task(
        &self, id: &str, content: &str, due_at: Option<i64>,
        ping_interval: Option<i64>, priority: Option<u8>, recurrence: Option<&str>,
    ) {
        let now = Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE tasks SET content=?1, due_at=?2, ping_interval=?3, priority=?4, recurrence=?5, last_ping_at=NULL, updated_at=?6 WHERE id=?7",
            params![content, due_at, ping_interval, priority.map(|p| p as i32), recurrence, now, id],
        ).ok();
    }

    pub fn update_task_note(&self, id: &str, note: Option<&str>) {
        let now = Utc::now().timestamp_millis();
        self.conn.execute("UPDATE tasks SET note=?1, updated_at=?2 WHERE id=?3", params![note, now, id]).ok();
    }

    pub fn toggle_task(&self, id: &str) {
        let now = Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE tasks SET completed = CASE WHEN completed=0 THEN 1 ELSE 0 END, updated_at=?1 WHERE id=?2",
            params![now, id],
        ).ok();
    }

    pub fn delete_task(&self, id: &str) {
        self.conn.execute("DELETE FROM tasks WHERE id=?1", [id]).ok();
    }

    pub fn swap_sort_order(&self, id_a: &str, id_b: &str) {
        let get = |id: &str| -> i64 {
            self.conn.query_row("SELECT COALESCE(sort_order,0) FROM tasks WHERE id=?1", [id], |r| r.get(0)).unwrap_or(0)
        };
        let (a, b) = (get(id_a), get(id_b));
        self.conn.execute("UPDATE tasks SET sort_order=?1 WHERE id=?2", params![b, id_a]).ok();
        self.conn.execute("UPDATE tasks SET sort_order=?1 WHERE id=?2", params![a, id_b]).ok();
    }

    pub fn snooze_task(&self, id: &str) {
        let now = Utc::now().timestamp_millis();
        self.conn.execute("UPDATE tasks SET last_ping_at=?1 WHERE id=?2", params![now, id]).ok();
    }

    pub fn get_pingable_tasks(&self) -> Vec<Task> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE completed=0 AND ping_interval IS NOT NULL", Self::TASK_COLS
        );
        let mut stmt = self.conn.prepare(&sql).unwrap();
        stmt.query_map([], Self::read_task).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn update_last_ping_at(&self, id: &str, timestamp: i64) {
        self.conn.execute("UPDATE tasks SET last_ping_at=?1 WHERE id=?2", params![timestamp, id]).ok();
    }

    pub fn get_task_list_id(&self, task_id: &str) -> Option<String> {
        self.conn.query_row("SELECT list_id FROM tasks WHERE id=?1", [task_id], |r| r.get(0)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn test_default_list_created() {
        let db = test_db();
        let lists = db.get_all_lists();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "Inbox");
    }

    #[test]
    fn test_full_lifecycle() {
        let db = test_db();

        // Create list
        let list = db.create_list("Work");
        let lists = db.get_all_lists();
        assert_eq!(lists.len(), 2);

        // Create task with all features
        let task = db.create_task(&list.id, "standup", Some(1000), Some(30), Some(1), Some("daily"));
        assert_eq!(task.content, "standup");
        assert_eq!(task.priority, Some(1));
        assert_eq!(task.recurrence.as_deref(), Some("daily"));
        assert_eq!(task.ping_interval, Some(30));

        // Verify persisted
        let tasks = db.get_tasks_by_list(&list.id);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].list_id, list.id);

        // Edit task
        db.update_task(&task.id, "daily standup", Some(2000), Some(60), Some(2), Some("weekly"));
        let tasks = db.get_tasks_by_list(&list.id);
        assert_eq!(tasks[0].content, "daily standup");
        assert_eq!(tasks[0].priority, Some(2));
        assert_eq!(tasks[0].ping_interval, Some(60));
        assert_eq!(tasks[0].recurrence.as_deref(), Some("weekly"));

        // Reorder: create second task and swap
        let task2 = db.create_task(&list.id, "review", None, None, Some(2), None);
        let before = db.get_tasks_by_list(&list.id);
        db.swap_sort_order(&task.id, &task2.id);
        let after = db.get_tasks_by_list(&list.id);
        assert_ne!(before[0].id, after[0].id);

        // Toggle complete
        db.toggle_task(&task.id);
        let tasks = db.get_tasks_by_list(&list.id);
        let t = tasks.iter().find(|t| t.id == task.id).unwrap();
        assert!(t.completed);

        // Toggle back
        db.toggle_task(&task.id);
        let tasks = db.get_tasks_by_list(&list.id);
        let t = tasks.iter().find(|t| t.id == task.id).unwrap();
        assert!(!t.completed);

        // Snooze
        db.snooze_task(&task2.id);
        let tasks = db.get_tasks_by_list(&list.id);
        let t = tasks.iter().find(|t| t.id == task2.id).unwrap();
        assert!(t.last_ping_at.is_some());

        // Delete
        db.delete_task(&task2.id);
        let tasks = db.get_tasks_by_list(&list.id);
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_notes() {
        let db = test_db();
        let lists = db.get_all_lists();
        let task = db.create_task(&lists[0].id, "test", None, None, None, None);
        assert_eq!(task.note, None);

        db.update_task_note(&task.id, Some("my note"));
        let tasks = db.get_tasks_by_list(&lists[0].id);
        assert_eq!(tasks[0].note.as_deref(), Some("my note"));

        db.update_task_note(&task.id, None);
        let tasks = db.get_tasks_by_list(&lists[0].id);
        assert_eq!(tasks[0].note, None);
    }

    #[test]
    fn test_today_tasks() {
        let db = test_db();
        let lists = db.get_all_lists();
        let now = Utc::now().timestamp_millis();

        db.create_task(&lists[0].id, "overdue", Some(now - 86_400_000), None, None, None);
        db.create_task(&lists[0].id, "future", Some(now + 86_400_000 * 365), None, None, None);
        db.create_task(&lists[0].id, "no due", None, None, None, None);

        let today_tasks = db.get_today_tasks(now + 86_400_000);
        assert_eq!(today_tasks.len(), 1);
        assert_eq!(today_tasks[0].content, "overdue");
    }

    #[test]
    fn test_agenda_queries() {
        let db = test_db();
        let lists = db.get_all_lists();
        let now = Utc::now().timestamp_millis();

        db.create_task(&lists[0].id, "past", Some(now - 3600_000), None, None, None);
        db.create_task(&lists[0].id, "soon", Some(now + 3600_000), None, None, None);
        db.create_task(&lists[0].id, "later", Some(now + 7200_000), None, None, None);

        let count = db.count_due_before(now + 86_400_000);
        assert_eq!(count, 3);

        let next = db.next_upcoming_task(now);
        assert!(next.is_some());
        assert_eq!(next.unwrap().0, "soon");
    }

    #[test]
    fn test_find_or_create_list() {
        let db = test_db();
        let id1 = db.find_or_create_list("Inbox");
        let id2 = db.find_or_create_list("inbox");
        assert_eq!(id1, id2);

        let id3 = db.find_or_create_list("Work");
        assert_ne!(id1, id3);
        assert_eq!(db.get_all_lists().len(), 2);
    }

    #[test]
    fn test_delete_list_cascades() {
        let db = test_db();
        let list = db.create_list("Temp");
        db.create_task(&list.id, "task1", None, None, None, None);
        db.create_task(&list.id, "task2", None, None, None, None);
        assert_eq!(db.get_tasks_by_list(&list.id).len(), 2);

        db.delete_list(&list.id);
        assert_eq!(db.get_tasks_by_list(&list.id).len(), 0);
        assert_eq!(db.get_all_lists().len(), 1); // only Inbox
    }

    #[test]
    fn test_priority_sorting() {
        let db = test_db();
        let lists = db.get_all_lists();
        let lid = &lists[0].id;

        db.create_task(lid, "low", None, None, Some(3), None);
        db.create_task(lid, "high", None, None, Some(1), None);
        db.create_task(lid, "none", None, None, None, None);
        db.create_task(lid, "medium", None, None, Some(2), None);

        let tasks = db.get_tasks_by_list(lid);
        assert_eq!(tasks[0].content, "high");
        assert_eq!(tasks[1].content, "medium");
        assert_eq!(tasks[2].content, "low");
        assert_eq!(tasks[3].content, "none");
    }
}
