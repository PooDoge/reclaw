use freya::prelude::*;

use crate::deck::settings::TextField;

/// The text states behind every text box in Deck mode, plus the accessibility ids that let a
/// gamepad press focus one (so the keyboard, or the OS's on-screen keyboard, types into it).
/// Created once by `DeckApp`; pages receive the handle.
///
/// **Entering a box from a pad or keyboard replaces its text.** Freya's `Input` starts its caret at
/// the beginning and has no way to move it from outside, so typing would land in front of the old
/// text. [`begin`](Self::begin) therefore clears the box (remembering the old text) and
/// [`finish`](Self::finish) puts the old text back if nothing was typed. A tap or click on the
/// box places the caret itself, so pointer entry keeps the text.
#[derive(Clone, Copy, PartialEq)]
pub struct TextBoxes {
    install_location: State<String>,
    launch_options: State<String>,
    sdl_override: State<String>,
    default_location: State<String>,
    /// A token being pasted. These two are emptied the moment the token is handed over: a token is not kept in the page's state.
    github_token: State<String>,
    gitlab_token: State<String>,
    /// What a box held before `begin` cleared it.
    before: [State<Option<String>>; 6],
    ids: [AccessibilityId; 6],
}

fn slot(field: TextField) -> usize {
    match field {
        TextField::InstallLocation => 0,
        TextField::LaunchOptions => 1,
        TextField::SdlOverride => 2,
        TextField::DefaultLocation => 3,
        TextField::GithubToken => 4,
        TextField::GitlabToken => 5,
    }
}

impl TextBoxes {
    /// Hooks: call once, unconditionally, from a component.
    pub fn use_new() -> Self {
        Self {
            install_location: use_state(|| "~/Reclaw/Apps".to_string()),
            launch_options: use_state(String::new),
            sdl_override: use_state(String::new),
            default_location: use_state(|| "~/Reclaw/Apps".to_string()),
            github_token: use_state(String::new),
            gitlab_token: use_state(String::new),
            before: std::array::from_fn(|_| use_state(|| None)),
            ids: std::array::from_fn(|_| use_a11y()),
        }
    }

    pub fn get(&self, field: TextField) -> (State<String>, AccessibilityId) {
        let state = match field {
            TextField::InstallLocation => self.install_location,
            TextField::LaunchOptions => self.launch_options,
            TextField::SdlOverride => self.sdl_override,
            TextField::DefaultLocation => self.default_location,
            TextField::GithubToken => self.github_token,
            TextField::GitlabToken => self.gitlab_token,
        };
        (state, self.ids[slot(field)])
    }

    pub fn value(&self, field: TextField) -> String {
        self.get(field).0.read().clone()
    }

    /// Empty a box (a token that has been handed over).
    pub fn clear(&self, field: TextField) {
        let mut text = self.get(field).0;
        text.set(String::new());
    }

    /// Start typing into the box: focus it, clearing it first when `replace` is set (see the type docs).
    pub fn begin(&self, field: TextField, replace: bool) {
        if replace {
            let (mut text, mut before) = (self.get(field).0, self.before[slot(field)]);
            before.set(Some(text.peek().clone()));
            text.set(String::new());
        }
        self.ids[slot(field)].request_focus();
    }

    /// Stop typing: the box keeps what was typed, or gets its old text back if it was left empty.
    /// Returns the text now in the box.
    pub fn finish(&self, field: TextField) -> String {
        let (mut text, mut before) = (self.get(field).0, self.before[slot(field)]);
        let previous = before.write().take();
        if let Some(old) = previous.filter(|_| text.peek().is_empty()) {
            text.set(old);
        }
        text.peek().clone()
    }
}
