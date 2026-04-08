use crate::app::{App, InputMode, View, VisibleEntry};
use crate::parser;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

const ACCENT: Color = Color::Rgb(80, 200, 120);
const BG: Color = Color::Rgb(12, 12, 12);
const SURFACE: Color = Color::Rgb(18, 18, 18);
const ELEVATED: Color = Color::Rgb(28, 28, 28);
const BORDER: Color = Color::Rgb(38, 38, 38);
const TEXT: Color = Color::Rgb(220, 220, 220);
const DIM: Color = Color::Rgb(100, 100, 100);
const FAINT: Color = Color::Rgb(55, 55, 55);
const DANGER: Color = Color::Rgb(240, 80, 80);
const WARN: Color = Color::Rgb(240, 200, 60);
const BLUE: Color = Color::Rgb(100, 160, 240);
const P1: Color = Color::Rgb(240, 80, 80);
const P2: Color = Color::Rgb(240, 200, 60);
const P3: Color = Color::Rgb(100, 160, 240);

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let detail_h = if app.expanded_task_id.is_some() { 4u16 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Length(1),  // separator
            Constraint::Min(3),    // tasks
            Constraint::Length(detail_h),
            Constraint::Length(1),  // separator
            Constraint::Length(1),  // input
            Constraint::Length(1),  // keybinds
        ])
        .split(area);

    draw_header(frame, app, chunks[0]);
    frame.render_widget(Paragraph::new("").style(Style::default().bg(BG)), chunks[1]);
    draw_tasks(frame, app, chunks[2]);
    if detail_h > 0 { draw_detail(frame, app, chunks[3]); }
    frame.render_widget(Paragraph::new(Line::from(Span::styled(
        "─".repeat(area.width as usize), Style::default().fg(BORDER),
    ))).style(Style::default().bg(BG)), chunks[4]);
    draw_input(frame, app, chunks[5]);
    draw_keybinds(frame, app, chunks[6]);

    match app.view {
        View::NewList | View::RenameList => draw_dialog(frame, app, area),
        View::ConfirmDeleteList => draw_confirm(frame, area,
            &format!("Delete '{}'? All tasks will be lost.",
                app.current_list().map(|l| l.name.as_str()).unwrap_or("?"))),
        View::ConfirmDeleteTask => draw_confirm(frame, area,
            &format!("Delete '{}'?",
                app.selected_task_data().map(|t| t.content.as_str()).unwrap_or("?"))),
        View::Help => draw_help(frame, area),
        _ => {}
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" chirp", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
    ];

    // Today tab
    if app.viewing_today {
        spans.push(Span::styled(" today ", Style::default().fg(BG).bg(WARN).add_modifier(Modifier::BOLD)));
    } else {
        spans.push(Span::styled(" today ", Style::default().fg(DIM)));
    }

    // List tabs
    for (i, list) in app.lists.iter().enumerate() {
        spans.push(Span::styled(" ", Style::default()));
        if !app.viewing_today && i == app.selected_list {
            spans.push(Span::styled(format!(" {} ", list.name),
                Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD)));
        } else {
            spans.push(Span::styled(format!(" {} ", list.name), Style::default().fg(DIM)));
        }
    }

    // Right side
    let mut right: Vec<Span> = Vec::new();

    if app.agenda_due_count > 0 {
        right.push(Span::styled(
            format!(" {} due ", app.agenda_due_count),
            Style::default().fg(WARN),
        ));
    }

    if app.daemon_running {
        right.push(Span::styled(" \u{26a1} ", Style::default().fg(ACCENT)));
    }

    let mode = if app.search_mode {
        Span::styled(" SEARCH ", Style::default().fg(BG).bg(WARN).add_modifier(Modifier::BOLD))
    } else if app.editing_task_id.is_some() {
        Span::styled(" EDIT ", Style::default().fg(BG).bg(BLUE).add_modifier(Modifier::BOLD))
    } else if app.input_mode == InputMode::Insert {
        Span::styled(" INSERT ", Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" NORMAL ", Style::default().fg(DIM).bg(ELEVATED))
    };
    right.push(mode);

    let used: usize = spans.iter().map(|s| s.content.len()).sum();
    let right_w: usize = right.iter().map(|s| s.content.len()).sum();
    let pad = (area.width as usize).saturating_sub(used + right_w);
    spans.push(Span::styled(" ".repeat(pad), Style::default()));
    spans.extend(right);

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(SURFACE)),
        area,
    );
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let active = app.input_mode == InputMode::Insert && matches!(app.view, View::Tasks);

    let icon = if app.search_mode { "/" }
    else if app.editing_task_id.is_some() { "~" }
    else { "\u{25b8}" };

    let icon_color = if app.search_mode { WARN }
    else if app.editing_task_id.is_some() { BLUE }
    else if active { ACCENT }
    else { DIM };

    let text = if app.input.is_empty() && !active {
        if let Some(msg) = &app.status_message {
            msg.clone()
        } else {
            format!("{} pending", app.pending_count())
        }
    } else if app.input.is_empty() && active {
        if app.search_mode { "type to search...".into() }
        else { "buy milk tomorrow 5pm ping 2h p1 daily".into() }
    } else {
        app.input.clone()
    };

    let text_style = if app.input.is_empty() && !active {
        if app.status_message.is_some() { Style::default().fg(ACCENT) }
        else { Style::default().fg(DIM) }
    } else if app.input.is_empty() {
        Style::default().fg(FAINT)
    } else {
        Style::default().fg(TEXT)
    };

    let line = Line::from(vec![
        Span::styled(format!(" {} ", icon), Style::default().fg(icon_color)),
        Span::styled(text, text_style),
    ]);

    frame.render_widget(Paragraph::new(line).style(Style::default().bg(BG)), area);

    if active {
        let cx = area.x + 3 + unicode_width(&app.input[..app.cursor_pos]) as u16;
        frame.set_cursor_position((cx.min(area.right().saturating_sub(1)), area.y));
    }
}

