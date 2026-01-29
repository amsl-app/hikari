use crate::date::error::DateError;
use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime, NaiveTime, TimeZone};
use std::ops::Add;

pub mod error;

/// Calculate the start and end of a day in UTC time.
pub fn get_day_bounds(date: DateTime<FixedOffset>) -> Result<(NaiveDateTime, NaiveDateTime), DateError> {
    let local_day = date.naive_local().date();
    // Explicit calls to make jetbrains linter happy
    let from_date = date
        .timezone()
        .from_local_datetime(&local_day.and_time(NaiveTime::default()))
        .earliest()
        .ok_or(DateError::InvalidLocalTime)?
        .naive_utc();
    let to_date = date
        .timezone()
        .from_local_datetime(
            &local_day
                .and_time(NaiveTime::default())
                .add(Duration::try_days(1).ok_or(DateError::Overflow)?),
        )
        .latest()
        .ok_or(DateError::InvalidLocalTime)?
        .naive_utc();
    Ok((from_date, to_date))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_get_day_bounds(date: &str, from: &str, to: &str) {
        let utc_date = DateTime::<FixedOffset>::parse_from_rfc3339(date).unwrap();
        let start = DateTime::<FixedOffset>::parse_from_rfc3339(from).unwrap().naive_utc();
        let end = DateTime::<FixedOffset>::parse_from_rfc3339(to).unwrap().naive_utc();
        assert_eq!(get_day_bounds(utc_date).unwrap(), (start, end));
    }

    #[test]
    fn test_get_day_bounds() {
        check_get_day_bounds("2023-11-03 12:00:00Z", "2023-11-03 00:00:00Z", "2023-11-04 00:00:00Z");
        check_get_day_bounds(
            "2023-11-03 12:00:00+02:00",
            "2023-11-03 00:00:00+02:00",
            "2023-11-04 00:00:00+02:00",
        );
        check_get_day_bounds(
            "2023-11-03 00:00:00+02:00",
            "2023-11-03 00:00:00+02:00",
            "2023-11-04 00:00:00+02:00",
        );
        check_get_day_bounds(
            "2023-11-03 00:00:00+02:00",
            "2023-11-02 22:00:00Z",
            "2023-11-03 22:00:00Z",
        );
        check_get_day_bounds(
            "2023-11-03 00:00:00+02:00",
            "2023-11-03 00:00:00+02:00",
            "2023-11-04 00:00:00+02:00",
        );
    }
}
