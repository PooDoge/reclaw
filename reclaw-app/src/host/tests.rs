use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use reclaw_net::{
    Net,
    testing::{Reply, TestServer, local_config},
};
use reclaw_ui::notices::NoticeKind;

use super::*;

#[derive(Default)]
struct Collector(Mutex<Vec<AppAction>>);

impl Sink for Collector {
    fn send(&self, action: AppAction) {
        self.0.lock().expect("lock").push(action);
    }
}

impl Collector {
    fn all(&self) -> Vec<AppAction> {
        self.0.lock().expect("lock").clone()
    }

    fn notices(&self) -> Vec<Notice> {
        self.all().into_iter().filter_map(|a| if let AppAction::Notify(n) = a { Some(n) } else { None }).collect()
    }

    fn last_games(&self) -> Option<Vec<reclaw_ui::model::GameEntry>> {
        self.all().into_iter().rev().find_map(|a| if let AppAction::SetGames(g) = a { Some(g) } else { None })
    }

    fn wait_for(&self, what: &str, done: impl Fn(&[AppAction]) -> bool) {
        let end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < end {
            if done(&self.all()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for {what}: {:#?}", self.all());
    }
}

const PLATFORM: &str = r#"{"formatRevision": 1, "generatedAt": "2026-10-04T12:29:25+00:00", "entries": [
  {"provider": "github", "repository": "o/one", "preferredRelease": null, "releaseTag": "v1.2.0", "assetNames": ["one-windows.zip"], "validatedAt": "2026-10-04T12:28:25+00:00", "selectionRevision": 1}]}"#;

fn catalog_server() -> TestServer {
    TestServer::start(|req, _| {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        match req.path.as_str() {
            "/index.json" => Reply::ok(format!(
                r#"{{"version": 2, "lists": [{{"id": "n", "remoteLocation": "http://{host}/n.json"}}], "platformMetadataUrl": "http://{host}/platform.json"}}"#
            )),
            "/n.json" => Reply::ok(
                r#"{"name": "Nintendo", "version": "1", "apps": [
                  {"name": "One", "project": "Port", "repository": "o/one", "folderName": "One", "tags": ["n64"]},
                  {"name": "Two", "repository": "o/two", "folderName": "Two", "tags": ["ps2", "playstation"]}]}"#,
            ),
            "/platform.json" => Reply::ok(PLATFORM),
            _ => Reply::new(404, "no"),
        }
    })
}

struct Rig {
    host: Host,
    sync: CatalogSync,
    sink: Arc<Collector>,
    initial: Initial,
    library_file: PathBuf,
    _dir: tempfile::TempDir,
    _server: Option<TestServer>,
}

fn rig(server: Option<TestServer>, library_text: Option<&str>) -> Rig {
    let dir = tempfile::tempdir().expect("tempdir");
    let library_file = dir.path().join("data").join("apps.json");
    if let Some(text) = library_text {
        std::fs::create_dir_all(library_file.parent().expect("dir")).expect("mkdir");
        std::fs::write(&library_file, text).expect("write");
    }
    let mut config = local_config();
    config.cache_dir = Some(dir.path().join("http"));
    config.attempts = 1;
    let net = Net::new(config).expect("net");
    let url = server.as_ref().map(|s| s.url("/index.json")).unwrap_or_else(|| "http://127.0.0.1:1/index.json".into());
    let sink = Arc::new(Collector::default());
    let (host, initial) = Host::open(Some(net.clone()), library_file.clone(), Some(url.clone()), sink.clone());
    let sync = CatalogSync::new(net).with_index_url(url);
    Rig { host, sync, sink, initial, library_file, _dir: dir, _server: server }
}

fn id_of(rig: &Rig, title: &str) -> u32 {
    let games = rig
        .sink
        .all()
        .into_iter()
        .rev()
        .find_map(|a| if let AppAction::SetProjects(p) = a { Some(p) } else { None })
        .expect("projects were sent");
    games.iter().find(|p| p.title == title).unwrap_or_else(|| panic!("no project {title}")).id
}

#[test]
fn a_first_start_with_nothing_saved_is_empty_and_quiet() {
    let rig = rig(None, None);
    assert!(rig.initial.loaded.projects.is_empty() && rig.initial.loaded.games.is_empty());
    assert!(rig.initial.notices.is_empty());
    assert_eq!(rig.initial.status.phase, CatalogPhase::Idle);
}

