use std::collections::HashMap;

use sternegit_reader_sdk::{read_text, search};

use crate::{
    scene::{object_names, script_guid, source_prefabs, targets_of},
    value::Lookup,
    yaml::{Document, documents},
};

const DEEPEST_LIBRARY: usize = 3;
const GUID_LINE: &str = "guid: ";
const META_SUFFIX: &str = ".meta";

pub(crate) fn lookup_for(docs: &[Document]) -> (Lookup, HashMap<String, Vec<Document>>) {
    let mut lookup = Lookup {
        assets: every_meta_guid(),
        ..Lookup::default()
    };
    let library = library(docs, &lookup);
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
    lookup.objects.extend(object_names(docs, &lookup.assets));
    (lookup, library)
}

fn library(docs: &[Document], lookup: &Lookup) -> HashMap<String, Vec<Document>> {
    let mut library: HashMap<String, Vec<Document>> = HashMap::new();
    let mut wanted = source_prefabs(docs);
    for _ in 0..DEEPEST_LIBRARY {
        wanted.retain(|guid| !library.contains_key(guid));
        wanted.sort();
        wanted.dedup();
        if wanted.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for guid in &wanted {
            let Some(text) = lookup.asset_path(guid).and_then(read_text) else {
                continue;
            };
            let docs = documents(&text);
            next.extend(source_prefabs(&docs));
            library.insert(guid.clone(), docs);
        }
        wanted = next;
    }
    library
}

fn every_meta_guid() -> HashMap<String, String> {
    let mut owners = HashMap::new();
    for found in search(&format!("^{GUID_LINE}[0-9a-fA-F]{{32}}"), "*.meta") {
        let Some(guid) = found.text.strip_prefix(GUID_LINE) else {
            continue;
        };
        let asset = found.path.strip_suffix(META_SUFFIX).unwrap_or(&found.path);
        owners.entry(guid.trim().to_owned()).or_insert_with(|| asset.to_owned());
    }
    owners
}
