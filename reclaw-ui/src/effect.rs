//! Commands the UI asks of its host. The UI never touches processes, windows or files itself; it
//! emits an [`Effect`] and the host (or the `Shell`) does the work. Both interfaces, desktop and
//! Deck, speak the same vocabulary.
use reclaw_input::InputOwner;

use reclaw_games::settings::{SettingKey, SettingValue};

use crate::{
    deck::settings::{SettingChange, TextField},
    model::ModProvider,
    nav::transition::TransitionConfig,
};

/// The interface the user asked for in Settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModePref {
    Auto,
    Desktop,
    Deck,
}

/// What the host must do. The state never touches processes, windows or files itself.
///
/// `SubmitInstall` is handled inside `DeckApp`, which holds the form's text and turns it into
/// `StartInstall` for the host; likewise `EndTextEntry` becomes `TextCommitted`.
#[derive(Clone, PartialEq, Debug)]
pub enum Effect {
    Launch(u32),
    /// Bring the running app's window forward and give it the pad.
    Resume(u32),
    /// Graceful quit; pressed again while stopping, the supervisor force-kills.
    Stop(u32),
    /// The Install page was submitted (internal; see above).
    SubmitInstall(u32),
    StartInstall {
        app: u32,
        location: String,
        game_file: Option<String>,
        shortcut: bool,
        prerelease: bool,
    },
    Update(u32),
    /// Pick the user's own game file; answer with `DeckApp::chosen_file`.
    ChooseFile(u32),
    OpenFolder(u32),
    Verify(u32),
    CheckUpdate(u32),
    Uninstall(u32),
    ToggleFavorite(u32),
    AddToCollection {
        app: u32,
        name: &'static str,
    },
    NewCollection(u32),
    CancelDownload(u32),
    Search,
    InstallMod {
        provider: ModProvider,
        id: String,
    },
    RemoveMod {
        provider: ModProvider,
        id: String,
    },
    /// Open a link (a project page, a release, a video) in the system browser or player.
    OpenUrl(String),
    SwitchToDesktop,
    /// A launch setting changed: for one game (`app`) or the defaults (`None`). `value: None` clears
    /// the choice so the next level decides. The `Shell` has already applied it to `HostState`; the
    /// host persists it.
    LaunchSetting {
        app: Option<u32>,
        key: SettingKey,
        value: Option<SettingValue>,
    },
    /// Page transition settings changed (already applied by the `Shell`); persist them.
    Transitions(TransitionConfig),
    /// The Interface choice in Settings changed.
    SetMode(ModePref),
    /// A toggle or choice changed; persist it.
    Setting(SettingChange),
    /// Focus a text box so the keyboard (or the OS's on-screen one) can type into it.
    BeginTextEntry(TextField),
    EndTextEntry(TextField),
    /// A text box was left; persist its value.
    TextCommitted {
        app: Option<u32>,
        field: TextField,
        value: String,
    },
    BringLauncherToFront,
    SendLauncherToBack,
    InputOwner(InputOwner),
}
