//! Installing real games from their real releases, by hand: `cargo test -p reclaw-install --test live -- --ignored --nocapture
//! --test-threads 1`. Each test downloads a few tens of megabytes from GitHub or GitLab, asks the host's API a few times, and
//! leaves nothing behind (everything goes into a temporary folder).
//!
//! The games are entries of the community catalog, chosen for the shapes their releases come in: a zip with a wrapper folder,
//! an AppImage that is renamed for each version, a lone program file, GitLab package links whose names carry no extension, a
//! 7z, and a Windows-only zip. The tags pinned here are real releases; when one is deleted upstream, pick another.
//!
//! `RECLAW_LIVE_GITHUB_DIRECT=1` skips GitHub's API and installs the GitHub cases from their download addresses alone, for a
//! network that lets downloads through but not `api.github.com` (the development sandbox's does that).
use std::{
    fs,
    path::{Path, PathBuf},
};

use reclaw_install::{
    Asset, Host, InstallError, Installed, Installer, Plan, Platform, Release, ReleaseSource, Request, Resolved, layout, programs, remove,
};
use reclaw_net::{Cancel, Net, NetConfig};

struct Rig {
    installer: Installer,
    dir: tempfile::TempDir,
}

fn rig() -> Rig {
    let dir = tempfile::tempdir().expect("tempdir");
    let (mut config, problems) = NetConfig::from_env(|k| std::env::var(k).ok());
    assert!(problems.is_empty(), "{problems:?}");
    config.cache_dir = Some(dir.path().join("http"));
    let net = Net::new(config).expect("net");
    let installer = Installer::new(net.clone(), ReleaseSource::new(net), dir.path().join("downloads"));
    Rig { installer, dir }
}

fn request(host: Host, repo: &str, folder: &Path) -> Request {
    Request {
        host,
        repo: repo.to_string(),
        folder: folder.to_path_buf(),
        platform: Platform::LinuxX64,
        filter: None,
        preferred_version: None,
        allow_prerelease: false,
        asset: None,
    }
}

fn github_direct() -> bool {
    std::env::var("RECLAW_LIVE_GITHUB_DIRECT").is_ok_and(|v| v == "1")
}

/// A GitHub release described by hand: its tag and the names of the files to offer, at their real download addresses.
fn by_hand(repo: &str, tag: &str, files: &[&str]) -> Release {
    let assets = files
        .iter()
        .map(|name| Asset {
            name: (*name).to_string(),
            url: format!("https://github.com/{repo}/releases/download/{tag}/{name}"),
            size: None,
            sha256: None,
        })
        .collect();
    Release { tag: tag.to_string(), assets, ..Release::default() }
}

impl Rig {
    fn resolve(&self, request: &Request) -> Resolved {
        match self.installer.resolve(request) {
            Ok(Plan::Ready(resolved)) => *resolved,
            Ok(Plan::Choose { release, choices }) => {
                panic!("{} {}: asks which file: {:?}", request.repo, release.tag, choices.iter().map(|a| &a.name).collect::<Vec<_>>())
            }
            Err(error) => panic!("{}: {error} ({:?})", request.repo, error.hint()),
        }
    }

    /// Resolve against the host, or (GitHub, direct mode) choose among the files of `fallback` the way `resolve` would.
    fn resolve_or(&self, request: &Request, fallback: Release) -> Resolved {
        if request.host == Host::GitHub && github_direct() {
            let selection = reclaw_install::Selection::of(&fallback, request.platform, request.filter.as_deref());
            let asset = match &request.asset {
                Some(name) => selection.choices().iter().find(|a| &a.name == name).cloned(),
                None => selection.automatic(request.platform).cloned(),
            }
            .unwrap_or_else(|| panic!("{}: no file chosen from {:?}: {:?}", request.repo, fallback.assets, selection.reason));
            let already_installed = layout::installed_version(&request.folder).is_some_and(|v| v == fallback.tag)
                && layout::is_complete(&request.folder, request.platform);
            return Resolved { release: fallback, asset, already_installed };
        }
        self.resolve(request)
    }

