mod app;
mod db;
mod import;
mod parser;
mod ui;

use app::{App, InputMode, View};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::panic;
use std::time::Duration;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Handle --import flag
    if let Some(pos) = args.iter().position(|a| a == "--import") {
        let path = args.get(pos + 1).unwrap_or_else(|| {
            eprintln!("Usage: chirp --import <file.json|file.csv>");
            std::process::exit(1);
        });
        match import::import_file(path) {
            Ok(count) => {
                println!("Imported {} tasks from {}", count, path);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Import failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("chirp - keyboard-first task manager for the terminal\n");
        println!("Usage:");
        println!("  chirp                  Launch the TUI");
        println!("  chirp --import <file>  Import tasks from JSON or CSV");
        println!("\nJSON format: [{{\"content\": \"task\", \"list\": \"Inbox\", \"priority\": 1, \"due\": \"2025-01-15T09:00:00\", \"ping\": \"30m\", \"recurrence\": \"daily\"}}]");
        println!("CSV format:  content,list,priority,due,ping,recurrence");
        return Ok(());
    }

    // Install panic hook to restore terminal on crash
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        app.check_pings();

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }

                app.status_message = None;
                handle_key(app, key);

                if app.should_quit {
                    return Ok(());
                }
            }
        }
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
        View::NewList | View::RenameList => handle_dialog_input(app, key),
        View::Tasks => {
            if app.input_mode == InputMode::Insert {
                handle_insert_mode(app, key);
            } else {
                handle_normal_mode(app, key);
            }
        }
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('?') => app.view = View::Help,

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.move_selection_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection_up(),
        KeyCode::Char('g') => app.move_selection_top(),
        KeyCode::Char('G') => app.move_selection_bottom(),
        KeyCode::Char('h') | KeyCode::Left => app.prev_list(),
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => app.next_list(),
        KeyCode::BackTab => app.prev_list(),

        // Reorder tasks (Shift+J / Shift+K)
        KeyCode::Char('J') => app.move_task_down(),
        KeyCode::Char('K') => app.move_task_up(),

        // Task actions
        KeyCode::Char('i') | KeyCode::Char('a') => {
            app.input_mode = InputMode::Insert;
            app.search_mode = false;
            app.editing_task_id = None;
            app.input.clear();
            app.cursor_pos = 0;
        }
        KeyCode::Char('e') => app.start_edit(),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_selected_task(),
        KeyCode::Char('x') => app.toggle_selected_task(),
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
                "Showing completed".to_string()
            } else {
                "Hiding completed".to_string()
            });
            app.clamp_selection();
        }

        // List actions
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

        _ => {}
    }
}

fn handle_insert_mode(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            if app.search_mode {
                app.cancel_search();
            } else if app.editing_task_id.is_some() {
                app.cancel_edit();
            } else {
                app.input_mode = InputMode::Normal;
                app.input.clear();
                app.cursor_pos = 0;
            }
        }
        (_, KeyCode::Enter) => {
            app.submit_input();
            if !app.search_mode {
                app.input_mode = InputMode::Normal;
            }
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

fn handle_dialog_input(app: &mut App, key: KeyEvent) {
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
