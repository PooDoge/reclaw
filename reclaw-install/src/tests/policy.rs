use super::release_with;
use crate::{format::Format, platform::Platform, policy::Selection};

fn names(selection: &Selection) -> Vec<&str> {
    selection.eligible.iter().map(|a| a.name.as_str()).collect()
}

#[test]
fn one_linux_build_among_others_is_the_automatic_choice() {
    let release =
        release_with("v1", false, &["Game-Windows.zip", "Game-Linux-x64.tar.gz", "Game-macos.zip", "Game-source.zip", "Game.sha256"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    // The Windows build stays in the list (a compatibility layer can run it), after the native one.
    assert_eq!(names(&selection), ["Game-Linux-x64.tar.gz", "Game-Windows.zip"]);
    assert_eq!(selection.automatic(Platform::LinuxX64).map(|a| a.name.as_str()), Some("Game-Linux-x64.tar.gz"));
    assert!(!selection.needs_choice(Platform::LinuxX64));
    assert_eq!(selection.reason, None);
}

#[test]
fn a_windows_build_alone_is_the_choice_on_linux() {
    let release = release_with("v1", false, &["Game-Windows.zip", "Game-macos.zip"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    assert_eq!(names(&selection), ["Game-Windows.zip"]);
    assert_eq!(selection.automatic(Platform::LinuxX64).map(|a| a.name.as_str()), Some("Game-Windows.zip"));
}

#[test]
fn two_native_builds_need_someone_to_choose() {
    let release = release_with("v1", false, &["Game-linux-x64.tar.gz", "Game-x86_64.AppImage"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    assert_eq!(selection.automatic(Platform::LinuxX64), None);
    assert!(selection.needs_choice(Platform::LinuxX64));
    assert_eq!(selection.choices().len(), 2);
}

#[test]
fn the_catalogs_filter_narrows_the_files() {
    let release = release_with("v1", false, &["Game-linux-x64.tar.gz", "Game-x86_64.AppImage"]);
    let selection = Selection::of(&release, Platform::LinuxX64, Some("appimage"));
    assert_eq!(names(&selection), ["Game-x86_64.AppImage"]);
    assert_eq!(selection.automatic(Platform::LinuxX64).map(|a| a.name.as_str()), Some("Game-x86_64.AppImage"));
}

#[test]
fn a_filter_that_matches_nothing_says_so() {
    let release = release_with("v1", false, &["Game-linux-x64.tar.gz"]);
    let selection = Selection::of(&release, Platform::LinuxX64, Some("steamdeck"));
    assert!(selection.eligible.is_empty());
    assert_eq!(selection.reason.as_deref(), Some("No download files match the release asset filter \"steamdeck\"."));
}

#[test]
fn a_release_with_only_metadata_has_nothing_to_install() {
    let release = release_with("v1", false, &["Game-source.zip", "Game.sha256", "release.json"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    assert_eq!(selection.reason.as_deref(), Some("This release has no installable download files."));
}

#[test]
fn another_platforms_files_leave_nothing_for_this_one() {
    let release = release_with("v1", false, &["Game-macos.zip", "Game-android.apk"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    assert!(selection.eligible.is_empty());
    assert_eq!(selection.reason.as_deref(), Some("This release has no recognized download for Linux-X64."));
}

#[test]
fn a_package_for_a_system_installer_is_named_not_offered() {
    let release = release_with("v1", false, &["Game-linux.deb", "Game-macos.zip"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    assert!(selection.eligible.is_empty());
    assert_eq!(
        selection.reason.as_deref(),
        Some("This release's download for Linux-X64 is a system package, which Reclaw does not install.")
    );
    // ...but a deb next to an AppImage does not get in the way of the AppImage.
    let both = release_with("v1", false, &["Game-linux.deb", "Game-x86_64.AppImage"]);
    assert_eq!(
        Selection::of(&both, Platform::LinuxX64, None).automatic(Platform::LinuxX64).map(|a| a.name.as_str()),
        Some("Game-x86_64.AppImage")
    );
}

#[test]
fn files_that_name_no_platform_are_offered_when_nothing_else_fits() {
    let release = release_with("v1", false, &["Game.tar", "Game-macos.zip"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    assert!(selection.eligible.is_empty());
    assert!(!Format::of_name("Game.tar").is_installable(), "a bare .tar is not a format this installer reads");
    assert!(selection.uncertain.is_empty());

    let release = release_with("v1", false, &["CrashBandicoot_Anything"]);
    let selection = Selection::of(&release, Platform::LinuxX64, None);
    assert_eq!(selection.uncertain.len(), 1);
    assert!(selection.needs_choice(Platform::LinuxX64));
}

#[test]
fn macos_gets_the_mac_build() {
    let release = release_with("v1", false, &["Game-Windows.zip", "Game-macos.zip", "Game-linux.tar.gz"]);
    let selection = Selection::of(&release, Platform::MacOs, None);
    assert_eq!(names(&selection), ["Game-macos.zip"]);
}
