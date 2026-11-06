use fastgit_protocol::{
    Answer, CheckState, Done, Failure, FieldError, Identity, Mergeability, ResponseHead,
    Visibility, major, ops, resolve, speaks_same_major,
};
use serde_json::json;

#[test]
fn enums_from_a_newer_protocol_read_as_unknown() {
    assert_eq!(
        serde_json::from_value::<Visibility>(json!("secret")).unwrap(),
        Visibility::Unknown
    );
    assert_eq!(
        serde_json::from_value::<CheckState>(json!("stale")).unwrap(),
        CheckState::Unknown
    );
    assert_eq!(
        serde_json::from_value::<Mergeability>(json!({ "kind": "queued", "position": 3 })).unwrap(),
        Mergeability::Unknown
    );
    assert_eq!(
        serde_json::from_value::<Failure>(json!({ "kind": "teapot", "short": true })).unwrap(),
        Failure::Unknown
    );
}

#[test]
fn failures_and_mergeability_carry_their_data_beside_a_kind() {
    let unfinished = Failure::Unfinished {
        posted: 2,
        failure: Box::new(Failure::Invalid {
            fields: vec![FieldError {
                field: "line".into(),
                message: "outdated".into(),
            }],
            message: "Validation failed".into(),
        }),
    };
    assert_eq!(
        serde_json::to_value(&unfinished).unwrap(),
        json!({ "kind": "unfinished", "posted": 2, "failure": {
            "kind": "invalid",
            "fields": [{ "field": "line", "message": "outdated" }],
            "message": "Validation failed"
        }})
    );
    assert_eq!(
        serde_json::to_value(Mergeability::Blocked { reason: "Needs approval".into() }).unwrap(),
        json!({ "kind": "blocked", "reason": "Needs approval" })
    );
    assert_eq!(
        serde_json::to_value(Mergeability::FailingChecks).unwrap(),
        json!({ "kind": "failing_checks" })
    );
    assert_eq!(
        serde_json::from_value::<Failure>(json!({ "kind": "signed_out" })).unwrap(),
        Failure::SignedOut
    );
}

#[test]
fn an_answer_is_ok_or_err() {
    assert_eq!(serde_json::to_value(Answer::Ok(Done {})).unwrap(), json!({ "ok": {} }));
    assert_eq!(
        serde_json::to_value(Answer::<Done>::Err(Failure::NotFound)).unwrap(),
        json!({ "err": { "kind": "not_found" } })
    );
}

#[test]
fn optional_fields_may_be_missing_and_unknown_fields_are_ignored() {
    let identity: Identity =
        serde_json::from_value(json!({ "id": "7", "login": "octo", "scopes": [], "plan": "pro" })).unwrap();
    assert_eq!((identity.name, identity.avatar_url), (None, None));
    assert!(serde_json::from_value::<Identity>(json!({ "login": "octo" })).is_err());
}

#[test]
fn responses_are_framed_behind_a_little_endian_head_length() {
    let head = ResponseHead {
        status: 201,
        headers: vec![("etag".into(), "\"x\"".into())],
    };
    let framed = head.frame(b"body");
    let head_bytes = br#"{"status":201,"headers":[["etag","\"x\""]]}"#;
    assert_eq!(framed[..4], (head_bytes.len() as u32).to_le_bytes());
    assert_eq!(&framed[4..4 + head_bytes.len()], head_bytes);
    assert_eq!(ResponseHead::unframe(&framed), Some((head, &b"body"[..])));
    assert_eq!(ResponseHead::unframe(&framed[..10]), None);
    assert_eq!(ResponseHead::unframe(&[1, 0]), None);
}

#[test]
fn relative_urls_land_under_the_api_endpoint_and_absolute_ones_stay() {
    let api = "https://ghe.example.com/api/v3/";
    assert_eq!(resolve(api, "user/repos"), "https://ghe.example.com/api/v3/user/repos");
    assert_eq!(resolve(api, "/user/repos"), "https://ghe.example.com/api/v3/user/repos");
    assert_eq!(
        resolve(api, "//evil.example/x"),
        "https://ghe.example.com/api/v3/evil.example/x"
    );
    assert_eq!(resolve("https://api.github.com", "user"), "https://api.github.com/user");
    assert_eq!(
        resolve(api, "HTTPS://api.github.com/user?page=2"),
        "HTTPS://api.github.com/user?page=2"
    );
}

#[test]
fn every_operation_has_its_own_name() {
    let mut names = ops::NAMES.to_vec();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), ops::NAMES.len());
    assert_eq!(names.len(), 21);
    assert_eq!(ops::named("pulls.merge"), Some("pulls.merge"));
    assert_eq!(ops::named("pulls.teleport"), None);
}

#[test]
fn protocol_versions_compare_by_major() {
    assert!(speaks_same_major("1.0") && speaks_same_major("1.12"));
    assert!(!speaks_same_major("2.0") && !speaks_same_major("one"));
    assert_eq!(major("10.4"), Some(10));
}
