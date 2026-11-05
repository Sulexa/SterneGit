use serde::{Deserialize, Serialize};

pub const ABI: &str = "fastgit:1";

const HEAD_LENGTH_BYTES: usize = 4;
const ABSOLUTE_SCHEMES: [&str; 2] = ["https://", "http://"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestHead {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseHead {
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

impl ResponseHead {
    pub fn frame(&self, body: &[u8]) -> Vec<u8> {
        let head = serde_json::to_vec(self).unwrap_or_default();
        let mut framed = Vec::with_capacity(HEAD_LENGTH_BYTES + head.len() + body.len());
        framed.extend_from_slice(&(head.len() as u32).to_le_bytes());
        framed.extend_from_slice(&head);
        framed.extend_from_slice(body);
        framed
    }

    pub fn unframe(framed: &[u8]) -> Option<(Self, &[u8])> {
        let (length, rest) = framed.split_first_chunk::<HEAD_LENGTH_BYTES>()?;
        let length = usize::try_from(u32::from_le_bytes(*length)).ok()?;
        let (head, body) = rest.split_at_checked(length)?;
        Some((serde_json::from_slice(head).ok()?, body))
    }
}

pub fn resolve(api: &str, url: &str) -> String {
    if is_absolute(url) {
        return url.to_owned();
    }
    let base = api.strip_suffix('/').unwrap_or(api);
    format!("{base}/{}", url.trim_start_matches('/'))
}

fn is_absolute(url: &str) -> bool {
    ABSOLUTE_SCHEMES.iter().any(|scheme| {
        url.get(..scheme.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(scheme))
    })
}
