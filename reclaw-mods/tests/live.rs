//! The real mod sites, by hand: `cargo test -p reclaw-mods --test live -- --ignored --nocapture --test-threads 1`. Each test lists
//! one game's mods, then installs and removes the most downloaded one in a temporary folder (a few megabytes at most), and leaves
//! nothing behind. The development sandbox's proxy refuses both sites, so these have not been run there.
//!
//! `RECLAW_LIVE_THUNDERSTORE=<community>` and `RECLAW_LIVE_GAMEBANANA=<game number>` pick other games than Zelda 64: Recompiled
//! (Thunderstore) and Mystical Ninja Starring Goemon (GameBanana 24290), both from the community catalog.
use std::fs;

use reclaw_catalog::mods::{ModLayout, ModSource, ModsConfig};
use reclaw_mods::{Document, ModInstaller, ModSites, Sort, Source, Target, install_with_dependencies, uninstall};
use reclaw_net::{Cancel, Net, NetConfig};

fn net(dir: &std::path::Path) -> Net {
    let (mut config, problems) = NetConfig::from_env(|k| std::env::var(k).ok());
    assert!(problems.is_empty(), "{problems:?}");
    config.cache_dir = Some(dir.join("http"));
    Net::new(config).expect("net")
}

fn list_install_remove(provider: &str, url: &str) {
    let dir = tempfile::tempdir().expect("tempdir");
    let net = net(dir.path());
    let sites = ModSites::new(net.clone());
    let config = ModsConfig {
        path: "mods".into(),
        layout: ModLayout::Flat,
        sources: vec![ModSource { provider: provider.into(), source_url: url.into() }],
    };
    let source = Source::all_of(&config).into_iter().next().expect("a source Reclaw reads");
    let listed = sites.list_up_to(&source, 60, Sort::MostDownloaded, true).expect("the site lists mods");
    eprintln!("{} mods listed on {} for {}", listed.len(), source.provider.label(), source.key);
    for package in listed.iter().take(5) {
        eprintln!("  {} by {} v{} ({} downloads)", package.name, package.owner, package.version, package.downloads);
    }
    let first = listed.iter().find(|p| !p.deprecated && !p.nsfw).expect("a mod to install");

    let game = dir.path().join("Game");
    fs::create_dir_all(&game).expect("mkdir");
    let target = Target { folder: game.clone(), config };
    let installer = ModInstaller::new(net, dir.path().join("downloads"));
    let records = install_with_dependencies(&sites, &installer, &target, first, &Cancel::new(), &mut |_| {}).expect("installs");
    for record in &records {
        eprintln!("installed {} v{}: {:?}", record.title(), record.version, record.files);
    }
    let document = Document::load(&game).expect("the record reads");
    assert!(document.mods.iter().any(|r| r.id.eq_ignore_ascii_case(&first.id)), "{document:?}");
    for record in &records {
        assert!(!record.files.is_empty(), "{} placed no files", record.title());
        assert!(record.files.iter().all(|f| game.join("mods").join(f).is_file()), "{record:?}");
    }
    for record in records.iter().rev() {
        let provider = record.provider().expect("a known provider");
        assert_eq!(uninstall(&target, provider, &record.id), Ok(true));
    }
    assert!(Document::load(&game).expect("the record reads").mods.is_empty());
}

#[test]
#[ignore = "asks thunderstore.io and downloads a mod"]
fn thunderstore() {
    let community = std::env::var("RECLAW_LIVE_THUNDERSTORE").unwrap_or_else(|_| "zelda-64-recompiled".into());
    list_install_remove("thunderstore", &format!("https://thunderstore.io/c/{community}/"));
}

#[test]
#[ignore = "asks gamebanana.com and downloads a mod"]
fn gamebanana() {
    let game = std::env::var("RECLAW_LIVE_GAMEBANANA").unwrap_or_else(|_| "24290".into());
    list_install_remove("gamebanana", &format!("https://gamebanana.com/games/{game}"));
}
