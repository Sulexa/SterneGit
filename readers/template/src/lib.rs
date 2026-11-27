use sternegit_reader_sdk::{Failure, Field, Model, Object, Part, ReadInput, Reader, Role, Scope, Value, export, read_text};

pub struct Ini;

fn value_of(raw: &str) -> Value {
    match raw {
        "true" | "yes" | "on" => Value::Flag(true),
        "false" | "no" | "off" => Value::Flag(false),
        _ if raw.parse::<f64>().is_ok() => Value::Number(raw.to_owned()),
        _ => Value::Text(raw.trim_matches('"').to_owned()),
    }
}

fn section(name: &str, first_line: u32) -> (Object, Scope) {
    let object = Object {
        id: name.to_owned(),
        parent: None,
        name: if name.is_empty() { "Top".to_owned() } else { name.to_owned() },
        role: Role::Asset {
            type_name: "Section".to_owned(),
            detail: None,
        },
        parts: vec![Part {
            id: "keys".to_owned(),
            title: "Keys".to_owned(),
            script: None,
            overrides: false,
            fields: Vec::new(),
        }],
        tags: Vec::new(),
    };
    let scope = Scope {
        lines: (first_line, first_line),
        label: object.name.clone(),
        object: Some(name.to_owned()),
    };
    (object, scope)
}

impl Reader for Ini {
    fn read(&self, input: ReadInput) -> Result<Model, Failure> {
        let text = read_text(&input.path).ok_or_else(|| Failure::Missing { path: input.path.clone() })?;
        let mut model = Model::default();
        let (top, scope) = section("", 1);
        model.objects.push(top);
        model.scopes.push(scope);
        for (ix, line) in text.lines().enumerate() {
            let number = ix as u32 + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with([';', '#']) {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
                let (object, scope) = section(name.trim(), number);
                model.objects.push(object);
                model.scopes.push(scope);
                continue;
            }
            let Some((key, raw)) = line.split_once('=') else {
                continue;
            };
            if let Some(scope) = model.scopes.last_mut() {
                scope.lines.1 = number;
            }
            if let Some(object) = model.objects.last_mut() {
                object.parts[0].fields.push(Field {
                    key: key.trim().to_owned(),
                    label: key.trim().to_owned(),
                    value: value_of(raw.trim()),
                    tags: Vec::new(),
                    children: Vec::new(),
                });
            }
        }
        model.objects.retain(|object| !object.id.is_empty() || !object.parts[0].fields.is_empty());
        Ok(model)
    }
}

export!(Ini);

#[cfg(test)]
mod tests {
    use std::fs;

    use sternegit_reader_sdk::{Folder, ReadInput, Reader, Side, Value, with_files};

    use super::Ini;

    #[test]
    fn each_section_is_an_object_and_each_key_a_field() {
        let root = std::env::temp_dir().join("sternegit-reader-template");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("game.ini"), "; settings\n[video]\nwidth = 1920\nvsync = on\n\n[audio]\nvolume = 0.8\n").unwrap();
        let input = ReadInput {
            kind: "ini".into(),
            path: "game.ini".into(),
            side: Side::After,
            options: Default::default(),
        };
        let model = with_files(Folder::new(&root), || Ini.read(input)).unwrap();
        let names: Vec<&str> = model.objects.iter().map(|object| object.name.as_str()).collect();
        assert_eq!(names, ["video", "audio"]);
        assert_eq!(model.objects[0].parts[0].fields[1].value, Value::Flag(true));
        assert_eq!(model.scopes[1].lines, (2, 4));
    }
}
