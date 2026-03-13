use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration as StdDuration;

/// Parses end time from command-line arguments.
///
/// # Errors
///
/// Returns an error if the arguments cannot be parsed as a valid time specification.
pub fn parse_end_time(args: &[String], now: DateTime<Local>) -> Result<DateTime<Local>, String> {
    if args.is_empty() {
        return Err("no arguments provided".to_string());
    }

    // 1. Single arg, plain integer -> now + N seconds
    if args.len() == 1
        && let Ok(secs) = args[0].parse::<u64>()
    {
        return Ok(now + Duration::seconds(secs.cast_signed()));
    }

    // 2. One or more tokens with h/m/s suffixes -> sum durations
    {
        let mut total_secs: i64 = 0;
        let mut all_matched = true;
        for token in args {
            if let Some(val) = token.strip_suffix('h') {
                if let Ok(n) = val.parse::<i64>() {
                    total_secs += n * 3600;
                } else {
                    all_matched = false;
                    break;
                }
            } else if let Some(val) = token.strip_suffix('m') {
                if let Ok(n) = val.parse::<i64>() {
                    total_secs += n * 60;
                } else {
                    all_matched = false;
                    break;
                }
            } else if let Some(val) = token.strip_suffix('s') {
                if let Ok(n) = val.parse::<i64>() {
                    total_secs += n;
                } else {
                    all_matched = false;
                    break;
                }
            } else {
                all_matched = false;
                break;
            }
        }
        if all_matched && !args.is_empty() {
            return Ok(now + Duration::seconds(total_secs));
        }
    }

    // All remaining formats expect exactly one argument
    if args.len() != 1 {
        return Err(format!("could not parse arguments: {args:?}"));
    }
    let s = &args[0];

    // 3. HH:MM -> today at that time; if in the past, tomorrow
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M") {
        let naive_dt = now.date_naive().and_time(t);
        let end = Local
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| "failed to convert local datetime".to_string())?;
        return Ok(if end <= now {
            end + Duration::days(1)
        } else {
            end
        });
    }

    // 4. HH:MM:SS -> today at that time; if in the past, tomorrow
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M:%S") {
        let naive_dt = now.date_naive().and_time(t);
        let end = Local
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| "failed to convert local datetime".to_string())?;
        return Ok(if end <= now {
            end + Duration::days(1)
        } else {
            end
        });
    }

    // 5. ISO 8601 with timezone offset: YYYYMMDDThhmmss+HHMM / -HHMM
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y%m%dT%H%M%S%z") {
        return Ok(dt.with_timezone(&Local));
    }

    // 6. ISO 8601 UTC: YYYYMMDDThhmmssZ
    if s.ends_with('Z') {
        let without_z = &s[..s.len() - 1];
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S") {
            let utc_dt = chrono::Utc.from_utc_datetime(&naive);
            return Ok(utc_dt.with_timezone(&Local));
        }
    }

    Err(format!("could not parse argument: {s}"))
}

