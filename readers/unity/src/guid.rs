use sternegit_reader_sdk::{Edit, Failure, Fix, FixOffer, Place, Warning, digest, read_file, search};

use crate::{
    lookup::lookup_for,
    names::{component_title, file_name, nicify},
    scene::{object_names, script_guid},
    value::Lookup,
    yaml::{Document, Node, documents, is_unity_yaml, plain},
};

const MOST_GUID_USERS: usize = 40;
const PLACES_PER_USER: usize = 3;
const EVERY_FILE: &str = "*";
pub(crate) const KEEP_OLD: &str = "keep_old_guid";
pub(crate) const POINT_USERS: &str = "point_users_to_new_guid";

pub(crate) struct GuidChange {
    before: String,
    after: String,
    asset: String,
    users: Vec<Place>,
}

pub(crate) fn guid_of(text: &str) -> Option<String> {
    plain(text).get("guid").and_then(Node::text).map(ToOwned::to_owned)
}

fn text_of(path: &str) -> Option<String> {
    read_file(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn users_of(guid: &str, meta: &str) -> Vec<String> {
    let mut users: Vec<String> = Vec::new();
    for found in search(guid, EVERY_FILE) {
        if found.path != meta && !users.contains(&found.path) {
            users.push(found.path);
        }
        if users.len() >= MOST_GUID_USERS {
            break;
        }
    }
    users
}

pub(crate) fn guid_change(meta: &str, before: &str, after: &str) -> Option<GuidChange> {
    let old = guid_of(before)?;
    let new = guid_of(after)?;
    if old == new {
        return None;
    }
    let (lookup, _) = lookup_for(&[]);
    let users = users_of(&old, meta)
        .into_iter()
        .map(|user| {
            let details = text_of(&user)
                .filter(|text| is_unity_yaml(text))
                .map(|text| where_used(&documents(&text), &old, &lookup))
                .unwrap_or_default();
            Place { path: user, details }
        })
        .collect();
    let asset = meta.strip_suffix(".meta").unwrap_or(meta);
    Some(GuidChange {
        before: old,
        after: new,
        asset: file_name(asset).to_owned(),
        users,
    })
}

fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

impl GuidChange {
    pub(crate) fn warning(&self) -> Warning {
        let count = self.users.len();
        let title = match count {
            0 => format!("The GUID of {} changed", self.asset),
            _ => format!("The GUID changed: {} {}", counted(count, "file loses", "files lose"), self.asset),
        };
        let about = match count {
            0 => "Unity finds assets by GUID, not by path. No file in the repository uses the old GUID, so nothing breaks.",
            _ => "Unity finds assets by GUID, not by path. These files still use the old GUID, so Unity shows the asset as Missing in them:",
        };
        let mut fixes = vec![FixOffer {
            id: KEEP_OLD.to_owned(),
            label: "Keep the old GUID".to_owned(),
            about: "Write the old GUID back into the .meta file".to_owned(),
        }];
        if count > 0 {
            let label = match self.users.as_slice() {
                [only] => format!("Point {} to the new GUID", file_name(&only.path)),
                _ => format!("Point the {} to the new GUID", counted(count, "file", "files")),
            };
            fixes.push(FixOffer {
                id: POINT_USERS.to_owned(),
                label,
                about: "Replace the old GUID with the new one in those files".to_owned(),
            });
        }
        Warning {
            title,
            about: about.to_owned(),
            places: self.users.clone(),
            fixes,
        }
    }

    pub(crate) fn fix(&self, meta: &str, fix: &str) -> Result<Fix, Failure> {
        let edit = |path: &str, change: &dyn Fn(&str) -> String| -> Option<Edit> {
            let bytes = read_file(path)?;
            let text = String::from_utf8_lossy(&bytes);
            Some(Edit {
                path: path.to_owned(),
                was: digest(&bytes),
                text: change(&text),
            })
        };
        match fix {
            KEEP_OLD => {
                let (old, new) = (format!("guid: {}", self.before), format!("guid: {}", self.after));
                let edits = edit(meta, &|text| text.replacen(&new, &old, 1)).into_iter().collect();
                Ok(Fix {
                    title: format!("Keep the old GUID of {}", self.asset),
                    edits,
                })
            }
            POINT_USERS => {
                let edits: Vec<Edit> = self
                    .users
                    .iter()
                    .filter_map(|user| edit(&user.path, &|text| text.replace(&self.before, &self.after)))
                    .collect();
                Ok(Fix {
                    title: format!("Point {} to the new GUID", counted(edits.len(), "file", "files")),
                    edits,
                })
            }
            _ => Err(Failure::Unsupported),
        }
    }
}

fn where_used(docs: &[Document], guid: &str, lookup: &Lookup) -> Vec<String> {
    let names = object_names(docs, &lookup.assets);
    let mut places = Vec::new();
    for document in docs {
        let mut found = Vec::new();
        search_node(&document.body, guid, &mut Vec::new(), &mut found);
        for field in found {
            let (object, kind) = names
                .get(&document.file_id)
                .cloned()
                .unwrap_or_else(|| (String::new(), component_title(&document.type_name)));
            let component = if document.type_name == "MonoBehaviour" {
                script_guid(&document.body)
                    .and_then(|script| lookup.script_name(script))
                    .map_or(kind, |name| nicify(&name))
            } else {
                kind
            };
            let place = [object, component, field].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" › ");
            if !places.contains(&place) {
                places.push(place);
            }
            if places.len() >= PLACES_PER_USER {
                return places;
            }
        }
    }
    places
}

fn search_node(node: &Node, guid: &str, path: &mut Vec<String>, found: &mut Vec<String>) {
    match node {
        Node::Map(entries) => {
            if node.get("guid").and_then(Node::text) == Some(guid) {
                found.push(path.join(" › "));
                return;
            }
            for (key, value) in entries {
                path.push(nicify(key));
                search_node(value, guid, path, found);
                path.pop();
            }
        }
        Node::List(items) => {
            for (ix, item) in items.iter().enumerate() {
                path.push(format!("Element {ix}"));
                search_node(item, guid, path, found);
                path.pop();
            }
        }
        Node::Scalar(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use sternegit_reader_sdk::{Folder, with_files};

    use super::{KEEP_OLD, POINT_USERS, guid_change};

    const OLD: &str = "0123456789abcdef0123456789abcdef";
    const NEW: &str = "fedcba9876543210fedcba9876543210";

    fn project(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("sternegit-unity-guid-{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("slime.png.meta"), format!("fileFormatVersion: 2\nguid: {NEW}\n")).unwrap();
        fs::write(
            root.join("Slime.prefab"),
            format!(
                "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1\nGameObject:\n  m_Name: Slime\n--- !u!212 &2\nSpriteRenderer:\n  m_GameObject: {{fileID: 1}}\n  m_Sprite: {{fileID: 21300000, guid: {OLD}, type: 3}}\n"
            ),
        )
        .unwrap();
        root
    }

    #[test]
    fn a_changed_guid_lists_the_files_and_places_still_using_the_old_one() {
        let root = project("warning");
        let warning = with_files(Folder::new(&root), || {
            guid_change("slime.png.meta", &format!("guid: {OLD}\n"), &format!("guid: {NEW}\n")).map(|change| change.warning())
        })
        .expect("a warning");
        assert_eq!(warning.title, "The GUID changed: 1 file loses slime.png");
        assert_eq!(warning.places[0].path, "Slime.prefab");
        assert_eq!(warning.places[0].details, ["Slime › Sprite Renderer › Sprite"]);
        let ids: Vec<&str> = warning.fixes.iter().map(|fix| fix.id.as_str()).collect();
        assert_eq!(ids, [KEEP_OLD, POINT_USERS]);
        assert_eq!(warning.fixes[1].label, "Point Slime.prefab to the new GUID");
    }

    #[test]
    fn each_fix_proposes_whole_files_without_writing_them() {
        let root = project("fixes");
        let (keep, point) = with_files(Folder::new(&root), || {
            let change = guid_change("slime.png.meta", &format!("guid: {OLD}\n"), &format!("guid: {NEW}\n")).unwrap();
            (change.fix("slime.png.meta", KEEP_OLD).unwrap(), change.fix("slime.png.meta", POINT_USERS).unwrap())
        });
        assert_eq!(keep.edits[0].text, format!("fileFormatVersion: 2\nguid: {OLD}\n"));
        assert!(point.edits[0].text.contains(NEW) && !point.edits[0].text.contains(OLD));
        assert!(fs::read_to_string(root.join("Slime.prefab")).unwrap().contains(OLD));
    }

    #[test]
    fn a_guid_is_found_where_a_script_field_points_at_it() {
        let docs = crate::yaml::documents(
            "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Component:
  - component: {fileID: 2}
  m_Name: Slime
--- !u!114 &2
MonoBehaviour:
  m_GameObject: {fileID: 1}
  m_Script: {fileID: 11500000, guid: 11112222333344445555666677778888, type: 3}
  hitSound: {fileID: 8300000, guid: 9a3c51e07d2b4f6c8e1a0b2c3d4e5f60, type: 3}
",
        );
        let mut lookup = crate::value::Lookup::default();
        lookup.assets.insert("11112222333344445555666677778888".to_owned(), "Assets/SlimeController.cs".to_owned());
        assert_eq!(
            super::where_used(&docs, "9a3c51e07d2b4f6c8e1a0b2c3d4e5f60", &lookup),
            vec!["Slime › Slime Controller › Hit Sound".to_owned()]
        );
    }
}
