use serde::{Deserialize, Serialize};

pub type Cursor = String;
pub type Timestamp = String;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<Cursor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    User,
    Organization,
    Group,
    Project,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
    Internal,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    pub login: String,
    pub kind: OwnerKind,
    pub avatar_url: Option<String>,
    pub can_create: bool,
    pub visibilities: Vec<Visibility>,
    pub can_initialize: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloneUrls {
    pub https: String,
    pub ssh: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    pub repo: RepoRef,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub default_branch: Option<String>,
    pub clone: CloneUrls,
    pub web_url: String,
    pub updated_at: Option<Timestamp>,
    pub fork: bool,
    pub archived: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryQuery {
    pub search: Option<String>,
    pub owner: Option<String>,
    pub cursor: Option<Cursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewRepository {
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub initialize: bool,
}
