//! Runs actions through the reducer and sends the resulting effects on. A few effects are
//! finished here because only this layer holds the text boxes: the Install submit becomes a full
//! request, and leaving a text box reports what was typed.
use std::time::{Duration, Instant};

use freya::prelude::*;
use reclaw_input::{Action, Button};

use super::{key_holds::KeyHolds, text_boxes::TextBoxes};
use crate::{
    deck::{
        DeckState, DeckView, Effect, InstallDraft, LastInput, Screen,
        settings::{SettingsTarget, TextField},
    },
    nav::Nav,
    settings::LaunchContext,
    store::Store,
    systems::Sort,
};

/// Everything needed to react to input. All handles are `Copy` states or cheap clones, so a
/// dispatcher can move into any closure.
#[derive(Clone)]
pub(super) struct Dispatcher {
    pub store: Store,
    pub nav: Nav,
    pub deck: State<DeckState>,
    pub texts: TextBoxes,
    pub root_focus: AccessibilityId,
    /// X and Y held on a keyboard.
    pub holds: State<KeyHolds>,
    pub on_effect: EventHandler<Effect>,
}

impl Dispatcher {
    /// Run `f` on the state with a fresh view of the host's data, then forward its effects.
    pub fn run(&self, f: impl FnOnce(&mut DeckState, &DeckView) -> Vec<Effect>) {
        let (effects, seeds) = {
            let mut deck = self.deck;
            let recents = self.nav.recents();
            self.store.with(|s| {
                let queue: Vec<_> = s.activity.queue().into_iter().cloned().collect();
                let launch = LaunchContext { env: &s.display, projects: &s.projects, prefs: &s.launch };
                let view = DeckView {
                    games: &s.games,
                    downloads: &queue,
                    launch: Some(launch),
                    notices: Some(&s.notices),
                    sort: Sort::from_settings(&s.settings),
                    recents: &recents,
                };
                let mut state = deck.write();
                // The settings live in the store, which both interfaces share; the reducer works on a copy.
                state.sync_values(&s.settings);
                let effects = f(&mut state, &view);
                (effects, state.take_text_seeds())
            })
        };
        // A page with text boxes was just shown: they start from the stored settings, not from what the last visit left.
        for (field, value) in seeds {
            self.texts.set(field, value);
        }
        self.forward(effects);
    }

    pub fn forward(&self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::SubmitInstall(app) => {
                    let draft: InstallDraft = self.deck.read().install_draft().clone();
                    self.on_effect.call(Effect::StartInstall {
                        app,
                        location: self.texts.value(TextField::InstallLocation),
                        prerelease: draft.prerelease,
                    });
                }
                Effect::BeginTextEntry(field) => {
                    let by_pointer = self.deck.read().last_input() == LastInput::Pointer;
                    self.texts.begin(field, !by_pointer);
                    self.on_effect.call(Effect::BeginTextEntry(field));
                }
                Effect::SubmitToken(provider) => {
                    // The typed text becomes a `Secret` here and the box is emptied: nothing keeps the token but the host.
                    let field = TextField::for_provider(provider);
                    let token = reclaw_log::Secret::new(self.texts.value(field).trim());
                    self.texts.clear(field);
                    self.on_effect.call(Effect::SaveToken { provider, token });
                }
                Effect::EndTextEntry(field) if field.is_secret() => {
                    // A token box keeps what was typed until Save token is pressed; it is not a setting and is not reported.
                    self.root_focus.request_focus();
                    self.texts.finish(field);
                    self.on_effect.call(Effect::EndTextEntry(field));
                }
                Effect::EndTextEntry(field) => {
                    // Give the keyboard back to the page, then report what was typed.
                    self.root_focus.request_focus();
                    let app = match self.deck.read().screen() {
                        Screen::Install(id) | Screen::Settings(SettingsTarget::App(id)) => Some(id),
                        _ => None,
                    };
                    let value = self.texts.finish(field);
                    self.on_effect.call(Effect::TextCommitted { app, field, value });
                    self.on_effect.call(Effect::EndTextEntry(field));
                }
                // The keyboard's holds follow the pad reader's: on while a toast is up. The host hears
                // it too, for the pad.
                Effect::NoticeHolds(on) => {
                    let mut holds = self.holds;
                    let cancelled = holds.write().set_enabled(on);
                    if let Some(action) = cancelled {
                        self.apply_all(vec![action]);
                    }
                    self.on_effect.call(Effect::NoticeHolds(on));
                }
                // The router is this layer's business: the host never hears of page changes.
                Effect::Navigate(route) => self.nav.open(route),
                Effect::Back => {
                    self.nav.back();
                }
                other => self.on_effect.call(other),
            }
        }
    }

    /// Apply keyboard-originated actions in order, as one step.
    fn apply_all(&self, actions: Vec<Action>) {
        if actions.is_empty() {
            return;
        }
        self.run(|state, view| {
            state.set_last_input(LastInput::Keyboard);
            actions.into_iter().flat_map(|action| state.apply(action, view)).collect()
        });
    }

    /// X or Y went down on the keyboard. With a toast up this starts a hold and a timer to finish it;
    /// without one it is the button's ordinary tap.
    pub fn key_down(&self, button: Button) {
        let mut holds = self.holds;
        let actions = holds.write().down(button, Instant::now());
        let started = actions.iter().any(|a| matches!(a, Action::Hold(_, reclaw_input::HoldPhase::Started)));
        self.apply_all(actions);
        if started {
            self.tick_hold();
        }
    }

    pub fn key_up(&self, button: Button) {
        let mut holds = self.holds;
        let actions = holds.write().release(button);
        self.apply_all(actions);
    }

    /// Check the held key against the clock until it completes or is let go.
    fn tick_hold(&self) {
        let this = self.clone();
        spawn(async move {
            while this.holds.peek().waiting() {
                timer(Duration::from_millis(30)).await;
                let mut holds = this.holds;
                let done = holds.write().tick(Instant::now());
                if let Some(action) = done {
                    this.apply_all(vec![action]);
                }
            }
        });
    }
}
