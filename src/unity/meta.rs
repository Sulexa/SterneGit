use super::{
    compare::{Options, State, changed_rows},
    scene::Model,
    value::{Field, Value},
};

const IMPORTER: &str = "importer";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImportChanges {
    pub(crate) file: String,
    pub(crate) columns: Vec<String>,
    pub(crate) before: Vec<String>,
    pub(crate) after: Vec<String>,
    pub(crate) same: usize,
}

fn importer(model: &Model) -> Option<&[Field]> {
    model
        .items
        .iter()
        .flat_map(|item| &item.parts)
        .find(|part| part.key == IMPORTER)
        .map(|part| part.fields.as_slice())
}

pub(crate) fn import_changes(before: &Model, after: &Model, file: &str) -> Option<ImportChanges> {
    let old = importer(before)?;
    let new = importer(after)?;
    let leaves = |list: &[Field]| list.iter().filter(|field| is_leaf(&field.value)).count();
    let (rows, _) = changed_rows(Some(old), Some(new), Options::default());
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
    let same = leaves(new).saturating_sub(columns.len());
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

#[cfg(test)]
mod tests {
    use super::import_changes;
    use crate::unity::ingest::tests::model_of;

    const BEFORE: &str = "fileFormatVersion: 2\nguid: 1\nTextureImporter:\n  serializedVersion: 12\n  filterMode: 1\n  maxTextureSize: 2048\n  spritePixelsToUnits: 100\n";
    const AFTER: &str = "fileFormatVersion: 2\nguid: 1\nTextureImporter:\n  serializedVersion: 12\n  filterMode: 0\n  maxTextureSize: 512\n  spritePixelsToUnits: 100\n";

    #[test]
    fn only_the_import_settings_that_changed_become_columns() {
        let changes = import_changes(&model_of("slime.png.meta", BEFORE), &model_of("slime.png.meta", AFTER), "slime.png.meta")
            .expect("two settings changed");
        assert_eq!(changes.columns, ["Filter Mode", "Max Texture Size"]);
        assert_eq!((changes.before.as_slice(), changes.after.as_slice()), (&["1".to_owned(), "2048".to_owned()][..], &["0".to_owned(), "512".to_owned()][..]));
        assert_eq!(changes.same, 2);
        assert!(import_changes(&model_of("a.meta", BEFORE), &model_of("a.meta", BEFORE), "a.meta").is_none());
    }
}
