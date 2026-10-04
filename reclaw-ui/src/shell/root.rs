//! The app root: shows the desktop or the Deck interface, and lets either be entered by hand.
use freya::prelude::*;
use reclaw_input::{Action, ActionMap, UiMode};

use super::{
    model::{ShellModel, keyboard_height},
    overrides::{DevOverrides, KeyboardStart},
};
use crate::{
    app_menu::MenuAction,
    deck::{ActionFeed, DeckApp},
    desktop::DesktopApp,
    effect::Effect,
    host::HostState,
};

/// Keys that work in both interfaces, for testing without a device:
/// **F10** switches between desktop and Deck mode; **F9** shows or hides a simulated on-screen
/// keyboard so keyboard avoidance can be seen on a desktop.
#[derive(Clone, PartialEq)]
pub struct Shell {
    pub host: HostState,
    pub feed: ActionFeed,
    pub map: ActionMap,
    /// Commands for the host. The shell handles mode changes itself and passes everything on.
    pub on_effect: EventHandler<Effect>,
    /// The interface to start in, from `reclaw_input::detect_environment`.
    pub detected: UiMode,
    pub dev: DevOverrides,
    /// Actions applied once when Deck mode opens (gallery and snapshot scenarios).
    pub script: Vec<Action>,
}

impl Component for Shell {
    fn render(&self) -> impl IntoElement {
        let host = self.host;
        let mut window = use_state(|| (1280.0f32, 800.0f32));
        let mut model = use_state({
            let (detected, dev) = (self.detected, self.dev);
            move || {
                let mut m = ShellModel::new(detected, dev.keyboard_follows_focus);
                m.keyboard = match dev.keyboard {
                    Some(KeyboardStart::Pixels(px)) => px,
                    Some(KeyboardStart::Share) => keyboard_height(800.),
                    None => 0.,
                };
                m
            }
        });

        // Both interfaces read the keyboard height from the shared host state.
        use_side_effect(move || {
            let height = model.read().keyboard;
            let mut inset = host.keyboard_inset;
            inset.set_if_modified(height);
        });

        let on_effect = {
            let host_handler = self.on_effect.clone();
            EventHandler::new(move |effect: Effect| {
                model.write().on_effect(&effect, window().1);
                host_handler.call(effect);
            })
        };
        let on_action = {
            let on_effect = on_effect.clone();
            EventHandler::new(move |(app, action): (u32, MenuAction)| {
                if let Some(effect) = action.confirmed_effect(app) {
                    on_effect.call(effect);
                }
            })
        };

        let content = match model.read().mode {
            UiMode::Deck => {
                DeckApp { host, feed: self.feed.clone(), on_effect, map: self.map.clone(), script: self.script.clone() }.into_element()
            }
            UiMode::Desktop => DesktopApp {
                host,
                layout: self.dev.layout,
                density: self.dev.density,
                on_action: Some(on_action),
                on_deck_mode: Some(EventHandler::new(move |()| model.write().toggle_mode())),
            }
            .into_element(),
        };

        rect()
            .expanded()
            .on_sized(move |e: Event<SizedEventData>| window.set_if_modified((e.area.width(), e.area.height())))
            .on_global_key_down(move |e: Event<KeyboardEventData>| match e.key {
                Key::Named(NamedKey::F10) => model.write().toggle_mode(),
                Key::Named(NamedKey::F9) => model.write().toggle_keyboard(window().1),
                _ => {}
            })
            .child(content)
    }
}