    fn install(&self, request: &Request, resolved: &Resolved) -> Result<Installed, InstallError> {
        let mut stages = Vec::new();
        let result = self.installer.install(request, resolved, &Cancel::new(), &mut |step| {
            if stages.last() != Some(&step.stage) {
                stages.push(step.stage);
            }
        });
        println!("  {} {} ({}): stages {stages:?}", request.repo, resolved.release.tag, resolved.asset.name);
        result
    }

    fn folder(&self, name: &str) -> PathBuf {
        self.dir.path().join("Games").join(name)
    }
}

/// What a finished install must look like, whatever the game.
fn assert_installed(done: &Installed, tag: &str) {
    let folder = &done.folder;
    let listing = top_level(folder);
    println!("  installed {tag} into {}: {listing:?}", folder.display());
    println!("  starts {:?} (needs a runner: {})", done.program.as_ref().map(|p| p.strip_prefix(folder).unwrap_or(p)), done.needs_runner);
    assert_eq!(layout::installed_version(folder).as_deref(), Some(tag));
    assert!(!folder.join(layout::INCOMPLETE_FILE).exists(), "the incomplete marker was removed");
    assert!(!folder.join(layout::STAGE_DIR).exists(), "the staging folder was removed");
    assert!(layout::is_complete(folder, Platform::LinuxX64), "{listing:?}");
    let program = done.program.as_ref().expect("a program was found");
    assert!(program.starts_with(folder) && program.is_file());
    let found = programs::find(folder, true, Platform::LinuxX64);
    assert_eq!(found.programs.first(), Some(program), "verify finds the same program the install reported");
    #[cfg(unix)]
    if !done.needs_runner {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(program).expect("program").permissions().mode();
        assert!(mode & 0o111 != 0, "{} is executable ({mode:o})", program.display());
    }
}

fn top_level(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(folder).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    names.sort();
    names
}

/// Uninstall and check nothing is left.
fn uninstall(rig: &Rig, folder: &Path) {
    let protected = [rig.dir.path().to_path_buf(), rig.dir.path().join("Games")];
    remove::uninstall(folder, &protected, Platform::LinuxX64).unwrap_or_else(|e| panic!("uninstall {}: {e}", folder.display()));
    assert!(!folder.exists(), "{} was removed", folder.display());
    assert!(rig.dir.path().join("Games").is_dir(), "the install location itself stays");
}

/// Install an older release, keep a "save" in the folder, update to a newer one: the save survives, the version moves, and a
/// second look says it is current. Then uninstall.
fn install_update_uninstall(rig: &Rig, mut request: Request, old: Release, new: Release) {
    let (old_tag, new_tag) = (old.tag.clone(), new.tag.clone());
    request.preferred_version = Some(old_tag.clone());
    let resolved = rig.resolve_or(&request, old);
    assert_eq!(resolved.release.tag, old_tag);
    assert!(!resolved.already_installed);
    let done = rig.install(&request, &resolved).unwrap_or_else(|e| panic!("{}: {e} ({:?})", request.repo, e.hint()));
    assert_installed(&done, &old_tag);
    let save = request.folder.join("saves/slot1.sav");
    fs::create_dir_all(save.parent().expect("dir")).expect("saves");
    fs::write(&save, b"progress").expect("save");

    request.preferred_version = Some(new_tag.clone());
    let resolved = rig.resolve_or(&request, new);
    assert_eq!(resolved.release.tag, new_tag);
    assert!(!resolved.already_installed, "{old_tag} is installed, {new_tag} is not");
    let done = rig.install(&request, &resolved).unwrap_or_else(|e| panic!("{}: {e} ({:?})", request.repo, e.hint()));
    assert_installed(&done, &new_tag);
    assert_eq!(fs::read(&save).expect("the save survived the update"), b"progress");

    let again = rig.resolve_or(&request, resolved.release.clone());
    assert!(again.already_installed, "asking again finds {new_tag} installed");
    uninstall(rig, &request.folder);
}

#[test]
#[ignore = "downloads real releases from GitHub"]
fn zelda64recomp_a_zip_with_a_wrapper_updates_over_itself() {
    let rig = rig();
    let repo = "Zelda64Recomp/Zelda64Recomp";
    let request = request(Host::GitHub, repo, &rig.folder("Zelda64Recomp"));
    install_update_uninstall(
        &rig,
        request,
        by_hand(repo, "v1.2.1", &["Zelda64Recompiled-v1.2.1-Linux-X64.zip"]),
        by_hand(repo, "v1.2.2", &["Zelda64Recompiled-v1.2.2-Linux-X64.zip"]),
    );
}

