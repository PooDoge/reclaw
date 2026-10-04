//! The Deck pages the router shows. Each draws the route it was built for from the frame the Deck
//! root shares, so a page leaving during a transition keeps drawing itself while the state has
//! moved on.
use std::rc::Rc;

use freya::prelude::*;

use super::{frame::Frame, screens};
use crate::{
    deck::settings::SettingsTarget,
    nav::{Section, use_page_motion},
};

/// The latest frame, shared by the Deck root with the pages under the router.
pub(super) type SharedFrame = State<Option<Rc<Frame>>>;

fn use_frame() -> Option<Rc<Frame>> {
    use_consume::<SharedFrame>().read().clone()
}

/// A tab's page: Library, Catalog, Downloads or Mods.
#[derive(PartialEq)]
pub struct HomeRoute {
    pub section: Section,
}

impl Component for HomeRoute {
    fn render(&self) -> impl IntoElement {
        let Some(frame) = use_frame() else { return rect().into_element() };
        screens::home(&frame, self.section)
    }
}

/// A game's page. It rises into place when it arrives with the hero style.
#[derive(PartialEq)]
pub struct GameRoute {
    pub id: u32,
}

impl Component for GameRoute {
    fn render(&self) -> impl IntoElement {
        let motion = use_page_motion();
        let Some(frame) = use_frame() else { return rect().into_element() };
        let (opacity, dy) = motion.rise(0.1, 0.9, 28.);
        rect().expanded().opacity(opacity).offset_y(dy).child(screens::game(&frame, self.id)).into_element()
    }
}

#[derive(PartialEq)]
pub struct InstallRoute {
    pub id: u32,
}

impl Component for InstallRoute {
    fn render(&self) -> impl IntoElement {
        let Some(frame) = use_frame() else { return rect().into_element() };
        screens::install(&frame, self.id)
    }
}

#[derive(PartialEq)]
pub struct SettingsRoute {
    pub target: SettingsTarget,
}

impl Component for SettingsRoute {
    fn render(&self) -> impl IntoElement {
        let Some(frame) = use_frame() else { return rect().into_element() };
        screens::settings(&frame, self.target)
    }
}

/// Deck has no page for a single mod yet; its tab stands in.
#[derive(PartialEq)]
pub struct ModDetailRoute {
    pub provider: String,
    pub mod_id: String,
}

impl Component for ModDetailRoute {
    fn render(&self) -> impl IntoElement {
        HomeRoute { section: Section::Mods }
    }
}
