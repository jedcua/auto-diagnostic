use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use clap::Parser;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Automatically performs diagnosis on your AWS environment with AI
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Configuration file to use
    pub file: String,

    /// Duration in seconds, since the current date time
    #[arg(long, default_value_t = 3600)]
    pub duration: u64,

    /// Start time [format: %Y-%m-%d %H:%M:%S].
    /// If provided, ignores duration argument
    #[arg(long)]
    pub start: Option<String>,

    /// End time [format: %Y-%m-%d %H:%M:%S].
    /// If provided, ignores duration argument
    #[arg(long)]
    pub end: Option<String>,

    /// Print the raw prompt data
    #[arg(long, default_value_t = false)]
    pub print_prompt_data: bool,

    /// Dry run mode, don't generate diagnosis
    #[arg(long, default_value_t = false)]
    pub dry_run: bool
}

pub fn build_start_and_end(args: &Args, time_zone: Tz) -> Result<(Duration, Duration), Box<dyn Error>> {
    let start_time: Duration;
    let end_time: Duration;

    match (&args.duration, &args.start, &args.end) {
        // start & end are both present
        (_, Some(s), Some(e)) => {
            start_time = parse_date_time(s, &time_zone)?;
            end_time = parse_date_time(e, &time_zone)?;
        },
        // start & end are both missing, use duration argument
        (_, None, None) => {
            end_time = SystemTime::now().duration_since(UNIX_EPOCH)?;
            start_time = end_time.checked_sub(Duration::from_secs(args.duration)).unwrap();
        }
        _ => {
            panic!("Both start and end arguments must be provided");
        }
    }

    Ok((start_time, end_time))
}

fn parse_date_time(s: &str, time_zone: &Tz) -> Result<Duration, Box<dyn Error>> {
    let format = "%Y-%m-%d %H:%M:%S";

    Ok(NaiveDateTime::parse_from_str(s, format)
        .map(|ndt| time_zone.from_local_datetime(&ndt).unwrap())
        .map(|dt| Duration::from_millis(dt.timestamp_millis() as u64))?)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ops::Sub;
    use std::panic;

    #[test]
    fn test_build_start_and_end_using_duration() {
        let args = Args {
            file: String::new(),
            duration: 100,
            start: None,
            end: None,
            print_prompt_data: false,
            dry_run: false,
        };
        let (start, end) = build_start_and_end(&args, Tz::UTC)
            .expect("Should not return an error");
        let diff = end.sub(start).as_secs();

        assert_eq!(args.duration, diff)
    }

    #[test]
    fn test_build_start_and_end_using_range() {
        let args = Args {
            file: String::new(),
            duration: 0,
            start: Some(String::from("2024-01-01 12:00:00")),
            end: Some(String::from("2024-01-02 12:00:00")),
            print_prompt_data: false,
            dry_run: false,
        };
        let (start, end) = build_start_and_end(&args, Tz::UTC)
            .expect("Should not return an error");
        let diff = end.sub(start).as_secs();

        assert_eq!(86400, diff)
    }

    #[test]
    fn test_build_start_and_end_using_range_should_have_both_duration() {
        let args = Args {
            file: String::new(),
            duration: 0,
            start: Some(String::from("2024-01-01 12:00:00")),
            end: None,
            print_prompt_data: false,
            dry_run: false,
        };

        let result = panic::catch_unwind(|| {
            build_start_and_end(&args, Tz::UTC)
        });

        assert!(result.is_err())
    }
}
