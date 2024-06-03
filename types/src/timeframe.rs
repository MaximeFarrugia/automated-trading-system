use chrono::{Datelike, TimeZone};
use garde::Validate;
use regex::Regex;
use serde::{de::Visitor, Deserialize};

#[derive(Debug, Validate)]
pub enum Timeframe {
    Month(#[garde(range(min = 1, max = 12))] i64),
    Week(#[garde(range(min = 1, max = 52))] i64),
    Day(#[garde(range(min = 1, max = 365))] i64),
    Hour(#[garde(range(min = 1, max = 24))] i64),
    Minute(#[garde(range(min = 1, max = 1440))] i64),
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Timeframe::Month(x) => write!(f, "{x}M"),
            Timeframe::Week(x) => write!(f, "{x}W"),
            Timeframe::Day(x) => write!(f, "{x}D"),
            Timeframe::Hour(x) => write!(f, "{x}h"),
            Timeframe::Minute(x) => write!(f, "{x}m"),
        };
    }
}

impl Timeframe {
    pub fn open_and_size(
        &self,
        date_time: &chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<(chrono::DateTime<chrono::Utc>, i64)> {
        self.validate(&())?;
        let year = date_time.year();
        let month = date_time.month();
        let day = date_time.day();
        let candle_open = match self {
            Timeframe::Month(x) => {
                let open = chrono::Utc
                    .with_ymd_and_hms(
                        year,
                        1 + ((month - 1) / u32::try_from(*x).unwrap()),
                        1,
                        0,
                        0,
                        0,
                    )
                    .unwrap();
                let x = u32::try_from(*x)?;
                let next_open = if open.month() + x > 12 {
                    chrono::Utc
                        .with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0)
                        .unwrap()
                } else {
                    chrono::Utc
                        .with_ymd_and_hms(year, open.month() + x, 1, 0, 0, 0)
                        .unwrap()
                };
                let size_in_millis = next_open.signed_duration_since(open).num_milliseconds();
                (open, size_in_millis)
            }
            Timeframe::Week(x) => {
                let iso_week = date_time.iso_week();
                let open = chrono::NaiveDate::from_isoywd_opt(
                    iso_week.year(),
                    iso_week.week() - (iso_week.week0() % u32::try_from(*x).unwrap()),
                    chrono::Weekday::Mon,
                )
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc();
                let next_year = chrono::NaiveDate::from_isoywd_opt(
                    iso_week.year() + 1,
                    1,
                    chrono::Weekday::Mon,
                )
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc();
                let duration_to = next_year.signed_duration_since(open).num_milliseconds();
                let size_in_millis =
                    duration_to.min(chrono::Duration::days(*x * 7).num_milliseconds());
                (open, size_in_millis)
            }
            Timeframe::Day(x) => {
                let january_first = chrono::Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
                let next_year = chrono::Utc
                    .with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0)
                    .unwrap();
                let duration_since = date_time
                    .signed_duration_since(january_first)
                    .num_milliseconds();
                let open = january_first
                    + chrono::Duration::days(
                        x * (duration_since / chrono::Duration::days(*x).num_milliseconds()),
                    );
                let duration_to = next_year.signed_duration_since(open).num_milliseconds();
                let size_in_millis = duration_to.min(chrono::Duration::days(*x).num_milliseconds());
                (open, size_in_millis)
            }
            Timeframe::Hour(x) => {
                let start_of_day = chrono::Utc
                    .with_ymd_and_hms(year, month, day, 0, 0, 0)
                    .unwrap();
                let next_day = start_of_day + chrono::Duration::days(1);
                let duration_since = date_time
                    .signed_duration_since(start_of_day)
                    .num_milliseconds();
                let open = start_of_day
                    + chrono::Duration::hours(
                        x * (duration_since / chrono::Duration::hours(*x).num_milliseconds()),
                    );
                let duration_to = next_day.signed_duration_since(open).num_milliseconds();
                let size_in_millis =
                    duration_to.min(chrono::Duration::hours(*x).num_milliseconds());
                (open, size_in_millis)
            }
            Timeframe::Minute(x) => {
                let start_of_day = chrono::Utc
                    .with_ymd_and_hms(year, month, day, 0, 0, 0)
                    .unwrap();
                let next_day = start_of_day + chrono::Duration::days(1);
                let duration_since = date_time
                    .signed_duration_since(start_of_day)
                    .num_milliseconds();
                let open = start_of_day
                    + chrono::Duration::minutes(
                        x * (duration_since / chrono::Duration::minutes(*x).num_milliseconds()),
                    );
                let duration_to = next_day.signed_duration_since(open).num_milliseconds();
                let size_in_millis =
                    duration_to.min(chrono::Duration::minutes(*x).num_milliseconds());
                (open, size_in_millis)
            }
        };

        return Ok(candle_open);
    }
}

struct TimeframeVisitor;

impl<'de> Visitor<'de> for TimeframeVisitor {
    type Value = Timeframe;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        return formatter.write_str(
            "a string in the format 'xM', 'xW', 'xD', 'xh' or 'xm' where x is a positive integer",
        );
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let regex = Regex::new(r"^(\d+)([MWDhm])$").map_err(E::custom)?;
        let captures = regex
            .captures(v)
            .ok_or_else(|| E::custom("invalid format"))?;
        let value_str = captures
            .get(1)
            .ok_or_else(|| E::custom("missing number"))?
            .as_str();
        let unit = captures
            .get(2)
            .ok_or_else(|| E::custom("missing unit"))?
            .as_str();
        let value = value_str.parse::<i64>().map_err(E::custom)?;

        let timeframe = match unit {
            "M" => Timeframe::Month(value),
            "W" => Timeframe::Week(value),
            "D" => Timeframe::Day(value),
            "h" => Timeframe::Hour(value),
            "m" => Timeframe::Minute(value),
            _ => return Err(E::custom("invalid unit")),
        };

        timeframe.validate(&()).map_err(E::custom)?;
        return Ok(timeframe);
    }
}

impl<'de> Deserialize<'de> for Timeframe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        return deserializer.deserialize_string(TimeframeVisitor);
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use crate::Timeframe;

