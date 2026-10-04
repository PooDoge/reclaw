//! Building a [`LaunchPlan`] from the effective settings: which outputs run, and what they emit.
use super::{
    capabilities::{Capabilities, ConfigFormat, ConfigPath, Target},
    environment::DisplayEnvironment,
    key::SettingKey,
    plan::{ConfigFileEdit, ConfigValue, KeyEdit, LaunchPlan},
    render::{Context, render, typed},
    resolve::Effective,
    value::{SettingValue, Size},
};

pub(super) fn build_plan(caps: &Capabilities, env: &DisplayEnvironment, effective: &[Effective]) -> LaunchPlan {
    let native = native_size(env, effective);
    let mut plan = LaunchPlan::default();
    for item in effective {
        let (Some(value), Some(binding)) = (&item.value, caps.binding(item.spec.key)) else { continue };
        let size = match value {
            SettingValue::Size(size) => Some(*size),
            SettingValue::Native => native,
            _ => None,
        };
        for output in binding.outputs.iter().filter(|o| o.when.as_ref().is_none_or(|when| when == value)) {
            emit(&mut plan, &output.target, &Context { value, size, map: &output.map });
        }
    }
    plan
}

/// The size "native" means: the monitor chosen by the effective Monitor setting, else the primary one.
fn native_size(env: &DisplayEnvironment, effective: &[Effective]) -> Option<Size> {
    let chosen = effective.iter().find(|e| e.spec.key == SettingKey::Monitor).and_then(|e| match &e.value {
        Some(SettingValue::Choice(id)) => env.monitors.iter().find(|m| &m.id == id),
        _ => None,
    });
    chosen.or_else(|| env.primary()).map(|m| m.native)
}

/// Adds what one output produces, or nothing at all if any part of it cannot be built.
fn emit(plan: &mut LaunchPlan, target: &Target, ctx: &Context) {
    match target {
        Target::Args { args } => {
            if let Some(rendered) = args.iter().map(|arg| render(arg, ctx)).collect::<Option<Vec<_>>>() {
                plan.args.extend(rendered);
            }
        }
        Target::Env { name, value } => {
            if let (false, Some(value)) = (name.is_empty(), render(value, ctx)) {
                set_env(plan, name, value);
            }
        }
        Target::Config { file, format, path, value, value_type } => {
            if let Some(value) = render(value, ctx).and_then(|text| typed(&text, *value_type)) {
                set_edit(plan, file, *format, path, value);
            }
        }
    }
}

/// A variable is set once: a later setting replaces the value but not the position.
fn set_env(plan: &mut LaunchPlan, name: &str, value: String) {
    match plan.env.iter_mut().find(|(n, _)| n == name) {
        Some(entry) => entry.1 = value,
        None => plan.env.push((name.to_string(), value)),
    }
}

/// Edits are grouped per file in first-seen order; a later edit to the same path wins.
fn set_edit(plan: &mut LaunchPlan, file: &ConfigPath, format: ConfigFormat, path: &str, value: ConfigValue) {
    let index = match plan.config_edits.iter().position(|c| &c.file == file && c.format == format) {
        Some(index) => index,
        None => {
            plan.config_edits.push(ConfigFileEdit { file: file.clone(), format, edits: Vec::new() });
            plan.config_edits.len() - 1
        }
    };
    let edits = &mut plan.config_edits[index].edits;
    match edits.iter_mut().find(|e| e.path == path) {
        Some(edit) => edit.value = value,
        None => edits.push(KeyEdit { path: path.to_string(), value }),
    }
}

#[cfg(test)]
mod tests;
