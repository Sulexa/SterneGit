use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use fastgit_protocol::{
    Cassette, Exchange,
    ops::{self, Op, Visit},
};
use serde_json::Value;

use super::{
    CallError, Forge, Greeting, Outcome, ReplayHost, Started, Stop, check_answer,
    manifest::{MANIFEST_FILE, Manifest},
};
use crate::net::services::ConnectorId;

pub(crate) struct Trial {
    pub(crate) id: ConnectorId,
    pub(crate) version: String,
    pub(crate) module: PathBuf,
    pub(crate) module_bytes: usize,
    pub(crate) start: Result<Greeting, Stop>,
    pub(crate) start_time: Duration,
    pub(crate) start_log: Vec<String>,
    pub(crate) played: Option<Played>,
}

pub(crate) struct Played {
    pub(crate) op: &'static str,
    pub(crate) cassette: PathBuf,
    pub(crate) outcome: Outcome,
    pub(crate) answer: Result<(), CallError>,
    pub(crate) log: Vec<String>,
    pub(crate) unplayed: Vec<Exchange>,
    pub(crate) expected: Option<Value>,
}

impl Trial {
    pub(crate) fn passed(&self) -> bool {
        self.start.is_ok() && self.played.as_ref().is_none_or(Played::passed)
    }
}

impl Played {
    pub(crate) fn output(&self) -> Option<Value> {
        let output = self.outcome.result.as_ref().ok()?;
        serde_json::from_slice(output).ok()
    }

    pub(crate) fn matches_expected(&self) -> Option<bool> {
        let expected = self.expected.as_ref()?;
        Some(self.output().as_ref() == Some(expected))
    }

    pub(crate) fn passed(&self) -> bool {
        let answered = !matches!(self.answer, Err(CallError::Stopped(_) | CallError::Net(_)));
        answered && self.unplayed.is_empty() && self.matches_expected() != Some(false)
    }
}

pub(crate) fn try_connector(folder: &Path, cassette: Option<&Path>) -> Result<Trial, String> {
    let manifest = read_manifest(folder)?;
    let module = folder.join(&manifest.module);
    let wasm = fs::read(&module).map_err(|error| format!("Can't read {}: {error}", module.display()))?;
    let cassette = match cassette {
        Some(path) => Some((path.to_owned(), read_cassette(path)?)),
        None => None,
    };
    let host = Arc::new(match &cassette {
        Some((_, cassette)) => ReplayHost::playing(cassette)?,
        None => ReplayHost::idle(),
    });
    let clock = Instant::now();
    let start = Started::start(manifest.id.clone(), &wasm, host.clone());
    let start_time = clock.elapsed();
    let start_log = host.take_log();
    let (start, played) = match (start, cassette) {
        (Ok((started, greeting)), Some((path, cassette))) => {
            (Ok(greeting), Some(play(Arc::new(started), &host, path, cassette)?))
        }
        (Ok((_, greeting)), None) => (Ok(greeting), None),
        (Err(stop), _) => (Err(stop), None),
    };
    Ok(Trial {
        id: manifest.id,
        version: manifest.version,
        module,
        module_bytes: wasm.len(),
        start,
        start_time,
        start_log,
        played,
    })
}

