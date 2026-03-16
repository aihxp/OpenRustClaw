//! Timezone-safe execution.

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use openrustclaw_core::error::{Error, Result, SchedulerError};

/// Parse a timezone string into a Tz.
pub fn parse_timezone(tz_str: &str) -> Result<Tz> {
    tz_str.parse::<Tz>().map_err(|_| {
        Error::Scheduler(SchedulerError::InvalidTrigger(format!(
            "Invalid timezone: {}",
            tz_str
        )))
    })
}

/// Convert a UTC datetime to a user's timezone for display.
pub fn to_user_timezone(utc: &DateTime<Utc>, tz: &Tz) -> String {
    utc.with_timezone(tz)
        .format("%Y-%m-%d %H:%M:%S %Z")
        .to_string()
}

/// Calculate an absolute run time from a user-specified time and timezone.
/// E.g., "2024-03-15 15:00" in "America/New_York" -> UTC datetime.
pub fn user_time_to_utc(naive_str: &str, tz: &Tz) -> Result<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(naive_str, "%Y-%m-%d %H:%M").map_err(|e| {
        Error::Scheduler(SchedulerError::InvalidTrigger(format!(
            "Invalid datetime format: {}",
            e
        )))
    })?;

    let local = naive.and_local_timezone(*tz).single().ok_or_else(|| {
        Error::Scheduler(SchedulerError::InvalidTrigger(
            "Ambiguous or invalid local time".to_string(),
        ))
    })?;

    Ok(local.with_timezone(&Utc))
}
