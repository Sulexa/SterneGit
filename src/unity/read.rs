use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::Path,
    sync::Arc,
};

use anyhow::Result;

use super::{
    animator::compare_controllers,
    clip::{Clip, ClipComparison, Keys, compare_clips, read_clip},
    compare::{Comparison, Options, compare},
    graph::{GraphComparison, compare_script_graphs, graph_json},
    hints::Scopes,
    layout::{Layout, View, layout, view_for},
    meta::guid_of,
    names::{UnityKind, component_title, file_name, nicify},
    scene::{Model, guids_in, guids_in_node, object_names, script_guid, source_prefabs, targets_of},
    sprites::{SpriteImage, crop},
    value::Lookup,
    yaml::{Document, Node, documents, is_unity_yaml, plain},
};
use crate::{
    git::{Repo, Revision},
    media::Picture,
};

const DEEPEST_LIBRARY: usize = 3;
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
    pub(crate) documents: Vec<Document>,
    pub(crate) model: Model,
    pub(crate) clip: Option<Clip>,
    pub(crate) graph: Option<serde_json::Value>,
    pub(crate) layout: Option<Layout>,
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
    pub(crate) lookup: Lookup,
    pub(crate) sprites: HashMap<(String, i64), Arc<SpriteImage>>,
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
        match self.shape {
            Shape::Controller => compare_controllers(
                self.before.as_ref().map(|version| version.documents.as_slice()),
                self.after.as_ref().map(|version| version.documents.as_slice()),
                &self.lookup,
                layer,
                options,
            ),
            _ => compare_script_graphs(
                self.before.as_ref().and_then(|version| version.graph.as_ref()),
                self.after.as_ref().and_then(|version| version.graph.as_ref()),
                options,
            ),
        }
    }

    pub(crate) fn layouts(&self) -> [Option<&Layout>; 2] {
        [
            self.before.as_ref().and_then(|version| version.layout.as_ref()),
            self.after.as_ref().and_then(|version| version.layout.as_ref()),
        ]
    }

    pub(crate) fn annotate(&self, patch: &str) -> String {
        let none: [Document; 0] = [];
        let before = self.before.as_ref().map_or(&none[..], |version| &version.documents);
        let after = self.after.as_ref().map_or(&none[..], |version| &version.documents);
        Scopes::new(before, after, &self.lookup).annotate(patch)
    }

    pub(crate) fn sprite(&self, guid: &str, file_id: i64) -> Option<&Arc<SpriteImage>> {
        self.sprites.get(&(guid.to_owned(), file_id))
    }
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

