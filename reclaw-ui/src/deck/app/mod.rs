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
    prelude::*,
    store::{
        AppChannel, Store, use_activity, use_channel, use_controller, use_display, use_games, use_keyboard_inset, use_launch, use_projects,
        use_settings,
    },
};
use dispatch::Dispatcher;
use frame::Frame;

/// Deck mode root. The host owns the processes and reports what they do to the store; this component
/// only reads it, so a run state change from the supervisor shows up the moment the host reports it.
#[derive(Clone, PartialEq)]
pub struct DeckApp {
    pub store: Store,
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
        let store = self.store;
        let (games, activity, controller, keyboard_inset) = (use_games(), use_activity(), use_controller(), use_keyboard_inset());
        let (settings, projects, display, launch) = (use_settings(), use_projects(), use_display(), use_launch());
        let (games_changed, activity_changed, mailbox) =
            (use_channel(AppChannel::Games), use_channel(AppChannel::Activity), use_channel(AppChannel::Mailbox));
        let texts = TextBoxes::use_new();
        let root_focus = use_a11y();

        let deck = {
            let (script, games, activity) = (self.script.clone(), games.clone(), activity.clone());
            use_state(move || {
                let queue: Vec<_> = activity.queue().into_iter().cloned().collect();
                let view = DeckView { games: &games, downloads: &queue, launch: None };
                let mut state = DeckState::new(&view);
                for action in script {
                    state.apply(action, &view);
                }
                state
            })
        };
        let mut window = use_state(|| (1280.0f32, 800.0f32));
        let mut was_active = use_state(|| games.iter().any(|g| g.run.is_active()));
        let dispatcher = Dispatcher { store, deck, texts, root_focus, on_effect: self.on_effect.clone() };

        // Gamepad actions: drained by one task for the lifetime of the app.
        use_hook({
            let (feed, dispatcher) = (self.feed.clone(), dispatcher.clone());
            move || {
                spawn(async move {
                    let Some(mut rx) = feed.take() else { return };
                    while let Some(action) = rx.next().await {
                        let kind = store.with(|s| s.controller.as_ref().map_or(ControllerKind::Generic, |c| c.kind));
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
                let active = games_changed.read().games.iter().any(|g| g.run.is_active());
                activity_changed.read();
                let ended = *was_active.peek() && !active;
                was_active.set_if_modified(active);
                // Reading subscribes to the host's answer; clearing it re-runs this once, harmlessly.
                let picked = mailbox.read().mailbox.chosen_file.clone();
                if let Some(path) = picked {
                    store.dispatch(crate::store::AppAction::ChosenFile(None));
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

        let mut state = deck.read().clone();
        state.sync_values(&settings);
        let typing_now = state.text_entry().is_some();
        let (g, d) = (games.clone(), activity.queue().into_iter().cloned().collect::<Vec<_>>());
        let pad = controller.clone();
        let kind = pad.as_ref().map_or(ControllerKind::Generic, |c| c.kind);
        let (schema, reveal, launch_text) = {
            let launch_ctx = crate::settings::LaunchContext { env: &display, projects: &projects, prefs: &launch };
            let view = DeckView { games: &g, downloads: &d, launch: Some(launch_ctx) };
            let target = state.settings_target();
            let schema = target.and_then(|t| state.settings_schema(t, &view));
            // The text of each launch row, worked out here so the page itself needs no engine.
            let texts: std::collections::HashMap<_, _> = match (target, &schema) {
                (Some(target), Some(schema)) => schema
                    .sections
                    .iter()
                    .flat_map(|s| s.rows())
                    .filter_map(|row| match row.kind {
                        crate::settings::RowKind::Launch { key } => launch_ctx.control(target, key).map(|c| (key, c.summary)),
                        _ => None,
                    })
                    .collect(),
                _ => Default::default(),
            };
            (schema, state.reveal_target(&view), texts)
        };
        let frame = Frame {
            ring: state.focus_visible(),
            last_input: match state.last_input() {
                LastInput::Gamepad(_) => LastInput::Gamepad(kind),
                other => other,
            },
            window: window(),
            keyboard_inset,
            map: self.map.clone(),
            texts,
            schema,
            launch_text,
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