fn draw_tasks(frame: &mut Frame, app: &mut App, area: Rect) {
    let entries = app.visible_entries();

    if entries.is_empty() {
        let msg = if app.search_mode && !app.input.is_empty() { "no matches" }
        else if app.viewing_today { "nothing due today" }
        else if app.tasks.is_empty() { "no tasks yet \u{2014} press i to add one" }
        else { "all done" };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!("  {}", msg), Style::default().fg(DIM))))
                .style(Style::default().bg(BG)),
            area,
        );
        return;
    }

    let mut sel_idx = 0;
    let sel_entry = entries.iter().enumerate().find_map(|(ei, e)| match e {
        VisibleEntry::Task(_) => {
            if sel_idx == app.selected_task { Some(ei) } else { sel_idx += 1; None }
        }
        VisibleEntry::Separator(_) => None,
    }).unwrap_or(0);

    let h = area.height as usize;
    if sel_entry < app.scroll_offset { app.scroll_offset = sel_entry; }
    else if sel_entry >= app.scroll_offset + h { app.scroll_offset = sel_entry - h + 1; }

    let mut counter = 0usize;
    let items: Vec<ListItem> = entries.iter().map(|e| match e {
        VisibleEntry::Separator(label) => {
            ListItem::new(Line::from(Span::styled(
                format!("  \u{2500}\u{2500} {} \u{2500}\u{2500}", label),
                Style::default().fg(FAINT).add_modifier(Modifier::ITALIC),
            ))).style(Style::default().bg(BG))
        }
        VisibleEntry::Task(idx) => {
            let task = &app.tasks[*idx];
            let selected = counter == app.selected_task;
            counter += 1;
            let tag = if app.viewing_today {
                Some(app.list_name_for_id(&task.list_id))
            } else { None };
            build_task(task, selected, tag.as_deref())
        }
    }).collect();

    let visible: Vec<ListItem> = items.into_iter()
        .skip(app.scroll_offset).take(h).collect();

    frame.render_widget(
        List::new(visible).block(Block::default().style(Style::default().bg(BG)).borders(Borders::NONE)),
        area,
    );

    if entries.len() > h {
        let mut state = ScrollbarState::new(entries.len().saturating_sub(h))
            .position(app.scroll_offset);
        let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(DIM))
            .track_style(Style::default().fg(Color::Rgb(22, 22, 22)));
        frame.render_stateful_widget(sb, area, &mut state);
    }
}

fn pri_color(p: u8) -> Color {
    match p { 1 => P1, 2 => P2, 3 => P3, _ => DIM }
}