#[test]
fn a_refresh_sends_loading_then_the_catalog_then_ready_in_that_order() {
    let rig = rig(Some(catalog_server()), None);
    rig.host.run_refresh(&rig.sync);
    let actions = rig.sink.all();
    let names: Vec<&str> = actions
        .iter()
        .map(|a| match a {
            AppAction::Catalog(s) if s.phase == CatalogPhase::Loading => "loading",
            AppAction::SetProjects(_) => "projects",
            AppAction::SetGames(_) => "games",
            AppAction::Catalog(s) if s.phase == CatalogPhase::Ready => "ready",
            AppAction::Notify(_) => "notice",
            _ => "other",
        })
        .collect();
    assert_eq!(names, ["loading", "projects", "games", "ready"], "{actions:#?}");
    let AppAction::SetProjects(projects) = &actions[1] else { panic!("projects") };
    assert_eq!(projects.len(), 2);
    let one = projects.iter().find(|p| p.title == "One").expect("One");
    assert_eq!((one.project.as_str(), one.latest_release().map(|r| r.tag.as_str())), ("Port", Some("v1.2.0")));
    assert_eq!(projects.iter().find(|p| p.title == "Two").map(|p| p.platform), Some(reclaw_games::project::Platform::Ps2));
}

#[test]
fn what_one_run_saved_is_there_at_the_next_start_before_any_network() {
    let dir_rig = rig(Some(catalog_server()), None);
    dir_rig.host.run_refresh(&dir_rig.sync);
    // A new host over the same cache and no reachable server.
    let mut config = local_config();
    config.cache_dir = Some(dir_rig._dir.path().join("http"));
    config.attempts = 1;
    let sink = Arc::new(Collector::default());
    let (_host, initial) = Host::open(Some(Net::new(config).expect("net")), dir_rig.library_file.clone(), Some(dir_rig.sync_url()), sink);
    assert_eq!(initial.loaded.projects.len(), 2, "the first frame already has the catalog");
    assert_eq!(initial.status.phase, CatalogPhase::Ready);
}

impl Rig {
    fn sync_url(&self) -> String {
        self._server.as_ref().map(|s| s.url("/index.json")).unwrap_or_default()
    }
}

#[test]
fn a_catalog_that_cannot_be_loaded_says_so_and_does_not_invent_one() {
    let rig = rig(None, None);
    rig.host.run_refresh(&rig.sync);
    let status =
        rig.sink.all().into_iter().rev().find_map(|a| if let AppAction::Catalog(s) = a { Some(s) } else { None }).expect("a status");
    assert_eq!(status.phase, CatalogPhase::Failed);
    let notices = rig.sink.notices();
    assert_eq!(notices.len(), 1);
    assert!(notices[0].kind.is_failure() && notices[0].body.contains("no saved copy"), "{:?}", notices[0]);
    assert!(!rig.sink.all().iter().any(|a| matches!(a, AppAction::SetProjects(_))), "nothing is sent as projects");
}

#[test]
fn adding_to_the_library_writes_the_file_and_tells_the_screens_and_removing_undoes_it() {
    let rig = rig(Some(catalog_server()), None);
    rig.host.run_refresh(&rig.sync);
    let id = id_of(&rig, "One");

    rig.host.handle(&Effect::AddToLibrary(id));
    let text = std::fs::read_to_string(&rig.library_file).expect("written");
    let saved = reclaw_catalog::parse_library(&text).expect("a valid library");
    assert_eq!(saved.len(), 1);
    assert_eq!((saved[0].name.as_str(), saved[0].repository.as_str()), ("One", "o/one"));
    let games = rig.sink.last_games().expect("games sent");
    assert_eq!(games.len(), 1);
    assert_eq!((games[0].id, games[0].in_library, games[0].version.as_ref()), (id, true, "v1.2.0"));

    rig.host.handle(&Effect::AddToLibrary(id));
    assert_eq!(
        reclaw_catalog::parse_library(&std::fs::read_to_string(&rig.library_file).expect("read")).expect("valid").len(),
        1,
        "adding twice adds once"
    );

    rig.host.handle(&Effect::RemoveFromLibrary(id));
    assert!(reclaw_catalog::parse_library(&std::fs::read_to_string(&rig.library_file).expect("read")).expect("valid").is_empty());
    assert!(rig.sink.last_games().expect("games").is_empty());
    assert!(rig.sink.notices().is_empty(), "no complaints along the way: {:?}", rig.sink.notices());
}

