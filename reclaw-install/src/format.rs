//! What kind of file a release asset is, and so what installing it means.
use std::path::Path;

/// How a downloaded file becomes an installed app.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Zip,
    TarGz,
    TarXz,
    SevenZip,
    Rar,
    /// A program in one file: copied into place and made executable.
    AppImage,
    /// A Windows program in one file.
    Exe,
    /// A single file with no extension (`CrashBandicoot_Linux`): a program.
    Bare,
    /// Something this installer does not handle (a package for a system installer, a phone app), by name.
    Unsupported(&'static str),
}

impl Format {
    /// What a file name says it is.
    pub fn of_name(name: &str) -> Self {
        let lower = name.trim().to_ascii_lowercase();
        let ends = |s: &str| lower.ends_with(s);
        if ends(".zip") {
            Self::Zip
        } else if ends(".tar.gz") || ends(".tgz") {
            Self::TarGz
        } else if ends(".tar.xz") || ends(".txz") {
            Self::TarXz
        } else if ends(".7z") {
            Self::SevenZip
        } else if ends(".rar") {
            Self::Rar
        } else if ends(".appimage") {
            Self::AppImage
        } else if ends(".exe") {
            Self::Exe
        } else if ends(".flatpak") {
            Self::Unsupported("a Flatpak bundle")
        } else if ends(".deb") || ends(".rpm") {
            Self::Unsupported("a system package")
        } else if ends(".apk") {
            Self::Unsupported("an Android app")
        } else if ends(".dmg") || ends(".pkg") {
            Self::Unsupported("a macOS installer")
        } else if ends(".msi") {
            Self::Unsupported("a Windows installer")
        } else if Path::new(&lower).extension().is_none() {
            Self::Bare
        } else {
            Self::Unsupported("a file of a kind Reclaw does not install")
        }
    }

    /// What a file is from its first bytes, for a download whose name says nothing (a GitLab package link).
    pub fn sniff(head: &[u8]) -> Option<Self> {
        match head {
            [b'P', b'K', ..] => Some(Self::Zip),
            [0x37, 0x7a, 0xbc, 0xaf, ..] => Some(Self::SevenZip),
            [b'R', b'a', b'r', b'!', ..] => Some(Self::Rar),
            [0x1f, 0x8b, ..] => Some(Self::TarGz),
            [0xfd, b'7', b'z', b'X', b'Z', 0x00, ..] => Some(Self::TarXz),
            _ => None,
        }
    }

    pub fn is_installable(self) -> bool {
        !matches!(self, Self::Unsupported(_))
    }

    pub fn is_archive(self) -> bool {
        matches!(self, Self::Zip | Self::TarGz | Self::TarXz | Self::SevenZip | Self::Rar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_say_what_a_file_is() {
        assert_eq!(Format::of_name("Game-Linux.tar.gz"), Format::TarGz);
        assert_eq!(Format::of_name("game.TGZ"), Format::TarGz);
        assert_eq!(Format::of_name("game.tar.xz"), Format::TarXz);
        assert_eq!(Format::of_name("Game-x86_64.AppImage"), Format::AppImage);
        assert_eq!(Format::of_name("Game.7z"), Format::SevenZip);
        assert_eq!(Format::of_name("Game_Windows.exe"), Format::Exe);
        assert_eq!(Format::of_name("CrashBandicoot_Linux"), Format::Bare);
        assert_eq!(Format::of_name("game.flatpak"), Format::Unsupported("a Flatpak bundle"));
        assert!(!Format::of_name("game.deb").is_installable());
        assert!(!Format::of_name("game.apk").is_installable());
        assert!(!Format::of_name("game.dmg").is_installable());
        assert!(!Format::of_name("setup.msi").is_installable());
        assert!(!Format::of_name("notes.pdf").is_installable());
    }

    #[test]
    fn the_first_bytes_say_it_when_the_name_does_not() {
        assert_eq!(Format::sniff(b"PK\x03\x04rest"), Some(Format::Zip));
        assert_eq!(Format::sniff(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]), Some(Format::SevenZip));
        assert_eq!(Format::sniff(b"Rar!\x1a\x07\x00"), Some(Format::Rar));
        assert_eq!(Format::sniff(&[0x1f, 0x8b, 8, 0]), Some(Format::TarGz));
        assert_eq!(Format::sniff(&[0xfd, b'7', b'z', b'X', b'Z', 0]), Some(Format::TarXz));
        assert_eq!(Format::sniff(b"\x7fELF"), None, "a program is not an archive");
        assert_eq!(Format::sniff(b""), None);
    }

    #[test]
    fn which_formats_are_archives() {
        assert!(Format::Zip.is_archive() && Format::Rar.is_archive());
        assert!(!Format::AppImage.is_archive() && !Format::Bare.is_archive() && !Format::Exe.is_archive());
    }
}
