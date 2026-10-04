use std::collections::BTreeMap;

use crate::state::LaunchSpec;

/// Controller setup for one launched app.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct InputProfile {
    /// A full SDL mapping line, passed as `SDL_GAMECONTROLLERCONFIG`.
    pub sdl_controller_config: Option<String>,
    /// Sets `SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1`, so an SDL app keeps getting pad events when
    /// its window is not focused (a launcher or overlay window may hold focus).
    pub allow_background_events: bool,
    /// Anything else the app wants.
    pub env: Vec<(String, String)>,
}

impl InputProfile {
    pub fn apply(&self, spec: &mut LaunchSpec) {
        if let Some(config) = &self.sdl_controller_config {
            spec.env.push(("SDL_GAMECONTROLLERCONFIG".into(), config.into()));
        }
        if self.allow_background_events {
            spec.env.push(("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS".into(), "1".into()));
        }
        for (k, v) in &self.env {
            spec.env.push((k.into(), v.into()));
        }
    }
}

/// One SDL game-controller mapping line:
/// `<guid>,<name>,a:b0,b:b1,...,platform:Linux`.
///
/// Reclaw can take the line SDL already has for a device and rewrite it (swap A and B for the
/// Nintendo layout, say), then hand the result to apps through [`InputProfile`].
#[derive(Clone, PartialEq, Debug)]
pub struct SdlMapping {
    pub guid: String,
    pub name: String,
    /// logical SDL name -> physical source (`b0`, `a1`, `h0.1`...). Sorted for stable output.
    pub bindings: BTreeMap<String, String>,
    pub platform: Option<String>,
}

impl SdlMapping {
    pub fn parse(line: &str) -> Option<Self> {
        let mut parts = line.trim().split(',');
        let guid = parts.next()?.trim().to_string();
        let name = parts.next()?.trim().to_string();
        if guid.is_empty() {
            return None;
        }
        let mut bindings = BTreeMap::new();
        let mut platform = None;
        for part in parts.filter(|p| !p.trim().is_empty()) {
            let (key, value) = part.trim().split_once(':')?;
            if key == "platform" {
                platform = Some(value.to_string());
            } else {
                bindings.insert(key.to_string(), value.to_string());
            }
        }
        Some(Self { guid, name, bindings, platform })
    }

    /// Exchange what two logical outputs read from, e.g. `swap("a", "b")`. Returns `false` if
    /// either output is not bound.
    pub fn swap(&mut self, a: &str, b: &str) -> bool {
        let (Some(va), Some(vb)) = (self.bindings.get(a).cloned(), self.bindings.get(b).cloned()) else {
            return false;
        };
        self.bindings.insert(a.to_string(), vb);
        self.bindings.insert(b.to_string(), va);
        true
    }

    pub fn to_line(&self) -> String {
        let mut out = format!("{},{}", self.guid, self.name);
        for (k, v) in &self.bindings {
            out.push_str(&format!(",{k}:{v}"));
        }
        if let Some(p) = &self.platform {
            out.push_str(&format!(",platform:{p}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "030000005e040000ff02000000007200,Xbox Controller,a:b0,b:b1,x:b2,y:b3,leftx:a0,dpup:h0.1,platform:Linux";

    #[test]
    fn parse_round_trips() {
        let m = SdlMapping::parse(LINE).unwrap();
        assert_eq!(m.guid, "030000005e040000ff02000000007200");
        assert_eq!(m.bindings["dpup"], "h0.1");
        assert_eq!(m.platform.as_deref(), Some("Linux"));
        // Output is key-sorted, so compare parsed forms.
        assert_eq!(SdlMapping::parse(&m.to_line()).unwrap(), m);
    }

    #[test]
    fn swap_a_and_b() {
        let mut m = SdlMapping::parse(LINE).unwrap();
        assert!(m.swap("a", "b"));
        assert_eq!((m.bindings["a"].as_str(), m.bindings["b"].as_str()), ("b1", "b0"));
        assert!(!m.swap("a", "start"), "unbound output is reported, not invented");
    }

    #[test]
    fn rejects_garbage() {
        assert!(SdlMapping::parse("").is_none());
        assert!(SdlMapping::parse("guid,name,not-a-binding").is_none());
    }

    #[test]
    fn profile_sets_the_sdl_variables() {
        let mut spec = LaunchSpec::new("/bin/true");
        InputProfile { sdl_controller_config: Some("g,n,a:b0".into()), allow_background_events: true, env: vec![("X".into(), "1".into())] }
            .apply(&mut spec);
        let keys: Vec<_> = spec.env.iter().map(|(k, _)| k.to_string_lossy().into_owned()).collect();
        assert_eq!(keys, ["SDL_GAMECONTROLLERCONFIG", "SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "X"]);
    }
}
