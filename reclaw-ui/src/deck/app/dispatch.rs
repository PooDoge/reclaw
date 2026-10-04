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
    model::{Download, GameEntry},
};

/// Everything needed to react to input. All handles are `Copy` states or cheap clones, so a
/// dispatcher can move into any closure.
#[derive(Clone)]
pub(super) struct Dispatcher {
    pub games: State<Vec<GameEntry>>,
    pub downloads: State<Vec<Download>>,
    pub deck: State<DeckState>,
    pub texts: TextBoxes,
    pub root_focus: AccessibilityId,
    pub on_effect: EventHandler<Effect>,
}

impl Dispatcher {
    /// Run `f` on the state with a fresh view of the host's data, then forward its effects.
    pub fn run(&self, f: impl FnOnce(&mut DeckState, &DeckView) -> Vec<Effect>) {
        let effects = {
            let (g, d) = (self.games.read(), self.downloads.read());
            let view = DeckView { games: &g, downloads: &d };
            let mut deck = self.deck;
            let mut state = deck.write();
            f(&mut state, &view)
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
                other => self.on_effect.call(other),
            }
        }
    }
}
