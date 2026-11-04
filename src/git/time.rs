const SECONDS_PER_DAY: i64 = 86_400;

pub(crate) fn format_time(seconds: i64, offset: i32) -> String {
    let local = seconds + i64::from(offset);
    let (year, month, day) = civil_from_days(local.div_euclid(SECONDS_PER_DAY));
    let minutes = local.rem_euclid(SECONDS_PER_DAY) / 60;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        minutes / 60,
        minutes % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = (if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    }) as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::format_time;

    #[test]
    fn times_use_the_authors_offset() {
        assert_eq!(format_time(0, 0), "1970-01-01 00:00");
        assert_eq!(format_time(1_793_707_200, 0), "2026-11-03 12:00");
        assert_eq!(format_time(1_793_707_200, 3_600), "2026-11-03 13:00");
        assert_eq!(format_time(1_793_707_200, -13 * 3_600), "2026-11-02 23:00");
        assert_eq!(format_time(1_709_164_800, 0), "2024-02-29 00:00");
    }
}
