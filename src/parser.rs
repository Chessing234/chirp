use chrono::{Datelike, Duration, Local, TimeZone, Timelike, Weekday};
use regex::Regex;

pub struct ParsedTask {
    pub content: String,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>, // minutes
    pub priority: Option<u8>,       // 1, 2, or 3
    pub recurrence: Option<String>, // "daily", "weekly", "monthly"
}

pub fn parse_task_input(input: &str) -> ParsedTask {
    let mut content = input.trim().to_string();
    let now = Local::now();
    let mut due_at: Option<i64> = None;
    let mut ping_interval: Option<i64> = None;
    let mut priority: Option<u8> = None;
    let mut recurrence: Option<String> = None;

    // Priority: p1, p2, p3
    let priority_re = Regex::new(r"(?i)\bp([1-3])\b").unwrap();
    if let Some(caps) = priority_re.captures(&content) {
        priority = Some(caps[1].parse().unwrap());
        content = priority_re.replace(&content, "").trim().to_string();
    }

    // Recurrence: daily, weekly, monthly
    let recurrence_re = Regex::new(r"(?i)\b(daily|weekly|monthly)\b").unwrap();
    if let Some(caps) = recurrence_re.captures(&content) {
        recurrence = Some(caps[1].to_lowercase());
        content = recurrence_re.replace(&content, "").trim().to_string();
    }

    // Time patterns (order matters - more specific first)
    type PatternHandler = Vec<(Regex, Box<dyn Fn(&regex::Captures) -> i64>)>;
    let patterns: PatternHandler = vec![
        // "tomorrow 5pm" or "tomorrow 5:30pm"
        (
            Regex::new(r"(?i)\btomorrow\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\b").unwrap(),
            Box::new(move |caps| {
                let tomorrow = now + Duration::days(1);
                let hours = resolve_hours(caps[1].parse().unwrap(), caps.get(3).map(|m| m.as_str()));
                let minutes: u32 = caps.get(2).map(|m| m.as_str().parse().unwrap()).unwrap_or(0);
                tomorrow.date_naive()
                    .and_hms_opt(hours, minutes, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
        // "tomorrow"
        (
            Regex::new(r"(?i)\btomorrow\b").unwrap(),
            Box::new(move |_| {
                let tomorrow = now + Duration::days(1);
                tomorrow.date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
        // "today 5pm"
        (
            Regex::new(r"(?i)\btoday\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\b").unwrap(),
            Box::new(move |caps| {
                let hours = resolve_hours(caps[1].parse().unwrap(), caps.get(3).map(|m| m.as_str()));
                let minutes: u32 = caps.get(2).map(|m| m.as_str().parse().unwrap()).unwrap_or(0);
                now.date_naive()
                    .and_hms_opt(hours, minutes, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
        // "at 5pm" or "5pm"
        (
            Regex::new(r"(?i)\b(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*(am|pm)\b").unwrap(),
            Box::new(move |caps| {
                let hours = resolve_hours(caps[1].parse().unwrap(), Some(&caps[3]));
                let minutes: u32 = caps.get(2).map(|m| m.as_str().parse().unwrap()).unwrap_or(0);
                let mut date = now.date_naive();
                if now.hour() > hours || (now.hour() == hours && now.minute() >= minutes) {
                    date += Duration::days(1);
                }
                date.and_hms_opt(hours, minutes, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
        // "in 30m" or "in 2h"
        (
            Regex::new(r"(?i)\bin\s+(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b").unwrap(),
            Box::new(move |caps| {
                let amount: i64 = caps[1].parse().unwrap();
                let unit = caps[2].to_lowercase();
                let duration = if unit.starts_with('m') {
                    Duration::minutes(amount)
                } else {
                    Duration::hours(amount)
                };
                (now + duration).timestamp_millis()
            }),
        ),
        // "next week"
        (
            Regex::new(r"(?i)\bnext\s+week\b").unwrap(),
            Box::new(move |_| {
                let next_week = now + Duration::weeks(1);
                next_week.date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
        // Day names
        (
            Regex::new(r"(?i)\b(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b").unwrap(),
            Box::new(move |caps| {
                let target = match caps[1].to_lowercase().as_str() {
                    "monday" => Weekday::Mon,
                    "tuesday" => Weekday::Tue,
                    "wednesday" => Weekday::Wed,
                    "thursday" => Weekday::Thu,
                    "friday" => Weekday::Fri,
                    "saturday" => Weekday::Sat,
                    "sunday" => Weekday::Sun,
                    _ => unreachable!(),
                };
                let current = now.weekday();
                let mut days_ahead =
                    (target.num_days_from_sunday() as i64) - (current.num_days_from_sunday() as i64);
                if days_ahead <= 0 {
                    days_ahead += 7;
                }
                let date = now + Duration::days(days_ahead);
                date.date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
    ];

    for (regex, handler) in &patterns {
        if let Some(caps) = regex.captures(&content) {
            due_at = Some(handler(&caps));
            content = regex.replace(&content, "").trim().to_string();
            break;
        }
    }

    // Ping patterns
    let ping_patterns = vec![
        Regex::new(r"(?i)\bping\s+(?:every\s+)?(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b").unwrap(),
        Regex::new(r"(?i)\bevery\s+(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b").unwrap(),
    ];

    for regex in &ping_patterns {
        if let Some(caps) = regex.captures(&content) {
            let amount: i64 = caps[1].parse().unwrap();
            let unit = caps[2].to_lowercase();
            ping_interval = Some(if unit.starts_with('m') { amount } else { amount * 60 });
            content = regex.replace(&content, "").trim().to_string();
            break;
        }
    }

    // Clean up extra whitespace
    let whitespace = Regex::new(r"\s+").unwrap();
    content = whitespace.replace_all(&content, " ").trim().to_string();

    // If recurrence is set but no due_at, set a sensible default
    if recurrence.is_some() && due_at.is_none() {
        due_at = Some(
            match recurrence.as_deref() {
                Some("daily") => (now + Duration::days(1))
                    .date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0),
                Some("weekly") => (now + Duration::weeks(1))
                    .date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0),
                Some("monthly") => (now + Duration::days(30))
                    .date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0),
                _ => 0,
            }
        );
    }

    ParsedTask { content, due_at, ping_interval, priority, recurrence }
}

fn resolve_hours(hour: u32, period: Option<&str>) -> u32 {
    match period.map(|s| s.to_lowercase()).as_deref() {
        Some("pm") if hour != 12 => hour + 12,
        Some("am") if hour == 12 => 0,
        None if hour < 8 => hour + 12,
        _ => hour,
    }
}

// === Formatting ===

pub fn format_due_date(timestamp: i64) -> String {
    let date = Local.timestamp_millis_opt(timestamp).unwrap();
    let now = Local::now();
    let tomorrow = now + Duration::days(1);
    let time_str = date.format("%-I:%M %p").to_string();

    if date.date_naive() == now.date_naive() {
        format!("Today {}", time_str)
    } else if date.date_naive() == tomorrow.date_naive() {
        format!("Tomorrow {}", time_str)
    } else {
        format!("{} {}", date.format("%a %b %-d"), time_str)
    }
}

pub fn format_ping_interval(minutes: i64) -> String {
    if minutes < 60 {
        format!("{}m", minutes)
    } else {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins > 0 { format!("{}h{}m", hours, mins) } else { format!("{}h", hours) }
    }
}

pub fn is_overdue(timestamp: i64) -> bool {
    timestamp < Local::now().timestamp_millis()
}

pub fn format_priority(p: u8) -> &'static str {
    match p {
        1 => "p1",
        2 => "p2",
        3 => "p3",
        _ => "p?",
    }
}

/// Time remaining until next ping fires. Returns None if no countdown is active.
pub fn ping_countdown(last_ping_at: Option<i64>, ping_interval: Option<i64>, due_at: Option<i64>) -> Option<String> {
    let interval = ping_interval?;
    let interval_ms = interval * 60 * 1000;
    let now = Local::now().timestamp_millis();

    let baseline = if let Some(last) = last_ping_at {
        last
    } else if let Some(due) = due_at {
        if now >= due { due } else { return Some("at due".to_string()); }
    } else {
        return None;
    };

    let next = baseline + interval_ms;
    let remaining_ms = next - now;

    if remaining_ms <= 0 {
        Some("now!".to_string())
    } else {
        let remaining_min = remaining_ms / 60_000;
        if remaining_min >= 60 {
            Some(format!("{}h{}m", remaining_min / 60, remaining_min % 60))
        } else {
            Some(format!("{}m", remaining_min.max(1)))
        }
    }
}

// === Reconstruction for edit mode ===

pub fn reconstruct_task_input(
    content: &str,
    due_at: Option<i64>,
    ping_interval: Option<i64>,
    priority: Option<u8>,
    recurrence: Option<&str>,
) -> String {
    let mut parts = vec![content.to_string()];

    if let Some(due) = due_at {
        parts.push(reconstruct_due_text(due));
    }

    if let Some(interval) = ping_interval {
        parts.push(format!("ping {}", format_ping_interval(interval)));
    }

    if let Some(p) = priority {
        parts.push(format!("p{}", p));
    }

    if let Some(r) = recurrence {
        parts.push(r.to_string());
    }

    parts.join(" ")
}

fn reconstruct_due_text(timestamp: i64) -> String {
    let date = Local.timestamp_millis_opt(timestamp).unwrap();
    let now = Local::now();
    let tomorrow = now + Duration::days(1);

    let time_part = if date.minute() == 0 {
        date.format("%-I%p").to_string().to_lowercase()
    } else {
        date.format("%-I:%M%p").to_string().to_lowercase()
    };

    if date.date_naive() == now.date_naive() {
        format!("today {}", time_part)
    } else if date.date_naive() == tomorrow.date_naive() {
        format!("tomorrow {}", time_part)
    } else {
        format!("{} {}", date.format("%A").to_string().to_lowercase(), time_part)
    }
}

// === Recurrence ===

pub fn next_recurrence_due(current_due: Option<i64>, recurrence: &str) -> Option<i64> {
    let base = if let Some(due) = current_due {
        Local.timestamp_millis_opt(due).unwrap()
    } else {
        Local::now()
    };

    let next = match recurrence {
        "daily" => base + Duration::days(1),
        "weekly" => base + Duration::weeks(1),
        "monthly" => base + Duration::days(30),
        _ => return None,
    };

    Some(next.timestamp_millis())
}

/// Parse a ping interval string like "30m" or "2h" into minutes.
pub fn parse_ping_str(s: &str) -> Option<i64> {
    let re = Regex::new(r"(?i)^(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)$").unwrap();
    let caps = re.captures(s.trim())?;
    let amount: i64 = caps[1].parse().ok()?;
    let unit = caps[2].to_lowercase();
    Some(if unit.starts_with('m') { amount } else { amount * 60 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modifiers_combined() {
        let parsed = parse_task_input("buy groceries p1 daily ping 1h tomorrow 5pm");
        assert_eq!(parsed.priority, Some(1));
        assert_eq!(parsed.recurrence, Some("daily".to_string()));
        assert_eq!(parsed.ping_interval, Some(60));
        assert!(parsed.due_at.is_some());
        assert_eq!(parsed.content, "buy groceries");
    }

    #[test]
    fn test_modifiers_different_order() {
        let parsed = parse_task_input("ping 30m daily call mom p2 tomorrow");
        assert_eq!(parsed.priority, Some(2));
        assert_eq!(parsed.recurrence, Some("daily".to_string()));
        assert_eq!(parsed.ping_interval, Some(30));
        assert!(parsed.due_at.is_some());
        assert_eq!(parsed.content, "call mom");
    }

    #[test]
    fn test_plain_task_no_modifiers() {
        let parsed = parse_task_input("buy milk");
        assert_eq!(parsed.content, "buy milk");
        assert!(parsed.priority.is_none());
        assert!(parsed.recurrence.is_none());
        assert!(parsed.ping_interval.is_none());
        assert!(parsed.due_at.is_none());
    }

    #[test]
    fn test_priority_levels() {
        for p in 1..=3 {
            let parsed = parse_task_input(&format!("task p{}", p));
            assert_eq!(parsed.priority, Some(p));
            assert_eq!(parsed.content, "task");
        }
    }

    #[test]
    fn test_recurrence_types() {
        for rec in ["daily", "weekly", "monthly"] {
            let parsed = parse_task_input(&format!("task {}", rec));
            assert_eq!(parsed.recurrence.as_deref(), Some(rec));
            assert_eq!(parsed.content, "task");
            // Recurrence without explicit due should get a default due_at
            assert!(parsed.due_at.is_some());
        }
    }

    #[test]
    fn test_ping_minutes_and_hours() {
        let m = parse_task_input("task ping 45m");
        assert_eq!(m.ping_interval, Some(45));

        let h = parse_task_input("task ping 2h");
        assert_eq!(h.ping_interval, Some(120));
    }

    #[test]
    fn test_relative_time() {
        let parsed = parse_task_input("meeting in 30m");
        assert!(parsed.due_at.is_some());
        assert_eq!(parsed.content, "meeting");

        let parsed2 = parse_task_input("review in 2h");
        assert!(parsed2.due_at.is_some());
    }

    #[test]
    fn test_format_ping_interval() {
        assert_eq!(format_ping_interval(30), "30m");
        assert_eq!(format_ping_interval(60), "1h");
        assert_eq!(format_ping_interval(90), "1h30m");
        assert_eq!(format_ping_interval(1), "1m");
    }

    #[test]
    fn test_format_priority() {
        assert_eq!(format_priority(1), "p1");
        assert_eq!(format_priority(2), "p2");
        assert_eq!(format_priority(3), "p3");
    }

    #[test]
    fn test_recurrence_next_due() {
        let now = Local::now().timestamp_millis();
        let next_daily = next_recurrence_due(Some(now), "daily").unwrap();
        let diff_hours = (next_daily - now) / (1000 * 60 * 60);
        assert_eq!(diff_hours, 24);

        let next_weekly = next_recurrence_due(Some(now), "weekly").unwrap();
        let diff_days = (next_weekly - now) / (1000 * 60 * 60 * 24);
        assert_eq!(diff_days, 7);

        assert!(next_recurrence_due(Some(now), "invalid").is_none());
    }

    #[test]
    fn test_parse_ping_str() {
        assert_eq!(parse_ping_str("30m"), Some(30));
        assert_eq!(parse_ping_str("2h"), Some(120));
        assert_eq!(parse_ping_str("1hr"), Some(60));
        assert_eq!(parse_ping_str("15min"), Some(15));
        assert_eq!(parse_ping_str("invalid"), None);
        assert_eq!(parse_ping_str(""), None);
    }

    #[test]
    fn test_reconstruct_roundtrip() {
        let result = reconstruct_task_input("buy milk", None, Some(30), Some(2), Some("daily"));
        assert!(result.contains("buy milk"));
        assert!(result.contains("ping 30m"));
        assert!(result.contains("p2"));
        assert!(result.contains("daily"));

        // Reconstruct with no modifiers
        let plain = reconstruct_task_input("simple task", None, None, None, None);
        assert_eq!(plain, "simple task");
    }

    #[test]
    fn test_tomorrow_with_time() {
        let parsed = parse_task_input("call mom tomorrow 3pm");
        assert!(parsed.due_at.is_some());
        assert_eq!(parsed.content, "call mom");
    }

    #[test]
    fn test_day_names() {
        for day in ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"] {
            let parsed = parse_task_input(&format!("task {}", day));
            assert!(parsed.due_at.is_some(), "failed for {}", day);
            assert_eq!(parsed.content, "task");
        }
    }

    #[test]
    fn test_whitespace_cleanup() {
        let parsed = parse_task_input("  lots   of   spaces   p1  ");
        assert_eq!(parsed.content, "lots of spaces");
        assert_eq!(parsed.priority, Some(1));
    }
}
