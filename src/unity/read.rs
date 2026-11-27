use std::{
    collections::HashMap,
    ffi::OsString,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::Result;
use sternegit_reader_protocol::{self as protocol, Edit, Failure, Fix, FixInput, ReadInput, Side, SideFilesInput, Warning, digest};

use super::{
    animator::{Machine, compare_controllers},
    clip::{Clip, ClipComparison, compare_clips},
    compare::{Comparison, Options, compare},
    graph::{GraphComparison, ScriptSide, compare_script_graphs},
    hints::{Labels, annotate},
    ingest,
    layout::{Layout, View},
    meta::{ImportChanges, import_changes},
    names::UnityKind,
    scene::Model,
    sprites::{SpriteImage, crop},
};
use crate::{
    git::{Repo, Revision},
    media::Picture,
    plugin::Stop,
    read::{ReadError, Reader, RevisionTree, Tree},
};

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

#[derive(Clone)]
pub(crate) struct Fixer {
    by: ByReader,
    revision: Revision,
    input: SideFilesInput,
}

impl Fixer {
    pub(crate) fn fix(&self, repo: &Repo, fix: &str) -> Result<Fix> {
        let tree: Arc<dyn Tree> = Arc::new(RevisionTree::new(repo.clone(), self.revision, self.input.path.clone()));
        let input = FixInput {
            kind: self.input.kind.clone(),
            path: self.input.path.clone(),
            before: self.input.before.clone(),
            after: self.input.after.clone(),
            fix: fix.to_owned(),
        };
        self.by.reader.fix(tree, input, &AtomicBool::new(false)).map_err(|error| stopped(&self.by, error))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Changed(pub(crate) String);

impl std::fmt::Display for Changed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} changed since it was read; nothing was written", self.0)
    }
}

impl std::error::Error for Changed {}

pub(crate) fn apply_edits(repo: &Repo, edits: &[Edit]) -> Result<()> {
    for edit in edits {
        let current = std::fs::read(repo.location().join(&edit.path))?;
        if digest(&current) != edit.was {
            return Err(Changed(edit.path.clone()).into());
        }
    }
    for edit in edits {
        std::fs::write(repo.location().join(&edit.path), &edit.text)?;
    }
    Ok(())
}

pub(crate) struct UnityFiles {
    pub(crate) kind: UnityKind,
    pub(crate) shape: Shape,
    pub(crate) path: String,
    pub(crate) before: Option<UnityVersion>,
    pub(crate) after: Option<UnityVersion>,
    pub(crate) sprites: HashMap<String, Arc<SpriteImage>>,
    pub(crate) warnings: Vec<Warning>,
    pub(crate) fixer: Option<Fixer>,
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

#[derive(Clone)]
pub(crate) struct ByReader {
    pub(crate) name: String,
    pub(crate) reader: Arc<Reader>,
    pub(crate) kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReaderStopped {
    pub(crate) reader: String,
    pub(crate) why: String,
    pub(crate) log: Vec<String>,
}

impl std::fmt::Display for ReaderStopped {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} couldn't read this file: {}", self.reader, self.why)
    }
}

impl std::error::Error for ReaderStopped {}

fn stopped(by: &ByReader, error: ReadError) -> anyhow::Error {
    let (why, log) = match error {
        ReadError::Stopped(Stop::Trapped { message, log }) => (message, log),
        ReadError::Stopped(Stop::OutOfMemory { .. }) => ("it ran out of memory".to_owned(), Vec::new()),
        ReadError::Stopped(Stop::OutOfFuel) => ("it ran too long".to_owned(), Vec::new()),
        ReadError::Stopped(stop) => (stop.told_of("reader"), Vec::new()),
        ReadError::Failed(failure) => (failure.to_string(), Vec::new()),
    };
    ReaderStopped {
        reader: by.name.clone(),
        why: why.trim_end_matches('.').to_owned(),
        log,
    }
    .into()
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
        Err(error) => Err(stopped(by, error)),
    }
}

pub(crate) fn imports(repo: &Repo, by: &ByReader, before: &Source, after: &Source, file: &str) -> Option<ImportChanges> {
    let old = read_side(repo, by, before, Side::Before).ok()?;
    let new = read_side(repo, by, after, Side::After).ok()?;
    import_changes(&ingest::model(&old), &ingest::model(&new), file)
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
    let (warnings, fixer) = match (&before, &after) {
        (Some(old), Some(new)) => {
            let input = SideFilesInput {
                kind: by.kind.clone(),
                path: path.to_owned(),
                before: Some(text_of(repo, old)?),
                after: Some(text_of(repo, new)?),
            };
            let tree: Arc<dyn Tree> = Arc::new(RevisionTree::new(repo.clone(), new.revision, path.to_owned()));
            let warnings = by
                .reader
                .side_files(tree, input.clone(), &AtomicBool::new(false))
                .unwrap_or_default();
            let fixer = Fixer {
                by: by.clone(),
                revision: new.revision,
                input,
            };
            (warnings, Some(fixer))
        }
        _ => (Vec::new(), None),
    };
    Ok(UnityFiles {
        kind,
        shape,
        path: path.to_owned(),
        before: read_before.as_ref().map(version),
        after: read_after.as_ref().map(version),
        sprites,
        warnings,
        fixer,
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
