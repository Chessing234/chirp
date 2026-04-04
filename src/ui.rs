use crate::app::{App, InputMode, View, VisibleEntry};
use crate::parser;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

const ACCENT: Color = Color::Rgb(74, 159, 110);
const BG: Color = Color::Rgb(10, 10, 10);
const SURFACE: Color = Color::Rgb(17, 17, 17);
const ELEVATED: Color = Color::Rgb(26, 26, 26);
const BORDER: Color = Color::Rgb(42, 42, 42);
const TEXT: Color = Color::Rgb(234, 234, 234);
const MUTED: Color = Color::Rgb(120, 120, 120);
const DANGER: Color = Color::Rgb(229, 80, 80);
const YELLOW: Color = Color::Rgb(229, 200, 80);
const BLUE: Color = Color::Rgb(100, 149, 237);

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Background fill
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // header/tabs
            Constraint::Length(3), // input
            Constraint::Min(3),   // tasks
            Constraint::Length(2), // status bar + keybind bar
        ])
        .split(area);

    draw_header(frame, app, chunks[0]);
    draw_input(frame, app, chunks[1]);
    draw_tasks(frame, app, chunks[2]);
    draw_status_bar(frame, app, chunks[3]);

    // Modal overlays
    match app.view {
        View::NewList | View::RenameList => draw_input_dialog(frame, app, area),
        View::ConfirmDeleteList => draw_confirm_dialog(
            frame,
            area,
            &format!(
                "Delete list '{}'? All tasks will be lost.",
                app.current_list().map(|l| l.name.as_str()).unwrap_or("?")
            ),
        ),
        View::ConfirmDeleteTask => draw_confirm_dialog(
            frame,
            area,
            &format!(
                "Delete '{}'?",
                app.selected_task_data()
                    .map(|t| t.content.as_str())
                    .unwrap_or("?")
            ),
        ),
        View::Help => draw_help(frame, area),
        _ => {}
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" chirp", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default().fg(BORDER)),
    ];

    for (i, list) in app.lists.iter().enumerate() {
        if i == app.selected_list {
            spans.push(Span::styled(
                format!(" {} ", list.name),
                Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", list.name),
                Style::default().fg(MUTED),
            ));
        }
        if i < app.lists.len() - 1 {
            spans.push(Span::styled(" ", Style::default()));
        }
    }

    // Mode indicator on the right
    let mode_text = match (&app.input_mode, app.search_mode) {
        (_, true) => " SEARCH ",
        (InputMode::Insert, _) => " INSERT ",
        (InputMode::Normal, _) => " NORMAL ",
    };
    let mode_style = match (&app.input_mode, app.search_mode) {
        (_, true) => Style::default().fg(BG).bg(YELLOW).add_modifier(Modifier::BOLD),
        (InputMode::Insert, _) => Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD),
        (InputMode::Normal, _) => Style::default().fg(MUTED).bg(ELEVATED),
    };

    // Calculate padding to push mode indicator to the right
    let used_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let mode_width = mode_text.len();
    let padding = (area.width as usize).saturating_sub(used_width + mode_width);
    if padding > 0 {
        spans.push(Span::styled(
            " ".repeat(padding),
            Style::default().bg(SURFACE),
        ));
    }
    spans.push(Span::styled(mode_text, mode_style));

    let header = Paragraph::new(Line::from(spans)).style(Style::default().bg(SURFACE));
    frame.render_widget(header, area);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.input_mode == InputMode::Insert && matches!(app.view, View::Tasks);

    let (icon, icon_style) = if app.search_mode {
        ("/", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))
    } else if is_active {
        (">", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        (">", Style::default().fg(MUTED))
    };

    let border_color = if is_active {
        if app.search_mode { YELLOW } else { ACCENT }
    } else {
        BORDER
    };

    let display_text = if app.input.is_empty() && !is_active {
        if app.search_mode {
            "type to search...".to_string()
        } else {
            "press 'i' to add a task, '/' to search, '?' for help".to_string()
        }
    } else if app.input.is_empty() && is_active {
        if app.search_mode {
            "type to search...".to_string()
        } else {
            "buy milk tomorrow 5pm ping 2h".to_string()
        }
    } else {
        app.input.clone()
    };

    let text_style = if app.input.is_empty() {
        Style::default().fg(Color::Rgb(80, 80, 80))
    } else if is_active {
        Style::default().fg(TEXT)
    } else {
        Style::default().fg(MUTED)
    };

    let input_line = Line::from(vec![
        Span::styled(format!(" {} ", icon), icon_style),
        Span::styled(display_text, text_style),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG));

    let input = Paragraph::new(input_line).block(block);
    frame.render_widget(input, area);

    // Cursor positioning
    if is_active {
        // " > " = 3 chars prefix, then cursor_pos characters into the input
        let cursor_x = area.x + 3 + unicode_display_width(&app.input[..app.cursor_pos]) as u16;
        let cursor_y = area.y;
        frame.set_cursor_position((cursor_x.min(area.right().saturating_sub(1)), cursor_y));
    }
}

