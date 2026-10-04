//! The app root: the router, the shell context, and the keys that work in both interfaces.
use freya::{prelude::*, router::*};
use reclaw_input::{Action, ActionMap, UiMode};

use super::{
    Services,
    ctx::ShellCtx,
    model::{ShellModel, keyboard_height, pref_from_settings},
    overrides::{DevOverrides, KeyboardStart},
};
use crate::{
    deck::ActionFeed,
    effect::Effect,
    nav::{Route, transition::TransitionConfig},
    store::{AppAction, AppChannel, Store, ui_action, use_channel},
    theme::ThemeKind,
    window::{run_command, use_window_driver},
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
    /// The window behind the UI, and the means of fetching pictures.
    pub services: Services,
    /// The page to open first: `Route::Library {}`, or what a `--open` argument named.
    pub start: Route,
    /// Actions applied once when Deck mode opens (gallery and snapshot scenarios).
    pub script: Vec<Action>,
}

impl Component for Shell {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let services = self.services.clone();
        let attached = services.window.attached;
        store.install();
        let mut window = use_state(|| (1280.0f32, 800.0f32));
        let mut model = use_state({
            let (detected, dev) = (self.detected, self.dev);
            move || {
                let mut m = ShellModel::new(detected, dev.keyboard_follows_focus);
                // What the Interface row of the settings chose last time (Auto when never touched).
                m.set_pref(store.with(|s| pref_from_settings(&s.settings)));
                m.keyboard = match dev.keyboard {
                    Some(KeyboardStart::Pixels(px)) => px,
                    Some(KeyboardStart::Share) => keyboard_height(800.),
                    None => 0.,
                };
                m
            }
        });
        let dev = self.dev;
        let mut transitions = use_state(move || dev.apply_motion(store.with(|s| TransitionConfig::from_settings(&s.settings))));
        let mut theme = use_consume::<State<ThemeKind>>();
        let settings = use_channel(AppChannel::Settings);

        // Page transitions and the theme follow the settings page, unless a RECLAW_* variable pins them.
        use_side_effect(move || {
            let values = settings.read().settings.clone();
            transitions.set_if_modified(dev.apply_motion(TransitionConfig::from_settings(&values)));
            theme.set_if_modified(dev.theme.unwrap_or_else(|| ThemeKind::from_settings(&values)));
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
                // Window commands are carried out here; the host still hears of them.
                if let (Effect::Window(command), true) = (&effect, attached) {
                    run_command(command, store);
                }
                // State the UI owns changes here; the host still hears about it.
                if let Some(action) = ui_action(&effect) {
                    store.dispatch(action);
                }
                host_handler.call(effect);
            })
        };
        // With no native border the window's edges are ours to resize from.
        let resize_bands = (services.window.frame == crate::window::Frame::Custom).then(|| {
            let on_effect = on_effect.clone();
            crate::window::ResizeBands::new(window(), attached, EventHandler::new(move |command| on_effect.call(Effect::Window(command))))
                .into_element()
        });
        // The context is created once, on the first render; everything in it is a stable handle.
        use_provide_context({
            let (feed, map, dev, script, on_effect) = (self.feed.clone(), self.map.clone(), self.dev, self.script.clone(), on_effect);
            move || ShellCtx { store, feed, map, on_effect, dev, services, script, model, transitions, window }
        });

        // Keeps the window in step: monitors, remembered geometry, Deck fullscreen, UI scale.
        let server = use_hook(|| crate::window::detect_server(|k| std::env::var(k).ok()));
        use_window_driver(store, model, server, attached);

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
            .maybe_child(resize_bands)
    }
}