fn build_task(task: &crate::db::Task, selected: bool, list_tag: Option<&str>) -> ListItem<'static> {
    let done = task.completed;

    // Selection indicator
    let sel = if selected {
        Span::styled(" \u{25b8} ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("   ", Style::default())
    };

    // Checkbox
    let check = if done {
        Span::styled("\u{2713} ", Style::default().fg(ACCENT))
    } else {
        Span::styled("\u{25cb} ", Style::default().fg(DIM))
    };

    // Content
    let content_style = if done {
        Style::default().fg(FAINT).add_modifier(Modifier::CROSSED_OUT)
    } else if selected {
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };

    let mut spans = vec![sel];

    // Priority dot
    if let Some(p) = task.priority {
        let c = if done { FAINT } else { pri_color(p) };
        spans.push(Span::styled(
            format!("{} ", parser::format_priority(p)),
            Style::default().fg(c).add_modifier(Modifier::BOLD),
        ));
    }

    spans.push(check);
    spans.push(Span::styled(task.content.clone(), content_style));

    // List tag in today view
    if let Some(name) = list_tag {
        spans.push(Span::styled(format!("  {}", name), Style::default().fg(FAINT)));
    }

    // Due date
    if let Some(due) = task.due_at {
        let overdue = parser::is_overdue(due) && !done;
        let color = if overdue { DANGER } else { WARN };
        spans.push(Span::styled(
            format!("  {}", parser::format_due_date(due)),
            Style::default().fg(color),
        ));
    }

    // Recurrence
    if let Some(ref rec) = task.recurrence {
        if !done {
            spans.push(Span::styled(format!("  {}", rec), Style::default().fg(ACCENT)));
        }
    }

    // Ping countdown
    if let Some(interval) = task.ping_interval {
        if !done {
            let itxt = parser::format_ping_interval(interval);
            let cd = parser::ping_countdown(task.last_ping_at, task.ping_interval, task.due_at);
            if let Some(ref c) = cd {
                let cc = if c == "now!" { DANGER } else if c == "at due" { WARN } else { ACCENT };
                spans.push(Span::styled(format!("  ~{} {}", itxt, c), Style::default().fg(cc)));
            } else {
                spans.push(Span::styled(format!("  ~{}", itxt), Style::default().fg(BLUE)));
            }
        }
    }

    // Note
    if task.note.is_some() && !done {
        spans.push(Span::styled("  +", Style::default().fg(FAINT)));
    }

    let bg = if selected { ELEVATED } else { BG };
    ListItem::new(Line::from(spans)).style(Style::default().bg(bg))
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect) {
    let task = match app.selected_task_data() {
        Some(t) if app.expanded_task_id.as_ref() == Some(&t.id) => t,
        _ => { frame.render_widget(Block::default().style(Style::default().bg(BG)), area); return; }
    };

    let list_name = app.list_name_for_id(&task.list_id);
    let d = Style::default().fg(DIM);
    let v = Style::default().fg(Color::Rgb(180, 180, 180));

    let mut l1 = vec![
        Span::styled("  list: ", d), Span::styled(&list_name, v),
    ];
    if let Some(due) = task.due_at {
        let c = if parser::is_overdue(due) { DANGER } else { WARN };
        l1.push(Span::styled("  due: ", d));
        l1.push(Span::styled(parser::format_due_date(due), Style::default().fg(c)));
    }
    if let Some(p) = task.priority {
        l1.push(Span::styled("  pri: ", d));
        l1.push(Span::styled(parser::format_priority(p), Style::default().fg(pri_color(p))));
    }

    let mut l2 = vec![Span::styled("  ", d)];
    if let Some(interval) = task.ping_interval {
        l2.push(Span::styled("ping: ", d));
        l2.push(Span::styled(format!("every {}", parser::format_ping_interval(interval)), Style::default().fg(BLUE)));
        if let Some(cd) = parser::ping_countdown(task.last_ping_at, task.ping_interval, task.due_at) {
            l2.push(Span::styled(format!(" ({})", cd), Style::default().fg(ACCENT)));
        }
        l2.push(Span::styled("  ", d));
    }
    if let Some(ref rec) = task.recurrence {
        l2.push(Span::styled("recurs: ", d));
        l2.push(Span::styled(rec.clone(), Style::default().fg(ACCENT)));
    }

    let note = task.note.as_deref().unwrap_or("(no note)");
    let l3 = vec![
        Span::styled("  note: ", d),
        Span::styled(note.to_string(), if task.note.is_some() { v } else { d }),
    ];

    frame.render_widget(Paragraph::new(vec![
        Line::from(Span::styled(
            format!("  {}", "\u{2500}".repeat(area.width.saturating_sub(4) as usize)),
            Style::default().fg(BORDER),
        )),
        Line::from(l1), Line::from(l2), Line::from(l3),
    ]).style(Style::default().bg(BG)), area);
}

