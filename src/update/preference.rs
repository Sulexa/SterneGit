use std::io;

use serde_json::Value;

use crate::settings::{Fields, settings_file};

const AUTOMATIC_FIELD: &str = "update_automatically";

pub(crate) fn updates_automatically() -> bool {
    settings_file().is_none_or(|file| automatic_in(&file.read()))
}

pub(crate) fn update_automatically(automatic: bool) -> io::Result<()> {
    let file = settings_file().ok_or_else(|| io::Error::other("There's no settings folder"))?;
    file.update(|fields| {
        fields.insert(AUTOMATIC_FIELD.into(), automatic.into());
    })
}

fn automatic_in(fields: &Fields) -> bool {
    fields.get(AUTOMATIC_FIELD).and_then(Value::as_bool).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::automatic_in;

    #[test]
    fn updates_are_automatic_unless_switched_off() {
        let fields = |value: serde_json::Value| value.as_object().cloned().unwrap();
        assert!(automatic_in(&fields(json!({ "zoom": 125 }))));
        assert!(automatic_in(&fields(json!({ "update_automatically": "no" }))));
        assert!(!automatic_in(&fields(json!({ "update_automatically": false }))));
    }
}
