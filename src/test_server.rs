use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Clone, Debug)]
pub(crate) struct Seen {
    pub(crate) method: String,
    pub(crate) target: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl Seen {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub(crate) struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    delay: Duration,
}

impl Reply {
    pub(crate) fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            delay: Duration::ZERO,
        }
    }

    pub(crate) fn unauthorized() -> Self {
        Self::status(401).header("WWW-Authenticate", "Basic realm=\"sternegit-test\"")
    }

    pub(crate) fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    pub(crate) fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub(crate) fn after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

type Respond = dyn Fn(&Seen) -> Reply + Send + Sync;

pub(crate) struct TestServer {
    address: SocketAddr,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl TestServer {
    pub(crate) fn start(respond: impl Fn(&Seen) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let respond: Arc<Respond> = Arc::new(respond);
        let recorded = seen.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let respond = respond.clone();
                let recorded = recorded.clone();
                thread::spawn(move || serve(stream, &*respond, &recorded));
            }
        });
        Self { address, seen }
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }

    pub(crate) fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

fn serve(mut stream: TcpStream, respond: &Respond, recorded: &Mutex<Vec<Seen>>) {
    let Some(seen) = read_request(&stream) else {
        return;
    };
    recorded.lock().unwrap().push(seen.clone());
    let reply = respond(&seen);
    thread::sleep(reply.delay);
    let mut head = format!(
        "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    for (name, value) in &reply.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&reply.body);
}

fn read_request(stream: &TcpStream) -> Option<Seen> {
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let mut words = first.split_whitespace();
    let method = words.next()?.to_owned();
    let target = words.next()?.to_owned();
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Seen {
        method,
        target,
        headers,
        body,
    })
}
