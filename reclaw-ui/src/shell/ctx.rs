use freya::prelude::*;
use reclaw_input::{Action, ActionMap};

use super::{model::ShellModel, overrides::DevOverrides};
use crate::{deck::ActionFeed, effect::Effect, nav::transition::TransitionConfig, store::Store};

/// What the shell hands to everything under the router. Pages and frames read it from context
/// instead of taking it as props, because a route's component is built by the router from its path
/// alone and cannot be given anything else.
#[derive(Clone)]
pub struct ShellCtx {
    pub store: Store,
    pub feed: ActionFeed,
    pub map: ActionMap,
    /// Commands for the host. The shell has already handled the ones that are its own.
    pub on_effect: EventHandler<Effect>,
    pub dev: DevOverrides,
    pub host_window: crate::window::WindowHost,
    /// Actions applied once when Deck mode opens (gallery and snapshot scenarios).
    pub script: Vec<Action>,
    pub model: State<ShellModel>,
    pub transitions: State<TransitionConfig>,
    /// The window's size in logical px, measured once at the root.
    pub window: State<(f32, f32)>,
}

#[track_caller]
pub fn use_shell() -> ShellCtx {
    use_consume::<ShellCtx>()
}

/// The shared state's handle, for dispatching from handlers.
#[track_caller]
pub fn use_store() -> Store {
    use_shell().store
}
