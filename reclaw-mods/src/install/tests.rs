//! Installing and removing mods against a fake Thunderstore and GameBanana on this machine, with real archives.
use std::{
    collections::HashMap,
    fs,
    io::Write,
    sync::{Arc, Mutex},
};

use reclaw_catalog::mods::{ModLayout, ModsConfig};
use reclaw_net::testing::{Reply, TestServer, local_net};
use serde_json::json;

use super::*;
use crate::client::{Endpoints, ModSites};

pub fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in files {
        writer.start_file(*name, zip::write::SimpleFileOptions::default()).expect("file");
        writer.write_all(data).expect("data");
    }
    writer.finish().expect("finish").into_inner()
}

/// A fake site: paths to bodies. `{host}` in a body becomes the server's address.
#[derive(Clone, Default)]
struct Routes(Arc<Mutex<HashMap<String, Vec<u8>>>>);

impl Routes {
    fn set(&self, path: &str, body: impl Into<Vec<u8>>) {
        self.0.lock().expect("lock").insert(path.to_string(), body.into());
    }

    /// A Thunderstore package: its experimental answer and its zip.
    fn thunderstore(&self, owner: &str, name: &str, version: &str, dependencies: &[&str], files: &[(&str, &[u8])]) {
        let zip = format!("/dl/{owner}-{name}-{version}.zip");
        self.set(
            &format!("/api/experimental/package/{owner}/{name}/"),
            json!({"namespace": owner, "name": name, "latest": {"version_number": version, "download_url": format!("http://{{host}}{zip}"), "dependencies": dependencies}})
                .to_string(),
        );
        self.set(&zip, zip_of(files));
    }
}

struct Rig {
    server: TestServer,
    routes: Routes,
    sites: ModSites,
    installer: ModInstaller,
    root: tempfile::TempDir,
    target: Target,
}

fn rig(layout: ModLayout) -> Rig {
    let routes = Routes::default();
    let served = routes.clone();
    let server = TestServer::start(move |req, _| {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        let path = req.path.split('?').next().unwrap_or_default().to_string();
        match served.0.lock().expect("lock").get(&path) {
            // Downloads are bytes; only the API's answers name the server.
            Some(body) if path.starts_with("/dl/") => Reply::ok(body.clone()),
            Some(body) => Reply::ok(String::from_utf8_lossy(body).replace("{host}", &host).into_bytes()),
            None => Reply::new(404, format!("no {path}")),
        }
    });
    let net = local_net();
    let base = server.url("");
    let sites = ModSites::new(net.clone()).with_endpoints(Endpoints { thunderstore: base.clone(), gamebanana: base });
    let root = tempfile::tempdir().expect("dir");
    let installer = ModInstaller::new(net, root.path().join("downloads"));
    let game = root.path().join("Game");
    fs::create_dir_all(&game).expect("mkdir");
    let target = Target { folder: game, config: ModsConfig { path: "mods".into(), layout, ..ModsConfig::default() } };
    Rig { server, routes, sites, installer, root, target }
}

fn package(id: &str) -> Package {
    let (owner, name) = id.split_once('-').expect("owner-name");
    Package::named(Provider::Thunderstore, "zelda-64-recompiled", id, owner, name)
}

fn install(rig: &Rig, id: &str) -> Result<Vec<Record>, ModError> {
    install_with_dependencies(&rig.sites, &rig.installer, &rig.target, &package(id), &Cancel::new(), &mut |_| {})
}

fn mods(rig: &Rig) -> std::path::PathBuf {
    rig.target.folder.join("mods")
}

#[test]
fn a_thunderstore_mod_arrives_with_its_dependency_and_without_its_description() {
    let rig = rig(ModLayout::Flat);
    rig.routes.thunderstore("Lib", "Core", "2.0.0", &[], &[("manifest.json", b"{}"), ("core.nrm", b"core")]);
    rig.routes.thunderstore(
        "Cam",
        "Better",
        "1.2.0",
        &["Lib-Core-2.0.0"],
        &[("manifest.json", b"{}"), ("icon.png", b"png"), ("README.md", b"#"), ("better.nrm", b"cam")],
    );

    let records = install(&rig, "Cam-Better").expect("installs");
    assert_eq!(records.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["Lib-Core", "Cam-Better"], "the dependency first");
    assert_eq!(fs::read(mods(&rig).join("better.nrm")).expect("placed"), b"cam");
    assert!(mods(&rig).join("core.nrm").exists());
    assert!(!mods(&rig).join("manifest.json").exists() && !mods(&rig).join("icon.png").exists());
    assert!(!rig.target.folder.join(STAGE_DIR).exists(), "nothing staged is left");

    let document = Document::load(&rig.target.folder).expect("record");
    assert_eq!(document.mods.len(), 2);
    let better = document.find(Provider::Thunderstore, "Cam-Better").next().expect("recorded");
    assert_eq!((better.version.as_str(), better.files.as_slice()), ("1.2.0", ["better.nrm".to_string()].as_slice()));

    // Installed already: the dependency is not fetched again.
    let before = rig.server.count();
    install(&rig, "Cam-Better").expect("reinstalls");
    let fetched_core = rig.server.requests()[before..].iter().any(|r| r.path.contains("Lib-Core") || r.path.contains("/Lib/Core/"));
    assert!(!fetched_core);
}

