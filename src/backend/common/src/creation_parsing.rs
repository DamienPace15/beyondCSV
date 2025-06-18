pub fn parse_boolean(s: &str) -> Option<bool> {
    match s.to_lowercase().trim() {
        "true" | "1" | "yes" | "y" | "t" => Some(true),
        "false" | "0" | "no" | "n" | "f" => Some(false),
        _ => None,
    }
}

pub fn parse_date_to_days(s: &str) -> Option<i32> {
    let s = s.trim();

    // Fast path for ISO format (YYYY-MM-DD)
    if s.len() == 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-') {
        if let Ok(parsed) = parse_date_string(s, "%Y-%m-%d") {
            return Some(parsed);
        }
    }

    // Try other formats
    let formats = ["%m/%d/%Y", "%d/%m/%Y", "%Y/%m/%d"];
    for format in &formats {
        if let Ok(parsed) = parse_date_string(s, format) {
            return Some(parsed);
        }
    }
    None
}

pub fn parse_date_string(date_str: &str, format: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = match format {
        "%Y-%m-%d" => date_str.split('-').collect(),
        "%m/%d/%Y" | "%d/%m/%Y" | "%Y/%m/%d" => date_str.split('/').collect(),
        _ => return Err("Unsupported format".into()),
    };

    if parts.len() != 3 {
        return Err("Invalid date format".into());
    }

    let (year, month, day) = match format {
        "%Y-%m-%d" | "%Y/%m/%d" => (
            parts[0].parse::<i32>()?,
            parts[1].parse::<u32>()?,
            parts[2].parse::<u32>()?,
        ),
        "%m/%d/%Y" => (
            parts[2].parse::<i32>()?,
            parts[0].parse::<u32>()?,
            parts[1].parse::<u32>()?,
        ),
        "%d/%m/%Y" => (
            parts[2].parse::<i32>()?,
            parts[1].parse::<u32>()?,
            parts[0].parse::<u32>()?,
        ),
        _ => return Err("Unsupported format".into()),
    };

    Ok(calculate_days_since_epoch(year, month, day).unwrap_or(0))
}

pub fn calculate_days_since_epoch(year: i32, month: u32, day: u32) -> Option<i32> {
    let epoch_year = 1970;
    let mut days = 0;

    // Add days for complete years
    for y in epoch_year..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Add days for complete months in the current year
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += if m == 2 && is_leap_year(year) {
            29
        } else {
            days_in_month[(m - 1) as usize]
        };
    }

    // Add remaining days
    days += (day - 1) as i32;
    Some(days)
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn parse_datetime_to_nanos(s: &str) -> Option<i64> {
    let s = s.trim();

    // Fast path for Unix timestamps
    if let Ok(timestamp) = s.parse::<i64>() {
        return if timestamp > 10_000_000_000 {
            Some(timestamp * 1_000_000) // ms to ns
        } else {
            Some(timestamp * 1_000_000_000) // s to ns
        };
    }

    // ISO datetime parsing
    parse_iso_datetime(s)
}

pub fn parse_iso_datetime(datetime_str: &str) -> Option<i64> {
    let datetime_str = datetime_str.replace('T', " ");
    let parts: Vec<&str> = datetime_str.split(' ').collect();

    if parts.len() != 2 {
        return None;
    }

    let date_part = parts[0];
    let time_part = parts[1].trim_end_matches('Z');

    // Parse date
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }

    let year = date_parts[0].parse::<i32>().ok()?;
    let month = date_parts[1].parse::<u32>().ok()?;
    let day = date_parts[2].parse::<u32>().ok()?;

    // Parse time
    let time_parts: Vec<&str> = time_part.split(':').collect();
    if time_parts.len() < 2 {
        return None;
    }

    let hour = time_parts[0].parse::<u32>().ok()?;
    let minute = time_parts[1].parse::<u32>().ok()?;
    let (second, nanos) = if time_parts.len() > 2 {
        let sec_parts: Vec<&str> = time_parts[2].split('.').collect();
        let whole_seconds = sec_parts[0].parse::<u32>().ok()?;
        let nanos = if sec_parts.len() > 1 {
            let frac_str = sec_parts[1];
            let frac_str = if frac_str.len() > 9 {
                &frac_str[..9]
            } else {
                frac_str
            };
            let frac_str = format!("{:0<9}", frac_str);
            frac_str.parse::<u32>().unwrap_or(0)
        } else {
            0
        };
        (whole_seconds, nanos)
    } else {
        (0, 0)
    };

    let days = calculate_days_since_epoch(year, month, day)?;
    let total_seconds =
        days as i64 * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    Some(total_seconds * 1_000_000_000 + nanos as i64)
}
