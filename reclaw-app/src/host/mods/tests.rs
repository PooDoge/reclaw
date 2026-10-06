//! Mods through the host: listing, installing, updating and removing against a fake Thunderstore, for a game in the library.
use std::sync::{Arc, Mutex};

use reclaw_mods::{Endpoints, ModSites};
use reclaw_net::{
    Net,
    testing::{Reply, TestServer, local_config},
};
use reclaw_ui::{
    effect::Effect,
    model::{ModEntry, ModProvider, ModStatus},
    store::AppAction,
};
use serde_json::json;

use crate::host::install::tests::{Rig, start, zip_of};

const LIBRARY: &str = r#"{"apps": [{"name": "Zelda", "repository": "", "folderName": "Zelda", "installPath": "~/Games/Zelda",
  "filesToAdd": ["portable.txt"], "mods": {"path": "mods", "sources": [{"provider": "thunderstore", "sourceUrl": "https://thunderstore.io/c/zelda/"}]}}]}"#;

/// The newest version the fake site offers.
type Latest = Arc<Mutex<&'static str>>;

fn site(latest: Latest) -> TestServer {
    TestServer::start(move |req, _| {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        let version = *latest.lock().expect("lock");
        let path = req.path.split('?').next().unwrap_or_default();
        match path {
            "/api/cyberstorm/listing/zelda/" => Reply::ok(
                json!({"count": 2, "next": null, "results": [
                    {"namespace": "Cam", "name": "Better_Camera", "description": "Free camera", "download_count": 900,
                     "icon_url": format!("http://{host}/icons/Cam-Better_Camera-{version}.png")},
                    {"namespace": "X", "name": "Old", "is_deprecated": true}
                ]})
                .to_string(),
            ),
            "/api/experimental/package/Cam/Better_Camera/" => Reply::ok(
                json!({"namespace": "Cam", "name": "Better_Camera", "latest": {"version_number": version,
                  "download_url": format!("http://{host}/dl/{version}"), "dependencies": []}})
                .to_string(),
            ),
            _ if path.starts_with("/dl/") => {
                Reply::ok(zip_of(&[("manifest.json", b"{}".to_vec()), ("better_camera.nrm", path.as_bytes().to_vec())]))
            }
            _ => Reply::new(404, "no"),
        }
    })
}

fn rig(latest: &Latest) -> (Rig, TestServer) {
    let server = site(latest.clone());
    let base = server.url("");
    let rig = start(vec![], Some(LIBRARY), |config, root| {
        std::fs::create_dir_all(root.join("home/Games/Zelda")).expect("mkdir");
        let mut net_config = local_config();
        net_config.cache_dir = Some(root.join("mods-http"));
        net_config.attempts = 1;
        let net = Net::new(net_config).expect("net");
        config.mod_sites = Some(ModSites::new(net).with_endpoints(Endpoints { thunderstore: base.clone(), gamebanana: base }));
    });
    (rig, server)
}

fn last_mods(rig: &Rig) -> Vec<ModEntry> {
    rig.sink.all().into_iter().rev().find_map(|a| if let AppAction::SetMods(m) = a { Some(m) } else { None }).unwrap_or_default()
}

fn wait_mods(rig: &Rig, what: &str, done: impl Fn(&[ModEntry]) -> bool) {
    rig.sink.wait_for(what, |all| {
        all.iter().rev().find_map(|a| if let AppAction::SetMods(m) = a { Some(m) } else { None }).is_some_and(|m| done(m))
    });
}

fn status_of(mods: &[ModEntry], id: &str) -> Option<ModStatus> {
    mods.iter().find(|m| m.id == id).map(|m| m.status)
}

