use sternegit_sdk::{
    CloneUrls, Failure, FieldError, NewRepository, Owner, OwnerKind, Page, Repository,
    RepositoryQuery, RepoRef, Request, Visibility,
};
use serde_json::{Value, json};

use crate::{
    api,
    place::{Collection, VERSION, branch, encoded, without_user},
    refusal::succeeded,
};

const PAGES_READ_AT_MOST: usize = 20;
const CONTINUATION: &str = "x-ms-continuationtoken";

pub(crate) fn owners(collection: &Collection) -> Result<Vec<Owner>, Failure> {
    let mut owners: Vec<Owner> = projects()?
        .iter()
        .filter_map(|project| {
            Some(Owner {
                login: collection.owner(project["name"].as_str()?),
                kind: OwnerKind::Project,
                avatar_url: None,
                can_create: true,
                visibilities: match visibility(&project["visibility"]) {
                    Visibility::Unknown => Vec::new(),
                    known => vec![known],
                },
                can_initialize: false,
            })
        })
        .collect();
    owners.sort_by_key(|owner| owner.login.to_lowercase());
    Ok(owners)
}

pub(crate) fn list(collection: &Collection, query: RepositoryQuery) -> Result<Page<Repository>, Failure> {
    let url = match query.owner.as_deref().and_then(|owner| collection.project_of(owner)) {
        Some(project) => format!("{}/_apis/git/repositories?{VERSION}", encoded(&project)),
        None => format!("_apis/git/repositories?{VERSION}"),
    };
    let answer: Value = succeeded(api(Request::get(url)).fetch())?.json()?;
    let wanted = query
        .search
        .map(|search| search.trim().to_lowercase())
        .filter(|search| !search.is_empty());
    let mut items = listed(&answer)?
        .iter()
        .map(|found| repository(collection, found))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(wanted) = wanted {
        items.retain(|item| item.repo.name.to_lowercase().contains(&wanted));
    }
    items.sort_by_key(|item| (item.repo.owner.to_lowercase(), item.repo.name.to_lowercase()));
    Ok(Page { items, next: None })
}

pub(crate) fn get(collection: &Collection, repo: &RepoRef) -> Result<Repository, Failure> {
    let url = format!("{}?{VERSION}", collection.repository(repo));
    repository(collection, &succeeded(api(Request::get(url)).fetch())?.json()?)
}

pub(crate) fn create(collection: &Collection, new: NewRepository) -> Result<Repository, Failure> {
    let project = collection.project_of(&new.owner).ok_or_else(|| Failure::Invalid {
        fields: vec![FieldError {
            field: "owner".to_owned(),
            message: "names no project".to_owned(),
        }],
        message: format!("{} names no project", new.owner),
    })?;
    let url = format!("_apis/projects/{}?{VERSION}", encoded(&project));
    let found: Value = succeeded(api(Request::get(url)).fetch())?.json()?;
    let id = found["id"].as_str().ok_or_else(|| Failure::Protocol {
        message: format!("Azure DevOps gave no id for the project {project}"),
    })?;
    let url = format!("{}/_apis/git/repositories?{VERSION}", encoded(&project));
    let body = json!({ "name": new.name.trim(), "project": { "id": id } });
    match succeeded(api(Request::post(url)).json(&body).fetch()) {
        Ok(created) => repository(collection, &created.json()?),
        Err(Failure::Conflict { message }) => Err(Failure::Invalid {
            fields: vec![FieldError {
                field: "name".to_owned(),
                message: message.clone(),
            }],
            message,
        }),
        Err(other) => Err(other),
    }
}

pub(crate) fn listed(answer: &Value) -> Result<&Vec<Value>, Failure> {
    answer["value"].as_array().ok_or_else(|| Failure::Protocol {
        message: "Azure DevOps answered without a list".to_owned(),
    })
}

fn projects() -> Result<Vec<Value>, Failure> {
    let mut found = Vec::new();
    let mut continuation: Option<String> = None;
    for _ in 0..PAGES_READ_AT_MOST {
        let mut url = format!("_apis/projects?$top=100&{VERSION}");
        if let Some(token) = &continuation {
            url.push_str(&format!("&continuationToken={}", encoded(token)));
        }
        let response = succeeded(api(Request::get(url)).fetch())?;
        let page: Value = response.json()?;
        found.extend(listed(&page)?.iter().cloned());
        continuation = response
            .header(CONTINUATION)
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_owned);
        if continuation.is_none() {
            break;
        }
    }
    Ok(found)
}

fn repository(collection: &Collection, found: &Value) -> Result<Repository, Failure> {
    let repo = collection.repo_ref(found).ok_or_else(|| Failure::Protocol {
        message: "Azure DevOps listed a repository without its name or project".to_owned(),
    })?;
    let https = found["remoteUrl"]
        .as_str()
        .map(without_user)
        .unwrap_or_else(|| collection.web(&repo));
    Ok(Repository {
        description: None,
        visibility: visibility(&found["project"]["visibility"]),
        default_branch: found["defaultBranch"].as_str().map(|_| branch(&found["defaultBranch"])),
        clone: CloneUrls {
            https,
            ssh: found["sshUrl"].as_str().map(str::to_owned),
        },
        web_url: found["webUrl"].as_str().map_or_else(|| collection.web(&repo), str::to_owned),
        updated_at: None,
        fork: found["isFork"].as_bool().unwrap_or(false),
        archived: found["isDisabled"].as_bool().unwrap_or(false),
        repo,
    })
}

fn visibility(value: &Value) -> Visibility {
    match value.as_str() {
        Some("private") => Visibility::Private,
        Some("public") => Visibility::Public,
        Some("organization") => Visibility::Internal,
        _ => Visibility::Unknown,
    }
}
