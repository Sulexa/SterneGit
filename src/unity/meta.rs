use super::{
    compare::{Options, State, changed_rows},
    value::{Lookup, Value, fields},
    yaml::{Node, plain},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImportChanges {
    pub(crate) file: String,
    pub(crate) columns: Vec<String>,
    pub(crate) before: Vec<String>,
    pub(crate) after: Vec<String>,
    pub(crate) same: usize,
}

fn importer(meta: &Node) -> Option<&Node> {
    meta.entries().iter().find(|(_, value)| matches!(value, Node::Map(_))).map(|(_, body)| body)
}

pub(crate) fn import_changes(before: Option<&str>, after: Option<&str>, file: &str) -> Option<ImportChanges> {
    let before = plain(before?);
    let after = plain(after?);
    let lookup = Lookup::default();
    let old = fields(importer(&before)?, &[], &lookup);
    let new = fields(importer(&after)?, &[], &lookup);
    let leaves = |list: &[super::value::Field]| list.iter().filter(|field| is_leaf(&field.value)).count();
    let (rows, _) = changed_rows(Some(&old), Some(&new), Options::default());
    let mut columns = Vec::new();
    let mut before_values = Vec::new();
    let mut after_values = Vec::new();
    let mut parents: Vec<String> = Vec::new();
    for row in &rows {
        parents.truncate(row.depth);
        let leaf = row.before.as_ref().or(row.after.as_ref()).is_some_and(is_leaf);
        if !leaf {
            parents.push(row.label.clone());
            continue;
        }
        if row.state == State::Same {
            continue;
        }
        let label = match parents.last() {
            Some(parent) if !parent.starts_with("Element") => format!("{parent} › {}", row.label),
            _ => row.label.clone(),
        };
        columns.push(label);
        before_values.push(row.before.as_ref().map(Value::text).unwrap_or_default());
        after_values.push(row.after.as_ref().map(Value::text).unwrap_or_default());
    }
    if columns.is_empty() {
        return None;
    }
    let same = leaves(&new).saturating_sub(columns.len());
    Some(ImportChanges {
        file: file.to_owned(),
        columns,
        before: before_values,
        after: after_values,
        same,
    })
}

fn is_leaf(value: &Value) -> bool {
    !matches!(value, Value::Count(_)) && !matches!(value, Value::Text(text) if text.is_empty())
}

pub(crate) fn guid_of(text: &str) -> Option<String> {
    plain(text).get("guid").and_then(Node::text).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{guid_of, import_changes};

    const BEFORE: &str = "fileFormatVersion: 2
guid: 9a3c51e07d2b4f6c8e1a0b2c3d4e5f60
TextureImporter:
  serializedVersion: 12
  filterMode: 1
  spritePixelsToUnits: 16
  maxTextureSize: 2048
  mipmaps:
    enableMipMap: 0
";

    const AFTER: &str = "fileFormatVersion: 2
guid: 9a3c51e07d2b4f6c8e1a0b2c3d4e5f60
TextureImporter:
  serializedVersion: 12
  filterMode: 0
  spritePixelsToUnits: 24
  maxTextureSize: 256
  mipmaps:
    enableMipMap: 0
";

    #[test]
    fn only_the_import_settings_that_changed_become_columns() {
        let changes = import_changes(Some(BEFORE), Some(AFTER), "slime.png.meta");
        let changes = changes.expect("three settings changed");
        assert_eq!(changes.columns, vec!["Filter Mode", "Sprite Pixels To Units", "Max Texture Size"]);
        assert_eq!(changes.before, vec!["1", "16", "2048"]);
        assert_eq!(changes.after, vec!["0", "24", "256"]);
        assert_eq!(changes.same, 2);
    }

    #[test]
    fn nothing_to_show_when_the_settings_are_the_same() {
        assert_eq!(import_changes(Some(BEFORE), Some(BEFORE), "slime.png.meta"), None);
        assert_eq!(guid_of(BEFORE).as_deref(), Some("9a3c51e07d2b4f6c8e1a0b2c3d4e5f60"));
    }
}