#[test]
fn the_library_a_run_saved_is_there_at_the_next_start() {
    let rig = rig(Some(catalog_server()), None);
    rig.host.run_refresh(&rig.sync);
    let id = id_of(&rig, "Two");
    rig.host.handle(&Effect::AddToLibrary(id));
    let mut config = local_config();
    config.cache_dir = Some(rig._dir.path().join("http"));
    let (_host, initial) =
        Host::open(Some(Net::new(config).expect("net")), rig.library_file.clone(), Some(rig.sync_url()), Arc::new(Collector::default()));
    let titles: Vec<_> = initial.loaded.games.iter().map(|g| g.title.to_string()).collect();
    assert_eq!(titles, ["Two"]);
    assert_eq!(initial.loaded.games[0].id, id, "the same number as before: favorites and settings stay attached");
}

#[test]
fn a_library_that_cannot_be_read_is_left_alone_and_the_user_is_told() {
    let damaged = r#"{"apps": [{"name": 5}]}"#;
    let rig = rig(Some(catalog_server()), Some(damaged));
    assert_eq!(rig.initial.notices.len(), 1);
    assert_eq!(rig.initial.notices[0].kind, NoticeKind::Problem);
    rig.host.run_refresh(&rig.sync);
    let id = id_of(&rig, "One");
    rig.host.handle(&Effect::AddToLibrary(id));
    assert_eq!(std::fs::read_to_string(&rig.library_file).expect("read"), damaged, "the file was not touched");
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("read-only")), "{:?}", rig.sink.notices());
}

#[test]
fn an_install_keeps_the_app_and_says_plainly_that_installing_is_not_built() {
    let rig = rig(Some(catalog_server()), None);
    rig.host.run_refresh(&rig.sync);
    let id = id_of(&rig, "One");
    rig.host.handle(&Effect::StartInstall { app: id, location: "/games".into(), game_file: None, shortcut: false, prerelease: false });
    assert_eq!(rig.sink.last_games().map(|g| g.len()), Some(1), "the app is in the library");
    let note = rig.sink.notices().pop().expect("a note");
    assert_eq!(note.kind, NoticeKind::Note);
    assert!(note.title.contains("Installing") && note.body.contains("One"), "{note:?}");
}

#[test]
fn things_that_need_an_installed_game_say_so_instead_of_pretending() {
    let rig = rig(Some(catalog_server()), None);
    rig.host.run_refresh(&rig.sync);
    let id = id_of(&rig, "One");
    for effect in [Effect::Launch(id), Effect::Update(id), Effect::Verify(id), Effect::Uninstall(id), Effect::OpenFolder(id)] {
        let before = rig.sink.notices().len();
        rig.host.handle(&effect);
        assert_eq!(rig.sink.notices().len(), before + 1, "{effect:?}");
    }
    assert!(rig.sink.notices().iter().all(|n| n.kind == NoticeKind::Note));
}

#[test]
fn a_link_that_is_not_public_https_is_refused_with_a_notice() {
    let rig = rig(None, None);
    rig.host.handle(&Effect::OpenUrl("file:///etc/passwd".into()));
    let notice = rig.sink.notices().pop().expect("a notice");
    assert_eq!(notice.kind, NoticeKind::Problem);
}

#[test]
fn a_refresh_asked_for_twice_at_once_runs_once() {
    let server = TestServer::start(|req, _| {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        let mut reply = match req.path.as_str() {
            "/index.json" => Reply::ok(format!(r#"{{"version": 2, "lists": [{{"id": "n", "remoteLocation": "http://{host}/n.json"}}]}}"#)),
            "/n.json" => Reply::ok(r#"{"name": "N", "version": "1", "apps": []}"#),
            _ => Reply::new(404, ""),
        };
        reply.delay = Duration::from_millis(300);
        reply
    });
    let rig = rig(Some(server), None);
    rig.host.refresh();
    rig.host.refresh();
    rig.sink.wait_for("the refresh to finish", |a| a.iter().any(|x| matches!(x, AppAction::Catalog(s) if s.phase == CatalogPhase::Ready)));
    let loadings = rig.sink.all().iter().filter(|a| matches!(a, AppAction::Catalog(s) if s.phase == CatalogPhase::Loading)).count();
    assert_eq!(loadings, 1);
    let index_requests = rig._server.as_ref().map(|s| s.requests().iter().filter(|r| r.path == "/index.json").count());
    assert_eq!(index_requests, Some(1));
}