pub(crate) fn read(repo: &Repo, path: &str, before: Option<Source>, after: Option<Source>) -> Result<UnityFiles> {
    let kind = UnityKind::of(path).ok_or(NotUnityText)?;
    let texts = [before, after].map(|source| source.map(|source| text_of(repo, &source)).transpose());
    let [before_text, after_text] = texts;
    let (before_text, after_text) = (before_text?, after_text?);
    let unity_text = |text: &Option<String>| text.as_deref().is_none_or(|text| kind == UnityKind::Meta || is_unity_yaml(text));
    if !unity_text(&before_text) || !unity_text(&after_text) || before_text.is_none() && after_text.is_none() {
        return Err(NotUnityText.into());
    }
    let parse = |text: &Option<String>| -> Option<(Vec<Document>, Option<Node>)> {
        let text = text.as_deref()?;
        Some(if kind == UnityKind::Meta { (Vec::new(), Some(plain(text))) } else { (documents(text), None) })
    };
    let parsed = [parse(&before_text), parse(&after_text)];
    let mut library: HashMap<String, Vec<Document>> = HashMap::new();
    let mut lookup = Lookup::default();
    let mut wanted: Vec<String> = parsed
        .iter()
        .flatten()
        .flat_map(|(docs, _)| source_prefabs(docs))
        .collect();
    for _ in 0..DEEPEST_LIBRARY {
        wanted.retain(|guid| !library.contains_key(guid));
        wanted.sort();
        wanted.dedup();
        if wanted.is_empty() {
            break;
        }
        lookup.assets.extend(repo.meta_files_owning(&wanted)?);
        let mut next = Vec::new();
        for guid in &wanted {
            let Some(text) = lookup.asset_path(guid).and_then(|prefab| on_disk(repo, prefab)) else {
                continue;
            };
            let docs = documents(&text);
            next.extend(source_prefabs(&docs));
            library.insert(guid.clone(), docs);
        }
        wanted = next;
    }
    let mut guids: HashSet<String> = HashSet::new();
    for (docs, meta) in parsed.iter().flatten() {
        guids.extend(guids_in(docs));
        if let Some(meta) = meta {
            guids_in_node(meta, &mut guids);
        }
    }
    for docs in library.values() {
        guids.extend(guids_in(docs));
    }
    let missing: Vec<String> = guids.into_iter().filter(|guid| !lookup.assets.contains_key(guid)).collect();
    lookup.assets.extend(repo.meta_files_owning(&missing)?);
    for (guid, docs) in &library {
        let mut targets = targets_of(guid, docs);
        for document in docs {
            if let Some(target) = targets.get_mut(&(guid.clone(), document.file_id))
                && document.type_name == "MonoBehaviour"
            {
                target.script = script_guid(&document.body).and_then(|script| lookup.script_name(script));
            }
        }
        lookup.targets.extend(targets);
    }
    for (docs, _) in parsed.iter().flatten() {
        lookup.objects.extend(object_names(docs, &lookup.assets));
    }
    let view = parsed
        .iter()
        .rev()
        .flatten()
        .next()
        .map_or(View::Front, |(docs, _)| view_for(docs));
    let shape = match kind {
        UnityKind::AnimationClip => Shape::Clip,
        UnityKind::AnimatorController => Shape::Controller,
        UnityKind::Asset if parsed.iter().flatten().any(|(docs, _)| graph_json(docs).is_some()) => Shape::Graph,
        _ => Shape::Objects,
    };
    let [before_parsed, after_parsed] = parsed;
    let version = |parsed: Option<(Vec<Document>, Option<Node>)>| {
        parsed.map(|(docs, meta)| {
            let model = match &meta {
                Some(meta) => Model::from_meta(meta, path.strip_suffix(".meta").unwrap_or(path), &lookup),
                None => Model::from_documents(kind, &docs, path, &lookup),
            };
            let layout = kind.has_hierarchy().then(|| layout(&docs, &model, &library, &lookup, view));
            UnityVersion {
                clip: (shape == Shape::Clip).then(|| read_clip(&docs, &lookup)).flatten(),
                graph: (shape == Shape::Graph).then(|| graph_json(&docs)).flatten(),
                layout,
                model,
                documents: docs,
            }
        })
    };
    let before = version(before_parsed);
    let after = version(after_parsed);
    let sprites = load_sprites(repo, &lookup, [before.as_ref(), after.as_ref()]);
    let guid_change = match (kind, &before_text, &after_text) {
        (UnityKind::Meta, Some(old), Some(new)) => guid_change(repo, path, old, new, &lookup),
        _ => None,
    };
    Ok(UnityFiles {
        kind,
        shape,
        path: path.to_owned(),
        before,
        after,
        lookup,
        sprites,
        guid_change,
        view,
    })
}

fn wanted_sprites(versions: [Option<&UnityVersion>; 2]) -> Vec<(String, i64)> {
    let mut wanted = Vec::new();
    for version in versions.into_iter().flatten() {
        if let Some(layout) = &version.layout {
            for spot in &layout.spots {
                wanted.extend(spot.sprites.iter().map(|sprite| (sprite.guid.clone(), sprite.file_id)));
            }
        }
        if let Some(clip) = &version.clip {
            for track in &clip.tracks {
                if let Keys::Objects(keys) = &track.keys {
                    wanted.extend(keys.iter().filter_map(|key| key.sprite.clone()));
                }
            }
        }
    }
    wanted.sort();
    wanted.dedup();
    wanted
}

fn load_sprites(repo: &Repo, lookup: &Lookup, versions: [Option<&UnityVersion>; 2]) -> HashMap<(String, i64), Arc<SpriteImage>> {
    let mut textures: HashMap<String, Option<(Picture, Node)>> = HashMap::new();
    let mut sprites = HashMap::new();
    for (guid, file_id) in wanted_sprites(versions) {
        let texture = textures.entry(guid.clone()).or_insert_with(|| {
            let path = lookup.asset_path(&guid)?;
            let bytes = std::fs::read(repo.location().join(path)).ok()?;
            let extension = Path::new(path).extension().and_then(|extension| extension.to_str()).map(str::to_lowercase);
            let picture = Picture::decode(&bytes, extension.as_deref()).ok()?;
            let meta = on_disk(repo, &format!("{path}.meta")).map(|text| plain(&text)).unwrap_or(Node::Map(Vec::new()));
            Some((picture, meta))
        });
        if let Some((picture, meta)) = texture
            && let Some(sprite) = crop(&picture.pixels, picture.width, picture.height, meta, file_id)
        {
            sprites.insert((guid, file_id), Arc::new(sprite));
        }
    }
    sprites
}

fn guid_change(repo: &Repo, path: &str, old: &str, new: &str, lookup: &Lookup) -> Option<GuidChange> {
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
