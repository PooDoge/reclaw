//! Whether a release file is a build for this machine. A port of Quiver's `PlatformAssetMatcher`: the same markers in the same
//! order, because the catalog's authors tested their releases against it and Reclaw should pick what Quiver would.
use crate::{names, platform::Platform};

fn has_any(name: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| name.contains(n))
}

fn is_flatpak(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".flatpak")
}

fn has_mac_marker(name: &str) -> bool {
    has_any(name, &["macos", "osx", "darwin", ".dmg", ".pkg", "apple"]) || names::has_mac_word(name)
}

fn has_non_windows_marker(name: &str) -> bool {
    has_any(
        name,
        &[
            "linux",
            "macos",
            "osx",
            "darwin",
            "apple",
            ".deb",
            ".rpm",
            "appimage",
            "flatpak",
            ".dmg",
            ".pkg",
            "android",
            "arm64-v8a",
            ".apk",
            "switch",
        ],
    ) || has_mac_marker(name)
}

fn has_explicit_windows_marker(name: &str) -> bool {
    has_any(name, &["windows", "win64", "win32", "win-x64", "win-x86", "-win.", "_win.", ".exe", ".msi", "msvc", "mingw"])
        || names::has_win_token(name)
}

/// A bare archive is taken to be the Windows build: that is how most releases that ship one build are made.
fn is_unlabeled_windows_archive(name: &str) -> bool {
    [".zip", ".7z", ".rar"].iter().any(|e| name.ends_with(e))
}

pub fn is_ios_asset(name: &str) -> bool {
    names::is_ios(&name.to_ascii_lowercase())
}

pub fn is_dedicated_device_asset(name: &str) -> bool {
    names::is_dedicated_device(&name.to_ascii_lowercase())
}

/// A Windows build: marked as one, or a bare archive, and not marked as anything else.
pub fn is_windows_asset(name: &str) -> bool {
    if name.trim().is_empty() {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    if names::is_ios(&lower) || names::is_dedicated_device(&lower) || names::is_auxiliary(&lower) || names::is_debug_symbols(&lower) {
        return false;
    }
    if has_non_windows_marker(&lower) {
        return false;
    }
    has_explicit_windows_marker(&lower) || is_unlabeled_windows_archive(&lower)
}

/// Whether `asset` is a build for `platform` (Quiver's identifier: `Linux-X64`, `Windows`, `macOS`...).
pub fn matches_platform(asset: &str, platform: &str) -> bool {
    if asset.trim().is_empty() || platform.trim().is_empty() {
        return false;
    }
    let name = asset.to_ascii_lowercase();
    let platform = platform.to_ascii_lowercase();
    if is_flatpak(asset) && !platform.starts_with("linux") {
        return false;
    }
    if names::is_ios(&name) || names::is_dedicated_device(&name) || names::is_auxiliary(&name) || names::is_debug_symbols(&name) {
        return false;
    }

    if platform.contains("windows") {
        return !has_non_windows_marker(&name) && (has_explicit_windows_marker(&name) || is_unlabeled_windows_archive(&name));
    }
    if platform.contains("mac") {
        if has_any(&name, &["linux", "windows", "win32", "win64", ".exe", ".msi", "switch", "android", ".apk"]) {
            return false;
        }
        return has_mac_marker(&name);
    }
    if platform.contains("linux") {
        if is_windows_asset(&name)
            || has_mac_marker(&name)
            || has_any(&name, &["windows", "win32", "win64", "macos", "osx", "darwin", ".exe", ".msi", ".dmg", "switch", "android", ".apk"])
        {
            return false;
        }
        if !has_any(&name, &["linux", "appimage", ".flatpak", ".deb", ".rpm", "tar.gz", "tar.xz"]) {
            return false;
        }
        if platform.contains("arm") || platform.contains("aarch64") {
            return !has_any(&name, &["x64", "x86", "amd64", "i686", "i386", "i586", "armv7", "armhf", "arm-"]);
        }
        // x86-64, or a build that does not say (`CrashBandicoot_Linux`).
        return !has_any(&name, &["i686", "i386", "i586", "x86-linux", "-i686-", "arm64", "aarch64", "armv7", "armhf", "arm-"]);
    }
    if platform.contains("android") {
        if has_mac_marker(&name)
            || has_any(
                &name,
                &[
                    "windows", "win32", "win64", "linux", "macos", "osx", "darwin", ".exe", ".msi", "appimage", ".dmg", ".deb", ".rpm",
                    "switch",
                ],
            )
        {
            return false;
        }
        return has_any(&name, &["android", "arm64-v8a"]) || name.ends_with(".apk");
    }
    name.contains(&platform)
}

impl Platform {
    /// Whether `asset` is a build made for this platform.
    pub fn accepts(self, asset: &str) -> bool {
        matches_platform(asset, self.identifier())
    }
}