fn draw_tasks(frame: &mut Frame, app: &mut App, area: Rect) {
    let entries = app.visible_entries();

    if entries.is_empty() {
        let msg = if app.search_mode && !app.input.is_empty() {
            "No matches found"
        } else if app.tasks.is_empty() {
            "No tasks yet -- press 'i' to add one"
        } else {
            "All done! Press 'c' to show completed"
        };
        let p = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(msg, Style::default().fg(MUTED)),
        ]))
        .style(Style::default().bg(BG));
        frame.render_widget(p, area);
        return;
    }

    // Map selected_task index to the entry index (accounting for separators)
    let mut selectable_idx = 0;
    let selected_entry_idx = entries
        .iter()
        .enumerate()
        .find_map(|(ei, entry)| match entry {
            VisibleEntry::Task(_) => {
                if selectable_idx == app.selected_task {
                    Some(ei)
                } else {
                    selectable_idx += 1;
                    None
                }
            }
            VisibleEntry::Separator(_) => None,
        })
        .unwrap_or(0);

    // Scrolling: ensure selected item is visible
    let visible_height = area.height as usize;
    if selected_entry_idx < app.scroll_offset {
        app.scroll_offset = selected_entry_idx;
    } else if selected_entry_idx >= app.scroll_offset + visible_height {
        app.scroll_offset = selected_entry_idx - visible_height + 1;
    }

    // Build list items
    let mut selectable_counter = 0usize;
    let items: Vec<ListItem> = entries
        .iter()
        .map(|entry| match entry {
            VisibleEntry::Separator(label) => {
                ListItem::new(Line::from(vec![Span::styled(
                    format!("  --- {} ---", label),
                    Style::default().fg(Color::Rgb(70, 70, 70)).add_modifier(Modifier::ITALIC),
                )]))
                .style(Style::default().bg(BG))
            }
            VisibleEntry::Task(task_idx) => {
                let task = &app.tasks[*task_idx];
                let is_selected = selectable_counter == app.selected_task;
                selectable_counter += 1;
                build_task_item(task, is_selected)
            }
        })
        .collect();

    // Apply scroll offset
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(app.scroll_offset)
        .take(visible_height)
        .collect();

    let task_list = List::new(visible_items).block(
        Block::default()
            .style(Style::default().bg(BG))
            .borders(Borders::NONE),
    );
    frame.render_widget(task_list, area);

    // Scrollbar
    if entries.len() > visible_height {
        let mut scrollbar_state = ScrollbarState::new(entries.len().saturating_sub(visible_height))
            .position(app.scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(MUTED))
            .track_style(Style::default().fg(Color::Rgb(30, 30, 30)));
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn build_task_item(task: &crate::db::Task, selected: bool) -> ListItem<'static> {
    let completed = task.completed;

    // Selection indicator
    let indicator = if selected {
        Span::styled("  > ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("    ", Style::default())
    };

    // Checkbox
    let (checkbox, checkbox_style) = if completed {
        ("[x]", Style::default().fg(ACCENT))
    } else {
        ("[ ]", Style::default().fg(MUTED))
    };

    // Content
    let content_style = if completed {
        Style::default()
            .fg(Color::Rgb(80, 80, 80))
            .add_modifier(Modifier::CROSSED_OUT)
    } else if selected {
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };

    let mut spans = vec![
        indicator,
        Span::styled(format!("{} ", checkbox), checkbox_style),
        Span::styled(task.content.clone(), content_style),
    ];

    // Due date badge
    if let Some(due_at) = task.due_at {
        let due_text = parser::format_due_date(due_at);
        let overdue = parser::is_overdue(due_at) && !completed;
        let due_style = if overdue {
            Style::default().fg(DANGER)
        } else {
            Style::default().fg(YELLOW)
        };
        spans.push(Span::styled(
            format!("  {}", due_text),
            due_style,
        ));
    }

    // Ping badge
    if let Some(interval) = task.ping_interval {
        let ping_text = parser::format_ping_interval(interval);
        spans.push(Span::styled(
            format!("  ~{}~", ping_text),
            Style::default().fg(BLUE),
        ));
    }

    let bg = if selected { ELEVATED } else { BG };
    ListItem::new(Line::from(spans)).style(Style::default().bg(bg))
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    // Two rows: top = status info, bottom = keybind hints
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // --- Top row: status message or task counts ---
    let pending = app.pending_count();
    let total = app.tasks.len();

    let left_text = if let Some(msg) = &app.status_message {
        Span::styled(format!(" {} ", msg), Style::default().fg(ACCENT))
    } else {
        Span::styled(
            format!(" {} pending / {} total ", pending, total),
            Style::default().fg(MUTED),
        )
    };

    let info_bar = Paragraph::new(Line::from(vec![left_text]))
        .style(Style::default().bg(SURFACE));
    frame.render_widget(info_bar, rows[0]);

    // --- Bottom row: context-sensitive keybind bar ---
    let binds: Vec<(&str, &str)> = match (&app.input_mode, app.search_mode, &app.view) {
        (_, _, View::ConfirmDeleteList | View::ConfirmDeleteTask) => {
            vec![("y", "confirm"), ("n/esc", "cancel")]
        }
        (_, _, View::Help) => {
            vec![("any key", "close")]
        }
        (_, _, View::NewList | View::RenameList) => {
            vec![("enter", "save"), ("esc", "cancel")]
        }
        (_, true, _) => {
            vec![
                ("esc", "cancel"),
                ("enter", "select"),
                ("^n/^p", "up/down"),
                ("^w", "del word"),
            ]
        }
        (InputMode::Insert, _, _) => {
            vec![
                ("enter", "add task"),
                ("esc", "cancel"),
                ("^a/^e", "home/end"),
                ("^w", "del word"),
                ("^u", "clear"),
            ]
        }
        (InputMode::Normal, _, _) => {
            vec![
                ("i", "add"),
                ("/", "search"),
                ("spc", "toggle"),
                ("d", "delete"),
                ("h/l", "lists"),
                ("n", "new list"),
                ("c", "completed"),
                ("?", "help"),
                ("q", "quit"),
            ]
        }
    };

    let key_style = Style::default().fg(BG).bg(MUTED).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(MUTED).bg(ELEVATED);
    let sep_style = Style::default().bg(ELEVATED);

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in binds.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", sep_style));
        }
        spans.push(Span::styled(format!(" {} ", key), key_style));
        spans.push(Span::styled(format!(" {} ", desc), desc_style));
    }

    // Fill remaining width
    let used: usize = spans.iter().map(|s| s.content.len()).sum();
    let remaining = (rows[1].width as usize).saturating_sub(used);
    if remaining > 0 {
        spans.push(Span::styled(" ".repeat(remaining), sep_style));
    }

    let keybind_bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(ELEVATED));
    frame.render_widget(keybind_bar, rows[1]);
}

