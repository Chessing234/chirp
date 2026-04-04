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

    /// Returns displayable task entries. Each entry is (original_index, &Task, is_separator).
    /// Separators are not real tasks and should not be selectable.
    pub fn visible_entries(&self) -> Vec<VisibleEntry> {
        let mut entries = Vec::new();
        let mut has_completed = false;

        // Pending tasks
        for &i in &self.filtered_indices {
            if let Some(task) = self.tasks.get(i) {
                if !task.completed {
                    entries.push(VisibleEntry::Task(i));
                }
            }
        }

        // Completed tasks
        if self.show_completed {
            for &i in &self.filtered_indices {
                if let Some(task) = self.tasks.get(i) {
                    if task.completed {
                        if !has_completed {
                            has_completed = true;
                            let count = self
                                .filtered_indices
                                .iter()
                                .filter(|&&j| self.tasks.get(j).map(|t| t.completed).unwrap_or(false))
                                .count();
                            entries.push(VisibleEntry::Separator(format!(
                                "completed ({})",
                                count
                            )));
                        }
                        entries.push(VisibleEntry::Task(i));
                    }
                }
            }
        }

        entries
    }

    /// Count of selectable (non-separator) entries
    pub fn selectable_count(&self) -> usize {
        self.visible_entries()
            .iter()
            .filter(|e| matches!(e, VisibleEntry::Task(_)))
            .count()
    }

    /// Get the nth selectable task (skipping separators)
    pub fn nth_selectable(&self, n: usize) -> Option<usize> {
        self.visible_entries()
            .iter()
            .filter_map(|e| match e {
                VisibleEntry::Task(i) => Some(*i),
                _ => None,
            })
            .nth(n)
    }

    pub fn selected_task_data(&self) -> Option<&Task> {
        self.nth_selectable(self.selected_task)
            .and_then(|i| self.tasks.get(i))
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
            let mut scored: Vec<(usize, i64)> = self
                .tasks
                .iter()
                .enumerate()
                .filter_map(|(i, task)| {
                    self.matcher
                        .fuzzy_match(&task.content, &self.input)
                        .map(|score| (i, score))
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
                    // Enter confirms search and goes to normal mode
                    self.search_mode = false;
                    self.input.clear();
                    self.cursor_pos = 0;
                    self.input_mode = InputMode::Normal;
                    self.apply_filter(); // reset filter so all tasks are visible again
                    return;
                }
                if text.is_empty() {
                    return;
                }
                // Add new task
                if let Some(list_id) = self.current_list_id() {
                    let parsed = parser::parse_task_input(&text);
                    self.db.create_task(
                        &list_id,
                        &parsed.content,
                        parsed.due_at,
                        parsed.ping_interval,
                    );
                    self.status_message = Some(format!("Added: {}", parsed.content));
                    self.input.clear();
                    self.cursor_pos = 0;
                    self.refresh_tasks();
                    // Select the newly added task (it's the first pending task, sorted by created_at DESC)
                    self.selected_task = 0;
                }
            }
            View::NewList => {
                if text.is_empty() {
                    return;
                }
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
                if text.is_empty() {
                    return;
                }
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

    pub fn toggle_selected_task(&mut self) {
        if let Some(task) = self.selected_task_data() {
            let id = task.id.clone();
            let was_completed = task.completed;
            self.db.toggle_task(&id);
            self.status_message = Some(if was_completed {
                "Unchecked".to_string()
            } else {
                "Done!".to_string()
            });
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

    // === Navigation ===

    pub fn move_selection_up(&mut self) {
        if self.selected_task > 0 {
            self.selected_task -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        let count = self.selectable_count();
        if count > 0 && self.selected_task < count - 1 {
            self.selected_task += 1;
        }
    }

    pub fn move_selection_top(&mut self) {
        self.selected_task = 0;
    }

    pub fn move_selection_bottom(&mut self) {
        let count = self.selectable_count();
        if count > 0 {
            self.selected_task = count - 1;
        }
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
            self.selected_list = if self.selected_list == 0 {
                self.lists.len() - 1
            } else {
                self.selected_list - 1
            };
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
        if self.search_mode {
            self.apply_filter();
        }
    }

    // === Text editing ===

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.on_input_changed();
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input.drain(self.cursor_pos - prev..self.cursor_pos);
            self.cursor_pos -= prev;
            self.on_input_changed();
        }
    }

    pub fn delete_char_at_cursor(&mut self) {
        if self.cursor_pos < self.input.len() {
            let next = self.input[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input.drain(self.cursor_pos..self.cursor_pos + next);
            self.on_input_changed();
        }
    }

    pub fn delete_word_before_cursor(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        // Skip trailing whitespace, then delete to previous whitespace
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        let new_end = trimmed
            .rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);
        self.input.drain(new_end..self.cursor_pos);
        self.cursor_pos = new_end;
        self.on_input_changed();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor_pos -= prev;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            let next = self.input[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor_pos += next;
        }
    }

    /// Pending task count for status bar
    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|t| !t.completed).count()
    }

    /// Check if any tasks with ping intervals need a notification.
    /// Called from the main loop; self-throttles to run every ~10 seconds.
    pub fn check_pings(&mut self) {
        if self.last_ping_check.elapsed() < std::time::Duration::from_secs(10) {
            return;
        }
        self.last_ping_check = Instant::now();

        let now = Utc::now().timestamp_millis();
        let tasks = self.db.get_pingable_tasks();

        for task in &tasks {
            let interval_ms = task.ping_interval.unwrap_or(0) * 60 * 1000;
            if interval_ms <= 0 {
                continue;
            }

            // Determine the baseline: last_ping_at if we've pinged before,
            // otherwise due_at if set and already passed, otherwise skip
            // (don't ping before the due time).
            let baseline = if let Some(last) = task.last_ping_at {
                last
            } else if let Some(due) = task.due_at {
                if now >= due {
                    due
                } else {
                    continue; // not due yet
                }
            } else {
                // No due date and never pinged — start pinging from now.
                // Record current time so the first ping fires after one interval.
                self.db.update_last_ping_at(&task.id, now);
                continue;
            };

            if now - baseline >= interval_ms {
                // Fire notification
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
                    .timeout(notify_rust::Timeout::Milliseconds(8000))
                    .show();

                self.db.update_last_ping_at(&task.id, now);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum VisibleEntry {
    Task(usize), // index into app.tasks
    Separator(String),
}
