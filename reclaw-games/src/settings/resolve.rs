use super::{
    adapt::adapt,
    build::build_plan,
    capabilities::Capabilities,
    environment::DisplayEnvironment,
    key::SettingKey,
    kinds::standard_kind,
    layer::SettingsLayer,
    narrow::narrow,
    plan::LaunchPlan,
    value::{SettingValue, ValueKind},
};

/// A setting as offered for one game on one display.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SettingSpec {
    pub key: SettingKey,
    pub kind: ValueKind,
}

/// Which level a shown value came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// The user set it for this game.
    GameOverride,
    /// The user's global default applies.
    UserDefault,
    /// Nothing is set: the game uses its own value and the launcher passes nothing.
    GameDefault,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Effective {
    pub spec: SettingSpec,
    /// `None` when `source` is `GameDefault`.
    pub value: Option<SettingValue>,
    pub source: Source,
}

/// Every setting the display can honor, with its standard range: what the global defaults page lists.
pub fn all_specs(env: &DisplayEnvironment) -> Vec<SettingSpec> {
    SettingKey::ALL.into_iter().filter_map(|key| Some(SettingSpec { key, kind: standard_kind(key, env)? })).collect()
}

/// The settings this game declares and this display can honor, narrowed by the game's constraints,
/// ordered by group then key. A game with no capabilities yields an empty list.
pub fn supported(caps: &Capabilities, env: &DisplayEnvironment) -> Vec<SettingSpec> {
    // `SettingKey::ALL` is already ordered by group, then by key.
    SettingKey::ALL
        .into_iter()
        .filter_map(|key| {
            let binding = caps.binding(key)?;
            let kind = narrow(standard_kind(key, env)?, &binding.constraint, env)?;
            Some(SettingSpec { key, kind })
        })
        .collect()
}

/// Merge the two layers for one game: override, then default, then the game's own.
pub fn effective(caps: &Capabilities, env: &DisplayEnvironment, defaults: &SettingsLayer, overrides: &SettingsLayer) -> Vec<Effective> {
    supported(caps, env)
        .into_iter()
        .map(|spec| {
            let fitted = |layer: &SettingsLayer| layer.get(spec.key).and_then(|v| adapt(&spec, v));
            let (value, source) = match (fitted(overrides), fitted(defaults)) {
                (Some(v), _) => (Some(v), Source::GameOverride),
                (None, Some(v)) => (Some(v), Source::UserDefault),
                (None, None) => (None, Source::GameDefault),
            };
            Effective { spec, value, source }
        })
        .collect()
}

/// The launch plan for the effective settings. Settings in `GameDefault` produce nothing.
pub fn plan(caps: &Capabilities, env: &DisplayEnvironment, defaults: &SettingsLayer, overrides: &SettingsLayer) -> LaunchPlan {
    build_plan(caps, env, &effective(caps, env, defaults, overrides))
}

#[cfg(test)]
mod tests;