fn draw_input_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.view {
        View::NewList => " New List ",
        View::RenameList => " Rename List ",
        _ => "",
    };

    let dialog_area = centered(area, 44, 5);
    frame.render_widget(Clear, dialog_area);

    let inner_text = if app.input.is_empty() {
        Line::from(Span::styled("enter a name...", Style::default().fg(Color::Rgb(80, 80, 80))))
    } else {
        Line::from(Span::styled(&app.input, Style::default().fg(TEXT)))
    };

    let input = Paragraph::new(inner_text).block(
        Block::default()
            .title(title)
            .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(SURFACE)),
    );
    frame.render_widget(input, dialog_area);

    // Cursor inside the dialog
    let cursor_x = dialog_area.x + 1 + unicode_display_width(&app.input[..app.cursor_pos]) as u16;
    frame.set_cursor_position((cursor_x.min(dialog_area.right().saturating_sub(2)), dialog_area.y + 1));
}

fn draw_confirm_dialog(frame: &mut Frame, area: Rect, message: &str) {
    let width = (message.len() as u16 + 6).min(area.width.saturating_sub(4)).max(30);
    let dialog_area = centered(area, width, 6);
    frame.render_widget(Clear, dialog_area);

    let dialog = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" {} ", message),
            Style::default().fg(TEXT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(" y ", Style::default().fg(BG).bg(DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(" yes   ", Style::default().fg(MUTED)),
            Span::styled(" n ", Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" no", Style::default().fg(MUTED)),
        ]),
    ])
    .wrap(Wrap { trim: false })
    .block(
        Block::default()
            .title(" Confirm ")
            .title_style(Style::default().fg(DANGER).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DANGER))
            .style(Style::default().bg(SURFACE)),
    );
    frame.render_widget(dialog, dialog_area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let w = 54u16.min(area.width.saturating_sub(4));
    let h = 24u16.min(area.height.saturating_sub(2));
    let dialog_area = centered(area, w, h);
    frame.render_widget(Clear, dialog_area);

    let key_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(TEXT);
    let section_style = Style::default().fg(YELLOW).add_modifier(Modifier::BOLD);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", section_style)),
        help_line("    j/k, arrows", "move up/down", key_style, desc_style),
        help_line("    h/l, tab", "switch lists", key_style, desc_style),
        help_line("    g / G", "jump to top / bottom", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Tasks", section_style)),
        help_line("    i, a", "add new task", key_style, desc_style),
        help_line("    space, enter, x", "toggle complete", key_style, desc_style),
        help_line("    d", "delete task", key_style, desc_style),
        help_line("    /", "fuzzy search", key_style, desc_style),
        help_line("    c", "show/hide completed", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Lists", section_style)),
        help_line("    n", "new list", key_style, desc_style),
        help_line("    r", "rename list", key_style, desc_style),
        help_line("    D", "delete list", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Input Editing", section_style)),
        help_line("    ctrl+a/e", "start / end of line", key_style, desc_style),
        help_line("    ctrl+w", "delete word", key_style, desc_style),
        help_line("    ctrl+u", "clear line", key_style, desc_style),
        Line::from(""),
        help_line("    q, esc", "quit", key_style, desc_style),
        Line::from(Span::styled(
            "  press any key to close",
            Style::default().fg(MUTED),
        )),
    ];

    let help = Paragraph::new(help_text).block(
        Block::default()
            .title(" Help ")
            .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(SURFACE)),
    );
    frame.render_widget(help, dialog_area);
}

fn help_line<'a>(key: &'a str, desc: &'a str, ks: Style, ds: Style) -> Line<'a> {
    let padding = 20usize.saturating_sub(key.len());
    Line::from(vec![
        Span::styled(key, ks),
        Span::raw(" ".repeat(padding)),
        Span::styled(desc, ds),
    ])
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Calculate display width of a string (ASCII-only approximation).
/// For proper Unicode width, you'd use the unicode-width crate,
/// but for ASCII task names this is sufficient.
fn unicode_display_width(s: &str) -> usize {
    s.chars().count()
}
