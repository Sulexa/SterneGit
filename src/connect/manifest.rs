use std::path::Path;

use fastgit_protocol::speaks_same_major;
use serde_json::Value;
use url::Url;

use super::{remote::PathPattern, template};
use crate::{
    accounts::{SignInMethod, TokenScheme},
    net::services::ConnectorId,
};

pub(crate) const MANIFEST_FILE: &str = "connector.json";
const ABI_VERSION: u64 = 1;
const LONGEST_ID: usize = 64;
const HOST_SUFFIX: &str = ".host";
const PROBE_DOMAIN: &str = "invalid";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub(crate) id: ConnectorId,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) module: String,
    pub(crate) protocol: String,
    pub(crate) features: Vec<String>,
    pub(crate) ssh_key_type: Option<String>,
    pub(crate) services: Vec<Service>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Service {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) git_user: String,
    pub(crate) fields: Vec<Field>,
    pub(crate) endpoints: Vec<(String, String)>,
    pub(crate) token_endpoints: Vec<String>,
    pub(crate) images: Vec<String>,
    pub(crate) remotes: Vec<RemotePattern>,
    pub(crate) sign_in: Vec<SignIn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) kind: FieldKind,
    pub(crate) example: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Url,
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemotePattern {
    pub(crate) hosts: Vec<String>,
    pub(crate) path: PathPattern,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SignIn {
    pub(crate) method: SignInMethod,
    pub(crate) scheme: TokenScheme,
    pub(crate) client_id: Option<String>,
    pub(crate) scopes: Vec<String>,
    pub(crate) urls: Vec<(String, String)>,
}

impl Manifest {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|error| format!("{MANIFEST_FILE} isn't JSON: {error}"))?;
        let manifest = Place::top(&value);
        let interface = manifest.object("interface")?;
        if interface.value.get("abi").and_then(Value::as_u64) != Some(ABI_VERSION) {
            return Err(format!(
                "The connector doesn't ask for ABI {ABI_VERSION}, the only one this FastGit offers"
            ));
        }
        let protocol = interface.text("protocol")?;
        if !speaks_same_major(&protocol) {
            return Err(format!(
                "The connector speaks protocol {protocol}, which this FastGit doesn't speak"
            ));
        }
        let id = manifest.text("id")?;
        if !is_plain_id(&id) {
            return Err(format!(
                "The connector's id {id:?} must be up to {LONGEST_ID} lowercase letters, digits, - or _"
            ));
        }
        let module = manifest.text("module")?;
        if !is_plain_file_name(&module) {
            return Err(format!("The module {module:?} must be a file beside {MANIFEST_FILE}"));
        }
        let services = manifest
            .objects("services")?
            .into_iter()
            .map(|place| Service::read(&place))
            .collect::<Result<Vec<_>, _>>()?;
        if services.is_empty() {
            return Err("The connector names no service".to_owned());
        }
        if let Some(twice) = first_repeated(services.iter().map(|service| service.id.as_str())) {
            return Err(format!("Two services share the id {twice}"));
        }
        Ok(Self {
            id: ConnectorId(id),
            name: manifest.text("name")?,
            version: manifest.text("version")?,
            module,
            protocol,
            features: manifest.texts("features")?,
            ssh_key_type: manifest.optional_text("ssh_key_type")?,
            services,
        })
    }
}

