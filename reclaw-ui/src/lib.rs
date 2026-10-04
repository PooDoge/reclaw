//! Reclaw UI: Freya components generated from the design system contract (reclaw.freya.json).

pub mod activity;
pub mod app_menu;
pub mod bootstrap;
pub mod catalog;
pub mod components;
pub mod deck;
pub mod desktop;
pub mod effect;

pub mod icon;
pub mod launch;
pub mod metrics;
pub mod model;
pub mod nav;
pub mod notices;
pub mod pages;
pub mod sample;

pub mod settings;
pub mod shell;
pub mod store;
pub mod surface;
pub mod theme;
mod tokens;
pub mod typography;

pub mod prelude {
    pub use crate::components::*;
    pub use crate::icon::{IconName, icon};
    pub use crate::metrics::*;
    pub use crate::model::*;
    pub use crate::theme::{Reclaw, ThemeKind, use_init_reclaw, use_reclaw};
    pub use crate::typography::TypeStyle;
}