#[must_use]
pub fn format_eta(end: &DateTime<Local>, now: &DateTime<Local>) -> String {
    let end_date = end.date_naive();
    let now_date = now.date_naive();

    if end_date == now_date {
        end.format("%H:%M:%S").to_string()
    } else if end_date.year() == now_date.year() {
        end.format("%m-%d %H:%M:%S").to_string()
    } else {
        end.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

pub async fn sleep_until_with_progress(end_time: DateTime<Local>) {
    let start_time = Local::now();
    let total_ms = (end_time - start_time).num_milliseconds().max(1000);
    let total_secs = u64::try_from(total_ms).unwrap_or(1000).div_ceil(1000);

    let pb = ProgressBar::new(total_secs);
    pb.set_style(
        ProgressStyle::with_template("⠿ [{bar:40.cyan/blue}] {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("█░"),
    );

    let eta_str = format_eta(&end_time, &Local::now());
    pb.set_message(format!(
        "{:02}:{:02}:{:02} | ETA {eta_str}",
        total_secs / 3600,
        (total_secs % 3600) / 60,
        total_secs % 60,
    ));

    let mut last_elapsed_secs: u64 = u64::MAX;
    let mut interval = tokio::time::interval(StdDuration::from_millis(50));
    loop {
        interval.tick().await;
        let remaining = (end_time - Local::now()).num_milliseconds();
        if remaining <= 0 {
            break;
        }
        let elapsed_secs =
            u64::try_from((Local::now() - start_time).num_seconds().max(0)).unwrap_or(0);
        if elapsed_secs == last_elapsed_secs {
            continue;
        }
        last_elapsed_secs = elapsed_secs;
        pb.set_position(elapsed_secs.min(total_secs));
        let remaining_secs = (remaining + 999) / 1000;
        let eta_str = format_eta(&end_time, &Local::now());
        pb.set_message(format!(
            "{:02}:{:02}:{:02} | ETA {eta_str}",
            remaining_secs / 3600,
            (remaining_secs % 3600) / 60,
            remaining_secs % 60,
        ));
    }
    pb.finish();
}

async fn sleep_until_without_progress(end_time: DateTime<Local>) {
    let remaining = (end_time - Local::now()).num_milliseconds();
    if remaining > 0 {
        tokio::time::sleep(StdDuration::from_millis(
            u64::try_from(remaining).unwrap_or(0),
        ))
        .await;
    }
}

pub async fn sleep_until(end_time: DateTime<Local>, quiet: bool) {
    if quiet {
        sleep_until_without_progress(end_time).await;
    } else {
        sleep_until_with_progress(end_time).await;
    }
}

#[must_use]
pub fn split_args(raw: &[String]) -> (bool, Vec<String>) {
    let quiet = raw.iter().any(|a| a == "-q" || a == "--quiet");
    let time_args = raw
        .iter()
        .filter(|a| *a != "-q" && *a != "--quiet")
        .cloned()
        .collect();
    (quiet, time_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    fn now_fixed() -> DateTime<Local> {
        // Fixed reference: 2026-02-20 10:00:00 local time
        Local
            .with_ymd_and_hms(2026, 2, 20, 10, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("fixed test time 2026-02-20 10:00:00 must be valid"))
    }

    #[test]
    fn test_seconds_only() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["10"]), now)?;
        assert_eq!((end - now).num_seconds(), 10);
        Ok(())
    }

    #[test]
    fn test_zero_seconds() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["0"]), now)?;
        assert_eq!((end - now).num_seconds(), 0);
        Ok(())
    }

    #[test]
    fn test_hours() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["2h"]), now)?;
        assert_eq!((end - now).num_seconds(), 7200);
        Ok(())
    }

    #[test]
    fn test_minutes() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["5m"]), now)?;
        assert_eq!((end - now).num_seconds(), 300);
        Ok(())
    }

    #[test]
    fn test_seconds_unit() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["30s"]), now)?;
        assert_eq!((end - now).num_seconds(), 30);
        Ok(())
    }

    #[test]
    fn test_hours_minutes() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["2h", "5m"]), now)?;
        assert_eq!((end - now).num_seconds(), 7500);
        Ok(())
    }

    #[test]
    fn test_minutes_seconds() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["5m", "30s"]), now)?;
        assert_eq!((end - now).num_seconds(), 330);
        Ok(())
    }

    #[test]
    fn test_hours_minutes_seconds() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["1h", "30m", "45s"]), now)?;
        assert_eq!((end - now).num_seconds(), 5445);
        Ok(())
    }

    #[test]
    fn test_hhmm_future() -> Result<(), String> {
        // now = 10:00:00, target = 12:30 -> same day
        let now = now_fixed();
        let end = parse_end_time(&args(&["12:30"]), now)?;
        assert_eq!(end.date_naive(), now.date_naive());
        assert_eq!(end.format("%H:%M:%S").to_string(), "12:30:00");
        Ok(())
    }

    #[test]
    fn test_hhmm_past() -> Result<(), String> {
        // now = 10:00:00, target = 08:00 -> next day
        let now = now_fixed();
        let end = parse_end_time(&args(&["08:00"]), now)?;
        let expected_date = now.date_naive() + Duration::days(1);
        assert_eq!(end.date_naive(), expected_date);
        assert_eq!(end.format("%H:%M:%S").to_string(), "08:00:00");
        Ok(())
    }

    #[test]
    fn test_hhmmss_future() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["12:30:45"]), now)?;
        assert_eq!(end.date_naive(), now.date_naive());
        assert_eq!(end.format("%H:%M:%S").to_string(), "12:30:45");
        Ok(())
    }

    #[test]
    fn test_hhmmss_past() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["08:00:00"]), now)?;
        let expected_date = now.date_naive() + Duration::days(1);
        assert_eq!(end.date_naive(), expected_date);
        Ok(())
    }

    #[test]
    fn test_iso8601_with_tz() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["20260220T123000+0900"]), now)?;
        // UTC+9 12:30:00 -> UTC 03:30:00
        let utc = end.with_timezone(&chrono::Utc);
        assert_eq!(utc.format("%H:%M:%S").to_string(), "03:30:00");
        assert_eq!(utc.format("%Y-%m-%d").to_string(), "2026-02-20");
        Ok(())
    }

    #[test]
    fn test_iso8601_utc() -> Result<(), String> {
        let now = now_fixed();
        let end = parse_end_time(&args(&["20260220T123000Z"]), now)?;
        let utc = end.with_timezone(&chrono::Utc);
        assert_eq!(utc.format("%H:%M:%S").to_string(), "12:30:00");
        assert_eq!(utc.format("%Y-%m-%d").to_string(), "2026-02-20");
        Ok(())
    }

    #[test]
    fn test_invalid_arg() {
        let now = now_fixed();
        assert!(parse_end_time(&args(&["abc"]), now).is_err());
    }

    #[test]
    fn test_empty_args() {
        let now = now_fixed();
        assert!(parse_end_time(&args(&[]), now).is_err());
    }

    #[test]
    fn test_invalid_unit_combo() {
        let now = now_fixed();
        assert!(parse_end_time(&args(&["2h", "abc"]), now).is_err());
    }

    // format_eta tests

    fn make_dt(year: i32, month: u32, day: u32, h: u32, m: u32, s: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, h, m, s)
            .single()
            .unwrap_or_else(|| {
                panic!(
                    "test datetime {year}-{month:02}-{day:02} {h:02}:{m:02}:{s:02} must be valid"
                )
            })
    }

    #[test]
    fn test_format_eta_same_day() {
        let now = make_dt(2026, 2, 20, 10, 0, 0);
        let end = make_dt(2026, 2, 20, 14, 30, 45);
        assert_eq!(format_eta(&end, &now), "14:30:45");
    }

    #[test]
    fn test_format_eta_next_day_same_year() {
        let now = make_dt(2026, 2, 20, 10, 0, 0);
        let end = make_dt(2026, 2, 21, 8, 0, 0);
        assert_eq!(format_eta(&end, &now), "02-21 08:00:00");
    }

    #[test]
    fn test_format_eta_next_year() {
        let now = make_dt(2026, 2, 20, 10, 0, 0);
        let end = make_dt(2027, 1, 1, 0, 0, 0);
        assert_eq!(format_eta(&end, &now), "2027-01-01 00:00:00");
    }

    #[test]
    fn test_format_eta_year_boundary() {
        // now = Dec 31, end = Jan 1 next year -> YYYY-MM-DD
        let now = make_dt(2026, 12, 31, 23, 0, 0);
        let end = make_dt(2027, 1, 1, 0, 0, 0);
        assert_eq!(format_eta(&end, &now), "2027-01-01 00:00:00");
    }

    // split_args tests

    #[test]
    fn test_split_args_short_flag_prefix() {
        let raw = args(&["-q", "3"]);
        let (quiet, time_args) = split_args(&raw);
        assert!(quiet);
        assert_eq!(time_args, args(&["3"]));
    }

    #[test]
    fn test_split_args_long_flag_suffix() {
        let raw = args(&["5m", "--quiet"]);
        let (quiet, time_args) = split_args(&raw);
        assert!(quiet);
        assert_eq!(time_args, args(&["5m"]));
    }

    #[test]
    fn test_split_args_no_flag() {
        let raw = args(&["2h", "30m"]);
        let (quiet, time_args) = split_args(&raw);
        assert!(!quiet);
        assert_eq!(time_args, args(&["2h", "30m"]));
    }

    #[test]
    fn test_split_args_flag_between() {
        let raw = args(&["1h", "-q", "30m"]);
        let (quiet, time_args) = split_args(&raw);
        assert!(quiet);
        assert_eq!(time_args, args(&["1h", "30m"]));
    }

    // sleep_until_without_progress tests

    #[tokio::test]
    async fn test_sleep_until_without_progress_past() {
        let past = Local::now() - Duration::seconds(1);
        // should return immediately without panicking
        sleep_until_without_progress(past).await;
    }

    #[tokio::test]
    async fn test_sleep_until_without_progress_near_future() {
        let future = Local::now() + Duration::milliseconds(100);
        let start = std::time::Instant::now();
        sleep_until_without_progress(future).await;
        assert!(start.elapsed() < std::time::Duration::from_millis(500));
    }
}
