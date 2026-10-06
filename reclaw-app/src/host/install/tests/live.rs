//! The whole life of real catalog apps through the host, against the real community catalog, the real GitHub and GitLab and
//! their real downloads, by hand: `cargo test -p reclaw --lib live -- --ignored --nocapture --test-threads 1`. Install from
//! the default location, Verify, Play and Stop, Check for updates, an update from an older version, Uninstall.
//!
//! Play starts the real game. Set `DISPLAY` (`xvfb-run` will do) or it closes at once for want of a screen, which is still a
//! launch: the game's own complaint is in its log. A GitHub-hosted app needs `api.github.com`; a network that refuses it (the
//! development sandbox's does) fails that test with the network's reason.
#![cfg(unix)]
use std::time::{Duration, Instant};

use reclaw_runtime::RunState;
use reclaw_ui::{activity::Kind, effect::Effect, notices::Notice};

use super::*;

/// The real catalog, the real hosts, everything else in a temporary folder.
fn real() -> (Host, Arc<Collector>, tempfile::TempDir) {
    let root = tempfile::tempdir().expect("tempdir");
    let (mut net_config, problems) = reclaw_net::NetConfig::from_env(|k| std::env::var(k).ok());
    assert!(problems.is_empty(), "{problems:?}");
    net_config.cache_dir = Some(root.path().join("http"));
    let net = Net::new(net_config).expect("net");
    let sink = Arc::new(Collector::default());
    let mut config = HostConfig::new(Some(net.clone()), root.path().join("data/apps.json"));
    config.downloads_dir = Some(root.path().join("downloads"));
    config.home = Some(root.path().join("home"));
    config.logs_dir = Some(root.path().join("logs"));
    config.protect = vec![root.path().join("data"), root.path().join("home")];
    config.default_location = Some(root.path().join("Games").display().to_string());
    config.platform = Some(Platform::LinuxX64);
    config.grace = Some(Duration::from_secs(3));
    let sink_dyn: Arc<dyn Sink> = sink.clone();
    let (host, _) = Host::open(config, sink_dyn);
    host.run_refresh(&reclaw_sync::CatalogSync::new(net));
    (host, sink, root)
}

