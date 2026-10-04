//! Builders and a throwaway directory shared by the unit tests of this module.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use super::{
    capabilities::{Base, Binding, Capabilities, ConfigFormat, ConfigPath, Constraint, Output, Target, ValueType},
    environment::{DisplayEnvironment, DisplayServer, Monitor},
    key::SettingKey,
    value::{SettingValue, Size},
};

pub fn monitor(id: &str, name: &str, width: u32, height: u32, primary: bool) -> Monitor {
    Monitor { id: id.into(), name: name.into(), native: Size::new(width, height), refresh_mhz: 60_000, primary }
}

pub fn env(server: DisplayServer, monitors: Vec<Monitor>) -> DisplayEnvironment {
    DisplayEnvironment { server, monitors }
}

/// One 2560x1440 screen on X11: a typical desktop.
pub fn desktop() -> DisplayEnvironment {
    env(DisplayServer::X11, vec![monitor("DP-1", "Main", 2560, 1440, true)])
}

pub fn args(when: Option<SettingValue>, args: &[&str]) -> Output {
    Output { when, map: BTreeMap::new(), target: Target::Args { args: args.iter().map(|a| a.to_string()).collect() } }
}

pub fn env_var(name: &str, value: &str) -> Output {
    Output { when: None, map: BTreeMap::new(), target: Target::Env { name: name.into(), value: value.into() } }
}

pub fn config(file: &str, format: ConfigFormat, path: &str, value: &str, value_type: ValueType) -> Output {
    let file = ConfigPath { base: Base::Config, relative: file.into() };
    Output { when: None, map: BTreeMap::new(), target: Target::Config { file, format, path: path.into(), value: value.into(), value_type } }
}

pub fn with_map(mut output: Output, pairs: &[(&str, &str)]) -> Output {
    output.map = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    output
}

pub fn bind(key: SettingKey, constraint: Constraint, outputs: Vec<Output>) -> Binding {
    Binding { key, constraint, outputs }
}

/// Every key bound with no constraint and one `--<id>={value}` argument, to test display rules alone.
pub fn everything() -> Capabilities {
    let settings =
        SettingKey::ALL.into_iter().map(|key| bind(key, Constraint::None, vec![args(None, &[&format!("--{}={{value}}", key.id())])]));
    Capabilities { settings: settings.collect() }
}

/// A directory that exists for the life of the value. std only: no extra dependency.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!("reclaw-games-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).expect("create the test directory");
        Self(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: a leftover directory in the system temp dir is harmless.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