impl Service {
    fn read(place: &Place) -> Result<Self, String> {
        let id = place.text("id")?;
        if !is_plain_id(&id) && !is_host(&id) {
            return Err(format!("{}: the id {id:?} can't be read", place.path));
        }
        let fields = place
            .optional_objects("fields")?
            .iter()
            .map(Field::read)
            .collect::<Result<Vec<_>, _>>()?;
        let endpoints = place
            .object("endpoints")?
            .entries()?
            .into_iter()
            .map(|(name, template)| {
                probe_url(&template, &fields, &[]).map_err(|problem| format!("{}.endpoints.{name}: {problem}", place.path))?;
                Ok((name, template))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if endpoints.is_empty() {
            return Err(format!("{}: the service names no endpoint", place.path));
        }
        let token_endpoints = place.texts("token_endpoints")?;
        if let Some(unknown) = token_endpoints
            .iter()
            .find(|name| !endpoints.iter().any(|(endpoint, _)| endpoint == *name))
        {
            return Err(format!("{}.token_endpoints names {unknown}, which isn't an endpoint", place.path));
        }
        let images = place.optional_texts("images")?;
        for image in &images {
            probe_url(image, &[], &[]).map_err(|problem| format!("{}.images: {problem}", place.path))?;
        }
        let remotes = place
            .objects("remotes")?
            .iter()
            .map(|remote| RemotePattern::read(remote, &fields))
            .collect::<Result<Vec<_>, _>>()?;
        let sign_in = place
            .objects("sign_in")?
            .iter()
            .map(|sign_in| SignIn::read(sign_in, &fields, &endpoints))
            .collect::<Result<Vec<_>, _>>()?;
        if sign_in.is_empty() {
            return Err(format!("{}: the service offers no way to sign in", place.path));
        }
        Ok(Self {
            id,
            label: place.text("label")?,
            git_user: place.text("git_user")?,
            fields,
            endpoints,
            token_endpoints,
            images,
            remotes,
            sign_in,
        })
    }
}

impl Field {
    fn read(place: &Place) -> Result<Self, String> {
        let id = place.text("id")?;
        if !is_plain_id(&id) {
            return Err(format!("{}: the id {id:?} can't be read", place.path));
        }
        let kind = match place.text("kind")?.as_str() {
            "url" => FieldKind::Url,
            "text" => FieldKind::Text,
            other => return Err(format!("{}: FastGit asks for url or text fields, not {other}", place.path)),
        };
        Ok(Self {
            id,
            label: place.text("label")?,
            kind,
            example: place.optional_text("example")?,
        })
    }
}

impl RemotePattern {
    fn read(place: &Place, fields: &[Field]) -> Result<Self, String> {
        let hosts = place.texts("hosts")?;
        for host in &hosts {
            let probed = template::expand(host, |name| {
                let field = name.strip_suffix(HOST_SUFFIX)?;
                is_url_field(fields, field).then(|| probe_host(field))
            });
            match probed {
                Ok(probed) if is_host(&probed) => {}
                Ok(_) => return Err(format!("{}.hosts: {host} isn't a host name", place.path)),
                Err(problem) => return Err(format!("{}.hosts: {problem}", place.path)),
            }
        }
        if hosts.is_empty() {
            return Err(format!("{}: no host is named", place.path));
        }
        let path = PathPattern::parse(&place.text("path")?).map_err(|problem| format!("{}: {problem}", place.path))?;
        Ok(Self { hosts, path })
    }
}

impl SignIn {
    fn read(place: &Place, fields: &[Field], endpoints: &[(String, String)]) -> Result<Self, String> {
        let method = place.text("method")?;
        let method = SignInMethod::named(&method)
            .ok_or_else(|| format!("{}: FastGit doesn't know the sign-in method {method}", place.path))?;
        let scheme = TokenScheme::from_json(place.object("token")?.value)
            .ok_or_else(|| format!("{}.token: the token scheme can't be read", place.path))?;
        let url_names: &[&str] = match method {
            SignInMethod::Device => &["device_code_url", "token_url"],
            SignInMethod::PastedToken => &["create_url"],
            SignInMethod::GitCredential => &[],
        };
        let mut urls = Vec::new();
        for name in url_names {
            let template = match method {
                SignInMethod::Device => place.text(name)?,
                _ => match place.optional_text(name)? {
                    Some(template) => template,
                    None => continue,
                },
            };
            probe_url(&template, fields, endpoints).map_err(|problem| format!("{}.{name}: {problem}", place.path))?;
            urls.push(((*name).to_owned(), template));
        }
        let client_id = place.optional_text("client_id")?.filter(|id| !id.trim().is_empty());
        if method == SignInMethod::Device && client_id.is_none() {
            return Err(format!("{}: the device flow needs a client_id", place.path));
        }
        Ok(Self {
            method,
            scheme,
            client_id,
            scopes: place.optional_texts("scopes")?,
            urls,
        })
    }
}

fn probe_url(template: &str, fields: &[Field], endpoints: &[(String, String)]) -> Result<Url, String> {
    let expanded = template::expand(template, |name| {
        if let Some((_, endpoint)) = endpoints.iter().find(|(endpoint, _)| endpoint == name) {
            return probe_url(endpoint, fields, &[]).ok().map(String::from);
        }
        let field = fields.iter().find(|field| field.id == name)?;
        Some(match field.kind {
            FieldKind::Url => format!("https://{}", probe_host(&field.id)),
            FieldKind::Text => field.id.clone(),
        })
    })?;
    let url = Url::parse(&expanded).map_err(|error| format!("{template} isn't a URL: {error}"))?;
    match (url.scheme(), url.host_str()) {
        ("https", Some(_)) if url.username().is_empty() && url.password().is_none() => Ok(url),
        _ => Err(format!("{template} must be an https address without a user name")),
    }
}

fn probe_host(field: &str) -> String {
    format!("{}.{PROBE_DOMAIN}", field.replace('_', "-"))
}

fn is_url_field(fields: &[Field], id: &str) -> bool {
    fields.iter().any(|field| field.id == id && field.kind == FieldKind::Url)
}

fn is_plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= LONGEST_ID
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte))
}

