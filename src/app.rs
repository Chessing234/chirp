use crate::db::{Database, List, Task};
use crate::parser;
use chrono::Utc;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use notify_rust::Notification;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Tasks,
    NewList,
    RenameList,
    ConfirmDeleteList,
    ConfirmDeleteTask,
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
}

pub struct App {
    pub db: Database,
    pub lists: Vec<List>,
    pub tasks: Vec<Task>,
    pub filtered_indices: Vec<usize>,
    pub selected_list: usize,
    pub selected_task: usize,
    pub scroll_offset: usize,
    pub input: String,
    pub cursor_pos: usize,
    pub input_mode: InputMode,
    pub view: View,
    pub show_completed: bool,
    pub search_mode: bool,
    pub editing_task_id: Option<String>,
    pub should_quit: bool,
    pub status_message: Option<String>,
    matcher: SkimMatcherV2,
    last_ping_check: Instant,
}

impl App {
    pub fn new() -> Self {
        let db = Database::new().expect("Failed to open database");
        let lists = db.get_all_lists();
        let tasks = if !lists.is_empty() {
            db.get_tasks_by_list(&lists[0].id)
        } else {
            vec![]
        };
        let filtered_indices = (0..tasks.len()).collect();

        Self {
            db,
            lists,
            tasks,
            filtered_indices,
            selected_list: 0,
            selected_task: 0,
            scroll_offset: 0,
            input: String::new(),
            cursor_pos: 0,
            input_mode: InputMode::Normal,
            view: View::Tasks,
            show_completed: true,
            search_mode: false,
            editing_task_id: None,
            should_quit: false,
            status_message: None,
            matcher: SkimMatcherV2::default(),
            last_ping_check: Instant::now(),
        }
    }

    pub fn current_list(&self) -> Option<&List> {
        self.lists.get(self.selected_list)
    }

    pub fn current_list_id(&self) -> Option<String> {
        self.current_list().map(|l| l.id.clone())
    }

    pub fn refresh_tasks(&mut self) {
        if let Some(list_id) = self.current_list_id() {
            self.tasks = self.db.get_tasks_by_list(&list_id);
        } else {
            self.tasks.clear();
        }
        self.apply_filter();
    }

    pub fn refresh_lists(&mut self) {
        self.lists = self.db.get_all_lists();
        if self.selected_list >= self.lists.len() && !self.lists.is_empty() {
            self.selected_list = self.lists.len() - 1;
        }
    }

    pub fn visible_entries(&self) -> Vec<VisibleEntry> {
        let mut entries = Vec::new();
        let mut has_completed = false;

        for &i in &self.filtered_indices {
            if let Some(task) = self.tasks.get(i) {
                if !task.completed {
                    entries.push(VisibleEntry::Task(i));
                }
            }
        }

        if self.show_completed {
            for &i in &self.filtered_indices {
                if let Some(task) = self.tasks.get(i) {
                    if task.completed {
                        if !has_completed {
                            has_completed = true;
                            let count = self.filtered_indices.iter()
                                .filter(|&&j| self.tasks.get(j).map(|t| t.completed).unwrap_or(false))
                                .count();
                            entries.push(VisibleEntry::Separator(format!("completed ({})", count)));
                        }
                        entries.push(VisibleEntry::Task(i));
                    }
                }
            }
        }

        entries
    }

    pub fn selectable_count(&self) -> usize {
        self.visible_entries().iter()
            .filter(|e| matches!(e, VisibleEntry::Task(_)))
            .count()
    }

    pub fn nth_selectable(&self, n: usize) -> Option<usize> {
        self.visible_entries().iter()
            .filter_map(|e| match e { VisibleEntry::Task(i) => Some(*i), _ => None })
            .nth(n)
    }

    pub fn selected_task_data(&self) -> Option<&Task> {
        self.nth_selectable(self.selected_task).and_then(|i| self.tasks.get(i))
    }

    pub fn clamp_selection(&mut self) {
        let count = self.selectable_count();
        if count == 0 {
            self.selected_task = 0;
        } else if self.selected_task >= count {
            self.selected_task = count - 1;
        }
    }

