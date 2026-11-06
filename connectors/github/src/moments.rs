const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;
const STAMP: &[u8; 20] = b"0000-00-00T00:00:00Z";
const DIGIT: u8 = b'0';
const DAYS_PER_ERA: i64 = 146_097;
const DAYS_FROM_YEAR_ZERO_TO_1970: i64 = 719_468;

pub(crate) fn epoch_seconds(stamp: &str) -> Option<i64> {
    let bytes = stamp.as_bytes();
    let shaped = bytes.len() == STAMP.len()
        && bytes.iter().zip(STAMP).all(|(byte, shape)| match *shape {
            DIGIT => byte.is_ascii_digit(),
            separator => *byte == separator,
        });
    if !shaped {
        return None;
    }
    let number = |from: usize, to: usize| stamp[from..to].parse::<i64>().ok();
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute, second) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);
    let valid = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second <= 60;
    valid.then(|| {
        days_since_epoch(year, month, day) * SECONDS_PER_DAY
            + hour * SECONDS_PER_HOUR
            + minute * SECONDS_PER_MINUTE
            + second
    })
}

pub(crate) fn spoken(seconds: i64) -> String {
    let hours = seconds / SECONDS_PER_HOUR;
    let minutes = seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
    let seconds = seconds % SECONDS_PER_MINUTE;
    match (hours, minutes, seconds) {
        (0, 0, seconds) => format!("{seconds}s"),
        (0, minutes, 0) => format!("{minutes}m"),
        (0, minutes, seconds) => format!("{minutes}m {seconds}s"),
        (hours, 0, _) => format!("{hours}h"),
        (hours, minutes, _) => format!("{hours}h {minutes}m"),
    }
}

fn days_since_epoch(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * DAYS_PER_ERA + day_of_era - DAYS_FROM_YEAR_ZERO_TO_1970
}

#[cfg(test)]
mod tests {
    use super::{epoch_seconds, spoken};

    #[test]
    fn a_utc_timestamp_counts_seconds_since_1970() {
        assert_eq!(epoch_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(epoch_seconds("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(epoch_seconds("2026-11-03T18:42:11Z"), Some(1_793_731_331));
        assert_eq!(epoch_seconds("1969-12-31T23:59:59Z"), Some(-1));
    }

    #[test]
    fn leap_days_are_counted() {
        let day = |stamp: &str| epoch_seconds(stamp).unwrap();
        assert_eq!(day("2024-03-01T00:00:00Z") - day("2024-02-28T00:00:00Z"), 2 * 86_400);
        assert_eq!(day("2026-03-01T00:00:00Z") - day("2026-02-28T00:00:00Z"), 86_400);
        assert_eq!(day("2100-03-01T00:00:00Z") - day("2100-02-28T00:00:00Z"), 86_400);
    }

    #[test]
    fn anything_but_a_plain_utc_timestamp_is_not_read() {
        for stamp in [
            "2026-11-03T18:42:11+01:00",
            "2026-11-03T18:42:11.250Z",
            "2026-11-03 18:42:11Z",
            "2026-13-03T18:42:11Z",
            "2026-11-00T18:42:11Z",
            "2026-11-03T24:00:00Z",
            "2026-11-03T18:4a:11Z",
            "+026-11-03T18:42:11Z",
            "",
        ] {
            assert_eq!(epoch_seconds(stamp), None, "{stamp}");
        }
    }

    #[test]
    fn a_duration_is_spoken_in_its_two_largest_units() {
        assert_eq!(spoken(0), "0s");
        assert_eq!(spoken(30), "30s");
        assert_eq!(spoken(72), "1m 12s");
        assert_eq!(spoken(252), "4m 12s");
        assert_eq!(spoken(180), "3m");
        assert_eq!(spoken(3_600), "1h");
        assert_eq!(spoken(3_903), "1h 5m");
    }
}
