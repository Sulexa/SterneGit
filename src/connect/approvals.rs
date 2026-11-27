use super::catalog::Connectors;

#[cfg(test)]
pub(crate) use crate::plugin::Hashes;

#[cfg(test)]
pub(crate) type Approval = crate::plugin::Approval<Connectors>;
pub(crate) type Approvals = crate::plugin::Approvals<Connectors>;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{Approval, Approvals, Hashes};
    use crate::{connect::grant::Grant, net::services::ConnectorId};

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-approvals-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    fn approval(id: &str) -> Approval {
        let files = Hashes::from([("connector.json".into(), "ab".repeat(32))]);
        let mut grant = Grant::default();
        grant.features.insert("pulls".into());
        Approval::new(ConnectorId(id.into()), "1.2.0".into(), files, grant)
    }

    #[test]
    fn a_recorded_approval_reads_back() {
        let folder = scratch("round-trip");
        let file = Some(folder.join("connectors.json"));
        let mut approvals = Approvals::at(file.clone());
        let mut gitea = approval("gitea");
        gitea.updated = true;
        approvals.record(gitea.clone()).unwrap();
        approvals.record(approval("gitlab")).unwrap();
        let read = Approvals::at(file);
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read.get(&ConnectorId("gitea".into())), Some(&gitea));
        assert_eq!(read.get(&ConnectorId("gitlab".into())).map(|found| found.updated), Some(false));
        assert_eq!(read.get(&ConnectorId("github".into())), None);
    }

    #[test]
    fn recording_again_replaces_and_keeps_what_sternegit_does_not_know() {
        let folder = scratch("unknown");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("connectors.json");
        let written = json!({
            "version": 1,
            "from_a_newer_sternegit": true,
            "connectors": [
                { "id": "gitea", "version": "1.0.0", "files": {}, "grant": {}, "pinned_by": "me" },
                { "id": "broken" }
            ]
        });
        std::fs::write(&path, written.to_string()).unwrap();
        let mut approvals = Approvals::at(Some(path.clone()));
        let mut newer = approvals.get(&ConnectorId("gitea".into())).unwrap().clone();
        newer.version = "1.1.0".into();
        approvals.record(newer).unwrap();
        let reread: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(reread["from_a_newer_sternegit"], true);
        assert_eq!(reread["connectors"][0]["version"], "1.1.0");
        assert_eq!(reread["connectors"][0]["pinned_by"], "me");
        assert_eq!(reread["connectors"][1], json!({ "id": "broken" }));
    }
}
