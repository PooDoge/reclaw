//! `DeckApp`: the Deck mode root. It owns the [`DeckState`], turns pad and keyboard input into
//! actions, runs them through the reducer, and renders the result.
//!
//! * `feed`: where pad actions arrive; `keys`: the keyboard fallback;
//! * `dispatch`: runs the reducer and finishes the effects only this layer can;
//! * `text_boxes`: the text states behind every field; `frame`: one render's worth of data;
//! * `screens` / `overlays`: build the page and what sits over it.
use freya::prelude::*;
use reclaw_input::{Action, ActionMap, ControllerKind, FocusId};

mod dispatch;
mod feed;
mod frame;
mod keys;
mod overlays;
mod screens;
mod text_boxes;

pub use feed::ActionFeed;
pub use text_boxes::TextBoxes;

use crate::{
    deck::{DeckState, DeckView, Effect, LastInput, ids},
    host::HostState,
    prelude::*,
};
use dispatch::Dispatcher;
use frame::Frame;

/// Deck mode root. The host owns the data (`HostState`) and the processes; this component only
/// reads them, so a run state change from the supervisor shows up the moment the host writes it.
#[derive(Clone, PartialEq)]
pub struct DeckApp {
    pub host: HostState,
    pub feed: ActionFeed,
    pub on_effect: EventHandler<Effect>,
    pub map: ActionMap,
    /// Actions applied once at startup (gallery and snapshot scenarios). Their effects are dropped.
    pub script: Vec<Action>,
}

impl Component for DeckApp {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        use_provide_context(|| ManagedFocus);
        let HostState { games, downloads, controller, keyboard_inset, mut chosen_file } = self.host;
        let texts = TextBoxes::use_new();
        let root_focus = use_a11y();

        let deck = {
            let script = self.script.clone();
            use_state(move || {
                let (g, d) = (games.read(), downloads.read());
                let view = DeckView { games: &g, downloads: &d };
                let mut state = DeckState::new(&view);
                for action in script {
                    state.apply(action, &view);
                }
                state
            })
        };
        let mut window = use_state(|| (1280.0f32, 800.0f32));
        let mut was_active = use_state(|| games.read().iter().any(|g| g.run.is_active()));
        let dispatcher = Dispatcher { games, downloads, deck, texts, root_focus, on_effect: self.on_effect.clone() };

        // Gamepad actions: drained by one task for the lifetime of the app.
        use_hook({
            let (feed, dispatcher) = (self.feed.clone(), dispatcher.clone());
            move || {
                spawn(async move {
                    let Some(mut rx) = feed.take() else { return };
                    while let Some(action) = rx.next().await {
                        let kind = controller.read().as_ref().map_or(ControllerKind::Generic, |c| c.kind);
                        dispatcher.run(|state, view| {
                            state.set_last_input(LastInput::Gamepad(kind));
                            state.apply(action, view)
                        });
                    }
                })
            }
        });

        // Host data changed: repair focus, report an ownership change, notice an app ending, and
        // take a file the host picked for the Install page.
        use_side_effect({
            let dispatcher = dispatcher.clone();
            move || {
                let active = games.read().iter().any(|g| g.run.is_active());
                downloads.read();
                let ended = *was_active.peek() && !active;
                was_active.set_if_modified(active);
                // Reading subscribes to the host's answer; clearing it re-runs this once, harmlessly.
                let picked = chosen_file.read().clone();
                if let Some(path) = picked {
                    chosen_file.set(None);
                    let mut deck = deck;
                    deck.write().set_install_file(path);
                }
                dispatcher.run(|state, view| if ended { state.on_app_ended(view) } else { state.sync(view) });
            }
        });

        let click = {
            let d = dispatcher.clone();
            EventHandler::new(move |id: FocusId| d.run(|state, view| state.click(id, view)))
        };
        let back = {
            let d = dispatcher.clone();
            EventHandler::new(move |()| d.run(|state, view| state.click(ids::PAGE_BACK, view)))
        };
        let dismiss = {
            let d = dispatcher.clone();
            EventHandler::new(move |()| d.run(|state, view| state.dismiss(view)))
        };
        let pick = {
            let d = dispatcher.clone();
            EventHandler::new(move |(level, index): (usize, usize)| d.run(|state, view| state.pick_menu(level, index, view)))
        };

        let state = deck.read().clone();
        let typing_now = state.text_entry().is_some();
        let (g, d) = (games.read().clone(), downloads.read().clone());
        let pad = controller.read().clone();
        let kind = pad.as_ref().map_or(ControllerKind::Generic, |c| c.kind);
        let (schema, reveal) = {
            let view = DeckView { games: &g, downloads: &d };
            (state.settings_target().and_then(|t| state.settings_schema(t, &view)), state.reveal_target(&view))
        };
        let frame = Frame {
            ring: state.focus_visible(),
            last_input: match state.last_input() {
                LastInput::Gamepad(_) => LastInput::Gamepad(kind),
                other => other,
            },
            window: window(),
            keyboard_inset: *keyboard_inset.read(),
            map: self.map.clone(),
            texts,
            schema,
            reveal,
            state,
            games: g,
            downloads: d,
            pad,
            kind,
            click,
            back,
            dismiss,
            pick,
        };

        let typing_dispatcher = dispatcher.clone();
        rect()
            .expanded()
            .background(t.deck_bg)
            .a11y_id(root_focus)
            // Not focusable while typing: then the text box is the only focus target, and Freya's
            // Tab and arrow focus moves have nowhere else to take the keyboard.
            .a11y_focusable(!typing_now)
            .on_sized(move |e: Event<SizedEventData>| {
                let size = (e.area.width(), e.area.height());
                if size != window() {
                    window.set(size);
                    dispatcher.run(|state, view| {
                        state.set_window(size, view);
                        Vec::new()
                    });
                }
            })
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                let typing = typing_dispatcher.deck.read().text_entry();
                let Some(action) = keys::key_action(&e.key, e.modifiers.contains(Modifiers::SHIFT)) else { return };
                if typing.is_some() && !keys::is_ours_while_typing(action) {
                    return;
                }
                typing_dispatcher.run(|state, view| {
                    state.set_last_input(LastInput::Keyboard);
                    state.apply(action, view)
                });
            })
            .child(screens::screen(&frame))
            .children(overlays::overlays(&frame))
    }
}
