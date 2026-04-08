mod app;
mod daemon;
mod db;
mod import;
mod parser;
mod ui;

use app::{App, InputMode, View};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use chrono::TimeZone;
use ratatui::prelude::*;
use std::io;
use std::panic;
use std::time::Duration;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|a| a == "--json");
    let sub = args.get(1).map(|s| s.as_str());

    match sub {
        Some("daemon") => {
            match args.get(2).map(|s| s.as_str()) {
                None | Some("start") => { daemon::run(); return Ok(()); }
                Some("stop") => { daemon::stop(); return Ok(()); }
                Some("restart") => { daemon::restart(); return Ok(()); }
                Some("install") => { daemon::install(); return Ok(()); }
                Some("uninstall") => { daemon::uninstall(); return Ok(()); }
                Some("status") => { daemon::status(); return Ok(()); }
                Some(other) => {
                    eprintln!("Unknown: chirp daemon {}", other);
                    std::process::exit(1);
                }
            }
        }
        Some("export") => {
            return export_tasks();
        }
        Some("add") => {
            return add_task(&args[2..]);
        }
        Some("list") => {
            return list_lists(json_mode);
        }
        Some("done") => {
            return show_done(json_mode);
        }
        Some("--import") => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Usage: chirp --import <file.json|file.csv>");
                std::process::exit(1);
            });
            match import::import_file(path) {
                Ok(n) => { println!("Imported {} tasks", n); return Ok(()); }
                Err(e) => { eprintln!("Import failed: {}", e); std::process::exit(1); }
            }
        }
        Some("--help") | Some("-h") => {
            println!("chirp - minimalist task manager\n");
            println!("Usage:");
            println!("  chirp [--list <name>]    Launch TUI (optionally into a list)");
            println!("  chirp add \"task text\"    Add task from CLI (supports --list)");
            println!("  chirp list [--json]      Show all lists with task counts");
            println!("  chirp done [--json]      Show today's completed tasks");
            println!("  chirp export             Dump all tasks as JSON to stdout");
            println!("  chirp daemon [cmd]       start|stop|restart|install|uninstall|status");
            println!("  chirp --import <file>    Import from JSON/CSV");
            return Ok(());
        }
        _ => {}
    }

    // Parse --list flag from remaining args
    let mut initial_list: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--list" {
            if let Some(name) = args.get(i + 1) {
                initial_list = Some(name.clone());
                i += 2;
                continue;
            } else {
                eprintln!("--list requires a list name");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // Auto-install daemon for persistent pings
    daemon::auto_install();

    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    if let Some(name) = initial_list {
        app.select_list_by_name(&name);
    }
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn export_tasks() -> io::Result<()> {
    let db = db::Database::new().map_err(|e| io::Error::other(e.to_string()))?;
    let lists = db.get_all_lists();
    let tasks = db.get_all_tasks();

    let mut out: Vec<serde_json::Value> = Vec::new();
    for task in &tasks {
        let list_name = lists.iter()
            .find(|l| l.id == task.list_id)
            .map(|l| l.name.as_str())
            .unwrap_or("?");

        let mut obj = serde_json::json!({
            "content": task.content,
            "list": list_name,
            "completed": task.completed,
        });

        if let Some(due) = task.due_at {
            obj["due_at"] = serde_json::json!(due);
            if let Some(dt) = chrono::Local.timestamp_millis_opt(due).single() {
                obj["due"] = serde_json::json!(dt.to_rfc3339());
            }
        }
        if let Some(p) = task.priority {
            obj["priority"] = serde_json::json!(p);
        }
        if let Some(ping) = task.ping_interval {
            obj["ping"] = serde_json::json!(parser::format_ping_interval(ping));
        }
        if let Some(ref rec) = task.recurrence {
            obj["recurrence"] = serde_json::json!(rec);
        }
        if let Some(ref note) = task.note {
            obj["note"] = serde_json::json!(note);
        }

        out.push(obj);
    }

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}

fn add_task(args: &[String]) -> io::Result<()> {
    let db = db::Database::new().map_err(|e| io::Error::other(e.to_string()))?;

    // Parse --list flag and collect remaining text
    let mut list_name: Option<String> = None;
    let mut text_parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--list" {
            if let Some(name) = args.get(i + 1) {
                list_name = Some(name.clone());
                i += 2;
                continue;
            } else {
                eprintln!("--list requires a name");
                std::process::exit(1);
            }
        }
        text_parts.push(args[i].clone());
        i += 1;
    }

    let text = text_parts.join(" ");
    if text.is_empty() {
        eprintln!("Usage: chirp add \"task text\" [--list <name>]");
        std::process::exit(1);
    }

    let (list_id, resolved_list_name) = if let Some(name) = list_name {
        let id = db.find_or_create_list(&name);
        (id, name)
    } else {
        let lists = db.get_all_lists();
        if let Some(first) = lists.first() {
            (first.id.clone(), first.name.clone())
        } else {
            let l = db.create_list("Inbox");
            (l.id, "Inbox".to_string())
        }
    };

    let parsed = parser::parse_task_input(&text);
    db.create_task(
        &list_id, &parsed.content, parsed.due_at, parsed.ping_interval,
        parsed.priority, parsed.recurrence.as_deref(),
    );

    println!("Added to {}: {}", resolved_list_name, parsed.content);
    Ok(())
}

