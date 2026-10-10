//! Installing an app quiverlauncher.com lists: what it did not verify waits for a second press, and a download whose file is gone is
//! reported to the site, which offers the release it verified before.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use reclaw_net::{Net, testing::Req};
use reclaw_sync::{CatalogSync, SiteClient};
use reclaw_ui::{activity::ActivityEvent, effect::Effect, model::AppStatus};

use super::*;

/// The fake catalog and GitHub of [`reply`], with the site's API for One beside them. `history` is the release history's items;
/// `gone` makes v2's file answer 404.
fn site_server(
    releases: Releases,
    verified: &'static str,
    history: &'static str,
    gone: Arc<AtomicBool>,
    seen: Arc<Mutex<Vec<String>>>,
) -> TestServer {
    TestServer::start(move |req: &Req, _| {
        seen.lock().expect("lock").push(format!("{} {}", req.method, req.path));
        match req.path.as_str() {
            "/api/v1/release-status?limit=100" => Reply::ok(format!(
                r#"{{"items":[{{"id":"k1","slug":"one","provider":"github","repository":"o/one"{verified}}}],"isDone":true}}"#
            )),
            "/api/v1/apps?limit=100" => {
                Reply::ok(r#"{"items":[{"id":"k1","slug":"one","name":"One","addedAt":1,"launcher":{"folderName":"One"}}],"isDone":true}"#)
            }
            "/api/v1/apps/one/release-history?limit=100" => Reply::ok(format!(r#"{{"items":{history},"isDone":true}}"#)),
            "/api/v1/apps/one/download-problem" if req.method == "POST" => Reply::new(202, "{}"),
            path if path.starts_with("/dl/v2/") && gone.load(Ordering::SeqCst) => Reply::new(404, "gone"),
            _ => reply(&releases, req),
        }
    })
}

/// The rig, refreshed once more with the site so One is linked.
fn linked_rig(releases: Vec<Rel>, verified: &'static str, history: &'static str, gone: bool) -> (Rig, Arc<Mutex<Vec<String>>>) {
    let releases: Releases = Arc::new(Mutex::new(releases));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let server = site_server(releases.clone(), verified, history, Arc::new(AtomicBool::new(gone)), seen.clone());
    let rig = start_on(releases, server, None, |_, _| {});
    let mut config = local_config();
    config.attempts = 1;
    let net = Net::new(config).expect("net");
    let sync = CatalogSync::new(net.clone())
        .with_index_url(rig.server.url("/index.json"))
        .with_site(SiteClient::with_base(net, rig.server.url("/api/v1")));
    rig.host.run_refresh(&sync);
    (rig, seen)
}

fn press(rig: &Rig) {
    rig.host.handle(&Effect::StartInstall { app: rig.app(), location: String::new(), prerelease: false });
}

#[test]
fn an_unverified_release_installs_only_when_install_is_pressed_again() {
    let history = r#"[{"version":"v1","state":"unverified","reasons":["Released 2 hours ago."],"assets":[{"filename":"Game-linux.zip"}]}]"#;
    let (rig, seen) = linked_rig(vec![rel("v1")], "", history, false);
    press(&rig);
    rig.wait_ended(1);
    let notice = rig.sink.notices().pop().expect("told");
    assert!(notice.title.contains("not verified"), "{notice:?}");
    assert!(
        notice.details.iter().any(|d| d == "Released 2 hours ago.") && notice.details.iter().any(|d| d.contains("press Install again")),
        "{notice:?}"
    );
    assert!(!seen.lock().expect("lock").iter().any(|r| r.contains("/dl/")), "nothing was downloaded");
    assert_ne!(rig.status(), Some(AppStatus::Installed));

    press(&rig);
    rig.wait_ended(2);
    assert_eq!(rig.status(), Some(AppStatus::Installed), "{:?}", rig.activity());
}

#[test]
fn a_verified_release_whose_file_is_gone_is_reported_and_the_one_verified_before_is_offered() {
    let history = r#"[{"version":"v2","state":"verified","assets":[{"filename":"Game-linux.zip"}]},
                      {"version":"v1","state":"verified","assets":[{"filename":"Game-linux.zip"}]}]"#;
    let (rig, seen) = linked_rig(vec![rel("v2"), rel("v1")], r#","verified":{"version":"v2"}"#, history, true);
    press(&rig);
    rig.wait_ended(1);
    let Some(ActivityEvent::Failed { details, .. }) = rig.activity().last().cloned() else { panic!("{:?}", rig.activity()) };
    assert!(details.iter().any(|d| d.contains("Press Install again to install v1")), "{details:?}");
    assert!(seen.lock().expect("lock").iter().any(|r| r == "POST /api/v1/apps/one/download-problem"), "the site was told");

    press(&rig);
    rig.wait_ended(2);
    assert_eq!(rig.status(), Some(AppStatus::Installed), "{:?}", rig.activity());
    assert_eq!(reclaw_install::layout::installed_version(&rig.folder()).as_deref(), Some("v1"));

    // The check for an update compares with the release the site verified, v2, without asking GitHub.
    let asked = seen.lock().expect("lock").len();
    rig.host.handle(&Effect::CheckUpdate(rig.app()));
    assert_eq!(rig.status(), Some(AppStatus::UpdateReady), "{:?}", rig.sink.notices());
    assert_eq!(seen.lock().expect("lock").len(), asked, "answered from the feed");
}
