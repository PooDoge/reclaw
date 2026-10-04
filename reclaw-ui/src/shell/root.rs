//! The app root: the router, the shell context, and the keys that work in both interfaces.
use freya::{prelude::*, router::*};
use reclaw_input::{Action, ActionMap, UiMode};

use super::{
    ctx::ShellCtx,
    model::{ShellModel, keyboard_height},
    overrides::{DevOverrides, KeyboardStart},
};
use crate::{
    deck::ActionFeed,
    effect::Effect,
    nav::Route,
    store::{AppAction, Store, ui_action},
};

/// The root. Keys that work in both interfaces, for testing without a device: **F10** switches
/// between desktop and Deck mode; **F9** shows or hides a simulated on-screen keyboard so keyboard
/// avoidance can be seen on a desktop.
///
/// It mounts the router once; mode switches happen below it, so the current page survives them.
#[derive(Clone, PartialEq)]
pub struct Shell {
    pub store: Store,
    pub feed: ActionFeed,
    pub map: ActionMap,
    /// Commands for the host. The shell handles mode changes itself and passes everything on.
    pub on_effect: EventHandler<Effect>,
    /// The interface to start in, from `reclaw_input::detect_environment`.
    pub detected: UiMode,
    pub dev: DevOverrides,
    /// The page to open first: `Route::Library {}`, or what a `--open` argument named.
    pub start: Route,
    /// Actions applied once when Deck mode opens (gallery and snapshot scenarios).
    pub script: Vec<Action>,
}

impl Component for Shell {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        store.install();
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
        let transitions = use_state({
            let dev = self.dev;
            move || dev.transitions()
        });

        // Both interfaces read the keyboard height from the shared store.
        use_side_effect(move || {
            let height = model.read().keyboard;
            store.dispatch(AppAction::SetKeyboardInset(height));
        });

        let on_effect = {
            let host_handler = self.on_effect.clone();
            EventHandler::new(move |effect: Effect| {
                model.write().on_effect(&effect, window().1);
                // State the UI owns changes here; the host still hears about it.
                if let Some(action) = ui_action(&effect) {
                    store.dispatch(action);
                }
                host_handler.call(effect);
            })
        };
        // The context is created once, on the first render; everything in it is a stable handle.
        use_provide_context({
            let (feed, map, dev, script, on_effect) = (self.feed.clone(), self.map.clone(), self.dev, self.script.clone(), on_effect);
            move || ShellCtx { store, feed, map, on_effect, dev, script, model, transitions, window }
        });

        let start = self.start.clone();
        rect()
            .expanded()
            .on_sized(move |e: Event<SizedEventData>| window.set_if_modified((e.area.width(), e.area.height())))
            .on_global_key_down(move |e: Event<KeyboardEventData>| match e.key {
                Key::Named(NamedKey::F10) => model.write().toggle_mode(),
                Key::Named(NamedKey::F9) => model.write().toggle_keyboard(window().1),
                _ => {}
            })
            .child(Router::<Route>::new(move || RouterConfig::default().with_initial_path(start.clone())))
    }
}
