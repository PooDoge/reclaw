use crate::matcher::*;

const LINUX: &str = "Linux-X64";
const ARM: &str = "Linux-ARM64";

#[test]
fn a_windows_build_is_labelled_or_a_bare_archive() {
    assert!(is_windows_asset("Game-Windows.zip"));
    assert!(is_windows_asset("Game_win64.7z"));
    assert!(is_windows_asset("Game.exe"));
    assert!(is_windows_asset("Game.zip"), "an unlabelled archive is taken to be the Windows build");
    assert!(is_windows_asset("Game.rar"));
    assert!(!is_windows_asset("Game-linux.zip"));
    assert!(!is_windows_asset("Game-macos.zip"));
    assert!(!is_windows_asset("Game-ios.zip"));
    assert!(!is_windows_asset("Game-windows.pdb.zip"), "symbols");
    assert!(!is_windows_asset("Game-xbox.zip"));
    assert!(!is_windows_asset(""));
    assert!(!is_windows_asset("Game.tar.gz"), "a tarball with no label is not Windows");
}

#[test]
fn linux_wants_a_linux_label() {
    assert!(matches_platform("Game-Linux-x64.tar.gz", LINUX));
    assert!(matches_platform("Game-x86_64.AppImage", LINUX));
    assert!(matches_platform("CrashBandicoot_Linux", LINUX), "no architecture is x86-64");
    assert!(matches_platform("Game-linux.tar.xz", LINUX));
    assert!(!matches_platform("Game.zip", LINUX), "a bare zip is the Windows build");
    assert!(!matches_platform("Game-Windows.zip", LINUX));
    assert!(!matches_platform("Game-macos.zip", LINUX));
    assert!(!matches_platform("Game-linux-arm64.tar.gz", LINUX));
    assert!(!matches_platform("Game-linux-i686.tar.gz", LINUX));
    assert!(!matches_platform("Game-linux-aarch64.AppImage", LINUX));
}

#[test]
fn arm_linux_refuses_the_other_architectures() {
    assert!(matches_platform("Game-linux-arm64.tar.gz", ARM));
    assert!(matches_platform("Game-linux.tar.gz", ARM), "no architecture named");
    assert!(!matches_platform("Game-linux-x64.tar.gz", ARM));
    assert!(!matches_platform("Game-linux-amd64.tar.gz", ARM));
}

#[test]
fn windows_wants_a_windows_label_and_nothing_else() {
    assert!(matches_platform("Game-win64.zip", "Windows"));
    assert!(matches_platform("Game.zip", "Windows"));
    assert!(!matches_platform("Game-linux.zip", "Windows"));
    assert!(!matches_platform("Game-switch.zip", "Windows"));
    assert!(!matches_platform("Game-mac.zip", "Windows"));
}

#[test]
fn mac_wants_a_mac_label() {
    assert!(matches_platform("Game-macos.zip", "macOS"));
    assert!(matches_platform("Game-mac.dmg", "macOS"));
    assert!(!matches_platform("Game-windows.zip", "macOS"));
    assert!(!matches_platform("Game.zip", "macOS"));
}

#[test]
fn android_wants_an_apk() {
    assert!(matches_platform("Game.apk", "Android"));
    assert!(matches_platform("Game-android.zip", "Android"));
    assert!(!matches_platform("Game-linux.zip", "Android"));
}

#[test]
fn flatpak_is_linux_only() {
    assert!(matches_platform("Game.flatpak", LINUX));
    assert!(!matches_platform("Game.flatpak", "Windows"));
}

#[test]
fn never_a_phone_a_console_symbols_or_metadata() {
    for name in ["Game-ios.zip", "Game-xbox.zip", "Game-portmaster.zip", "Game-linux.pdb.zip", "Game.json", "Game-linux.tar.gz.sha256"] {
        assert!(!matches_platform(name, LINUX), "{name}");
        assert!(!matches_platform(name, "Windows"), "{name}");
    }
}

#[test]
fn blanks_match_nothing() {
    assert!(!matches_platform("", LINUX));
    assert!(!matches_platform("Game-linux.tar.gz", "  "));
}

#[test]
fn another_platform_name_is_a_substring_match() {
    assert!(matches_platform("Game-Switch-build.zip", "Switch"));
    assert!(!matches_platform("Game-linux.zip", "Switch"));
}
