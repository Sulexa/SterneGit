use fastgit_sdk::{
    AccountContext, CloneUrls, Failure, NewRepository, Owner, OwnerKind, Page, RepoRef, Repository,
    RepositoryQuery, Request, Visibility,
};
use serde_json::{Value, json};

use crate::{
    api,
    listing::{cursor_on_api, encoded, every_page, on_api, repo_path, text},
    refusal::succeeded,
};

const PER_PAGE: &str = "per_page=100";
const ADMIN: &str = "admin";

pub(crate) fn owners(account: &AccountContext) -> Result<Vec<Owner>, Failure> {
    let me: Value = succeeded(api(Request::get("user")).fetch())?.json()?;
    let mut owners = vec![Owner {
        login: text(&me["login"]).unwrap_or_else(|| account.login.clone()),
        kind: OwnerKind::User,
        avatar_url: text(&me["avatar_url"]),
        can_create: true,
        visibilities: vec![Visibility::Public, Visibility::Private],
        can_initialize: true,
    }];
    let memberships = every_page(account, &format!("user/memberships/orgs?state=active&{PER_PAGE}"))?;
    let members_only: Vec<_> = memberships
        .iter()
        .filter(|membership| membership["role"].as_str() != Some(ADMIN))
        .filter_map(|membership| text(&membership["organization"]["login"]))
        .map(|login| (login.clone(), api(Request::get(format!("orgs/{}", encoded(&login)))).send()))
        .collect();
    let mut settings: Vec<(String, Value)> = Vec::new();
    for (login, sent) in members_only {
        let response = sent.wait();
        if response.succeeded() {
            settings.push((login, response.json()?));
        }
    }
    for membership in &memberships {
        let organization = &membership["organization"];
        let Some(login) = text(&organization["login"]) else {
            continue;
        };
        let admin = membership["role"].as_str() == Some(ADMIN);
        let rules = settings
            .iter()
            .find(|(known, _)| *known == login)
            .map(|(_, rules)| rules);
        owners.push(organization_owner(login, text(&organization["avatar_url"]), admin, rules));
    }
    Ok(owners)
}

fn organization_owner(login: String, avatar_url: Option<String>, admin: bool, rules: Option<&Value>) -> Owner {
    let allows = |name: &str| admin || rules.and_then(|rules| rules[name].as_bool()).unwrap_or(true);
    let mut visibilities = Vec::new();
    if allows("members_can_create_public_repositories") {
        visibilities.push(Visibility::Public);
    }
    if allows("members_can_create_private_repositories") {
        visibilities.push(Visibility::Private);
    }
    if rules.and_then(|rules| rules["members_can_create_internal_repositories"].as_bool()) == Some(true) {
        visibilities.push(Visibility::Internal);
    }
    Owner {
        login,
        kind: OwnerKind::Organization,
        avatar_url,
        can_create: allows("members_can_create_repositories") && !visibilities.is_empty(),
        visibilities,
        can_initialize: true,
    }
}

pub(crate) fn list(account: &AccountContext, query: RepositoryQuery) -> Result<Page<Repository>, Failure> {
    let search = query.search.as_deref().map(str::trim).filter(|search| !search.is_empty());
    let url = match (query.cursor, search, query.owner.as_deref()) {
        (Some(cursor), ..) => cursor_on_api(account, cursor)?,
        (None, Some(search), owner) => search_url(account, search, owner)?,
        (None, None, Some(owner)) if owner.eq_ignore_ascii_case(&account.login) => {
            format!("user/repos?affiliation=owner&sort=updated&{PER_PAGE}")
        }
        (None, None, Some(owner)) => format!("orgs/{}/repos?sort=updated&{PER_PAGE}", encoded(owner)),
        (None, None, None) => {
            format!("user/repos?affiliation=owner,collaborator,organization_member&sort=updated&{PER_PAGE}")
        }
    };
    let response = succeeded(api(Request::get(url)).fetch())?;
    let body: Value = response.json()?;
    let listed = match &body {
        Value::Array(items) => items,
        found => found["items"].as_array().ok_or_else(|| Failure::Protocol {
            message: "GitHub's answer lists no repositories".to_owned(),
        })?,
    };
    Ok(Page {
        items: listed.iter().map(repository).collect::<Result<_, _>>()?,
        next: on_api(account, response.next_page()),
    })
}

