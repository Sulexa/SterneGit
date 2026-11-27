mod animator;
mod clip;
mod convert;
mod graph;
mod guid;
mod hints;
mod layout;
mod lookup;
mod names;
mod scene;
mod sprites;
mod ui;
mod value;
mod views;
mod yaml;

use sternegit_reader_sdk::{Failure, Fix, FixInput, Model, ReadInput, Reader, SideFilesInput, Warning, export, read_text};

use crate::{
    lookup::lookup_for,
    names::UnityKind,
    yaml::{documents, is_unity_yaml, plain},
};

pub struct Unity;

impl Reader for Unity {
    fn read(&self, input: ReadInput) -> Result<Model, Failure> {
        let kind = UnityKind::of(&input.path).ok_or_else(|| not_unity(&input.path))?;
        let text = read_text(&input.path).ok_or_else(|| Failure::Missing {
            path: input.path.clone(),
        })?;
        if kind == UnityKind::Meta {
            let (lookup, _) = lookup_for(&[]);
            let asset = input.path.strip_suffix(".meta").unwrap_or(&input.path);
            return Ok(convert::model(&scene::Model::from_meta(&plain(&text), asset, &lookup)));
        }
        if !is_unity_yaml(&text) {
            return Err(not_unity(&input.path));
        }
        let docs = documents(&text);
        let (lookup, library) = lookup_for(&docs);
        let unity = scene::Model::from_documents(kind, &docs, &input.path, &lookup);
        let mut model = convert::model(&unity);
        match kind {
            UnityKind::AnimationClip => model.curves = views::curves(&docs, &lookup),
            UnityKind::AnimatorController => model.graph = Some(views::controller(&docs, &lookup)),
            UnityKind::Asset if views::shaped(kind, &docs) => {
                model.graph = views::script_graph(&docs);
                model.text = views::readable_graph(&docs);
            }
            UnityKind::Prefab | UnityKind::Scene => model.layout = Some(views::layout(&docs, &unity, &library, &lookup)),
            _ => {}
        }
        model.pictures = views::pictures(&model, &lookup);
        let (scopes, notes) = views::scopes_and_notes(&text, &docs, &lookup);
        model.scopes = scopes;
        model.notes = notes;
        Ok(model)
    }

    fn side_files(&self, input: SideFilesInput) -> Result<Vec<Warning>, Failure> {
        let (Some(before), Some(after)) = (&input.before, &input.after) else {
            return Ok(Vec::new());
        };
        if UnityKind::of(&input.path) != Some(UnityKind::Meta) {
            return Ok(Vec::new());
        }
        Ok(guid::guid_change(&input.path, before, after).map(|change| change.warning()).into_iter().collect())
    }

    fn fix(&self, input: FixInput) -> Result<Fix, Failure> {
        let (Some(before), Some(after)) = (&input.before, &input.after) else {
            return Err(Failure::Unsupported);
        };
        let change = guid::guid_change(&input.path, before, after).ok_or(Failure::Unsupported)?;
        change.fix(&input.path, &input.fix)
    }
}

fn not_unity(path: &str) -> Failure {
    Failure::NotThisFormat {
        message: format!("{path} isn't a Unity text asset"),
    }
}

export!(Unity);
