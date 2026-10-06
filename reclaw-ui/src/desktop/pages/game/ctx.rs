use freya::prelude::*;

use crate::{
    activity::FailureHint,
    catalog::GameView,
    desktop::{DesktopEnv, GameDialogs},
    effect::Effect,
    nav::Nav,
    prelude::*,
};

/// What the sections of the Game page share. Built once in `GamePage::render`: the sections are
/// plain functions, so they never call hooks (a hook under a condition panics on the next render).
pub(super) struct Ctx {
    pub t: Reclaw,
    pub env: DesktopEnv,
    pub view: GameView,
    pub nav: Nav,
    pub dialogs: GameDialogs,
    pub on_effect: EventHandler<Effect>,
    /// What the Failed badge says and opens, when the game is shown as failed.
    pub failure: Option<FailureHint>,
}

impl Ctx {
    /// A handler that opens a link in the system browser or player.
    pub fn open_url(&self) -> EventHandler<String> {
        let on_effect = self.on_effect.clone();
        EventHandler::new(move |url: String| on_effect.call(Effect::OpenUrl(url)))
    }
}
