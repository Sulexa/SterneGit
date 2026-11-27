use std::{fmt::Debug, sync::Arc};

use serde_json::Value;

use super::{Pin, Stop};

pub(crate) trait Kind: Sized + 'static {
    type Id: Clone + Debug + PartialEq;
    type Manifest: Debug;
    type Grant: Clone + Debug + PartialEq;
    type Started;
    type Host: ?Sized;

    const MANIFEST_FILE: &'static str;
    const APPROVALS_FILE: &'static str;
    const APPROVALS_LIST: &'static str;
    const NOUN: &'static str;

    fn pins() -> &'static [Pin];
    fn id_named(name: String) -> Self::Id;
    fn name_of(id: &Self::Id) -> &str;
    fn parse(bytes: &[u8]) -> Result<Self::Manifest, String>;
    fn id_of(manifest: &Self::Manifest) -> &Self::Id;
    fn version_of(manifest: &Self::Manifest) -> &str;
    fn module_of(manifest: &Self::Manifest) -> &str;
    fn asked(manifest: &Self::Manifest) -> Self::Grant;
    fn beyond(asked: &Self::Grant, granted: &Self::Grant) -> Option<Self::Grant>;
    fn grant_from_json(value: &Value) -> Self::Grant;
    fn grant_to_json(grant: &Self::Grant) -> Value;
    fn start(id: &Self::Id, wasm: &[u8], host: &Arc<Self::Host>) -> Result<Self::Started, Stop>;
}
