//! Values that must never reach a log: the [`Secret`] wrapper, and the list of exact values the scrubber removes from every
//! line whatever shape they have.
use std::{
    fmt,
    sync::{PoisonError, RwLock},
};

/// What replaces a secret in text.
pub const REDACTED: &str = "‹redacted›";

/// Shorter values are not registered: replacing every "abc" in every line would hide more than it protects.
const MIN_LEN: usize = 8;

static REGISTERED: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// A credential. Printing it, in `{:?}` or `{}`, shows no part of it; the only way to the text is [`expose`](Self::expose),
/// which makes every use of the real value visible in review.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret({REDACTED})")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// From now on `value` is removed from everything the logger writes. Call it wherever a credential enters the program (read from
/// disk, typed, taken from the environment): the pattern rules in [`scrub`](crate::scrub) know the common token shapes, this one
/// covers a token of any shape, a company's own, say.
pub fn register_secret(value: &str) {
    let value = value.trim();
    if value.len() < MIN_LEN {
        return;
    }
    let mut list = REGISTERED.write().unwrap_or_else(PoisonError::into_inner);
    if !list.iter().any(|v| v == value) {
        list.push(value.to_string());
    }
}

/// The opposite, for a credential that was removed or replaced.
pub fn forget_secret(value: &str) {
    let value = value.trim();
    REGISTERED.write().unwrap_or_else(PoisonError::into_inner).retain(|v| v != value);
}

/// Every registered value, longest first, so a secret that contains another is replaced whole.
pub(crate) fn registered() -> Vec<String> {
    let mut list = REGISTERED.read().unwrap_or_else(PoisonError::into_inner).clone();
    list.sort_by_key(|v| std::cmp::Reverse(v.len()));
    list
}

#[cfg(test)]
pub(crate) fn clear_registered() {
    REGISTERED.write().unwrap_or_else(PoisonError::into_inner).clear();
}
