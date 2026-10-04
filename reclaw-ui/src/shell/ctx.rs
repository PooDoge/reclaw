use freya::prelude::*;
use reclaw_input::{Action, ActionMap};

use super::{model::ShellModel, overrides::DevOverrides};
use crate::{deck::ActionFeed, effect::Effect, host::HostState, nav::transition::TransitionConfig};

/// What the shell hands to everything under the router. Pages and frames read it from context
/// instead of taking it as props, because a route's component is built by the router from its path
/// alone and cannot be given anything else.
#[derive(Clone)]
pub struct ShellCtx {
    pub host: HostState,
    pub feed: ActionFeed,
    pub map: ActionMap,
    /// Commands for the host. The shell has already handled the ones that are its own.
    pub on_effect: EventHandler<Effect>,
    pub dev: DevOverrides,
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
