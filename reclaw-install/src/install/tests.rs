//! The whole install, against a fake GitHub on this machine serving real archives.
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use reclaw_net::{
    Cancel,
    testing::{Reply, TestServer, local_net, sha256_hex},
};
use serde_json::json;

use super::*;
use crate::{
    archive::tests::{Item, elf, targz_bytes, zip_bytes},
    source::{Api, ReleaseSource},
};

/// A file the fake host serves: its name, its bytes, and what the release says about it.
#[derive(Clone)]
struct Served {
    name: &'static str,
    body: Vec<u8>,
    digest: Option<String>,
}

impl Served {
    fn new(name: &'static str, body: Vec<u8>) -> Self {
        Self { name, body, digest: None }
    }

    fn with_digest(mut self) -> Self {
        self.digest = Some(format!("sha256:{}", sha256_hex(&self.body)));
        self
    }
}

type Releases = Arc<Mutex<Vec<(&'static str, bool, Vec<Served>)>>>;

/// A fake GitHub: `/repos/o/r/releases[/latest]` describe `releases` (newest first), `/dl/<tag>/<name>` serves the bytes.
fn github(releases: Releases) -> TestServer {
    TestServer::start(move |req, _| {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        let releases = releases.lock().expect("lock").clone();
        let describe = |(tag, pre, files): &(&'static str, bool, Vec<Served>)| {
            json!({
                "tag_name": tag, "prerelease": pre, "draft": false, "body": format!("Notes for {tag}"), "html_url": format!("https://github.com/o/r/releases/tag/{tag}"),
                "assets": files.iter().map(|f| json!({
                    "name": f.name, "browser_download_url": format!("http://{host}/dl/{tag}/{}", f.name), "size": f.body.len(), "digest": f.digest,
                })).collect::<Vec<_>>()
            })
        };
        let path = req.path.split('?').next().unwrap_or_default();
        if path == "/repos/o/r/releases/latest" {
            return match releases.iter().find(|r| !r.1) {
                Some(release) => Reply::ok(describe(release).to_string()),
                None => Reply::new(404, "{}"),
            };
        }
        if path == "/repos/o/r/releases" {
            return Reply::ok(json!(releases.iter().map(describe).collect::<Vec<_>>()).to_string());
        }
        if let Some(rest) = path.strip_prefix("/dl/")
            && let Some((tag, name)) = rest.split_once('/')
            && let Some(file) = releases.iter().filter(|r| r.0 == tag).flat_map(|r| &r.2).find(|f| f.name == name)
        {
            return Reply::ok(file.body.clone());
        }
        Reply::new(404, format!("no {}", req.path))
    })
}

struct Rig {
    server: TestServer,
    releases: Releases,
    installer: Installer,
    root: tempfile::TempDir,
}

impl Rig {
    fn new(releases: Vec<(&'static str, bool, Vec<Served>)>) -> Self {
        let releases: Releases = Arc::new(Mutex::new(releases));
        let server = github(releases.clone());
        let root = tempfile::tempdir().expect("temp dir");
        let net = local_net();
        let source = ReleaseSource::new(net.clone()).with_api(Api { github: server.url(""), gitlab: server.url("/gitlab") });
        let installer = Installer::new(net, source, root.path().join("downloads"));
        Self { server, releases, installer, root }
    }

    fn folder(&self) -> PathBuf {
        self.root.path().join("apps/Game")
    }

    fn request(&self) -> Request {
        Request {
            host: Host::GitHub,
            repo: "o/r".to_string(),
            folder: self.folder(),
            platform: Platform::LinuxX64,
            filter: None,
            preferred_version: None,
            allow_prerelease: false,
            asset: None,
            releases: None,
        }
    }

    fn plan(&self, request: &Request) -> Plan {
        self.installer.resolve(request).expect("a plan")
    }

    fn ready(&self, request: &Request) -> Resolved {
        match self.plan(request) {
            Plan::Ready(resolved) => *resolved,
            Plan::Choose { choices, .. } => panic!("expected a single choice, got {choices:?}"),
        }
    }

    fn install(&self, request: &Request) -> Result<Installed, InstallError> {
        let resolved = self.ready(request);
        self.installer.install(request, &resolved, &Cancel::new(), &mut |_| {})
    }
}

fn game_zip(version_marker: &str) -> Vec<u8> {
    zip_bytes(&[
        Item::Dir("Game-1.0/"),
        Item::File("Game-1.0/game.x86_64", [elf(), version_marker.as_bytes().to_vec()].concat(), 0o644),
        Item::File("Game-1.0/data/readme.txt", b"hello".to_vec(), 0o644),
    ])
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref()).unwrap_or_else(|e| panic!("{}: {e}", path.as_ref().display()))
}

mod flow;
mod problems;
