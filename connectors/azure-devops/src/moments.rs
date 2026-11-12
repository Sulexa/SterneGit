use serde_json::Value;

const WHOLE_SECONDS: usize = "2026-11-03T18:50:12".len();

pub(crate) fn moment(value: &Value) -> Option<String> {
    let stamp = value.as_str()?;
    let shaped = stamp.len() >= WHOLE_SECONDS
        && stamp.is_char_boundary(WHOLE_SECONDS)
        && stamp.as_bytes()[10] == b'T'
        && stamp.ends_with('Z');
    match shaped {
        true => Some(format!("{}Z", &stamp[..WHOLE_SECONDS])),
        false => Some(stamp.to_owned()),
    }
    .filter(|stamp| !stamp.starts_with("0001-01-01"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::moment;

    #[test]
    fn server_times_drop_their_fractions_of_a_second() {
        assert_eq!(moment(&json!("2026-11-03T18:50:12.4372361Z")).as_deref(), Some("2026-11-03T18:50:12Z"));
        assert_eq!(moment(&json!("2026-11-03T18:50:12Z")).as_deref(), Some("2026-11-03T18:50:12Z"));
        assert_eq!(moment(&json!("2026-11-03T18:50:12+02:00")).as_deref(), Some("2026-11-03T18:50:12+02:00"));
    }

    #[test]
    fn the_empty_date_azure_devops_writes_for_never_is_no_time() {
        assert_eq!(moment(&json!("0001-01-01T00:00:00")), None);
        assert_eq!(moment(&json!(null)), None);
    }
}
