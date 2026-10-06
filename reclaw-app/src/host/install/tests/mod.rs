//! Installing through the host, against a fake catalog and a fake GitHub on this machine serving real archives.
mod flow;
mod problems;

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use reclaw_install::{Api, Platform};
use reclaw_net::{
    Net,
    testing::{Reply, TestServer, local_config},
};
use reclaw_ui::{activity::ActivityEvent, store::AppAction};
use serde_json::json;

use crate::host::{Host, HostConfig, Initial, Sink, tests::Collector};

/// An ELF program header good enough to be recognised as a program.
pub fn elf(extra: &str) -> Vec<u8> {
    let mut header = vec![0u8; 64];
    header[..4].copy_from_slice(b"\x7fELF");
    header[4] = 2;
    header[5] = 1;
    header[6] = 1;
    header[16] = 2;
    header[24] = 0x10;
    header.extend(extra.as_bytes());
    header
}

pub fn zip_of(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in files {
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o644);
        writer.start_file(*name, options).expect("start");
        writer.write_all(data).expect("write");
    }
    writer.finish().expect("finish").into_inner()
}

/// One release the fake GitHub offers: newest first in the list.
#[derive(Clone)]
pub struct Rel {
    pub tag: &'static str,
    pub files: Vec<(&'static str, Vec<u8>)>,
    /// Seconds the download of its files is held back.
    pub slow: Duration,
}

pub type Releases = Arc<Mutex<Vec<Rel>>>;

/// The catalog (one app, "One", repository o/one, folder One) and GitHub's release API and downloads, on one server.
pub fn server(releases: Releases) -> TestServer {
    TestServer::start(move |req, _| {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        let path = req.path.split('?').next().unwrap_or_default().to_string();
        match path.as_str() {
            "/index.json" => {
                return Reply::ok(format!(r#"{{"version": 2, "lists": [{{"id": "n", "remoteLocation": "http://{host}/n.json"}}]}}"#));
            }
            "/n.json" => {
                return Reply::ok(
                    r#"{"name": "N", "version": "1", "apps": [{"name": "One", "repository": "o/one", "folderName": "One", "tags": ["n64"]}]}"#,
                );
            }
            _ => {}
        }
        let releases = releases.lock().expect("lock").clone();
        let describe = |r: &Rel| {
            json!({
                "tag_name": r.tag, "prerelease": false, "draft": false, "body": format!("Notes for {}", r.tag),
                "assets": r.files.iter().map(|(n, b)| json!({"name": n, "browser_download_url": format!("http://{host}/dl/{}/{n}", r.tag), "size": b.len()})).collect::<Vec<_>>()
            })
        };
        if path == "/repos/o/one/releases/latest" {
            return releases.first().map_or_else(|| Reply::new(404, "{}"), |r| Reply::ok(describe(r).to_string()));
        }
        if path == "/repos/o/one/releases" {
            return Reply::ok(json!(releases.iter().map(describe).collect::<Vec<_>>()).to_string());
        }
        if let Some(rest) = path.strip_prefix("/dl/")
            && let Some((tag, name)) = rest.split_once('/')
            && let Some(rel) = releases.iter().find(|r| r.tag == tag)
            && let Some((_, body)) = rel.files.iter().find(|(n, _)| *n == name)
        {
            let mut reply = Reply::ok(body.clone());
            reply.delay = rel.slow;
            return reply;
        }
        Reply::new(404, "no")
    })
}

pub struct Rig {
    pub host: Host,
    pub sink: Arc<Collector>,
    pub initial: Initial,
    pub root: tempfile::TempDir,
    pub releases: Releases,
    pub server: TestServer,
    pub library_file: PathBuf,
}

pub fn game_zip(version: &str) -> Vec<u8> {
    zip_of(&[("Game/game.x86_64", elf(version)), ("Game/data/readme.txt", b"hello".to_vec())])
}

pub fn rel(tag: &'static str) -> Rel {
    Rel { tag, files: vec![("Game-linux.zip", game_zip(tag))], slow: Duration::ZERO }
}

pub fn start(releases: Vec<Rel>, library_text: Option<&str>, configure: impl FnOnce(&mut HostConfig, &Path)) -> Rig {
    let releases: Releases = Arc::new(Mutex::new(releases));
    let server = server(releases.clone());
    let root = tempfile::tempdir().expect("tempdir");
    let library_file = root.path().join("data/apps.json");
    if let Some(text) = library_text {
        std::fs::create_dir_all(library_file.parent().expect("dir")).expect("mkdir");
        std::fs::write(&library_file, text).expect("write");
    }
    let mut net_config = local_config();
    net_config.cache_dir = Some(root.path().join("http"));
    net_config.attempts = 1;
    let net = Net::new(net_config).expect("net");
    let sink = Arc::new(Collector::default());
    let mut config = HostConfig::new(Some(net.clone()), library_file.clone());
    config.index_url = Some(server.url("/index.json"));
    config.api = Some(Api { github: server.url(""), gitlab: server.url("/gitlab") });
    config.platform = Some(Platform::LinuxX64);
    config.downloads_dir = Some(root.path().join("downloads"));
    config.home = Some(root.path().join("home"));
    config.protect = vec![root.path().join("data"), root.path().join("home")];
    config.default_location = Some(root.path().join("default-apps").display().to_string());
    configure(&mut config, root.path());
    let sink_dyn: Arc<dyn Sink> = sink.clone();
    let (host, initial) = Host::open(config, sink_dyn);
    let sync = reclaw_sync::CatalogSync::new(net).with_index_url(server.url("/index.json"));
    host.run_refresh(&sync);
    Rig { host, sink, initial, root, releases, server, library_file }
}

impl Rig {
    pub fn app(&self) -> u32 {
        self.sink
            .all()
            .into_iter()
            .rev()
            .find_map(|a| if let AppAction::SetProjects(p) = a { p.into_iter().find(|p| p.title == "One") } else { None })
            .expect("the catalog has One")
            .id
    }

    pub fn folder(&self) -> PathBuf {
        self.root.path().join("default-apps/One")
    }

    pub fn status(&self) -> Option<reclaw_ui::model::AppStatus> {
        self.sink.last_games().and_then(|g| g.into_iter().find(|g| g.title == "One")).map(|g| g.status)
    }

    pub fn activity(&self) -> Vec<ActivityEvent> {
        self.sink.all().into_iter().filter_map(|a| if let AppAction::Activity(e) = a { Some(e) } else { None }).collect()
    }

    pub fn wait_activity_ends(&self) {
        self.sink.wait_for("the job to end", |all| {
            all.iter().any(|a| {
                matches!(
                    a,
                    AppAction::Activity(ActivityEvent::Finished { .. } | ActivityEvent::Failed { .. } | ActivityEvent::Cancelled { .. })
                )
            })
        });
    }

    /// Wait until the activity events number `n` jobs that ended.
    pub fn wait_ended(&self, n: usize) {
        self.sink.wait_for("the jobs to end", |all| {
            all.iter()
                .filter(|a| {
                    matches!(
                        a,
                        AppAction::Activity(
                            ActivityEvent::Finished { .. } | ActivityEvent::Failed { .. } | ActivityEvent::Cancelled { .. }
                        )
                    )
                })
                .count()
                >= n
        });
    }
}
