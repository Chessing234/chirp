use chrono::{Datelike, Duration, Local, TimeZone, Timelike, Weekday};
use regex::Regex;

pub struct ParsedTask {
    pub content: String,
    pub due_at: Option<i64>,
    pub ping_interval: Option<i64>, // minutes
}

pub fn parse_task_input(input: &str) -> ParsedTask {
    let mut content = input.trim().to_string();
    let now = Local::now();
    let mut due_at: Option<i64> = None;
    let mut ping_interval: Option<i64> = None;

    // Time patterns (order matters - more specific first)
    let patterns: Vec<(Regex, Box<dyn Fn(&regex::Captures) -> i64>)> = vec![
        // "tomorrow 5pm" or "tomorrow 5:30pm"
        (
            Regex::new(r"(?i)\btomorrow\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\b").unwrap(),
            Box::new(move |caps| {
                let tomorrow = now + Duration::days(1);
                let hours = resolve_hours(
                    caps[1].parse().unwrap(),
                    caps.get(3).map(|m| m.as_str()),
                );
                let minutes: u32 = caps.get(2).map(|m| m.as_str().parse().unwrap()).unwrap_or(0);
                tomorrow
                    .date_naive()
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
                tomorrow
                    .date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
        // "today 5pm"
        (
            Regex::new(r"(?i)\btoday\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\b").unwrap(),
            Box::new(move |caps| {
                let hours = resolve_hours(
                    caps[1].parse().unwrap(),
                    caps.get(3).map(|m| m.as_str()),
                );
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
                next_week
                    .date_naive()
                    .and_hms_opt(9, 0, 0)
                    .map(|dt| Local.from_local_datetime(&dt).unwrap().timestamp_millis())
                    .unwrap_or(0)
            }),
        ),
        // Day names
        (
            Regex::new(r"(?i)\b(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b")
                .unwrap(),
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
        Regex::new(r"(?i)\bping\s+(?:every\s+)?(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b")
            .unwrap(),
        Regex::new(r"(?i)\bevery\s+(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b").unwrap(),
    ];

    for regex in &ping_patterns {
        if let Some(caps) = regex.captures(&content) {
            let amount: i64 = caps[1].parse().unwrap();
            let unit = caps[2].to_lowercase();
            ping_interval = Some(if unit.starts_with('m') {
                amount
            } else {
                amount * 60
            });
            content = regex.replace(&content, "").trim().to_string();
            break;
        }
    }

    // Clean up extra whitespace
    let whitespace = Regex::new(r"\s+").unwrap();
    content = whitespace.replace_all(&content, " ").trim().to_string();

    ParsedTask {
        content,
        due_at,
        ping_interval,
    }
}

fn resolve_hours(hour: u32, period: Option<&str>) -> u32 {
    match period.map(|s| s.to_lowercase()).as_deref() {
        Some("pm") if hour != 12 => hour + 12,
        Some("am") if hour == 12 => 0,
        None if hour < 8 => hour + 12,
        _ => hour,
    }
}

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
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    }
}

pub fn is_overdue(timestamp: i64) -> bool {
    let now = Local::now().timestamp_millis();
    timestamp < now
}