fn list_lists(json_mode: bool) -> io::Result<()> {
    let db = db::Database::new().map_err(|e| io::Error::other(e.to_string()))?;
    let lists = db.get_all_lists();

    if json_mode {
        let out: Vec<serde_json::Value> = lists.iter().map(|list| {
            let (pending, total) = db.list_task_counts(&list.id);
            serde_json::json!({
                "name": list.name,
                "pending": pending,
                "total": total,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        for list in &lists {
            let (pending, total) = db.list_task_counts(&list.id);
            println!("  {} ({}/{})", list.name, pending, total);
        }
    }
    Ok(())
}

fn show_done(json_mode: bool) -> io::Result<()> {
    let db = db::Database::new().map_err(|e| io::Error::other(e.to_string()))?;
    let lists = db.get_all_lists();

    let now = chrono::Local::now();
    let start_of_today = now.date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| chrono::Local.from_local_datetime(&dt).unwrap().timestamp_millis())
        .unwrap_or(0);

    let tasks = db.get_completed_today(start_of_today);

    if json_mode {
        let out: Vec<serde_json::Value> = tasks.iter().map(|task| {
            let list_name = lists.iter()
                .find(|l| l.id == task.list_id)
                .map(|l| l.name.as_str())
                .unwrap_or("?");
            serde_json::json!({
                "content": task.content,
                "list": list_name,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else if tasks.is_empty() {
        println!("  nothing completed today");
    } else {
        println!("  completed today ({}):", tasks.len());
        for task in &tasks {
            let list_name = lists.iter()
                .find(|l| l.id == task.list_id)
                .map(|l| l.name.as_str())
                .unwrap_or("?");
            println!("  \u{2713} {}  [{}]", task.content, list_name);
        }
    }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        app.check_daemon_status();
        app.check_pings();

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != event::KeyEventKind::Press { continue; }
                    app.status_message = None;
                    handle_key(app, key);
                    if app.should_quit { return Ok(()); }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(app, mouse);
                }
                _ => {}
            }
        }
    }
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    // Only handle mouse in normal mode, tasks view
    if app.input_mode != InputMode::Normal || app.view != View::Tasks {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let row = mouse.row;
            let col = mouse.column;

            // Check if click is in the task area
            if row >= app.task_area_y && row < app.task_area_y + app.task_area_height {
                let clicked_entry = (row - app.task_area_y) as usize + app.scroll_offset;
                let entries = app.visible_entries();

                // Map entry index to selectable task index
                let mut sel_idx = 0;
                for (ei, entry) in entries.iter().enumerate() {
                    if ei == clicked_entry {
                        if let app::VisibleEntry::Task(_) = entry {
                            // Click on checkbox area (columns 0..6) toggles done
                            if col < 7 {
                                app.selected_task = sel_idx;
                                app.toggle_selected_task();
                            } else {
                                app.selected_task = sel_idx;
                            }
                        }
                        break;
                    }
                    if matches!(entry, app::VisibleEntry::Task(_)) {
                        sel_idx += 1;
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            app.move_selection_up();
        }
        MouseEventKind::ScrollDown => {
            app.move_selection_down();
        }
        _ => {}
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.view {
        View::ConfirmDeleteList => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.delete_current_list(),
            _ => app.view = View::Tasks,
        },
        View::ConfirmDeleteTask => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.delete_selected_task();
                app.view = View::Tasks;
            }
            _ => app.view = View::Tasks,
        },
        View::Help => { app.view = View::Tasks; }
        View::NewList | View::RenameList => handle_dialog(app, key),
        View::Tasks => {
            if app.input_mode == InputMode::Insert {
                handle_insert(app, key);
            } else {
                handle_normal(app, key);
            }
        }
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('?') => app.view = View::Help,

        KeyCode::Char('j') | KeyCode::Down => app.move_selection_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection_up(),
        KeyCode::Char('g') => app.move_selection_top(),
        KeyCode::Char('G') => app.move_selection_bottom(),
        KeyCode::Char('h') | KeyCode::Left => app.prev_list(),
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => app.next_list(),
        KeyCode::BackTab => app.prev_list(),
        KeyCode::Char(']') => app.cycle_list(true),
        KeyCode::Char('[') => app.cycle_list(false),
        KeyCode::Char('t') => app.toggle_today(),

        KeyCode::Char('J') => app.move_task_down(),
        KeyCode::Char('K') => app.move_task_up(),

        KeyCode::Char('i') | KeyCode::Char('a') => {
            app.input_mode = InputMode::Insert;
            app.search_mode = false;
            app.editing_task_id = None;
            app.input.clear();
            app.cursor_pos = 0;
        }
        KeyCode::Char('e') => app.start_edit(),
        KeyCode::Enter => app.toggle_detail_pane(),
        KeyCode::Char(' ') | KeyCode::Char('x') => app.toggle_selected_task(),
        KeyCode::Char('s') => app.snooze_selected(),
        KeyCode::Char('d') => {
            if app.selected_task_data().is_some() {
                app.view = View::ConfirmDeleteTask;
            }
        }
        KeyCode::Char('/') => app.start_search(),
        KeyCode::Char('c') => {
            app.show_completed = !app.show_completed;
            app.status_message = Some(if app.show_completed {
                "showing completed".into()
            } else {
                "hiding completed".into()
            });
            app.clamp_selection();
        }

        KeyCode::Char('n') => {
            app.view = View::NewList;
            app.input_mode = InputMode::Insert;
            app.input.clear();
            app.cursor_pos = 0;
        }
        KeyCode::Char('r') => {
            if let Some(list) = app.current_list() {
                app.input = list.name.clone();
                app.cursor_pos = app.input.len();
                app.view = View::RenameList;
                app.input_mode = InputMode::Insert;
            }
        }
        KeyCode::Char('D') => { app.view = View::ConfirmDeleteList; }
        KeyCode::Char('u') => app.undo(),

        _ => {}
    }
}

fn handle_insert(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            if app.search_mode { app.cancel_search(); }
            else if app.editing_task_id.is_some() { app.cancel_edit(); }
            else {
                app.input_mode = InputMode::Normal;
                app.input.clear();
                app.cursor_pos = 0;
            }
        }
        (_, KeyCode::Enter) => {
            app.submit_input();
            if !app.search_mode { app.input_mode = InputMode::Normal; }
        }
        (_, KeyCode::Backspace) => app.delete_char_before_cursor(),
        (_, KeyCode::Delete) => app.delete_char_at_cursor(),
        (_, KeyCode::Left) => app.move_cursor_left(),
        (_, KeyCode::Right) => app.move_cursor_right(),
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => app.cursor_pos = 0,
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => app.cursor_pos = app.input.len(),
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.input.drain(..app.cursor_pos);
            app.cursor_pos = 0;
            app.on_input_changed();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => app.delete_word_before_cursor(),
        (_, KeyCode::Home) => app.cursor_pos = 0,
        (_, KeyCode::End) => app.cursor_pos = app.input.len(),
        (KeyModifiers::CONTROL, KeyCode::Char('n')) | (_, KeyCode::Down) if app.search_mode => {
            app.move_selection_down();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('p')) | (_, KeyCode::Up) if app.search_mode => {
            app.move_selection_up();
        }
        (_, KeyCode::Char(c)) => app.insert_char(c),
        _ => {}
    }
}

fn handle_dialog(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input.clear();
            app.cursor_pos = 0;
            app.input_mode = InputMode::Normal;
            app.view = View::Tasks;
        }
        KeyCode::Enter => app.submit_input(),
        KeyCode::Backspace => app.delete_char_before_cursor(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Char(c) => app.insert_char(c),
        _ => {}
    }
}
