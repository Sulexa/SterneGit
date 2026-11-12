use sternegit_sdk::{AccountContext, Failure, RepoRef};
use serde_json::Value;

pub(crate) const VERSION: &str = "api-version=6.0";
pub(crate) const PREVIEW: &str = "api-version=6.0-preview.1";

const API_ENDPOINT: &str = "api";
const UNRESERVED: &[u8] = b"-._~";
const BRANCHES: &str = "refs/heads/";

pub(crate) struct Collection {
    base: String,
    path: Vec<String>,
}

impl Collection {
    pub(crate) fn of(account: &AccountContext) -> Result<Self, Failure> {
        let api = account.endpoint(API_ENDPOINT).ok_or_else(|| Failure::Protocol {
            message: "The account has no api endpoint".to_owned(),
        })?;
        Ok(Self::at(api))
    }

    pub(crate) fn at(api: &str) -> Self {
        let base = format!("{}/", api.trim_end_matches('/'));
        let after_scheme = base.split_once("://").map_or(base.as_str(), |(_, rest)| rest);
        let path = after_scheme
            .split_once('/')
            .map(|(_, path)| path)
            .unwrap_or_default()
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(decoded)
            .collect();
        Self { base, path }
    }

    pub(crate) fn user_id(&self, user: &str) -> String {
        let scope: Vec<String> = self
            .path
            .iter()
            .map(|segment| {
                segment
                    .to_ascii_lowercase()
                    .chars()
                    .map(|character| match character.is_ascii_alphanumeric() || "-_".contains(character) {
                        true => character,
                        false => '-',
                    })
                    .collect()
            })
            .collect();
        match scope.is_empty() {
            true => user.to_owned(),
            false => format!("{}.{user}", scope.join(".")),
        }
    }

    pub(crate) fn owner(&self, project: &str) -> String {
        self.path
            .iter()
            .map(String::as_str)
            .chain([project])
            .collect::<Vec<_>>()
            .join("/")
    }

    pub(crate) fn project_of(&self, owner: &str) -> Option<String> {
        let parts: Vec<String> = owner
            .split('/')
            .filter(|part| !part.is_empty())
            .map(decoded)
            .collect();
        let within = self.path.len() <= parts.len()
            && self
                .path
                .iter()
                .zip(&parts)
                .all(|(mine, theirs)| mine.eq_ignore_ascii_case(theirs));
        let rest = match within {
            true => &parts[self.path.len()..],
            false => &parts[..],
        };
        rest.last().cloned()
    }

    pub(crate) fn project(&self, repo: &RepoRef) -> String {
        self.project_of(&repo.owner).unwrap_or_else(|| decoded(&repo.name))
    }

    pub(crate) fn repository(&self, repo: &RepoRef) -> String {
        format!(
            "{}/_apis/git/repositories/{}",
            encoded(&self.project(repo)),
            encoded(&decoded(&repo.name))
        )
    }

    pub(crate) fn web(&self, repo: &RepoRef) -> String {
        self.in_project(repo, &format!("_git/{}", encoded(&decoded(&repo.name))))
    }

    pub(crate) fn in_project(&self, repo: &RepoRef, rest: &str) -> String {
        format!("{}{}/{rest}", self.base, encoded(&self.project(repo)))
    }

    pub(crate) fn repo_ref(&self, repository: &Value) -> Option<RepoRef> {
        Some(RepoRef {
            owner: self.owner(repository["project"]["name"].as_str()?),
            name: repository["name"].as_str()?.to_owned(),
        })
    }
}

pub(crate) fn branch(reference: &Value) -> String {
    let name = reference.as_str().unwrap_or_default();
    name.strip_prefix(BRANCHES).unwrap_or(name).to_owned()
}

pub(crate) fn reference(branch: &str) -> String {
    match branch.starts_with("refs/") {
        true => branch.to_owned(),
        false => format!("{BRANCHES}{branch}"),
    }
}