fn is_host(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
}

pub(crate) fn is_plain_file_name(name: &str) -> bool {
    Path::new(name)
        .file_name()
        .is_some_and(|file_name| file_name == name)
}

fn first_repeated<'a>(items: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut seen = Vec::new();
    for item in items {
        if seen.contains(&item) {
            return Some(item);
        }
        seen.push(item);
    }
    None
}

struct Place<'a> {
    value: &'a Value,
    path: String,
}

impl<'a> Place<'a> {
    fn top(value: &'a Value) -> Self {
        Self {
            value,
            path: MANIFEST_FILE.to_owned(),
        }
    }

    fn missing(&self, name: &str, what: &str) -> String {
        format!("{}.{name} must be {what}", self.path)
    }

    fn get(&self, name: &str) -> Option<&'a Value> {
        self.value.get(name).filter(|value| !value.is_null())
    }

    fn text(&self, name: &str) -> Result<String, String> {
        self.optional_text(name)?
            .filter(|text| !text.is_empty())
            .ok_or_else(|| self.missing(name, "a string"))
    }

    fn optional_text(&self, name: &str) -> Result<Option<String>, String> {
        match self.get(name) {
            None => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.clone())),
            Some(_) => Err(self.missing(name, "a string")),
        }
    }

    fn texts(&self, name: &str) -> Result<Vec<String>, String> {
        match self.get(name) {
            Some(_) => self.optional_texts(name),
            None => Err(self.missing(name, "a list of strings")),
        }
    }

    fn optional_texts(&self, name: &str) -> Result<Vec<String>, String> {
        let Some(value) = self.get(name) else {
            return Ok(Vec::new());
        };
        value
            .as_array()
            .and_then(|items| items.iter().map(|item| item.as_str().map(str::to_owned)).collect())
            .ok_or_else(|| self.missing(name, "a list of strings"))
    }

    fn object(&self, name: &str) -> Result<Place<'a>, String> {
        match self.get(name) {
            Some(value) if value.is_object() => Ok(Place {
                value,
                path: format!("{}.{name}", self.path),
            }),
            _ => Err(self.missing(name, "an object")),
        }
    }

    fn entries(&self) -> Result<Vec<(String, String)>, String> {
        let object = self.value.as_object().into_iter().flatten();
        object
            .map(|(name, value)| match value.as_str() {
                Some(text) if is_plain_id(name) => Ok((name.clone(), text.to_owned())),
                _ => Err(format!("{}.{name} must be a string named in lowercase", self.path)),
            })
            .collect()
    }

    fn objects(&self, name: &str) -> Result<Vec<Place<'a>>, String> {
        match self.get(name) {
            Some(_) => self.optional_objects(name),
            None => Err(self.missing(name, "a list of objects")),
        }
    }

    fn optional_objects(&self, name: &str) -> Result<Vec<Place<'a>>, String> {
        let Some(value) = self.get(name) else {
            return Ok(Vec::new());
        };
        let items = value
            .as_array()
            .ok_or_else(|| self.missing(name, "a list of objects"))?;
        items
            .iter()
            .enumerate()
            .map(|(index, item)| match item.is_object() {
                true => Ok(Place {
                    value: item,
                    path: format!("{}.{name}[{index}]", self.path),
                }),
                false => Err(format!("{}.{name}[{index}] must be an object", self.path)),
            })
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod samples {
    use serde_json::{Value, json};

    pub(crate) fn github() -> Value {
        serde_json::from_str(include_str!("../../connectors/github/dist/connector.json")).unwrap()
    }

    pub(crate) fn minimal(id: &str) -> Value {
        json!({
            "interface": { "abi": 1, "protocol": "1.0" },
            "id": id, "name": "Fake", "version": "0.0.1", "module": "fake.wasm",
            "features": [],
            "services": [{
                "id": "example.com", "label": "Example", "git_user": "git",
                "endpoints": { "api": "https://api.example.com/v1/" },
                "token_endpoints": ["api"],
                "remotes": [{ "hosts": ["example.com"], "path": "{owner}/{name}" }],
                "sign_in": [{ "method": "pasted_token", "token": { "kind": "bearer" } }]
            }]
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{FieldKind, Manifest, samples};
    use crate::accounts::{SignInMethod, TokenScheme};

    fn parse(value: &Value) -> Result<Manifest, String> {
        Manifest::parse(value.to_string().as_bytes())
    }

    fn refusal(change: impl FnOnce(&mut Value)) -> String {
        let mut manifest = samples::minimal("fake");
        change(&mut manifest);
        parse(&manifest).expect_err("the manifest should be refused")
    }

    #[test]
    fn the_bundled_github_manifest_reads_whole() {
        let github = parse(&samples::github()).unwrap();
        assert_eq!((github.id.0.as_str(), github.module.as_str()), ("github", "github.wasm"));
        let [dot_com, enterprise] = github.services.as_slice() else {
            panic!("expected two services, got {:?}", github.services);
        };
        assert_eq!(dot_com.token_endpoints, ["api", "graphql"]);
        assert_eq!(dot_com.remotes[0].hosts, ["github.com", "ssh.github.com"]);
        assert_eq!(dot_com.sign_in[0].method, SignInMethod::PastedToken);
        assert_eq!(dot_com.sign_in[0].scheme, TokenScheme::Bearer);
        assert_eq!(enterprise.fields[0].kind, FieldKind::Url);
        assert_eq!(
            enterprise.endpoints[0],
            ("api".to_owned(), "{server}/api/v3/".to_owned())
        );
    }

    #[test]
    fn a_manifest_must_ask_for_abi_1_and_the_same_protocol_major() {
        assert!(refusal(|m| m["interface"]["abi"] = json!(2)).contains("ABI 1"));
        assert!(refusal(|m| m["interface"]["protocol"] = json!("2.0")).contains("protocol 2.0"));
        assert!(parse(&{
            let mut later = samples::minimal("fake");
            later["interface"]["protocol"] = json!("1.4");
            later
        })
        .is_ok());
    }

    #[test]
    fn ids_and_the_module_must_be_plain_names() {
        assert!(refusal(|m| m["id"] = json!("../github")).contains("lowercase letters"));
        assert!(refusal(|m| m["id"] = json!("GitHub")).contains("lowercase letters"));
        assert!(refusal(|m| m["module"] = json!("../fake.wasm")).contains("beside connector.json"));
        assert!(refusal(|m| m["module"] = json!("bin/fake.wasm")).contains("beside connector.json"));
    }

    #[test]
    fn missing_or_mistyped_fields_name_their_place() {
        assert_eq!(
            refusal(|m| m["services"][0]["label"] = json!(5)),
            "connector.json.services[0].label must be a string"
        );
        assert_eq!(refusal(|m| m["version"] = Value::Null), "connector.json.version must be a string");
        assert_eq!(refusal(|m| m["services"] = json!([])), "The connector names no service");
    }

    #[test]
    fn endpoints_must_be_https_and_may_only_name_the_services_fields() {
        let plain_http = refusal(|m| m["services"][0]["endpoints"]["api"] = json!("http://api.example.com/"));
        assert!(plain_http.contains("must be an https address"), "{plain_http}");
        let with_user = refusal(|m| m["services"][0]["endpoints"]["api"] = json!("https://me@api.example.com/"));
        assert!(with_user.contains("without a user name"), "{with_user}");
        let unknown = refusal(|m| m["services"][0]["endpoints"]["api"] = json!("{server}/api/"));
        assert!(unknown.contains("names {server}, which isn't defined"), "{unknown}");
        let mut with_field = samples::minimal("fake");
        with_field["services"][0]["fields"] = json!([{ "id": "server", "label": "Server", "kind": "url" }]);
        with_field["services"][0]["endpoints"]["api"] = json!("{server}/api/");
        with_field["services"][0]["remotes"][0]["hosts"] = json!(["{server.host}"]);
        assert!(parse(&with_field).is_ok());
    }

    #[test]
    fn the_token_may_only_go_to_named_endpoints_and_in_a_known_scheme() {
        let stray = refusal(|m| m["services"][0]["token_endpoints"] = json!(["api", "upload"]));
        assert!(stray.contains("names upload, which isn't an endpoint"), "{stray}");
        let in_url = refusal(|m| m["services"][0]["sign_in"][0]["token"] = json!({ "kind": "query", "name": "access_token" }));
        assert!(in_url.contains("token scheme can't be read"), "{in_url}");
    }

    #[test]
    fn remote_hosts_must_be_host_names_or_a_url_fields_host() {
        assert!(refusal(|m| m["services"][0]["remotes"][0]["hosts"] = json!(["example.com/x"])).contains("isn't a host name"));
        assert!(refusal(|m| m["services"][0]["remotes"][0]["hosts"] = json!(["{server.host}"])).contains("isn't defined"));
        assert!(refusal(|m| m["services"][0]["remotes"][0]["path"] = json!("{name}")).contains("owner part"));
    }

    #[test]
    fn the_device_flow_needs_its_client_id_and_https_addresses() {
        let device = |m: &mut Value| {
            m["services"][0]["sign_in"] = json!([{
                "method": "device", "token": { "kind": "bearer" }, "client_id": "Ov23li",
                "device_code_url": "https://example.com/login/device/code",
                "token_url": "https://example.com/login/oauth/access_token",
                "scopes": ["repo"]
            }]);
        };
        let mut manifest = samples::minimal("fake");
        device(&mut manifest);
        let sign_in = &parse(&manifest).unwrap().services[0].sign_in[0];
        assert_eq!(sign_in.client_id.as_deref(), Some("Ov23li"));
        assert_eq!(sign_in.urls.len(), 2);
        assert!(refusal(|m| {
            device(m);
            m["services"][0]["sign_in"][0]["client_id"] = json!(" ");
        })
        .contains("needs a client_id"));
        assert!(refusal(|m| {
            device(m);
            m["services"][0]["sign_in"][0]["token_url"] = json!("http://example.com/token");
        })
        .contains("token_url"));
    }

    #[test]
    fn a_create_url_may_name_the_services_endpoints() {
        let mut manifest = samples::minimal("fake");
        manifest["services"][0]["endpoints"]["web"] = json!("https://example.com/");
        manifest["services"][0]["sign_in"][0]["create_url"] = json!("{web}settings/tokens");
        assert_eq!(
            parse(&manifest).unwrap().services[0].sign_in[0].urls,
            [("create_url".to_owned(), "{web}settings/tokens".to_owned())]
        );
    }
}
