use std::{
    collections::HashMap,
    ffi::OsString,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::Result;
use sternegit_reader_protocol::{self as protocol, Failure, ReadInput, Side};

use super::{
    animator::{Machine, compare_controllers},
    clip::{Clip, ClipComparison, compare_clips},
    compare::{Comparison, Options, compare},
    graph::{GraphComparison, ScriptSide, compare_script_graphs},
    hints::{Labels, annotate},
    ingest,
    layout::{Layout, View},
    meta::guid_of,
    names::{UnityKind, component_title, file_name, nicify},
    scene::{Model, object_names, script_guid},
    sprites::{SpriteImage, crop},
    value::Lookup,
    yaml::{Document, Node, documents, is_unity_yaml},
};
use crate::{
    git::{Repo, Revision},
    media::Picture,
    read::{ReadError, Reader, RevisionTree, Tree},
};

const MOST_GUID_USERS: usize = 40;
const PLACES_PER_USER: usize = 3;

#[derive(Debug)]
pub(crate) struct NotUnityText;

impl std::fmt::Display for NotUnityText {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("This file isn't saved as Unity text")
    }
}

impl std::error::Error for NotUnityText {}

#[derive(Clone, Debug)]
pub(crate) struct Source {
    pub(crate) revision: Revision,
    pub(crate) path: OsString,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    Objects,
    Clip,
    Controller,
    Graph,
}

