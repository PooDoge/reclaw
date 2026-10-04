use freya::prelude::*;

use super::game::GamePage;
use crate::{
    desktop::use_desktop_ui,
    nav::{Route, use_nav},
};

/// `/game/:id/install`: the Game page with its install form open over it. Closing the form (cancel,
/// or after starting the install) goes back to the game, so a deep link to the form still has a way out.
#[derive(PartialEq)]
pub struct InstallPage {
    pub id: u32,
}

impl Component for InstallPage {
    fn render(&self) -> impl IntoElement {
        let (ui, nav) = (use_desktop_ui(), use_nav());
        let (id, dialogs) = (self.id, ui.dialogs);
        // Open the form once, as the page arrives.
        use_hook(move || dialogs.install(id));
        use_side_effect(move || {
            if dialogs.installing().is_none() && nav.here() == (Route::Install { id }) {
                nav.back();
            }
        });
        GamePage { id }
    }
}
