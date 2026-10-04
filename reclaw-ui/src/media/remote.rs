use freya::prelude::*;
use reclaw_media::{Cached, MediaHub, Want};

use crate::{
    settings::{KEY_REMOTE_MEDIA, SettingsTarget},
    shell::try_shell,
    store::use_settings,
};

/// Where a fetch is.
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteFile {
    /// Nothing is being fetched: no address, no network in this build, or the setting is off. The
    /// placeholder stays.
    Off,
    Loading,
    Ready(Cached),
    /// Why it failed, for a tooltip or a log; the placeholder stays.
    Failed(String),
}

/// Whether the user allows artwork and READMEs to be downloaded. On unless switched off.
pub fn use_media_enabled() -> bool {
    use_settings().toggle(SettingsTarget::Global, KEY_REMOTE_MEDIA, true)
}

fn start_state(hub: &Option<MediaHub>, url: &Option<String>, want: Want, enabled: bool) -> RemoteFile {
    match (hub, url, enabled) {
        (Some(hub), Some(url), true) => hub.peek(url, want).map_or(RemoteFile::Loading, RemoteFile::Ready),
        _ => RemoteFile::Off,
    }
}

/// The file for `url`, fetched in the background. A hook: the component must be keyed by `url`.
///
/// A component shown with no shell around it (a preview of one widget) fetches nothing and stays
/// `Off`. Whether there is a shell does not change while a component lives, so the hooks below run
/// in the same order on every render of it.
///
/// If the file was already fetched in this run it is ready on the first render, so a redraw or a
/// return to a page never flashes the placeholder.
pub fn use_remote_file(url: Option<&str>, want: Want) -> RemoteFile {
    let Some(shell) = try_shell() else { return RemoteFile::Off };
    let hub = shell.services.media;
    let enabled = use_media_enabled();
    let url = url.map(str::to_string);
    let mut state = use_state(|| start_state(&hub, &url, want, enabled));

    use_hook({
        let (hub, url) = (hub.clone(), url.clone());
        move || {
            if let (Some(hub), Some(url), true, RemoteFile::Loading) = (hub, url, enabled, state.peek().clone()) {
                let answer = hub.request(&url, want);
                spawn(async move {
                    let result = match answer.await {
                        Ok(Ok(file)) => RemoteFile::Ready(file),
                        Ok(Err(why)) => RemoteFile::Failed(why.to_string()),
                        Err(_) => RemoteFile::Failed("the download was cancelled".to_string()),
                    };
                    // The component may be gone by now (the page was left); then there is no one to tell.
                    if let Some(mut slot) = state.try_write() {
                        *slot = result;
                    }
                });
            }
        }
    });

    // Turning the setting off hides what is shown; turning it on again shows it without a restart.
    if enabled { state.read().clone() } else { RemoteFile::Off }
}
