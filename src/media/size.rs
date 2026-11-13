pub(crate) fn file_size(bytes: usize) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.;
    let mut unit = 0;
    while value >= 1024. && unit + 1 < UNITS.len() {
        value /= 1024.;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

pub(crate) fn signed_size(delta: i64) -> String {
    let sign = if delta < 0 { "−" } else { "+" };
    format!("{sign}{}", file_size(delta.unsigned_abs() as usize))
}

#[cfg(test)]
mod tests {
    use super::{file_size, signed_size};

    #[test]
    fn sizes_read_in_the_largest_unit_that_fits() {
        assert_eq!(file_size(742), "742 B");
        assert_eq!(file_size(2 * 1024 * 1024 + 400 * 1024), "2.4 MB");
        assert_eq!(signed_size(171), "+171 B");
        assert_eq!(signed_size(-2048), "−2.0 KB");
    }
}
