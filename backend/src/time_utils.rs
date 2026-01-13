use chrono::{DateTime, FixedOffset, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;

#[derive(Clone, Copy)]
enum ParsedTimezone {
    Named(Tz),
    Fixed(FixedOffset),
}

fn parse_fixed_offset(raw: &str) -> Option<FixedOffset> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (sign, rest) = match trimmed.chars().next()? {
        '+' => (1, &trimmed[1..]),
        '-' => (-1, &trimmed[1..]),
        _ => return None,
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let (hours, minutes) = if let Some((h, m)) = rest.split_once(':') {
        (h.parse::<i32>().ok()?, m.parse::<i32>().ok()?)
    } else if rest.len() > 2 {
        let (h, m) = rest.split_at(rest.len() - 2);
        (h.parse::<i32>().ok()?, m.parse::<i32>().ok()?)
    } else {
        (rest.parse::<i32>().ok()?, 0)
    };

    if hours > 14 || minutes > 59 {
        return None;
    }

    let total_seconds = sign * (hours * 3600 + minutes * 60);
    FixedOffset::east_opt(total_seconds)
}

fn parse_timezone(raw: &str) -> Option<ParsedTimezone> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = if trimmed.eq_ignore_ascii_case("utc") || trimmed.eq_ignore_ascii_case("gmt") {
        "UTC".to_string()
    } else if trimmed == "Europe/Kiev" {
        "Europe/Kyiv".to_string()
    } else {
        trimmed.to_string()
    };

    if normalized.eq_ignore_ascii_case("UTC") {
        return FixedOffset::east_opt(0).map(ParsedTimezone::Fixed);
    }

    if normalized.to_uppercase().starts_with("UTC")
        || normalized.to_uppercase().starts_with("GMT")
    {
        let offset = normalized
            .trim_start_matches("UTC")
            .trim_start_matches("GMT");
        if offset.is_empty() {
            return FixedOffset::east_opt(0).map(ParsedTimezone::Fixed);
        }
        if let Some(parsed) = parse_fixed_offset(offset) {
            return Some(ParsedTimezone::Fixed(parsed));
        }
    }

    normalized
        .parse::<Tz>()
        .ok()
        .map(ParsedTimezone::Named)
}

pub fn normalize_timezone(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed == "Europe/Kiev" {
        "Europe/Kyiv".to_string()
    } else if trimmed.eq_ignore_ascii_case("utc") || trimmed.eq_ignore_ascii_case("gmt") {
        "UTC".to_string()
    } else {
        trimmed.to_string()
    };

    parse_timezone(&normalized).map(|_| normalized)
}

pub fn local_components(raw_tz: &str, utc_dt: DateTime<Utc>) -> (NaiveDate, i16, i16) {
    match parse_timezone(raw_tz) {
        Some(ParsedTimezone::Named(tz)) => {
            let local = utc_dt.with_timezone(&tz);
            (
                local.date_naive(),
                local.hour() as i16,
                local.minute() as i16,
            )
        }
        Some(ParsedTimezone::Fixed(offset)) => {
            let local = utc_dt.with_timezone(&offset);
            (
                local.date_naive(),
                local.hour() as i16,
                local.minute() as i16,
            )
        }
        None => (
            utc_dt.date_naive(),
            utc_dt.hour() as i16,
            utc_dt.minute() as i16,
        ),
    }
}

pub fn format_local_time(raw_tz: &str, utc_dt: DateTime<Utc>) -> String {
    let (date, hour, minute) = local_components(raw_tz, utc_dt);
    format!("{} {:02}:{:02}", date, hour, minute)
}
