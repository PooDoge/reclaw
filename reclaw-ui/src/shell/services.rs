use reclaw_media::MediaHub;

use crate::window::WindowHost;

/// What the host provides to the UI besides its commands: the window behind it, and the means of
/// fetching pictures and documents. Bundled so a new service is a new field here and not a new
/// parameter on everything that builds a `Shell`.
#[derive(Clone, PartialEq)]
pub struct Services {
    pub window: WindowHost,
    /// `None` when nothing is fetched (the tests, previews). Artwork then stays a placeholder.
    pub media: Option<MediaHub>,
}

impl Services {
    /// No window and no network: what the headless tests run with.
    pub const fn headless() -> Self {
        Self { window: WindowHost::DETACHED, media: None }
    }

    pub fn new(window: WindowHost, media: Option<MediaHub>) -> Self {
        Self { window, media }
    }
}
