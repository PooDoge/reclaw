//! Commands the UI asks of its host. The UI never touches processes, windows or files itself; it
//! emits an [`Effect`] and the host (or the `Shell`) does the work. Both interfaces, desktop and
//! Deck, speak the same vocabulary.
use reclaw_input::InputOwner;
use reclaw_log::Secret;
use reclaw_net::Provider;

use reclaw_games::settings::{SettingKey, SettingValue};

use crate::{
    activity::ActivityId,
    model::ModProvider,
    notices::NoticeId,
    settings::{SettingChange, TextField},
};

/// The interface the user asked for in Settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModePref {
    Auto,
    Desktop,
    Deck,
}

impl Effect {
    /// What a settings row changing to `value` asks for: the saved change, and for the Interface row
    /// the switch itself. The Deck reducer and the desktop settings page both use this, so a row
    /// does the same in either.
    pub fn setting(target: crate::settings::SettingsTarget, key: &'static str, value: crate::settings::SettingValue) -> Vec<Effect> {
        use crate::settings::{KEY_INTERFACE_MODE, SettingValue};
        let mut effects = vec![Effect::Setting(SettingChange { app: target.app(), key, value })];
        if let (KEY_INTERFACE_MODE, SettingValue::Choice(i)) = (key, value) {
            effects.push(Effect::SetMode(match i {
                1 => ModePref::Desktop,
                2 => ModePref::Deck,
                _ => ModePref::Auto,
            }));
        }
        effects
    }
}

/// What the host must do. The state never touches processes, windows or files itself.
///
/// `SubmitInstall` is handled inside `DeckApp`, which holds the form's text and turns it into
/// `StartInstall` for the host; likewise `EndTextEntry` becomes `TextCommitted` (except for a token's box, whose text never
/// travels as a plain string: `SubmitToken` becomes `SaveToken`).
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
        /// Where the app's folder goes; empty means the Library default (`settings::resolve_install_location`).
        location: String,
        prerelease: bool,
    },
    Update(u32),
    OpenFolder(u32),
    Verify(u32),
    CheckUpdate(u32),
    Uninstall(u32),
    ToggleFavorite(u32),
    /// Keep this catalog project in the library (the user's own list, `apps.json`).
    AddToLibrary(u32),
    /// Forget it. Files it installed are not touched.
    RemoveFromLibrary(u32),
    /// Load the catalog again now.
    RefreshCatalog,
    /// Use this access token for the service from now on, and keep it. The text is a [`Secret`]: it prints as nothing.
    SaveToken {
        provider: Provider,
        token: Secret,
    },
    /// Deck mode: the Save token row was pressed (internal; `DeckApp` holds the typed text and turns this into `SaveToken`).
    SubmitToken(Provider),
    /// Stop using the saved token and forget it.
    RemoveToken(Provider),
    /// Ask the service whether the token works and how many requests it allows.
    CheckToken(Provider),
    /// Show the folder the log files are in.
    OpenLogFolder,
    /// Pull the newest commits of the checkout this copy was built from and rebuild it (`scripts/update.sh`).
    UpdateSources,
    /// Write a report of this installation (versions, settings, what each service answers, the end of the log) and show where.
    SaveDiagnostics,
    /// Stop a download or install. The host answers with an `ActivityEvent::Cancelled`.
    CancelActivity(ActivityId),
    /// Remove a finished or failed row from the Downloads list.
    DismissActivity(ActivityId),
    DismissNotice(NoticeId),
    DismissAllNotices,
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
    /// Show a page. Deck's reducer asks for this when it moves; the Deck frame hands it to the router
    /// and the host never sees it.
    Navigate(crate::nav::Route),
    /// Go back one page, as the router sees it. Same path as `Navigate`.
    Back,
    /// A launch setting changed: for one game (`app`) or the defaults (`None`). `value: None` clears
    /// the choice so the next level decides. The `Shell` has already applied it to the store, which
    /// saves it.
    LaunchSetting {
        app: Option<u32>,
        key: SettingKey,
        value: Option<SettingValue>,
    },
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
    /// A notification toast appeared (true) or went away (false): while it is up the pad reader treats
    /// X and Y as holds, so a tap still does its usual job on release.
    NoticeHolds(bool),
    /// Minimize, maximize, close, fill a monitor. The `Shell` carries it out; the host may also watch.
    Window(crate::window::WindowCommand),
}
