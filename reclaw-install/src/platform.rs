//! Which build of a game this machine wants. The names are Quiver's (`Linux-X64`, `Windows`...), because the catalog's asset
//! filters and the release assets are written against them.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Platform {
    Windows,
    MacOs,
    LinuxX64,
    LinuxArm64,
    LinuxX86,
    LinuxArm,
    Android,
}

impl Platform {
    /// The platform this program was built for. Unknown Linux architectures are asked for x86-64 builds, as Quiver does.
    pub fn detect() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "android") {
            Self::Android
        } else if cfg!(target_arch = "aarch64") {
            Self::LinuxArm64
        } else if cfg!(target_arch = "x86") {
            Self::LinuxX86
        } else if cfg!(target_arch = "arm") {
            Self::LinuxArm
        } else {
            Self::LinuxX64
        }
    }

    pub fn identifier(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::LinuxX64 => "Linux-X64",
            Self::LinuxArm64 => "Linux-ARM64",
            Self::LinuxX86 => "Linux-X86",
            Self::LinuxArm => "Linux-ARM",
            Self::Android => "Android",
        }
    }

    pub fn is_linux(self) -> bool {
        matches!(self, Self::LinuxX64 | Self::LinuxArm64 | Self::LinuxX86 | Self::LinuxArm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_quivers() {
        assert_eq!(Platform::LinuxX64.identifier(), "Linux-X64");
        assert_eq!(Platform::LinuxArm64.identifier(), "Linux-ARM64");
        assert_eq!(Platform::MacOs.identifier(), "macOS");
        assert_eq!(Platform::Windows.identifier(), "Windows");
    }

    #[test]
    fn only_the_linux_ones_are_linux() {
        assert!(Platform::LinuxX86.is_linux() && Platform::LinuxArm.is_linux());
        assert!(!Platform::Windows.is_linux() && !Platform::Android.is_linux() && !Platform::MacOs.is_linux());
    }

    #[test]
    fn this_machine_is_something() {
        // Whatever the build host is, detection answers with an identifier the matcher knows.
        let id = Platform::detect().identifier().to_ascii_lowercase();
        assert!(["windows", "macos", "linux", "android"].iter().any(|p| id.contains(p)), "{id}");
    }
}