#[test]
fn a_mod_is_listed_installed_updated_and_removed() {
    let latest: Latest = Arc::new(Mutex::new("1.0.0"));
    let (rig, _site) = rig(&latest);
    let folder = rig.root.path().join("home/Games/Zelda");
    wait_mods(&rig, "the listing", |m| status_of(m, "Cam-Better_Camera") == Some(ModStatus::Available));
    let listed = last_mods(&rig);
    assert_eq!(listed.len(), 1, "the deprecated mod is not offered: {listed:?}");
    assert_eq!((listed[0].title.as_str(), listed[0].version.as_str()), ("Better Camera", "1.0.0"));
    let game = listed[0].game_id;

    rig.host.handle(&Effect::InstallMod { game, provider: ModProvider::Thunderstore, id: "Cam-Better_Camera".into() });
    rig.wait_ended(1);
    wait_mods(&rig, "the install", |m| status_of(m, "Cam-Better_Camera") == Some(ModStatus::Installed));
    assert_eq!(std::fs::read(folder.join("mods/better_camera.nrm")).expect("placed"), b"/dl/1.0.0");
    assert!(folder.join("portable.txt").exists(), "the recomp is told to read its own folder");
    assert!(!folder.join("mods/manifest.json").exists());

    // The site lists a newer version: the mod shows an update, and installing it again replaces the file.
    *latest.lock().expect("lock") = "1.1.0";
    rig.host.handle(&Effect::RefreshMods);
    wait_mods(&rig, "the update", |m| status_of(m, "Cam-Better_Camera") == Some(ModStatus::UpdateReady));
    rig.host.handle(&Effect::InstallMod { game, provider: ModProvider::Thunderstore, id: "Cam-Better_Camera".into() });
    rig.wait_ended(2);
    wait_mods(&rig, "the updated mod", |m| m.iter().any(|e| e.installed_version.as_deref() == Some("1.1.0")));
    assert_eq!(std::fs::read(folder.join("mods/better_camera.nrm")).expect("placed"), b"/dl/1.1.0");

    rig.host.handle(&Effect::RemoveMod { game, provider: ModProvider::Thunderstore, id: "Cam-Better_Camera".into() });
    wait_mods(&rig, "the removal", |m| status_of(m, "Cam-Better_Camera") == Some(ModStatus::Available));
    assert!(!folder.join("mods/better_camera.nrm").exists());
}

#[test]
fn a_failed_install_is_reported_and_changes_nothing() {
    let latest: Latest = Arc::new(Mutex::new("1.0.0"));
    let (rig, _site) = rig(&latest);
    wait_mods(&rig, "the listing", |m| !m.is_empty());
    let game = last_mods(&rig)[0].game_id;
    // Another mod already owns the file this one would place.
    let record =
        json!({"mods": [{"provider": "thunderstore", "id": "Other-Mod", "name": "Other", "version": "1", "files": ["better_camera.nrm"]}]});
    let folder = rig.root.path().join("home/Games/Zelda");
    std::fs::write(folder.join(".quiver-mods.json"), record.to_string()).expect("write");

    rig.host.handle(&Effect::InstallMod { game, provider: ModProvider::Thunderstore, id: "Cam-Better_Camera".into() });
    rig.wait_ended(1);
    let failed = rig.activity().into_iter().find_map(|e| match e {
        reclaw_ui::activity::ActivityEvent::Failed { reason, details, .. } => Some((reason, details)),
        _ => None,
    });
    let (reason, details) = failed.expect("the job failed");
    assert!(reason.contains("Other"), "{reason}");
    assert!(details.iter().any(|d| d.contains("Remove Other first")), "{details:?}");
    assert!(!folder.join("mods/better_camera.nrm").exists());
    wait_mods(&rig, "the list after", |m| status_of(m, "Cam-Better_Camera") == Some(ModStatus::Available));
}

#[test]
fn an_unknown_game_is_refused_with_a_notice() {
    let latest: Latest = Arc::new(Mutex::new("1.0.0"));
    let (rig, _site) = rig(&latest);
    rig.host.handle(&Effect::InstallMod { game: 999_999, provider: ModProvider::GameBanana, id: "1".into() });
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("cannot take mods")));
    assert!(rig.activity().is_empty());
}
