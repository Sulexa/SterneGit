use std::{fs, path::Path};

use fastgit_protocol::{Call, Cassette, Exchange};
use serde_json::Value;

use crate::{
    Connector, dispatch,
    host::{Tape, playing},
};

const API_ENDPOINT: &str = "api";

#[derive(Clone, Debug, PartialEq)]
pub struct Replayed {
    pub output: Value,
    pub log: Vec<String>,
    pub unplayed: Vec<Exchange>,
}

pub fn replay(connector: &impl Connector, cassette: &Cassette) -> Replayed {
    let call = Call {
        op: cassette.op.clone(),
        account: Some(cassette.account.clone()),
        repo: cassette.repo.clone(),
        input: cassette.input.clone(),
    };
    let input = serde_json::to_vec(&call).expect("a call serializes as JSON");
    let api = cassette.account.endpoint(API_ENDPOINT).unwrap_or_default();
    let (output, tape) = playing(Tape::new(api, cassette.reel()), || dispatch(connector, &input));
    Replayed {
        output: serde_json::from_slice(&output).expect("the SDK answers in JSON"),
        log: tape.log,
        unplayed: tape.reel.unplayed().cloned().collect(),
    }
}

pub fn check(connector: &impl Connector, cassette: impl AsRef<Path>) -> Replayed {
    let path = cassette.as_ref();
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("Can't read {}: {error}", path.display()));
    let cassette: Cassette = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} isn't a cassette: {error}", path.display()));
    let replayed = replay(connector, &cassette);
    if let Some(expected) = &cassette.output {
        assert_eq!(
            &replayed.output,
            expected,
            "the connector's answer differs from the output in {}",
            path.display()
        );
    }
    let unplayed: Vec<String> = replayed
        .unplayed
        .iter()
        .map(|exchange| format!("{} {}", exchange.method, exchange.url))
        .collect();
    assert!(
        unplayed.is_empty(),
        "the connector never asked for {} from {}",
        unplayed.join(", "),
        path.display()
    );
    replayed
}