fn draw_keybinds(frame: &mut Frame, app: &App, area: Rect) {
    let binds: Vec<(&str, &str)> = match (&app.input_mode, app.search_mode, &app.view) {
        (_, _, View::ConfirmDeleteList | View::ConfirmDeleteTask) => vec![("y", "yes"), ("n", "no")],
        (_, _, View::Help) => vec![("any", "close")],
        (_, _, View::NewList | View::RenameList) => vec![("\u{21b5}", "save"), ("esc", "cancel")],
        (_, true, _) => vec![("esc", "cancel"), ("\u{21b5}", "select")],
        (InputMode::Insert, _, _) if app.editing_task_id.is_some() => vec![("\u{21b5}", "save"), ("esc", "cancel")],
        (InputMode::Insert, _, _) => vec![("\u{21b5}", "add"), ("esc", "cancel")],
        (InputMode::Normal, _, _) => vec![
            ("i", "add"), ("e", "edit"), ("\u{21b5}", "detail"), ("\u{2423}", "done"),
            ("s", "snooze"), ("d", "del"), ("/", "search"), ("t", "today"),
            ("?", "help"), ("q", "quit"),
        ],
    };

    let k = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let ds = Style::default().fg(DIM);

    let mut spans: Vec<Span> = vec![Span::styled(" ", Style::default())];
    for (i, (key, desc)) in binds.iter().enumerate() {
        if i > 0 { spans.push(Span::styled("  ", Style::default())); }
        spans.push(Span::styled(*key, k));
        spans.push(Span::styled(format!(" {}", desc), ds));
    }

    let used: usize = spans.iter().map(|s| s.content.len()).sum();
    let rem = (area.width as usize).saturating_sub(used);
    spans.push(Span::styled(" ".repeat(rem), Style::default()));

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(SURFACE)),
        area,
    );
}

fn draw_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.view { View::NewList => " New List ", View::RenameList => " Rename ", _ => "" };
    let r = centered(area, 40, 5);
    frame.render_widget(Clear, r);

    let text = if app.input.is_empty() {
        Line::from(Span::styled("enter a name...", Style::default().fg(FAINT)))
    } else {
        Line::from(Span::styled(&app.input, Style::default().fg(TEXT)))
    };

    frame.render_widget(Paragraph::new(text).block(
        Block::default().title(title)
            .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL).border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(SURFACE)),
    ), r);

    let cx = r.x + 1 + unicode_width(&app.input[..app.cursor_pos]) as u16;
    frame.set_cursor_position((cx.min(r.right().saturating_sub(2)), r.y + 1));
}

fn draw_confirm(frame: &mut Frame, area: Rect, message: &str) {
    let w = (message.len() as u16 + 6).min(area.width.saturating_sub(4)).max(30);
    let r = centered(area, w, 5);
    frame.render_widget(Clear, r);

    frame.render_widget(Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!(" {} ", message), Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(" y ", Style::default().fg(BG).bg(DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(" yes  ", Style::default().fg(DIM)),
            Span::styled(" n ", Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" no", Style::default().fg(DIM)),
        ]),
    ]).wrap(Wrap { trim: false }).block(
        Block::default().title(" Confirm ")
            .title_style(Style::default().fg(DANGER).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL).border_style(Style::default().fg(DANGER))
            .style(Style::default().bg(SURFACE)),
    ), r);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let w = 54u16.min(area.width.saturating_sub(4));
    let h = 30u16.min(area.height.saturating_sub(2));
    let r = centered(area, w, h);
    frame.render_widget(Clear, r);

    let k = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let d = Style::default().fg(TEXT);
    let s = Style::default().fg(WARN).add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", s)),
        hline("    j/k", "move up/down", k, d),
        hline("    h/l, tab", "switch lists", k, d),
        hline("    [ / ]", "cycle (incl. today)", k, d),
        hline("    t", "today view", k, d),
        hline("    g / G", "top / bottom", k, d),
        Line::from(""),
        Line::from(Span::styled("  Tasks", s)),
        hline("    i", "add task", k, d),
        hline("    e", "edit task", k, d),
        hline("    enter", "detail pane", k, d),
        hline("    space", "toggle done", k, d),
        hline("    d", "delete", k, d),
        hline("    s", "snooze ping", k, d),
        hline("    J / K", "reorder", k, d),
        hline("    /", "search", k, d),
        hline("    c", "show/hide done", k, d),
        hline("    note <text>", "attach note", k, d),
        Line::from(""),
        Line::from(Span::styled("  Syntax", s)),
        hline("    p1/p2/p3", "priority", k, d),
        hline("    daily/weekly/monthly", "recurring", k, d),
        hline("    ping 30m", "reminder", k, d),
        hline("    tomorrow 5pm", "due date", k, d),
        Line::from(""),
        hline("    q / esc", "quit", k, d),
        Line::from(Span::styled("  press any key to close", Style::default().fg(DIM))),
    ];

    frame.render_widget(Paragraph::new(lines).block(
        Block::default().title(" Help ")
            .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL).border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(SURFACE)),
    ), r);
}

fn hline<'a>(key: &'a str, desc: &'a str, ks: Style, ds: Style) -> Line<'a> {
    let pad = 26usize.saturating_sub(key.len());
    Line::from(vec![Span::styled(key, ks), Span::raw(" ".repeat(pad)), Span::styled(desc, ds)])
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(
        area.x + (area.width.saturating_sub(w)) / 2,
        area.y + (area.height.saturating_sub(h)) / 2,
        w, h,
    )
}

fn unicode_width(s: &str) -> usize { s.chars().count() }
