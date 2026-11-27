#![allow(dead_code)]

mod convert;
mod lookup;
mod names;
mod scene;
mod size;
mod value;
mod yaml;

use sternegit_reader_sdk::{Failure, Model, ReadInput, Reader, export, read_text};

use crate::{
    lookup::lookup_for,
    names::UnityKind,
    yaml::{documents, is_unity_yaml},
};

pub struct Unity;

impl Reader for Unity {
    fn read(&self, input: ReadInput) -> Result<Model, Failure> {
        let kind = UnityKind::of(&input.path).ok_or_else(|| not_unity(&input.path))?;
        let text = read_text(&input.path).ok_or_else(|| Failure::Missing {
            path: input.path.clone(),
        })?;
        if !is_unity_yaml(&text) {
            return Err(not_unity(&input.path));
        }
        let docs = documents(&text);
        let lookup = lookup_for(&docs);
        let unity = scene::Model::from_documents(kind, &docs, &input.path, &lookup);
        Ok(convert::model(&unity))
    }
}

fn not_unity(path: &str) -> Failure {
    Failure::NotThisFormat {
        message: format!("{path} isn't a Unity text asset"),
    }
}

export!(Unity);
