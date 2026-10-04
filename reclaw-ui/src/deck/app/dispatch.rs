//! Runs actions through the reducer and sends the resulting effects on. A few effects are
//! finished here because only this layer holds the text boxes: the Install submit becomes a full
//! request, and leaving a text box reports what was typed.
use freya::prelude::*;

use super::text_boxes::TextBoxes;
use crate::{
    deck::{
        DeckState, DeckView, Effect, InstallDraft, LastInput, Screen,
        settings::{SettingsTarget, TextField},
    },
    nav::Nav,
    settings::LaunchContext,
    store::Store,
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
    pub on_effect: EventHandler<Effect>,
}

impl Dispatcher {
    /// Run `f` on the state with a fresh view of the host's data, then forward its effects.
    pub fn run(&self, f: impl FnOnce(&mut DeckState, &DeckView) -> Vec<Effect>) {
        let effects = {
            let mut deck = self.deck;
            self.store.with(|s| {
                let queue: Vec<_> = s.activity.queue().into_iter().cloned().collect();
                let launch = LaunchContext { env: &s.display, projects: &s.projects, prefs: &s.launch };
                let view = DeckView { games: &s.games, downloads: &queue, launch: Some(launch) };
                let mut state = deck.write();
                // The settings live in the store, which both interfaces share; the reducer works on a copy.
                state.sync_values(&s.settings);
                f(&mut state, &view)
            })
        };
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
                        game_file: draft.game_file,
                        shortcut: draft.shortcut,
                        prerelease: draft.prerelease,
                    });
                }
                Effect::BeginTextEntry(field) => {
                    let by_pointer = self.deck.read().last_input() == LastInput::Pointer;
                    self.texts.begin(field, !by_pointer);
                    self.on_effect.call(Effect::BeginTextEntry(field));
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
                // The router is this layer's business: the host never hears of page changes.
                Effect::Navigate(route) => self.nav.open(route),
                Effect::Back => {
                    self.nav.back();
                }
                other => self.on_effect.call(other),
            }
        }
    }
}