    #[test]
    fn error_handling() {
        Timeframe::Minute(0)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Minute(< 0) should not be valid");
        Timeframe::Minute(1)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Minute(>= 1) should be valid");
        Timeframe::Minute(1440)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Minute(<= 1440) should be valid");
        Timeframe::Minute(1441)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Minute(> 1440) should not be valid");

        Timeframe::Hour(0)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Hour(< 0) should not be valid");
        Timeframe::Hour(1)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Hour(>= 1) should be valid");
        Timeframe::Hour(24)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Hour(<= 24) should be valid");
        Timeframe::Hour(25)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Hour(> 24) should not be valid");

        Timeframe::Day(0)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Day(< 0) should not be valid");
        Timeframe::Day(1)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Day(>= 1) should be valid");
        Timeframe::Day(365)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Day(<= 365 should be valid");
        Timeframe::Day(366)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Day(> 365 should not be valid");

        Timeframe::Week(0)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Week(< 0) should not be valid");
        Timeframe::Week(1)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Week(>= 1) should be valid");
        Timeframe::Week(52)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Week(<= 52 should be valid");
        Timeframe::Week(53)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Week(> 52 should not be valid");

        Timeframe::Month(0)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Month(< 0) should not be valid");
        Timeframe::Month(1)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Month(>= 1) should be valid");
        Timeframe::Month(12)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect("Timeframe::Month(<= 12 should be valid");
        Timeframe::Month(13)
            .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
            .expect_err("Timeframe::Month(> 12 should not be valid");
    }

    #[test]
    fn candle_open_minutes() {
        assert_eq!(
            Timeframe::Minute(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::minutes(1).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Minute(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 36, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 36, 0).unwrap(),
                chrono::Duration::minutes(1).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Minute(5)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 3, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::minutes(5).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Minute(5)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 35, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 35, 0).unwrap(),
                chrono::Duration::minutes(5).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Minute(138)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 1, 15, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::minutes(138).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Minute(138)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 3, 0, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 2, 18, 0).unwrap(),
                chrono::Duration::minutes(138).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Minute(138)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 23, 0, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 23, 0, 0).unwrap(),
                chrono::Duration::minutes(60).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Minute(138)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                chrono::Duration::minutes(138).num_milliseconds()
            ),
        );
    }

    #[test]
    fn candle_open_days() {
        assert_eq!(
            Timeframe::Day(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(1).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Day(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(1).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Day(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(2).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Day(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 31, 0, 0, 0).unwrap(),
                chrono::Duration::days(2).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Day(365)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap(),
                chrono::Duration::days(1).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Day(365)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2023, 12, 31, 0, 0, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(365).num_milliseconds()
            ),
        );
    }

    #[test]
    fn candle_open_hours() {
        assert_eq!(
            Timeframe::Hour(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::hours(1).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Hour(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 0).unwrap(),
                chrono::Duration::hours(1).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Hour(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::hours(2).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Hour(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::hours(2).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Hour(24)
                .open_and_size(
                    &chrono::Utc
                        .with_ymd_and_hms(2024, 1, 1, 23, 59, 59)
                        .unwrap()
                )
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::hours(24).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Hour(24)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                chrono::Duration::hours(24).num_milliseconds()
            ),
        );
    }

    #[test]
    fn candle_open_months() {
        assert_eq!(
            Timeframe::Month(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(31).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Month(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 00).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(29).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Month(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(60).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Month(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(60).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Month(12)
                .open_and_size(
                    &chrono::Utc
                        .with_ymd_and_hms(2024, 1, 1, 23, 59, 59)
                        .unwrap()
                )
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(366).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Month(12)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(366).num_milliseconds()
            ),
        );
    }

    #[test]
    fn candle_open_weeks() {
        assert_eq!(
            Timeframe::Week(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(7).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 8, 10, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap(),
                chrono::Duration::days(7).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(1)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 00).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2022, 12, 26, 0, 0, 0).unwrap(),
                chrono::Duration::days(7).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Week(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(14).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(14).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(2)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2022, 12, 19, 0, 0, 0).unwrap(),
                chrono::Duration::days(14).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Week(7)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(49).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(7)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 2, 18, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(49).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(7)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 2, 19, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 2, 19, 0, 0, 0).unwrap(),
                chrono::Duration::days(49).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(7)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2022, 12, 12, 0, 0, 0).unwrap(),
                chrono::Duration::days(21).num_milliseconds()
            ),
        );

        assert_eq!(
            Timeframe::Week(52)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                chrono::Duration::days(364).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(52)
                .open_and_size(
                    &chrono::Utc
                        .with_ymd_and_hms(2024, 12, 30, 0, 0, 36)
                        .unwrap()
                )
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2024, 12, 30, 0, 0, 0).unwrap(),
                chrono::Duration::days(364).num_milliseconds()
            ),
        );
        assert_eq!(
            Timeframe::Week(52)
                .open_and_size(&chrono::Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 36).unwrap())
                .unwrap(),
            (
                chrono::Utc.with_ymd_and_hms(2022, 1, 3, 0, 0, 0).unwrap(),
                chrono::Duration::days(364).num_milliseconds()
            ),
        );
    }
}
