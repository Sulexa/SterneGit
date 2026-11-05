use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AccountContext, RepoRef, ResponseHead};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cassette {
    pub op: String,
    pub account: AccountContext,
    #[serde(default)]
    pub repo: Option<RepoRef>,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub exchanges: Vec<Exchange>,
    #[serde(default)]
    pub output: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exchange {
    pub method: String,
    pub url: String,
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Value,
}

impl Exchange {
    pub fn answers(&self, method: &str, url: &str) -> bool {
        self.method.eq_ignore_ascii_case(method) && self.url == url
    }

    pub fn head(&self) -> ResponseHead {
        ResponseHead {
            status: self.status,
            headers: self.headers.clone(),
        }
    }

    pub fn body_bytes(&self) -> Vec<u8> {
        match &self.body {
            Value::Null => Vec::new(),
            Value::String(text) => text.clone().into_bytes(),
            document => document.to_string().into_bytes(),
        }
    }
}

impl Cassette {
    pub fn reel(&self) -> Reel {
        Reel {
            exchanges: self.exchanges.clone(),
            played: vec![false; self.exchanges.len()],
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Reel {
    exchanges: Vec<Exchange>,
    played: Vec<bool>,
}

impl Reel {
    pub fn play(&mut self, method: &str, url: &str) -> Option<&Exchange> {
        let found = self
            .exchanges
            .iter()
            .zip(&self.played)
            .position(|(exchange, played)| !played && exchange.answers(method, url))?;
        self.played[found] = true;
        Some(&self.exchanges[found])
    }

    pub fn unplayed(&self) -> impl Iterator<Item = &Exchange> {
        self.exchanges
            .iter()
            .zip(&self.played)
            .filter(|(_, played)| !**played)
            .map(|(exchange, _)| exchange)
    }
}