#[test]
fn an_update_replaces_the_earlier_version_and_drops_what_it_no_longer_has() {
    let rig = rig(ModLayout::Flat);
    rig.routes.thunderstore("Cam", "Better", "1.0.0", &[], &[("mods/better.nrm", b"v1"), ("mods/extra/old.bin", b"old")]);
    install(&rig, "Cam-Better").expect("installs");
    assert!(mods(&rig).join("extra/old.bin").exists(), "a folder named like the mods folder is dropped, not nested");
    assert!(!mods(&rig).join("mods").exists());

    rig.routes.thunderstore("Cam", "Better", "1.1.0", &[], &[("better.nrm", b"v2")]);
    install(&rig, "Cam-Better").expect("updates");
    assert_eq!(fs::read(mods(&rig).join("better.nrm")).expect("read"), b"v2");
    assert!(!mods(&rig).join("extra").exists(), "the old file and its folder are gone");
    let document = Document::load(&rig.target.folder).expect("record");
    assert_eq!(document.mods.len(), 1);
    assert_eq!(document.mods[0].version, "1.1.0");
}

#[test]
fn another_mods_file_is_never_replaced() {
    let rig = rig(ModLayout::Flat);
    rig.routes.thunderstore("A", "One", "1.0.0", &[], &[("shared.nrm", b"one")]);
    rig.routes.thunderstore("B", "Two", "1.0.0", &[], &[("shared.nrm", b"two"), ("two.nrm", b"two")]);
    install(&rig, "A-One").expect("installs");
    match install(&rig, "B-Two") {
        Err(ModError::Conflict { owner, files, .. }) => {
            assert_eq!((owner.as_str(), files.as_slice()), ("One", ["shared.nrm".to_string()].as_slice()))
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert_eq!(fs::read(mods(&rig).join("shared.nrm")).expect("read"), b"one");
    assert!(!mods(&rig).join("two.nrm").exists(), "nothing of the refused mod was placed");
    assert_eq!(Document::load(&rig.target.folder).expect("record").mods.len(), 1);
}

#[test]
fn removing_a_mod_removes_its_files_folders_and_record() {
    let rig = rig(ModLayout::FolderPerMod);
    rig.routes.thunderstore("Cam", "Better", "1.0.0", &[], &[("better.dll", b"x"), ("data/a.bin", b"y")]);
    install(&rig, "Cam-Better").expect("installs");
    assert!(mods(&rig).join("Better/better.dll").exists() && mods(&rig).join("Better/data/a.bin").exists(), "loose files get a folder");
    fs::write(mods(&rig).join("mine.txt"), "the person's own").expect("write");

    assert_eq!(uninstall(&rig.target, Provider::Thunderstore, "cam-better"), Ok(true));
    assert!(!mods(&rig).join("Better").exists());
    assert!(mods(&rig).join("mine.txt").exists(), "what no mod recorded stays");
    assert!(Document::load(&rig.target.folder).expect("record").mods.is_empty());
    assert_eq!(uninstall(&rig.target, Provider::Thunderstore, "Cam-Better"), Ok(false));
}

#[test]
fn an_unreadable_record_stops_everything_before_a_download() {
    let rig = rig(ModLayout::Flat);
    rig.routes.thunderstore("Cam", "Better", "1.0.0", &[], &[("better.nrm", b"x")]);
    fs::write(Document::path(&rig.target.folder), "{ not json").expect("write");
    let download = rig.sites.download_for(&package("Cam-Better")).expect("resolves");
    let before = rig.server.count();
    let result = rig.installer.install(&rig.target, &package("Cam-Better"), &download, &Cancel::new(), &mut |_| {});
    assert!(matches!(result, Err(ModError::Unreadable { .. })));
    assert_eq!(rig.server.count(), before, "nothing was downloaded");
    assert!(matches!(uninstall(&rig.target, Provider::Thunderstore, "Cam-Better"), Err(ModError::Unreadable { .. })));
    assert_eq!(fs::read_to_string(Document::path(&rig.target.folder)).expect("read"), "{ not json", "left as it was");
}

#[test]
fn a_gamebanana_mod_in_one_file_is_placed_as_it_is() {
    let rig = rig(ModLayout::Flat);
    // A `.nrm` is a zip inside; its name says it is the mod, not an archive of it.
    let nrm = zip_of(&[("mod.json", b"{}")]);
    rig.routes.set(
        "/apiv11/Mod/777",
        json!({"_sVersion": "1.0", "_aFiles": [{"_idRow": 5, "_sFile": "goemon_fix.nrm", "_sDownloadUrl": "http://{host}/dl/5"}]})
            .to_string(),
    );
    rig.routes.set("/dl/5", nrm.clone());
    let mut mod_777 = Package::named(Provider::GameBanana, "24290", "777", "Ana", "Goemon Fix");
    mod_777.full_name = "Ana-Goemon Fix".into();
    install_with_dependencies(&rig.sites, &rig.installer, &rig.target, &mod_777, &Cancel::new(), &mut |_| {}).expect("installs");
    assert_eq!(fs::read(mods(&rig).join("goemon_fix.nrm")).expect("placed"), nrm);
    let document = Document::load(&rig.target.folder).expect("record");
    assert_eq!(document.mods[0].download_file_id.as_deref(), Some("5"));
    assert_eq!(document.mods[0].download_file_name.as_deref(), Some("goemon_fix.nrm"));
}

#[test]
fn a_missing_dependency_fails_the_install_and_places_nothing() {
    let rig = rig(ModLayout::Flat);
    rig.routes.thunderstore("Cam", "Better", "1.0.0", &["Gone-Lib-1.0.0"], &[("better.nrm", b"x")]);
    assert!(matches!(install(&rig, "Cam-Better"), Err(ModError::NoDownload(_))));
    assert!(!mods(&rig).join("better.nrm").exists());
    let _ = &rig.root;
}
