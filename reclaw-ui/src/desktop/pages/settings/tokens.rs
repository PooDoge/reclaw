use freya::prelude::*;
use reclaw_net::Provider;

/// The two boxes an access token is pasted into. They belong to the Settings page being shown: nothing else reads them, and
/// pressing Save token empties the box, so a pasted token is not kept in the page's state once it has been handed to the host.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct TokenBoxes {
    github: State<String>,
    gitlab: State<String>,
    ids: [AccessibilityId; 2],
}

impl TokenBoxes {
    /// Hooks: call once, unconditionally, from the page.
    pub fn use_new() -> Self {
        Self { github: use_state(String::new), gitlab: use_state(String::new), ids: [use_a11y(), use_a11y()] }
    }

    pub fn get(&self, provider: Provider) -> (State<String>, AccessibilityId) {
        match provider {
            Provider::GitHub => (self.github, self.ids[0]),
            Provider::GitLab => (self.gitlab, self.ids[1]),
        }
    }
}
