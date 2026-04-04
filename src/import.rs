use crate::db::Database;
use crate::parser;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ImportTask {
    content: String,
    #[serde(default = "default_list")]
    list: String,
    #[serde(default)]
    priority: Option<u8>,
    #[serde(default)]
    due: Option<String>,
    #[serde(default)]
    ping: Option<String>,
    #[serde(default)]
    recurrence: Option<String>,
}

fn default_list() -> String {
    "Inbox".to_string()
}

pub fn import_file(path: &str) -> Result<usize, String> {
    let db = Database::new().map_err(|e| format!("Failed to open database: {}", e))?;
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let tasks = match ext.as_str() {
        "json" => read_json(path)?,
        "csv" => read_csv(path)?,
        _ => return Err(format!("Unsupported file format '.{}'. Use .json or .csv", ext)),
    };

    let mut count = 0;
    for task in &tasks {
        let list_id = db.find_or_create_list(&task.list);

        let due_at = task.due.as_ref().and_then(|d| {
            chrono::DateTime::parse_from_rfc3339(d)
                .or_else(|_| chrono::DateTime::parse_from_str(d, "%Y-%m-%dT%H:%M:%S"))
                .ok()
                .map(|dt| dt.timestamp_millis())
                .or_else(|| {
                    // Try natural language as fallback
                    let parsed = parser::parse_task_input(&format!("_ {}", d));
                    parsed.due_at
                })
        });

        let ping_interval = task.ping.as_ref().and_then(|p| parser::parse_ping_str(p));

        let priority = task.priority.filter(|&p| (1..=3).contains(&p));

        let recurrence = task.recurrence.as_deref()
            .filter(|r| matches!(*r, "daily" | "weekly" | "monthly"));

        db.create_task(&list_id, &task.content, due_at, ping_interval, priority, recurrence);
        count += 1;
    }

    Ok(count)
}

fn read_json(path: &str) -> Result<Vec<ImportTask>, String> {
    let data = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    serde_json::from_str(&data).map_err(|e| format!("Invalid JSON: {}", e))
}

fn read_csv(path: &str) -> Result<Vec<ImportTask>, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|e| format!("Failed to read CSV: {}", e))?;
    let mut tasks = Vec::new();
    for result in reader.deserialize() {
        let task: ImportTask = result.map_err(|e| format!("Invalid CSV row: {}", e))?;
        tasks.push(task);
    }
    Ok(tasks)
}
