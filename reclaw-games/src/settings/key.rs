use serde::{Deserialize, Serialize};

/// Where a setting sits on the settings page.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Group {
    Display,
    Upscaling,
    Graphics,
}

impl Group {
    pub const ALL: [Group; 3] = [Self::Display, Self::Upscaling, Self::Graphics];

    pub fn label(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Upscaling => "Upscaling",
            Self::Graphics => "Graphics",
        }
    }
}

/// The launcher's fixed catalog of launch settings. Adding one is a code change on purpose: each
/// key needs a label, a value shape and an answer for every display environment.
///
/// The serialized form is [`id`](Self::id) (`display.window_mode`), which is also what a catalog
/// file writes, so renaming a variant never breaks stored settings; changing an id does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum SettingKey {
    WindowMode,
    Resolution,
    AspectRatio,
    Monitor,
    Vsync,
    FrameLimit,
    UpscaleMethod,
    RenderScale,
    Sharpness,
    Msaa,
    TextureFilter,
}

impl SettingKey {
    pub const ALL: [SettingKey; 11] = [
        Self::WindowMode,
        Self::Resolution,
        Self::AspectRatio,
        Self::Monitor,
        Self::Vsync,
        Self::FrameLimit,
        Self::UpscaleMethod,
        Self::RenderScale,
        Self::Sharpness,
        Self::Msaa,
        Self::TextureFilter,
    ];

    /// Stable identifier used in files and in `Capabilities`.
    pub fn id(self) -> &'static str {
        match self {
            Self::WindowMode => "display.window_mode",
            Self::Resolution => "display.resolution",
            Self::AspectRatio => "display.aspect_ratio",
            Self::Monitor => "display.monitor",
            Self::Vsync => "display.vsync",
            Self::FrameLimit => "display.frame_limit",
            Self::UpscaleMethod => "upscale.method",
            Self::RenderScale => "upscale.render_scale",
            Self::Sharpness => "upscale.sharpness",
            Self::Msaa => "graphics.msaa",
            Self::TextureFilter => "graphics.texture_filter",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.id() == id)
    }

    pub fn group(self) -> Group {
        match self {
            Self::WindowMode | Self::Resolution | Self::AspectRatio | Self::Monitor | Self::Vsync | Self::FrameLimit => Group::Display,
            Self::UpscaleMethod | Self::RenderScale | Self::Sharpness => Group::Upscaling,
            Self::Msaa | Self::TextureFilter => Group::Graphics,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::WindowMode => "Window mode",
            Self::Resolution => "Resolution",
            Self::AspectRatio => "Aspect ratio",
            Self::Monitor => "Monitor",
            Self::Vsync => "Vertical sync",
            Self::FrameLimit => "Frame rate limit",
            Self::UpscaleMethod => "Upscaling",
            Self::RenderScale => "Render scale",
            Self::Sharpness => "Sharpness",
            Self::Msaa => "Anti-aliasing",
            Self::TextureFilter => "Texture filtering",
        }
    }

    /// One short line for the row under the label.
    pub fn description(self) -> &'static str {
        match self {
            Self::WindowMode => "Windowed, borderless or exclusive fullscreen.",
            Self::Resolution => "Size the game renders at.",
            Self::AspectRatio => "Shape of the picture.",
            Self::Monitor => "Which screen the game opens on.",
            Self::Vsync => "Match the screen refresh to avoid tearing.",
            Self::FrameLimit => "Cap the frame rate; 0 means no cap.",
            Self::UpscaleMethod => "How a lower render size is scaled up.",
            Self::RenderScale => "Render below or above the output size.",
            Self::Sharpness => "Strength of the sharpening pass.",
            Self::Msaa => "Smooths jagged edges.",
            Self::TextureFilter => "How textures are sampled.",
        }
    }
}

impl Serialize for SettingKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for SettingKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Self::from_id(&id).ok_or_else(|| serde::de::Error::custom(format!("unknown setting key {id:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_has_a_unique_round_tripping_id() {
        let mut seen = std::collections::HashSet::new();
        for key in SettingKey::ALL {
            assert!(seen.insert(key.id()), "duplicate id {}", key.id());
            assert_eq!(SettingKey::from_id(key.id()), Some(key));
        }
        assert_eq!(SettingKey::from_id("display.nope"), None);
    }

    #[test]
    fn serde_uses_the_id() {
        let json = serde_json::to_string(&SettingKey::Vsync).expect("serialize");
        assert_eq!(json, "\"display.vsync\"");
        assert_eq!(serde_json::from_str::<SettingKey>(&json).expect("parse"), SettingKey::Vsync);
        assert!(serde_json::from_str::<SettingKey>("\"x.y\"").is_err());
    }

    #[test]
    fn every_group_has_a_key() {
        for group in Group::ALL {
            assert!(SettingKey::ALL.iter().any(|k| k.group() == group), "{group:?} is empty");
        }
    }
}