fn play(
    started: Arc<Started>,
    host: &Arc<ReplayHost>,
    cassette_path: PathBuf,
    cassette: Cassette,
) -> Result<Played, String> {
    struct Described;

    impl Visit for Described {
        type Result = (&'static str, bool);

        fn visit<O: Op>(self) -> Self::Result {
            (O::NAME, O::ON_REPOSITORY)
        }
    }

    let (op, on_repository) = ops::visit(&cassette.op, Described)
        .ok_or_else(|| format!("FastGit never calls {}", cassette.op))?;
    let account = host
        .account_id()
        .ok_or_else(|| "The cassette names no account".to_owned())?;
    let forge = Forge::new(started, host.clone(), account, cassette.repo.clone());
    let outcome = futures::executor::block_on(forge.exchange(op, on_repository, &cassette.input));
    let answer = match &outcome.result {
        Ok(output) => check_answer(op, output),
        Err(error) => Err(error.clone()),
    };
    Ok(Played {
        op,
        cassette: cassette_path,
        outcome,
        answer,
        log: host.take_log(),
        unplayed: host.unplayed(),
        expected: cassette.output,
    })
}

fn read_manifest(folder: &Path) -> Result<Manifest, String> {
    let path = folder.join(MANIFEST_FILE);
    let bytes = fs::read(&path).map_err(|error| format!("Can't read {}: {error}", path.display()))?;
    Manifest::parse(&bytes).map_err(|problem| format!("{}: {problem}", path.display()))
}

fn read_cassette(path: &Path) -> Result<Cassette, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Can't read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("{} isn't a cassette: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use serde_json::{Value, json};

    use super::try_connector;
    use crate::connect::{CallError, Stop, fakes::two_faced, manifest::samples};

    const IDENTITY: &str = r#"{"ok":{"id":"1","login":"octo","name":null,"avatar_url":null,"scopes":[]}}"#;

    fn folder(name: &str, manifest: Value, wasm: &[u8]) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("fastgit-trial-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("connector.json"), manifest.to_string()).unwrap();
        fs::write(folder.join("fake.wasm"), wasm).unwrap();
        folder
    }

    fn manifest() -> Value {
        samples::minimal("fake")
    }

    fn cassette(folder: &Path, output: Value) -> PathBuf {
        let path = folder.join("who_am_i.json");
        let cassette = json!({
            "op": "account.who_am_i",
            "account": { "id": "fake:example.com:1", "login": "octo", "endpoints": { "api": "https://api.example.com/v1/" } },
            "exchanges": [
                { "method": "GET", "url": "https://api.example.com/v1/user", "status": 200, "body": { "login": "octo" } }
            ],
            "output": output
        });
        fs::write(&path, cassette.to_string()).unwrap();
        path
    }

    #[test]
    fn a_connector_folder_starts_and_replays_its_cassette() {
        let folder = folder("passes", manifest(), &two_faced(IDENTITY));
        let cassette = cassette(&folder, serde_json::from_str(IDENTITY).unwrap());
        let trial = try_connector(&folder, Some(&cassette)).unwrap();
        assert_eq!((trial.id.0.as_str(), trial.version.as_str()), ("fake", "0.0.1"));
        assert_eq!(trial.start.as_ref().unwrap().hello.protocol, "1.0");
        let played = trial.played.as_ref().unwrap();
        assert_eq!(played.op, "account.who_am_i");
        assert_eq!(played.answer, Ok(()));
        assert_eq!(played.log, ["Info asked for the user"]);
        assert!(played.unplayed.is_empty());
        assert_eq!(played.matches_expected(), Some(true));
        assert_eq!(played.outcome.usage.requests, 1);
        assert!(trial.passed());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn an_answer_that_differs_from_the_cassette_fails_the_trial() {
        let folder = folder("differs", manifest(), &two_faced(IDENTITY));
        let someone_else = json!({ "ok": { "id": "2", "login": "someone", "name": null, "avatar_url": null, "scopes": [] } });
        let cassette = cassette(&folder, someone_else);
        let trial = try_connector(&folder, Some(&cassette)).unwrap();
        assert_eq!(trial.played.as_ref().unwrap().matches_expected(), Some(false));
        assert!(!trial.passed());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn an_unreadable_answer_fails_the_trial_and_names_its_path() {
        let folder = folder("unreadable", manifest(), &two_faced(r#"{"ok":{"login":5}}"#));
        let cassette = cassette(&folder, Value::Null);
        let trial = try_connector(&folder, Some(&cassette)).unwrap();
        let played = trial.played.as_ref().unwrap();
        assert!(
            matches!(&played.answer, Err(CallError::Stopped(Stop::BadOutput { path, .. })) if path == "login"),
            "{:?}",
            played.answer
        );
        assert_eq!(played.matches_expected(), None);
        assert!(!trial.passed());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn without_a_cassette_the_connector_only_starts() {
        let folder = folder("starts", manifest(), &two_faced(IDENTITY));
        let trial = try_connector(&folder, None).unwrap();
        assert!(trial.played.is_none());
        assert!(trial.passed());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_connector_that_does_not_start_fails_the_trial() {
        let folder = folder("refused", manifest(), b"not webassembly");
        let trial = try_connector(&folder, None).unwrap();
        assert!(matches!(trial.start, Err(Stop::Refused(_))));
        assert!(!trial.passed());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_manifest_must_ask_for_abi_1_and_protocol_1_and_name_a_module_beside_it() {
        let wasm = two_faced(IDENTITY);
        let mut escaping = manifest();
        escaping["module"] = json!("../fake.wasm");
        let mut newer_abi = manifest();
        newer_abi["interface"]["abi"] = json!(2);
        let mut newer_protocol = manifest();
        newer_protocol["interface"]["protocol"] = json!("2.0");
        for (name, manifest) in [("escaping", escaping), ("abi", newer_abi), ("protocol", newer_protocol)] {
            let folder = folder(name, manifest, &wasm);
            assert!(try_connector(&folder, None).is_err(), "{name}");
            fs::remove_dir_all(folder).unwrap();
        }
    }

    #[test]
    fn a_cassette_calling_an_operation_fastgit_never_calls_is_refused() {
        let folder = folder("unknown-op", manifest(), &two_faced(IDENTITY));
        let path = folder.join("teleport.json");
        let cassette = json!({ "op": "teleport", "account": { "id": "fake:example.com:1", "login": "octo", "endpoints": { "api": "https://api.example.com/v1/" } } });
        fs::write(&path, cassette.to_string()).unwrap();
        assert_eq!(
            try_connector(&folder, Some(&path)).err(),
            Some("FastGit never calls teleport".to_owned())
        );
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn the_committed_github_module_answers_every_cassette_inside_the_sandbox() {
        let github = Path::new(env!("CARGO_MANIFEST_DIR")).join("connectors").join("github");
        let mut played = 0;
        for entry in fs::read_dir(github.join("tests").join("cassettes")).unwrap() {
            let cassette = entry.unwrap().path();
            let trial = try_connector(&github.join("dist"), Some(&cassette)).unwrap();
            assert!(trial.passed(), "{}", cassette.display());
            played += 1;
        }
        assert!(played >= 19, "only {played} cassettes");
    }
}