pub(crate) fn without_user(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let authority_ends = rest.find('/').unwrap_or(rest.len());
    match rest[..authority_ends].rsplit_once('@') {
        Some((_, host)) => format!("{scheme}://{host}{}", &rest[authority_ends..]),
        None => url.to_owned(),
    }
}

pub(crate) fn encoded(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || UNRESERVED.contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

pub(crate) fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = (bytes[at] == b'%')
            .then(|| text.get(at + 1..at + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                at += 3;
            }
            None => {
                decoded.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use sternegit_sdk::RepoRef;
    use serde_json::json;

    use super::{Collection, branch, decoded, encoded, reference, without_user};

    fn repo(owner: &str, name: &str) -> RepoRef {
        RepoRef {
            owner: owner.into(),
            name: name.into(),
        }
    }

    #[test]
    fn an_organization_on_dev_azure_com_is_the_first_part_of_every_owner() {
        let contoso = Collection::at("https://dev.azure.com/contoso/");
        assert_eq!(contoso.owner("Web"), "contoso/Web");
        assert_eq!(contoso.user_id("6c8f1a2b-0d4e"), "contoso.6c8f1a2b-0d4e");
        assert_eq!(contoso.project(&repo("contoso/Web", "app")), "Web");
        assert_eq!(contoso.project(&repo("Contoso/Game%20Tools", "app")), "Game Tools");
        assert_eq!(contoso.project(&repo("contoso", "app")), "app");
        assert_eq!(contoso.repository(&repo("contoso/Game%20Tools", "build%20scripts")), "Game%20Tools/_apis/git/repositories/build%20scripts");
        assert_eq!(contoso.web(&repo("contoso/Web", "app")), "https://dev.azure.com/contoso/Web/_git/app");
    }

    #[test]
    fn a_collection_on_a_server_keeps_its_whole_path_and_an_old_address_has_none() {
        let server = Collection::at("http://devops.example.com:8080/tfs/DefaultCollection");
        assert_eq!(server.owner("Game"), "tfs/DefaultCollection/Game");
        assert_eq!(server.user_id("1"), "tfs.defaultcollection.1");
        assert_eq!(server.project(&repo("tfs/DefaultCollection/Game", "client")), "Game");
        let old = Collection::at("https://contoso.visualstudio.com/");
        assert_eq!(old.owner("Web"), "Web");
        assert_eq!(old.user_id("1"), "1");
        assert_eq!(old.project(&repo("DefaultCollection/Web", "app")), "Web");
        let found = old.repo_ref(&json!({ "name": "app", "project": { "name": "Web" } }));
        assert_eq!(found, Some(repo("Web", "app")));
    }

    #[test]
    fn names_are_encoded_once_whether_or_not_they_came_encoded() {
        assert_eq!(encoded("Game Tools"), "Game%20Tools");
        assert_eq!(encoded(&decoded("Game%20Tools")), "Game%20Tools");
        assert_eq!(decoded("%C3%A9t%C3%A9 100%"), "été 100%");
        assert_eq!(encoded("a/b&c"), "a%2Fb%26c");
    }

    #[test]
    fn branches_drop_and_regain_their_refs_heads_prefix() {
        assert_eq!(branch(&json!("refs/heads/topic/tabs")), "topic/tabs");
        assert_eq!(reference("topic/tabs"), "refs/heads/topic/tabs");
        assert_eq!(reference("refs/heads/main"), "refs/heads/main");
    }

    #[test]
    fn a_clone_address_loses_the_user_azure_devops_writes_into_it() {
        assert_eq!(
            without_user("https://contoso@dev.azure.com/contoso/Web/_git/app"),
            "https://dev.azure.com/contoso/Web/_git/app"
        );
        assert_eq!(without_user("https://dev.azure.com/contoso/Web/_git/app"), "https://dev.azure.com/contoso/Web/_git/app");
        assert_eq!(without_user("git@ssh.dev.azure.com:v3/contoso/Web/app"), "git@ssh.dev.azure.com:v3/contoso/Web/app");
    }
}
