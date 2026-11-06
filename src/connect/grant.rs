use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::manifest::Manifest;

const ENDPOINTS: &str = "endpoints";
const IMAGES: &str = "images";
const REMOTE_HOSTS: &str = "remote_hosts";
const SIGN_IN: &str = "sign_in";
const FEATURES: &str = "features";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Grant {
    pub(crate) endpoints: BTreeSet<String>,
    pub(crate) images: BTreeSet<String>,
    pub(crate) remote_hosts: BTreeSet<String>,
    pub(crate) sign_in: BTreeSet<String>,
    pub(crate) features: BTreeSet<String>,
}

impl Grant {
    pub(crate) fn asked_by(manifest: &Manifest) -> Self {
        let mut asked = Self::default();
        for service in &manifest.services {
            asked
                .endpoints
                .extend(service.endpoints.iter().map(|(_, template)| template.clone()));
            asked.images.extend(service.images.iter().cloned());
            asked
                .remote_hosts
                .extend(service.remotes.iter().flat_map(|remote| remote.hosts.iter().cloned()));
            asked.sign_in.extend(
                service
                    .sign_in
                    .iter()
                    .flat_map(|sign_in| sign_in.urls.iter().map(|(_, template)| template.clone())),
            );
        }
        asked.features.extend(manifest.features.iter().cloned());
        asked
    }

    pub(crate) fn beyond(&self, granted: &Grant) -> Option<Grant> {
        let more = |asked: &BTreeSet<String>, granted: &BTreeSet<String>| {
            asked.difference(granted).cloned().collect()
        };
        let additions = Grant {
            endpoints: more(&self.endpoints, &granted.endpoints),
            images: more(&self.images, &granted.images),
            remote_hosts: more(&self.remote_hosts, &granted.remote_hosts),
            sign_in: more(&self.sign_in, &granted.sign_in),
            features: more(&self.features, &granted.features),
        };
        (!additions.is_empty()).then_some(additions)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lists().iter().all(|(_, list)| list.is_empty())
    }

    pub(crate) fn from_json(value: &Value) -> Self {
        let list = |name: &str| -> BTreeSet<String> {
            value
                .get(name)
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .unwrap_or_default()
        };
        Self {
            endpoints: list(ENDPOINTS),
            images: list(IMAGES),
            remote_hosts: list(REMOTE_HOSTS),
            sign_in: list(SIGN_IN),
            features: list(FEATURES),
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        let fields: Map<String, Value> = self
            .lists()
            .into_iter()
            .map(|(name, list)| (name.to_owned(), list.iter().cloned().collect()))
            .collect();
        Value::Object(fields)
    }

    fn lists(&self) -> [(&'static str, &BTreeSet<String>); 5] {
        [
            (ENDPOINTS, &self.endpoints),
            (IMAGES, &self.images),
            (REMOTE_HOSTS, &self.remote_hosts),
            (SIGN_IN, &self.sign_in),
            (FEATURES, &self.features),
        ]
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Grant;
    use crate::connect::manifest::{Manifest, samples};

    fn github() -> Grant {
        let manifest = Manifest::parse(samples::github().to_string().as_bytes()).unwrap();
        Grant::asked_by(&manifest)
    }

    fn set(items: &[&str]) -> std::collections::BTreeSet<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn a_connector_asks_for_every_origin_host_and_sign_in_address_its_services_name() {
        let asked = github();
        assert_eq!(
            asked.endpoints,
            set(&[
                "https://api.github.com/",
                "https://api.github.com/graphql",
                "https://github.com/",
                "{server}/",
                "{server}/api/graphql",
                "{server}/api/v3/",
            ])
        );
        assert_eq!(asked.images, set(&["https://avatars.githubusercontent.com/"]));
        assert_eq!(asked.remote_hosts, set(&["github.com", "ssh.github.com", "{server.host}"]));
        assert_eq!(asked.sign_in.len(), 2);
    }

    #[test]
    fn only_what_the_grant_lacks_is_asked_again() {
        let granted = github();
        assert_eq!(granted.beyond(&granted), None);
        let mut update = granted.clone();
        update.features.insert("pulls.review".into());
        update.remote_hosts.insert("gist.github.com".into());
        update.endpoints.remove("https://github.com/");
        assert_eq!(
            update.beyond(&granted),
            Some(Grant {
                remote_hosts: set(&["gist.github.com"]),
                features: set(&["pulls.review"]),
                ..Grant::default()
            })
        );
    }

    #[test]
    fn a_grant_reads_back_what_it_wrote_and_ignores_strays() {
        let granted = github();
        assert_eq!(Grant::from_json(&granted.to_json()), granted);
        let stray = json!({ "features": ["pulls", 7], "unknown": ["x"] });
        assert_eq!(Grant::from_json(&stray), Grant { features: set(&["pulls"]), ..Grant::default() });
    }
}
