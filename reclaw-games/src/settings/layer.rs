use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{key::SettingKey, value::SettingValue};

/// One level of user choices: the global defaults, or one game's overrides. A key that is absent
/// means "not set at this level", never "off".
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SettingsLayer(BTreeMap<SettingKey, SettingValue>);

impl SettingsLayer {
    pub fn get(&self, key: SettingKey) -> Option<&SettingValue> {
        self.0.get(&key)
    }

    pub fn set(&mut self, key: SettingKey, value: SettingValue) {
        self.0.insert(key, value);
    }

    /// Forget the choice at this level so the next level (or the game) decides.
    pub fn clear(&mut self, key: SettingKey) -> Option<SettingValue> {
        self.0.remove(&key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (SettingKey, &SettingValue)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