fn search_url(account: &AccountContext, search: &str, owner: Option<&str>) -> Result<String, Failure> {
    let owners = match owner {
        Some(owner) => vec![owner.to_owned()],
        None => {
            let organizations: Vec<Value> = succeeded(api(Request::get(format!("user/orgs?{PER_PAGE}"))).fetch())?.json()?;
            std::iter::once(account.login.clone())
                .chain(organizations.iter().filter_map(|organization| text(&organization["login"])))
                .collect()
        }
    };
    let qualifiers: String = owners.iter().map(|owner| format!(" user:{owner}")).collect();
    let terms = format!("{search} in:name fork:true{qualifiers}");
    Ok(format!("search/repositories?q={}&sort=updated&{PER_PAGE}", encoded(&terms)))
}

pub(crate) fn get(repo: &RepoRef) -> Result<Repository, Failure> {
    repository(&succeeded(api(Request::get(repo_path(repo))).fetch())?.json()?)
}

pub(crate) fn create(account: &AccountContext, new: NewRepository) -> Result<Repository, Failure> {
    let url = if new.owner.eq_ignore_ascii_case(&account.login) {
        "user/repos".to_owned()
    } else {
        format!("orgs/{}/repos", encoded(&new.owner))
    };
    let mut body = json!({
        "name": new.name,
        "private": new.visibility != Visibility::Public,
        "auto_init": new.initialize,
    });
    if let Some(description) = new.description.filter(|description| !description.trim().is_empty()) {
        body["description"] = description.into();
    }
    if new.visibility == Visibility::Internal {
        body["visibility"] = "internal".into();
    }
    repository(&succeeded(api(Request::post(url)).json(&body).fetch())?.json()?)
}

pub(crate) fn repository(item: &Value) -> Result<Repository, Failure> {
    let (Some(owner), Some(name), Some(https), Some(web_url)) = (
        text(&item["owner"]["login"]),
        text(&item["name"]),
        text(&item["clone_url"]),
        text(&item["html_url"]),
    ) else {
        return Err(Failure::Protocol {
            message: "GitHub described a repository without its owner, name or addresses".to_owned(),
        });
    };
    Ok(Repository {
        repo: RepoRef { owner, name },
        description: text(&item["description"]).filter(|description| !description.trim().is_empty()),
        visibility: visibility(item),
        default_branch: text(&item["default_branch"]),
        clone: CloneUrls {
            https,
            ssh: text(&item["ssh_url"]),
        },
        web_url,
        updated_at: text(&item["pushed_at"]).or_else(|| text(&item["updated_at"])),
        fork: item["fork"].as_bool().unwrap_or(false),
        archived: item["archived"].as_bool().unwrap_or(false),
    })
}

fn visibility(item: &Value) -> Visibility {
    match item["visibility"].as_str() {
        Some("public") => Visibility::Public,
        Some("private") => Visibility::Private,
        Some("internal") => Visibility::Internal,
        _ if item["private"].as_bool() == Some(true) => Visibility::Private,
        _ => Visibility::Public,
    }
}

#[cfg(test)]
mod tests {
    use fastgit_sdk::{OwnerKind, Visibility};
    use serde_json::json;

    use super::{organization_owner, visibility};

    #[test]
    fn an_organization_admin_may_create_any_visibility() {
        let owner = organization_owner("acme".into(), None, true, None);
        assert_eq!(owner.kind, OwnerKind::Organization);
        assert!(owner.can_create);
        assert_eq!(owner.visibilities, [Visibility::Public, Visibility::Private]);
    }

    #[test]
    fn a_member_follows_the_organizations_rules_when_github_shows_them() {
        let rules = json!({
            "members_can_create_repositories": true,
            "members_can_create_public_repositories": false,
            "members_can_create_private_repositories": true,
            "members_can_create_internal_repositories": true
        });
        let owner = organization_owner("acme".into(), None, false, Some(&rules));
        assert!(owner.can_create);
        assert_eq!(owner.visibilities, [Visibility::Private, Visibility::Internal]);
        let closed = json!({ "members_can_create_repositories": false });
        assert!(!organization_owner("acme".into(), None, false, Some(&closed)).can_create);
    }

    #[test]
    fn visibility_falls_back_to_the_private_flag() {
        assert_eq!(visibility(&json!({ "visibility": "internal" })), Visibility::Internal);
        assert_eq!(visibility(&json!({ "private": true })), Visibility::Private);
        assert_eq!(visibility(&json!({})), Visibility::Public);
    }
}