pub(crate) struct UnityVersion {
    pub(crate) model: Model,
    pub(crate) clip: Option<Clip>,
    pub(crate) script: ScriptSide,
    pub(crate) machine: Machine,
    pub(crate) layout: Option<Layout>,
    pub(crate) scopes: Vec<protocol::Scope>,
    pub(crate) notes: Vec<protocol::Note>,
    pub(crate) text: Option<String>,
    pub(crate) ports: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GuidUser {
    pub(crate) path: String,
    pub(crate) places: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GuidChange {
    pub(crate) before: String,
    pub(crate) after: String,
    pub(crate) asset: String,
    pub(crate) meta: String,
    pub(crate) users: Vec<GuidUser>,
}

pub(crate) struct UnityFiles {
    pub(crate) kind: UnityKind,
    pub(crate) shape: Shape,
    pub(crate) path: String,
    pub(crate) before: Option<UnityVersion>,
    pub(crate) after: Option<UnityVersion>,
    pub(crate) sprites: HashMap<String, Arc<SpriteImage>>,
    pub(crate) guid_change: Option<GuidChange>,
    pub(crate) view: View,
}

impl UnityFiles {
    pub(crate) fn compare(&self, options: Options) -> Comparison {
        compare(
            self.before.as_ref().map(|version| &version.model),
            self.after.as_ref().map(|version| &version.model),
            options,
        )
    }

    pub(crate) fn clip(&self, options: Options) -> ClipComparison {
        compare_clips(
            self.before.as_ref().and_then(|version| version.clip.as_ref()),
            self.after.as_ref().and_then(|version| version.clip.as_ref()),
            options.unchanged,
        )
    }

    pub(crate) fn graph(&self, layer: usize, options: Options) -> GraphComparison {
        let empty_machine = Machine::default();
        let empty_script = ScriptSide::default();
        match self.shape {
            Shape::Controller => compare_controllers(
                self.before.as_ref().map_or(&empty_machine, |version| &version.machine),
                self.after.as_ref().map_or(&empty_machine, |version| &version.machine),
                layer,
                options,
            ),
            _ => {
                let ports = self.after.as_ref().or(self.before.as_ref()).is_none_or(|version| version.ports);
                compare_script_graphs(
                    self.before.as_ref().map_or(&empty_script, |version| &version.script),
                    self.after.as_ref().map_or(&empty_script, |version| &version.script),
                    ports,
                    options,
                )
            }
        }
    }

    pub(crate) fn layouts(&self) -> [Option<&Layout>; 2] {
        [
            self.before.as_ref().and_then(|version| version.layout.as_ref()),
            self.after.as_ref().and_then(|version| version.layout.as_ref()),
        ]
    }

    pub(crate) fn annotate(&self, patch: &str) -> String {
        annotate(patch, labels(self.before.as_ref()), labels(self.after.as_ref()))
    }

    pub(crate) fn sprite(&self, picture: &str) -> Option<&Arc<SpriteImage>> {
        self.sprites.get(picture)
    }
}

fn labels(version: Option<&UnityVersion>) -> Labels<'_> {
    version.map_or_else(Labels::default, |version| Labels {
        scopes: &version.scopes,
        notes: &version.notes,
    })
}

fn text_of(repo: &Repo, source: &Source) -> Result<String> {
    let bytes = repo.file_version(source.revision, &source.path)?.bytes;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn on_disk(repo: &Repo, path: &str) -> Option<String> {
    std::fs::read(repo.location().join(path))
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

#[derive(Clone)]
pub(crate) struct ByReader {
    pub(crate) reader: Arc<Reader>,
    pub(crate) kind: String,
}

fn read_side(repo: &Repo, by: &ByReader, source: &Source, side: Side) -> Result<protocol::Model> {
    let path = source.path.to_string_lossy().into_owned();
    let tree: Arc<dyn Tree> = Arc::new(RevisionTree::new(repo.clone(), source.revision, path.clone()));
    let input = ReadInput {
        kind: by.kind.clone(),
        path,
        side,
        options: Default::default(),
    };
    match by.reader.read(tree, input, &AtomicBool::new(false)) {
        Ok(read) => Ok(read),
        Err(ReadError::Failed(Failure::NotThisFormat { .. })) => Err(NotUnityText.into()),
        Err(error) => Err(anyhow::anyhow!("{error}")),
    }
}

fn version(read: &protocol::Model) -> UnityVersion {
    UnityVersion {
        model: ingest::model(read),
        clip: read.curves.as_ref().map(ingest::clip),
        script: ingest::script_side(read.graph.as_ref()),
        machine: ingest::machine(read.graph.as_ref()),
        layout: read.layout.as_ref().map(ingest::layout),
        scopes: read.scopes.clone(),
        notes: read.notes.clone(),
        text: read.text.clone(),
        ports: read.graph.as_ref().is_none_or(|graph| graph.ports),
    }
}

pub(crate) fn read(
    repo: &Repo,
    path: &str,
    before: Option<Source>,
    after: Option<Source>,
    by: Option<&ByReader>,
) -> Result<UnityFiles> {
    let kind = UnityKind::of(path).ok_or(NotUnityText)?;
    let by = by.ok_or(NotUnityText)?;
    if before.is_none() && after.is_none() {
        return Err(NotUnityText.into());
    }
    let read_before = before.as_ref().map(|source| read_side(repo, by, source, Side::Before)).transpose()?;
    let read_after = after.as_ref().map(|source| read_side(repo, by, source, Side::After)).transpose()?;
    let shape = match kind {
        UnityKind::AnimationClip => Shape::Clip,
        UnityKind::AnimatorController => Shape::Controller,
        UnityKind::Asset if [&read_before, &read_after].into_iter().flatten().any(|read| read.graph.is_some()) => {
            Shape::Graph
        }
        _ => Shape::Objects,
    };
    let view = [&read_after, &read_before]
        .into_iter()
        .flatten()
        .find_map(|read| read.layout.as_ref().map(ingest::view))
        .unwrap_or(View::Front);
    let sprites = load_sprites(repo, [&read_before, &read_after]);
    let guid_change = match kind {
        UnityKind::Meta => match (&before, &after) {
            (Some(before), Some(after)) => guid_change(repo, path, &text_of(repo, before)?, &text_of(repo, after)?),
            _ => None,
        },
        _ => None,
    };
    Ok(UnityFiles {
        kind,
        shape,
        path: path.to_owned(),
        before: read_before.as_ref().map(version),
        after: read_after.as_ref().map(version),
        sprites,
        guid_change,
        view,
    })
}

fn load_sprites(repo: &Repo, reads: [&Option<protocol::Model>; 2]) -> HashMap<String, Arc<SpriteImage>> {
    let mut decoded: HashMap<String, Option<Picture>> = HashMap::new();
    let mut sprites = HashMap::new();
    for picture in reads.into_iter().flatten().flat_map(|read| &read.pictures) {
        if sprites.contains_key(&picture.id) {
            continue;
        }
        let image = decoded.entry(picture.path.clone()).or_insert_with(|| {
            let bytes = std::fs::read(repo.location().join(&picture.path)).ok()?;
            let extension = Path::new(&picture.path)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_lowercase);
            Picture::decode(&bytes, extension.as_deref()).ok()
        });
        if let Some(image) = image
            && let Some(sprite) = crop(&image.pixels, image.width, image.height, picture)
        {
            sprites.insert(picture.id.clone(), Arc::new(sprite));
        }
    }
    sprites
}

fn guid_change(repo: &Repo, path: &str, old: &str, new: &str) -> Option<GuidChange> {
    let lookup = Lookup::default();
    let before = guid_of(old)?;
    let after = guid_of(new)?;
    if before == after {
        return None;
    }
    let users = repo
        .files_mentioning(&before)
        .unwrap_or_default()
        .into_iter()
        .filter(|user| user != path)
        .take(MOST_GUID_USERS)
        .map(|user| {
            let places = on_disk(repo, &user)
                .filter(|text| is_unity_yaml(text))
                .map(|text| {
                    let docs = documents(&text);
                    let mut known = lookup.clone();
                    let scripts: Vec<String> = docs
                        .iter()
                        .filter_map(|document| script_guid(&document.body))
                        .filter(|guid| !known.assets.contains_key(*guid))
                        .map(ToOwned::to_owned)
                        .collect();
                    known.assets.extend(repo.meta_files_owning(&scripts).unwrap_or_default());
                    where_used(&docs, &before, &known)
                })
                .unwrap_or_default();
            GuidUser { path: user, places }
        })
        .collect();
    let asset = path.strip_suffix(".meta").unwrap_or(path);
    Some(GuidChange {
        before,
        after,
        asset: file_name(asset).to_owned(),
        meta: path.to_owned(),
        users,
    })
}

fn where_used(docs: &[Document], guid: &str, lookup: &Lookup) -> Vec<String> {
    let names = object_names(docs, &lookup.assets);
    let mut places = Vec::new();
    for document in docs {
        let mut found = Vec::new();
        search(&document.body, guid, &mut Vec::new(), &mut found);
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

fn search(node: &Node, guid: &str, path: &mut Vec<String>, found: &mut Vec<String>) {
    match node {
        Node::Map(entries) => {
            if node.get("guid").and_then(Node::text) == Some(guid) {
                found.push(path.join(" › "));
                return;
            }
            for (key, value) in entries {
                path.push(nicify(key));
                search(value, guid, path, found);
                path.pop();
            }
        }
        Node::List(items) => {
            for (ix, item) in items.iter().enumerate() {
                path.push(format!("Element {ix}"));
                search(item, guid, path, found);
                path.pop();
            }
        }
        Node::Scalar(_) => {}
    }
}

pub(crate) fn keep_old_guid(repo: &Repo, change: &GuidChange) -> Result<()> {
    let file = repo.location().join(&change.meta);
    let text = std::fs::read_to_string(&file)?;
    let replaced = text.replacen(&format!("guid: {}", change.after), &format!("guid: {}", change.before), 1);
    std::fs::write(file, replaced)?;
    Ok(())
}

pub(crate) fn point_users_to_new_guid(repo: &Repo, change: &GuidChange) -> Result<()> {
    for user in &change.users {
        let file = repo.location().join(&user.path);
        let text = std::fs::read_to_string(&file)?;
        std::fs::write(file, text.replace(&change.before, &change.after))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::where_used;
    use crate::unity::{value::Lookup, yaml::documents};

    #[test]
    fn a_guid_is_found_where_a_script_field_points_at_it() {
        let docs = documents(
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
        let mut lookup = Lookup::default();
        lookup.assets.insert("11112222333344445555666677778888".to_owned(), "Assets/SlimeController.cs".to_owned());
        assert_eq!(
            where_used(&docs, "9a3c51e07d2b4f6c8e1a0b2c3d4e5f60", &lookup),
            vec!["Slime › Slime Controller › Hit Sound".to_owned()]
        );
    }
}
