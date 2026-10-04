//! Turning one chosen value into text: template placeholders and typed config values.
use std::collections::BTreeMap;

use super::{
    capabilities::ValueType,
    plan::ConfigValue,
    value::{SettingValue, Size},
};

/// What a template may refer to for one chosen value.
pub(super) struct Context<'a> {
    pub value: &'a SettingValue,
    /// Behind `{width}` and `{height}`: the size itself, or the monitor's size for `Native`.
    pub size: Option<Size>,
    /// The output's `map`, behind `{mapped}`.
    pub map: &'a BTreeMap<String, String>,
}

/// The value as the text `{value}` stands for, and the key that `map` is searched with.
pub(super) fn value_text(value: &SettingValue) -> String {
    match value {
        SettingValue::Bool(b) => b.to_string(),
        SettingValue::Int(n) => n.to_string(),
        SettingValue::Choice(id) => id.clone(),
        SettingValue::Size(size) => size.label(),
        SettingValue::Native => "native".to_string(),
    }
}

const PLACEHOLDERS: [&str; 4] = ["value", "width", "height", "mapped"];

impl Context<'_> {
    fn lookup(&self, name: &str) -> Option<String> {
        match name {
            "value" => Some(value_text(self.value)),
            "width" => self.size.map(|s| s.width.to_string()),
            "height" => self.size.map(|s| s.height.to_string()),
            "mapped" => self.map.get(&value_text(self.value)).cloned(),
            _ => None,
        }
    }
}

/// `template` with `{value}`, `{width}`, `{height}` and `{mapped}` replaced, or `None` when one of
/// them has no data. Anything else in braces is left alone, and replaced text is never scanned again.
pub(super) fn render(template: &str, ctx: &Context) -> Option<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let known = PLACEHOLDERS.iter().find(|name| tail[1..].strip_prefix(**name).is_some_and(|after| after.starts_with('}')));
        match known {
            Some(name) => {
                out.push_str(&ctx.lookup(name)?);
                rest = &tail[name.len() + 2..];
            }
            None => {
                out.push('{');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    Some(out)
}

/// The rendered text as the config file should hold it, or `None` when it is not what `value_type`
/// says (an `Int` file cannot be given `fsr1`).
pub(super) fn typed(text: &str, value_type: ValueType) -> Option<ConfigValue> {
    match value_type {
        ValueType::String => Some(ConfigValue::Text(text.to_string())),
        ValueType::Int => text.trim().parse().ok().map(ConfigValue::Int),
        ValueType::Bool => match text.trim() {
            "true" => Some(ConfigValue::Bool(true)),
            "false" => Some(ConfigValue::Bool(false)),
            _ => None,
        },
        ValueType::Float => text.trim().parse::<f64>().ok().filter(|f| f.is_finite()).map(ConfigValue::Float),
        ValueType::Auto => Some(match text {
            "true" => ConfigValue::Bool(true),
            "false" => ConfigValue::Bool(false),
            // Only the plain spelling: "007" and "+5" are names, and turning them into numbers would change them.
            _ => match text.parse::<i64>() {
                Ok(n) if n.to_string() == text => ConfigValue::Int(n),
                _ => ConfigValue::Text(text.to_string()),
            },
        }),
    }
}

#[cfg(test)]
mod tests;
