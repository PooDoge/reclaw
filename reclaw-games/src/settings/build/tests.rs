use super::*;
use crate::settings::{
    capabilities::{Binding, Capabilities, ConfigFormat, Constraint, ValueType},
    environment::DisplayServer,
    layer::SettingsLayer,
    plan::{ConfigValue, LaunchPlan},
    resolve::plan,
    testing::{args, bind, config, desktop, env, env_var, everything, monitor, with_map},
};

fn caps(bindings: Vec<Binding>) -> Capabilities {
    Capabilities { settings: bindings }
}

fn layer(items: &[(SettingKey, SettingValue)]) -> SettingsLayer {
    let mut layer = SettingsLayer::default();
    for (key, value) in items {
        layer.set(*key, value.clone());
    }
    layer
}

fn plan_of(caps: &Capabilities, env: &DisplayEnvironment, defaults: &[(SettingKey, SettingValue)]) -> LaunchPlan {
    plan(caps, env, &layer(defaults), &SettingsLayer::default())
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn size(w: u32, h: u32) -> SettingValue {
    SettingValue::Size(Size::new(w, h))
}

fn resolution_args() -> Capabilities {
    caps(vec![bind(SettingKey::Resolution, Constraint::None, vec![args(None, &["--width", "{width}", "--height", "{height}"])])])
}

fn two_monitors(server: DisplayServer) -> DisplayEnvironment {
    env(server, vec![monitor("DP-1", "Left", 2560, 1440, true), monitor("HDMI-1", "TV", 3840, 2160, false)])
}

fn edit(path: &str, value: ConfigValue) -> KeyEdit {
    KeyEdit { path: path.to_string(), value }
}

mod args;
mod config;
mod native;