fn wait(sink: &Collector, what: &str, within: Duration, done: impl Fn(&[AppAction]) -> bool) {
    let end = Instant::now() + within;
    while Instant::now() < end {
        if done(&sink.all()) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {what}; notices: {:#?}", sink.notices());
}

fn ended(all: &[AppAction]) -> usize {
    all.iter()
        .filter(|a| {
            matches!(
                a,
                AppAction::Activity(ActivityEvent::Finished { .. } | ActivityEvent::Failed { .. } | ActivityEvent::Cancelled { .. })
            )
        })
        .count()
}

/// The catalog app whose repository is `owner/name`.
fn app_of(sink: &Collector, repo: &str) -> (u32, String) {
    let projects = sink
        .all()
        .into_iter()
        .rev()
        .find_map(|a| if let AppAction::SetProjects(p) = a { Some(p) } else { None })
        .expect("the catalog loaded");
    let found = projects
        .iter()
        .find(|p| format!("{}/{}", p.repo.owner, p.repo.name).eq_ignore_ascii_case(repo))
        .unwrap_or_else(|| panic!("{repo} is in the catalog ({} apps)", projects.len()));
    (found.id, found.title.clone())
}

fn notice_after(sink: &Collector, before: usize, what: &str) -> Notice {
    wait(sink, what, Duration::from_secs(60), |_| sink.notices().len() > before);
    let notice = sink.notices().into_iter().nth(before).expect("notice");
    println!("  {what}: [{:?}] {} - {} {:?}", notice.kind, notice.title, notice.body, notice.details);
    notice
}

fn runs(sink: &Collector, app: u32) -> Vec<RunState> {
    sink.all().into_iter().filter_map(|a| if let AppAction::SetRun { id, run } = a { (id == app).then_some(run) } else { None }).collect()
}

fn last_activity(sink: &Collector) -> ActivityEvent {
    sink.all().into_iter().rev().find_map(|a| if let AppAction::Activity(e) = a { Some(e) } else { None }).expect("activity")
}

fn life(repo: &str, folder_name: &str) {
    let (host, sink, root) = real();
    let (app, title) = app_of(&sink, repo);
    println!("{title} ({repo})");
    let folder = root.path().join("Games").join(folder_name);

    // Install into the default location.
    host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    wait(&sink, "the install", Duration::from_secs(600), |all| ended(all) >= 1);
    let end = last_activity(&sink);
    assert!(matches!(end, ActivityEvent::Finished { changelog: None, .. }), "{end:?}; {:#?}", sink.notices());
    let version = std::fs::read_to_string(folder.join("version.txt")).expect("version.txt");
    println!("  installed {version} into {}", folder.display());

    // Verify.
    let before = sink.notices().len();
    host.handle(&Effect::Verify(app));
    let verified = notice_after(&sink, before, "verify");
    assert!(verified.title.ends_with("looks complete"), "{verified:?}");

    // Play, give it a moment, Stop.
    host.handle(&Effect::Launch(app));
    std::thread::sleep(Duration::from_secs(4));
    let states = runs(&sink, app);
    println!("  play: {states:?}");
    assert!(matches!(states.first(), Some(RunState::Starting)), "{states:?}; {:#?}", sink.notices());
    if matches!(states.last(), Some(RunState::Running { .. })) {
        host.handle(&Effect::Stop(app));
        wait(&sink, "the game to stop", Duration::from_secs(20), |_| {
            !matches!(runs(&sink, app).last(), Some(RunState::Starting | RunState::Running { .. } | RunState::Stopping { .. }))
        });
        println!("  after stop: {:?}", runs(&sink, app).last());
    }
    let log = root.path().join("logs/games").join(format!("{folder_name}.log"));
    let output = std::fs::read_to_string(&log).unwrap_or_default();
    println!("  the game's log ({} bytes): {}", output.len(), output.lines().take(8).collect::<Vec<_>>().join(" | "));

    // Check for updates: current.
    let before = sink.notices().len();
    host.handle(&Effect::CheckUpdate(app));
    let checked = notice_after(&sink, before, "check for updates");
    assert!(checked.title.ends_with("is up to date"), "{checked:?}");

    // Pretend an older version is installed: the check offers the update, Update installs it over the folder.
    std::fs::write(folder.join("version.txt"), "v0.0.0-old").expect("older version");
    std::fs::write(folder.join("my-save.dat"), b"keep me").expect("save");
    let before = sink.all().len();
    host.handle(&Effect::CheckUpdate(app));
    wait(&sink, "the update to be offered", Duration::from_secs(60), |all| {
        all[before..].iter().any(|a| matches!(a, AppAction::SetGames(g) if g.iter().any(|g| g.id == app && g.status == reclaw_ui::model::AppStatus::UpdateReady)))
            || sink.notices().iter().any(|n| n.title.contains("Could not check"))
    });
    host.handle(&Effect::Update(app));
    wait(&sink, "the update", Duration::from_secs(600), |all| ended(all) >= 2);
    let end = last_activity(&sink);
    println!("  update: {end:?}");
    assert!(
        matches!(&end, ActivityEvent::Finished { changelog: Some(c), .. } if c.from == "v0.0.0-old" && c.to == version),
        "{end:?}; {:#?}",
        sink.notices()
    );
    assert!(sink.all().iter().any(|a| matches!(a, AppAction::Activity(ActivityEvent::Started { kind: Kind::Update, .. }))));
    assert_eq!(std::fs::read(folder.join("my-save.dat")).expect("the save survived"), b"keep me");

    // Uninstall.
    let before = sink.notices().len();
    host.handle(&Effect::Uninstall(app));
    let removed = notice_after(&sink, before, "uninstall");
    assert!(removed.title.ends_with("was uninstalled"), "{removed:?}");
    assert!(!folder.exists());
}

#[test]
#[ignore = "the real catalog and GitLab; downloads a game"]
fn live_gitlab_extreme_g() {
    life("sonicdcer/ExtremeGRecomp", "ExtremeG-ExtremeGRecompiled");
}

#[test]
#[ignore = "the real catalog and GitLab; downloads a game"]
fn live_gitlab_mario_kart_64() {
    life("sonicdcer/MarioKart64Recomp", "MarioKart64-MarioKart64Recompiled");
}

#[test]
#[ignore = "the real catalog and GitHub; downloads a game"]
fn live_github_zelda64recomp() {
    life("Zelda64Recomp/Zelda64Recomp", "TheLegendOfZeldaMajorasMaskAndOcarinaOfTime-Zelda64Recompiled");
}