    fn apply_filter(&mut self) {
        if self.search_mode && !self.input.is_empty() {
            let mut scored: Vec<(usize, i64)> = self.tasks.iter().enumerate()
                .filter_map(|(i, task)| {
                    self.matcher.fuzzy_match(&task.content, &self.input).map(|score| (i, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
        } else {
            self.filtered_indices = (0..self.tasks.len()).collect();
        }
        self.clamp_selection();
    }

    // === Actions ===

    pub fn submit_input(&mut self) {
        let text = self.input.trim().to_string();

        match self.view {
            View::Tasks => {
                if self.search_mode {
                    self.search_mode = false;
                    self.input.clear();
                    self.cursor_pos = 0;
                    self.input_mode = InputMode::Normal;
                    self.apply_filter();
                    return;
                }
                if text.is_empty() {
                    return;
                }

                let parsed = parser::parse_task_input(&text);

                if let Some(edit_id) = self.editing_task_id.take() {
                    // Update existing task
                    self.db.update_task(
                        &edit_id,
                        &parsed.content,
                        parsed.due_at,
                        parsed.ping_interval,
                        parsed.priority,
                        parsed.recurrence.as_deref(),
                    );
                    self.status_message = Some(format!("Updated: {}", parsed.content));
                } else if let Some(list_id) = self.current_list_id() {
                    // Create new task
                    self.db.create_task(
                        &list_id,
                        &parsed.content,
                        parsed.due_at,
                        parsed.ping_interval,
                        parsed.priority,
                        parsed.recurrence.as_deref(),
                    );
                    self.status_message = Some(format!("Added: {}", parsed.content));
                }

                self.input.clear();
                self.cursor_pos = 0;
                self.refresh_tasks();
                if self.editing_task_id.is_none() {
                    self.selected_task = 0;
                }
            }
            View::NewList => {
                if text.is_empty() { return; }
                self.db.create_list(&text);
                self.refresh_lists();
                self.selected_list = self.lists.len() - 1;
                self.refresh_tasks();
                self.status_message = Some(format!("Created list: {}", text));
                self.input.clear();
                self.cursor_pos = 0;
                self.input_mode = InputMode::Normal;
                self.view = View::Tasks;
            }
            View::RenameList => {
                if text.is_empty() { return; }
                if let Some(list) = self.current_list() {
                    let id = list.id.clone();
                    self.db.rename_list(&id, &text);
                    self.refresh_lists();
                    self.status_message = Some(format!("Renamed to: {}", text));
                }
                self.input.clear();
                self.cursor_pos = 0;
                self.input_mode = InputMode::Normal;
                self.view = View::Tasks;
            }
            _ => {}
        }
    }

    pub fn start_edit(&mut self) {
        if let Some(task) = self.selected_task_data() {
            let reconstructed = parser::reconstruct_task_input(
                &task.content,
                task.due_at,
                task.ping_interval,
                task.priority,
                task.recurrence.as_deref(),
            );
            self.editing_task_id = Some(task.id.clone());
            self.input = reconstructed;
            self.cursor_pos = self.input.len();
            self.input_mode = InputMode::Insert;
            self.search_mode = false;
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing_task_id = None;
        self.input.clear();
        self.cursor_pos = 0;
        self.input_mode = InputMode::Normal;
    }

    pub fn toggle_selected_task(&mut self) {
        if let Some(task) = self.selected_task_data() {
            let id = task.id.clone();
            let was_completed = task.completed;
            let recurrence = task.recurrence.clone();
            let due_at = task.due_at;
            let ping_interval = task.ping_interval;
            let priority = task.priority;
            let content = task.content.clone();

            self.db.toggle_task(&id);

            if !was_completed {
                // Completing a task — check for recurrence
                if let Some(ref rec) = recurrence {
                    if let Some(list_id) = self.db.get_task_list_id(&id) {
                        let next_due = parser::next_recurrence_due(due_at, rec);
                        self.db.create_task(
                            &list_id, &content, next_due, ping_interval, priority, Some(rec),
                        );
                        let due_text = next_due
                            .map(|d| parser::format_due_date(d))
                            .unwrap_or_else(|| "soon".to_string());
                        self.status_message = Some(format!("Done! Next: {}", due_text));
                    }
                } else {
                    self.status_message = Some("Done!".to_string());
                }
            } else {
                self.status_message = Some("Unchecked".to_string());
            }
            self.refresh_tasks();
        }
    }

    pub fn delete_selected_task(&mut self) {
        if let Some(task) = self.selected_task_data() {
            let id = task.id.clone();
            self.db.delete_task(&id);
            self.status_message = Some("Deleted".to_string());
            self.refresh_tasks();
        }
    }

    pub fn delete_current_list(&mut self) {
        if self.lists.len() <= 1 {
            self.status_message = Some("Can't delete the last list".to_string());
            self.view = View::Tasks;
            return;
        }
        if let Some(list) = self.current_list() {
            let id = list.id.clone();
            let name = list.name.clone();
            self.db.delete_list(&id);
            self.status_message = Some(format!("Deleted list: {}", name));
            self.refresh_lists();
            self.refresh_tasks();
        }
        self.view = View::Tasks;
    }

    pub fn snooze_selected(&mut self) {
        if let Some(task) = self.selected_task_data() {
            if task.ping_interval.is_some() {
                let id = task.id.clone();
                let interval_text = task.ping_interval
                    .map(|i| parser::format_ping_interval(i))
                    .unwrap_or_default();
                self.db.snooze_task(&id);
                self.status_message = Some(format!("Snoozed for {}", interval_text));
                self.refresh_tasks();
            } else {
                self.status_message = Some("No ping to snooze".to_string());
            }
        }
    }

    pub fn move_task_up(&mut self) {
        if self.selected_task == 0 { return; }
        let cur = self.nth_selectable(self.selected_task);
        let prev = self.nth_selectable(self.selected_task - 1);
        if let (Some(ci), Some(pi)) = (cur, prev) {
            let ct = &self.tasks[ci];
            let pt = &self.tasks[pi];
            if ct.completed == pt.completed && ct.priority == pt.priority {
                let (cid, pid) = (ct.id.clone(), pt.id.clone());
                self.db.swap_sort_order(&cid, &pid);
                self.selected_task -= 1;
                self.refresh_tasks();
            } else {
                self.status_message = Some("Can't reorder across priority/completion groups".to_string());
            }
        }
    }

    pub fn move_task_down(&mut self) {
        let count = self.selectable_count();
        if count == 0 || self.selected_task >= count - 1 { return; }
        let cur = self.nth_selectable(self.selected_task);
        let next = self.nth_selectable(self.selected_task + 1);
        if let (Some(ci), Some(ni)) = (cur, next) {
            let ct = &self.tasks[ci];
            let nt = &self.tasks[ni];
            if ct.completed == nt.completed && ct.priority == nt.priority {
                let (cid, nid) = (ct.id.clone(), nt.id.clone());
                self.db.swap_sort_order(&cid, &nid);
                self.selected_task += 1;
                self.refresh_tasks();
            } else {
                self.status_message = Some("Can't reorder across priority/completion groups".to_string());
            }
        }
    }

    // === Navigation ===

    pub fn move_selection_up(&mut self) {
        if self.selected_task > 0 { self.selected_task -= 1; }
    }

    pub fn move_selection_down(&mut self) {
        let count = self.selectable_count();
        if count > 0 && self.selected_task < count - 1 { self.selected_task += 1; }
    }

    pub fn move_selection_top(&mut self) { self.selected_task = 0; }

    pub fn move_selection_bottom(&mut self) {
        let count = self.selectable_count();
        if count > 0 { self.selected_task = count - 1; }
    }

    pub fn next_list(&mut self) {
        if !self.lists.is_empty() {
            self.selected_list = (self.selected_list + 1) % self.lists.len();
            self.selected_task = 0;
            self.scroll_offset = 0;
            self.refresh_tasks();
        }
    }

    pub fn prev_list(&mut self) {
        if !self.lists.is_empty() {
            self.selected_list = if self.selected_list == 0 { self.lists.len() - 1 } else { self.selected_list - 1 };
            self.selected_task = 0;
            self.scroll_offset = 0;
            self.refresh_tasks();
        }
    }

    // === Search ===

    pub fn start_search(&mut self) {
        self.search_mode = true;
        self.input_mode = InputMode::Insert;
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn cancel_search(&mut self) {
        self.search_mode = false;
        self.input.clear();
        self.cursor_pos = 0;
        self.input_mode = InputMode::Normal;
        self.apply_filter();
    }

    pub fn on_input_changed(&mut self) {
        if self.search_mode { self.apply_filter(); }
    }

    // === Text editing ===

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.on_input_changed();
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos].chars().last().map(|c| c.len_utf8()).unwrap_or(0);
            self.input.drain(self.cursor_pos - prev..self.cursor_pos);
            self.cursor_pos -= prev;
            self.on_input_changed();
        }
    }

    pub fn delete_char_at_cursor(&mut self) {
        if self.cursor_pos < self.input.len() {
            let next = self.input[self.cursor_pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            self.input.drain(self.cursor_pos..self.cursor_pos + next);
            self.on_input_changed();
        }
    }

    pub fn delete_word_before_cursor(&mut self) {
        if self.cursor_pos == 0 { return; }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        let new_end = trimmed.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
        self.input.drain(new_end..self.cursor_pos);
        self.cursor_pos = new_end;
        self.on_input_changed();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos].chars().last().map(|c| c.len_utf8()).unwrap_or(0);
            self.cursor_pos -= prev;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            let next = self.input[self.cursor_pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            self.cursor_pos += next;
        }
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|t| !t.completed).count()
    }

    /// Check if any tasks with ping intervals need a notification.
    pub fn check_pings(&mut self) {
        if self.last_ping_check.elapsed() < std::time::Duration::from_secs(10) {
            return;
        }
        self.last_ping_check = Instant::now();

        let now = Utc::now().timestamp_millis();
        let tasks = self.db.get_pingable_tasks();

        for task in &tasks {
            let interval_ms = task.ping_interval.unwrap_or(0) * 60 * 1000;
            if interval_ms <= 0 { continue; }

            let baseline = if let Some(last) = task.last_ping_at {
                last
            } else if let Some(due) = task.due_at {
                if now >= due { due } else { continue; }
            } else {
                self.db.update_last_ping_at(&task.id, now);
                continue;
            };

            if now - baseline >= interval_ms {
                let body = if let Some(due) = task.due_at {
                    if parser::is_overdue(due) {
                        format!("{} (overdue)", task.content)
                    } else {
                        format!("{} (due {})", task.content, parser::format_due_date(due))
                    }
                } else {
                    task.content.clone()
                };

                let _ = Notification::new()
                    .summary("Chirp")
                    .body(&body)
                    .sound_name("Glass")
                    .timeout(notify_rust::Timeout::Milliseconds(8000))
                    .show();

                self.db.update_last_ping_at(&task.id, now);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum VisibleEntry {
    Task(usize),
    Separator(String),
}