#[test]
#[ignore = "downloads real releases from GitHub"]
fn openrct2_an_appimage_renamed_per_version_leaves_one_behind() {
    let rig = rig();
    let repo = "OpenRCT2/OpenRCT2";
    let folder = rig.folder("OpenRCT2");
    let request = request(Host::GitHub, repo, &folder);
    install_update_uninstall(
        &rig,
        request.clone(),
        by_hand(repo, "v0.5.4", &["OpenRCT2-v0.5.4-linux-x86_64.AppImage"]),
        by_hand(repo, "v0.5.5", &["OpenRCT2-v0.5.5-linux-x86_64.AppImage"]),
    );
}

#[test]
#[ignore = "downloads a real release from GitHub"]
fn doukutsu_rs_a_lone_program_file() {
    let rig = rig();
    let repo = "doukutsu-rs/doukutsu-rs";
    let request = request(Host::GitHub, repo, &rig.folder("CaveStory"));
    let resolved =
        rig.resolve_or(&request, by_hand(repo, "1.0.0", &["doukutsu-rs_linux_1.0.0.x86_64.elf", "doukutsu-rs_macos_1.0.0.x86_64.zip"]));
    let done = rig.install(&request, &resolved).unwrap_or_else(|e| panic!("{repo}: {e} ({:?})", e.hint()));
    assert_installed(&done, &resolved.release.tag);
    uninstall(&rig, &request.folder);
}

#[test]
#[ignore = "asks GitLab and downloads real releases"]
fn gitlab_package_links_without_extensions_install() {
    let rig = rig();
    for (repo, folder) in [
        ("sonicdcer/MarioKart64Recomp", "MarioKart64Recomp"),
        ("sonicdcer/Starfox64Recomp", "Starfox64Recomp"),
        ("sonicdcer/DNZHRecomp", "DukeNukemZeroHour"),
        ("sonicdcer/ExtremeGRecomp", "ExtremeG"),
    ] {
        let request = request(Host::GitLab, repo, &rig.folder(folder));
        let resolved = rig.resolve(&request);
        println!("{repo}: {} chose {}", resolved.release.tag, resolved.asset.name);
        assert!(resolved.asset.name.contains("Linux-X64"), "the native build, not Flatpak, ARM or Windows: {}", resolved.asset.name);
        let done = rig.install(&request, &resolved).unwrap_or_else(|e| panic!("{repo}: {e} ({:?})", e.hint()));
        assert_installed(&done, &resolved.release.tag);
        assert!(rig.resolve(&request).already_installed);
        uninstall(&rig, &request.folder);
    }
}

#[test]
#[ignore = "asks GitLab and downloads real releases"]
fn gitlab_a_7z_updates_from_a_pinned_older_release() {
    let rig = rig();
    let repo = "bighead.0/ladxhd_updated";
    let request = request(Host::GitLab, repo, &rig.folder("LinksAwakeningDXHD"));
    let found = rig.installer.releases().fetch(Host::GitLab, repo, true, true).expect("the release list");
    let tags: Vec<&str> = found.list.iter().map(|r| r.tag.as_str()).collect();
    println!("{repo}: {tags:?}");
    assert!(tags.len() >= 2, "an older release to update from");
    let (new, old) = (found.list[0].clone(), found.list[1].clone());
    install_update_uninstall(&rig, request, old, new);
}

#[test]
#[ignore = "asks GitLab and downloads a real release"]
fn gitlab_a_windows_only_zip_installs_for_a_runner() {
    let rig = rig();
    let repo = "ethan4love/psx-recomp-port";
    let request = request(Host::GitLab, repo, &rig.folder("DigimonWorld"));
    let resolved = rig.resolve(&request);
    println!("{repo}: {} chose {}", resolved.release.tag, resolved.asset.name);
    let done = rig.install(&request, &resolved).unwrap_or_else(|e| panic!("{repo}: {e} ({:?})", e.hint()));
    assert!(done.needs_runner, "a Windows build on Linux needs Wine or Proton");
    assert!(done.program.as_ref().is_some_and(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))));
    assert_installed(&done, &resolved.release.tag);
    uninstall(&rig, &request.folder);
}
