//! A small HTTP/1.1 server on this machine whose every answer a test decides: status, headers, a body that is cut off or
//! stalls halfway, a request counter. Raw sockets, so what the client sees is exactly what the test wrote.
#![allow(dead_code)]
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use reclaw_net::{AddressPolicy, Net, NetConfig, ProxyMode};

#[derive(Clone, Debug)]
pub struct Req {
    pub method: String,
    pub path: String,
    /// Lower-case names.
    pub headers: HashMap<String, String>,
}

impl Req {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

#[derive(Clone, Debug)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Promise the whole body but send only this much, then close the connection.
    pub cut_after: Option<usize>,
    /// Send this much, then say nothing for this long.
    pub stall_after: Option<(usize, Duration)>,
    /// Send the body only after this pause.
    pub delay: Duration,
    /// Send no `Content-Length`; the body ends when the connection closes.
    pub unframed: bool,
}

impl Reply {
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self { status, headers: Vec::new(), body: body.into(), cut_after: None, stall_after: None, delay: Duration::ZERO, unframed: false }
    }

    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, body)
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

type Handler = dyn Fn(&Req, usize) -> Reply + Send + Sync;

pub struct TestServer {
    pub addr: SocketAddr,
    seen: Arc<Mutex<Vec<Req>>>,
    stop: Arc<AtomicBool>,
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        302 => "Found",
        304 => "Not Modified",
        403 => "Forbidden",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

fn read_request(stream: &TcpStream) -> Option<Req> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next()?.to_string(), parts.next()?.to_string());
    let mut headers = HashMap::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some((k, v)) = header.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    Some(Req { method, path, headers })
}

fn answer(mut stream: TcpStream, reply: Reply) {
    let mut head = format!("HTTP/1.1 {} {}\r\nConnection: close\r\n", reply.status, reason(reply.status));
    for (k, v) in &reply.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    if !reply.unframed {
        head.push_str(&format!("Content-Length: {}\r\n", reply.body.len()));
    }
    head.push_str("\r\n");
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    thread::sleep(reply.delay);
    if let Some((n, pause)) = reply.stall_after {
        let n = n.min(reply.body.len());
        let _ = stream.write_all(&reply.body[..n]);
        let _ = stream.flush();
        thread::sleep(pause);
        return;
    }
    let n = reply.cut_after.map_or(reply.body.len(), |c| c.min(reply.body.len()));
    let _ = stream.write_all(&reply.body[..n]);
    let _ = stream.flush();
}

impl TestServer {
    /// `handler` gets each request and its place in the order (0 for the first).
    pub fn start(handler: impl Fn(&Req, usize) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (seen, stop) = (Arc::new(Mutex::new(Vec::new())), Arc::new(AtomicBool::new(false)));
        let (handler, counter): (Arc<Handler>, Arc<AtomicUsize>) = (Arc::new(handler), Arc::new(AtomicUsize::new(0)));
        let (seen2, stop2) = (seen.clone(), stop.clone());
        thread::spawn(move || {
            for stream in listener.incoming() {
                if stop2.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let (handler, counter, seen) = (handler.clone(), counter.clone(), seen2.clone());
                thread::spawn(move || {
                    let Some(req) = read_request(&stream) else { return };
                    let nth = counter.fetch_add(1, Ordering::SeqCst);
                    seen.lock().expect("lock").push(req.clone());
                    answer(stream, handler(&req, nth));
                });
            }
        });
        Self { addr, seen, stop }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    pub fn requests(&self) -> Vec<Req> {
        self.seen.lock().expect("lock").clone()
    }

    pub fn count(&self) -> usize {
        self.requests().len()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so its thread ends.
        let _ = TcpStream::connect(self.addr);
    }
}

/// A configuration for talking to a server on this machine: no proxy, quick retries, the test address policy.
pub fn local_config() -> NetConfig {
    NetConfig {
        address_policy: AddressPolicy::AnyHttpForTests,
        proxy: ProxyMode::None,
        stall_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(2),
        ..NetConfig::default()
    }
}

pub fn local_net() -> Net {
    Net::new(local_config()).expect("net")
}

/// Bytes that are not all the same, so a wrong offset shows up in the hash.
pub fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(31).wrapping_add(i >> 8) % 251) as u8).collect()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}
