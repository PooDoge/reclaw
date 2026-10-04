//! Stand-in Deck route pages while the Deck interface is moved onto the router.
use freya::prelude::*;

use crate::{deck::settings::SettingsTarget, nav::Section};

macro_rules! stub { ($name:ident { $($f:ident: $t:ty),* }) => {
    #[derive(PartialEq)]
    pub struct $name { $(pub $f: $t),* }
    impl Component for $name { fn render(&self) -> impl IntoElement { rect() } }
}; }
stub!(HomeRoute { section: Section });
stub!(GameRoute { id: u32 });
stub!(InstallRoute { id: u32 });
stub!(SettingsRoute { target: SettingsTarget });
stub!(ModDetailRoute { provider: String, mod_id: String });
