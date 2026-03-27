use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{
    AppHandle, Manager, State, WebviewWindow, WindowEvent,
    menu::{Menu, MenuItem, MenuItemKind, Submenu, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_autostart::MacosLauncher;
use uuid::Uuid;

// Database wrapper
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    fn new() -> Result<Self, rusqlite::Error> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("PingPal");
        std::fs::create_dir_all(&data_dir).ok();

        let db_path = data_dir.join("pingpal.db");
        let conn = Connection::open(db_path)?;

        // Create tables
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

        Ok(Self { conn: Mutex::new(conn) })
    }
}

// Data types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct List {
    pub id: String,
    pub name: String,
    pub color: String,
    pub reminder: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub list_id: String,
    pub content: String,
    pub completed: bool,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>,
    pub last_ping_at: Option<i64>,
    pub parent_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub list_id: String,
    pub content: String,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    pub content: Option<String>,
    pub completed: Option<bool>,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>,
    pub last_ping_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateListInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub reminder: Option<String>,
}

// IPC Commands - Lists
#[tauri::command]
fn get_all_lists(db: State<Arc<Database>>) -> Result<Vec<List>, String> {
    let conn = db.conn.lock();
    let mut stmt = conn
        .prepare("SELECT id, name, color, reminder, created_at, updated_at FROM lists ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let lists = stmt
        .query_map([], |row| {
            Ok(List {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                reminder: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(lists)
}

#[tauri::command]
fn create_list(db: State<Arc<Database>>, name: String, color: Option<String>) -> Result<List, String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let color = color.unwrap_or_else(|| "#4a9f6e".to_string());

    let conn = db.conn.lock();
    conn.execute(
        "INSERT INTO lists (id, name, color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, color, now, now],
    ).map_err(|e| e.to_string())?;

    Ok(List {
        id,
        name,
        color,
        reminder: None,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
fn update_list(db: State<Arc<Database>>, id: String, updates: UpdateListInput) -> Result<List, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let conn = db.conn.lock();

    // Get current list
    let mut stmt = conn
        .prepare("SELECT id, name, color, reminder, created_at FROM lists WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    let mut list: List = stmt
        .query_row([&id], |row| {
            Ok(List {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                reminder: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: now,
            })
        })
        .map_err(|e| e.to_string())?;

    // Apply updates
    if let Some(name) = updates.name {
        list.name = name;
    }
    if let Some(color) = updates.color {
        list.color = color;
    }
    if let Some(reminder) = updates.reminder {
        list.reminder = Some(reminder);
    }

    conn.execute(
        "UPDATE lists SET name = ?1, color = ?2, reminder = ?3, updated_at = ?4 WHERE id = ?5",
        params![list.name, list.color, list.reminder, now, id],
    ).map_err(|e| e.to_string())?;

    Ok(list)
}

#[tauri::command]
fn delete_list(db: State<Arc<Database>>, id: String) -> Result<(), String> {
    log::info!("delete_list called with id: {}", id);
    let conn = db.conn.lock();
    let tasks_deleted = conn.execute("DELETE FROM tasks WHERE list_id = ?1", [&id])
        .map_err(|e| {
            log::error!("Failed to delete tasks: {}", e);
            e.to_string()
        })?;
    log::info!("Deleted {} tasks", tasks_deleted);
    let lists_deleted = conn.execute("DELETE FROM lists WHERE id = ?1", [&id])
        .map_err(|e| {
            log::error!("Failed to delete list: {}", e);
            e.to_string()
        })?;
    log::info!("Deleted {} lists", lists_deleted);
    Ok(())
}

// IPC Commands - Tasks
#[tauri::command]
fn get_all_tasks(db: State<Arc<Database>>) -> Result<Vec<Task>, String> {
    let conn = db.conn.lock();
    let mut stmt = conn
        .prepare("SELECT id, list_id, content, completed, due_at, ping_interval, last_ping_at, parent_id, created_at, updated_at FROM tasks ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                list_id: row.get(1)?,
                content: row.get(2)?,
                completed: row.get::<_, i32>(3)? != 0,
                due_at: row.get(4)?,
                ping_interval: row.get(5)?,
                last_ping_at: row.get(6)?,
                parent_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(tasks)
}

#[tauri::command]
fn get_tasks_by_list(db: State<Arc<Database>>, list_id: String) -> Result<Vec<Task>, String> {
    let conn = db.conn.lock();
    let mut stmt = conn
        .prepare("SELECT id, list_id, content, completed, due_at, ping_interval, last_ping_at, parent_id, created_at, updated_at FROM tasks WHERE list_id = ?1 ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map([&list_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                list_id: row.get(1)?,
                content: row.get(2)?,
                completed: row.get::<_, i32>(3)? != 0,
                due_at: row.get(4)?,
                ping_interval: row.get(5)?,
                last_ping_at: row.get(6)?,
                parent_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(tasks)
}

#[tauri::command]
fn create_task(db: State<Arc<Database>>, task: CreateTaskInput) -> Result<Task, String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    let conn = db.conn.lock();
    conn.execute(
        "INSERT INTO tasks (id, list_id, content, completed, due_at, ping_interval, parent_id, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8)",
        params![id, task.list_id, task.content, task.due_at, task.ping_interval, task.parent_id, now, now],
    ).map_err(|e| e.to_string())?;

    Ok(Task {
        id,
        list_id: task.list_id,
        content: task.content,
        completed: false,
        due_at: task.due_at,
        ping_interval: task.ping_interval,
        last_ping_at: None,
        parent_id: task.parent_id,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
fn update_task(db: State<Arc<Database>>, id: String, updates: UpdateTaskInput) -> Result<Task, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let conn = db.conn.lock();

    // Get current task
    let mut stmt = conn
        .prepare("SELECT id, list_id, content, completed, due_at, ping_interval, last_ping_at, parent_id, created_at FROM tasks WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    let mut task: Task = stmt
        .query_row([&id], |row| {
            Ok(Task {
                id: row.get(0)?,
                list_id: row.get(1)?,
                content: row.get(2)?,
                completed: row.get::<_, i32>(3)? != 0,
                due_at: row.get(4)?,
                ping_interval: row.get(5)?,
                last_ping_at: row.get(6)?,
                parent_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: now,
            })
        })
        .map_err(|e| e.to_string())?;

    // Apply updates
    if let Some(content) = updates.content {
        task.content = content;
    }
    if let Some(completed) = updates.completed {
        task.completed = completed;
    }
    if updates.due_at.is_some() {
        task.due_at = updates.due_at;
    }
    if updates.ping_interval.is_some() {
        task.ping_interval = updates.ping_interval;
    }
    if updates.last_ping_at.is_some() {
        task.last_ping_at = updates.last_ping_at;
    }

    conn.execute(
        "UPDATE tasks SET content = ?1, completed = ?2, due_at = ?3, ping_interval = ?4, last_ping_at = ?5, updated_at = ?6 WHERE id = ?7",
        params![task.content, task.completed as i32, task.due_at, task.ping_interval, task.last_ping_at, now, id],
    ).map_err(|e| e.to_string())?;

    Ok(task)
}

#[tauri::command]
fn delete_task(db: State<Arc<Database>>, id: String) -> Result<(), String> {
    let conn = db.conn.lock();
    conn.execute("DELETE FROM tasks WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn toggle_task_complete(db: State<Arc<Database>>, id: String) -> Result<Task, String> {
    let conn = db.conn.lock();
    let now = chrono::Utc::now().timestamp_millis();

    // Get and toggle
    let completed: i32 = conn
        .query_row("SELECT completed FROM tasks WHERE id = ?1", [&id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let new_completed = if completed == 0 { 1 } else { 0 };

    conn.execute(
        "UPDATE tasks SET completed = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_completed, now, id],
    ).map_err(|e| e.to_string())?;

    // Return updated task
    drop(conn);
    let db_clone = db.inner().clone();
    let conn = db_clone.conn.lock();

    let task: Task = conn
        .query_row(
            "SELECT id, list_id, content, completed, due_at, ping_interval, last_ping_at, parent_id, created_at, updated_at FROM tasks WHERE id = ?1",
            [&id],
            |row| {
                Ok(Task {
                    id: row.get(0)?,
                    list_id: row.get(1)?,
                    content: row.get(2)?,
                    completed: row.get::<_, i32>(3)? != 0,
                    due_at: row.get(4)?,
                    ping_interval: row.get(5)?,
                    last_ping_at: row.get(6)?,
                    parent_id: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(task)
}

// Window commands
#[tauri::command]
fn hide_window(window: WebviewWindow) {
    window.hide().ok();
}

#[tauri::command]
fn show_window(window: WebviewWindow) {
    window.show().ok();
    window.set_focus().ok();
}

#[tauri::command]
fn minimize_window(window: WebviewWindow) {
    window.minimize().ok();
}

fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            window.hide().ok();
        } else {
            window.show().ok();
            window.set_focus().ok();
        }
    }
}

fn setup_app_menu(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Create a custom "Hide" menu item with Cmd+Q shortcut
    let hide_item = MenuItem::with_id(
        app,
        "hide_app",
        "Hide PingPal",
        true,
        Some("CmdOrCtrl+Q"),
    )?;

    let about = PredefinedMenuItem::about(app, Some("About PingPal"), None)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let app_menu = Submenu::with_items(
        app,
        "PingPal",
        true,
        &[&about, &separator, &hide_item],
    )?;

    let menu = Menu::with_items(app, &[&app_menu])?;
    app.set_menu(menu)?;

    // Handle the hide menu item
    app.on_menu_event(move |app, event| {
        if event.id().as_ref() == "hide_app" {
            if let Some(window) = app.get_webview_window("main") {
                window.hide().ok();
            }
        }
    });

    Ok(())
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let quit = MenuItem::with_id(app, "quit", "Quit PingPal", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show PingPal", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("PingPal - Click to show, right-click for menu")
        .icon_as_template(true)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "quit" => {
                    app.exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click { .. } = event {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn setup_global_shortcut(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Try multiple shortcuts in case one is already taken
    let shortcuts_to_try = [
        "Command+E",
        "Command+Shift+E",
        "Command+Option+E",
    ];

    for shortcut_str in shortcuts_to_try {
        match shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                log::info!("Attempting to register shortcut: {}", shortcut_str);

                match app.global_shortcut().on_shortcut(shortcut.clone(), |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        log::info!("Shortcut triggered!");
                        toggle_window(app);
                    }
                }) {
                    Ok(_) => {
                        log::info!("Successfully registered shortcut: {}", shortcut_str);
                        return Ok(());
                    }
                    Err(e) => {
                        log::warn!("Failed to register {}: {}", shortcut_str, e);
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to parse shortcut {}: {}", shortcut_str, e);
            }
        }
    }

    log::error!("Could not register any global shortcut. On macOS, grant Accessibility permissions in System Preferences > Privacy & Security > Accessibility");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Arc::new(Database::new().expect("Failed to initialize database"));

    // Create default list if empty
    {
        let conn = db.conn.lock();
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM lists", [], |row| row.get(0))
            .unwrap_or(0);

        if count == 0 {
            let id = Uuid::new_v4().to_string();
            let now = chrono::Utc::now().timestamp_millis();
            conn.execute(
                "INSERT INTO lists (id, name, color, created_at, updated_at) VALUES (?1, 'Inbox', '#4a9f6e', ?2, ?3)",
                params![id, now, now],
            ).ok();
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .manage(db)
        .setup(|app| {
            // Setup logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Enable autostart (launch at login)
            use tauri_plugin_autostart::ManagerExt;
            let autostart_manager = app.autolaunch();
            if !autostart_manager.is_enabled().unwrap_or(false) {
                let _ = autostart_manager.enable();
                log::info!("Enabled launch at login");
            }

            // Setup custom app menu with "Hide" instead of "Quit" for Cmd+Q
            setup_app_menu(app.handle())?;

            // Setup tray
            setup_tray(app.handle())?;

            // Setup global shortcut
            if let Err(e) = setup_global_shortcut(app.handle()) {
                log::warn!("Failed to setup global shortcut: {}", e);
            }

            // Show window on first launch (not when started hidden)
            let args: Vec<String> = std::env::args().collect();
            if !args.contains(&"--hidden".to_string()) {
                if let Some(window) = app.get_webview_window("main") {
                    window.show()?;
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_lists,
            create_list,
            update_list,
            delete_list,
            get_all_tasks,
            get_tasks_by_list,
            create_task,
            update_task,
            delete_task,
            toggle_task_complete,
            hide_window,
            show_window,
            minimize_window,
        ])
        .on_window_event(|window, event| {
            // Prevent window from actually closing - just hide it instead
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().ok();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Prevent Cmd+Q from quitting - hide window instead
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
                if let Some(window) = app.get_webview_window("main") {
                    window.hide().ok();
                }
            }
        });
}
