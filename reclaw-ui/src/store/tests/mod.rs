//! Tests for the reducer and for what is saved.
mod persistence;
mod reduce;

use super::*;

pub(super) fn state() -> AppState {
    AppState::sample()
}
