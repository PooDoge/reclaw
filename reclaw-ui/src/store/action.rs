use reclaw_games::{
    project::ProjectInfo,
    settings::{DisplayEnvironment, SettingKey, SettingValue},
};
use reclaw_input::ControllerInfo;
use reclaw_runtime::RunState;

use reclaw_net::Provider;

use crate::{
    activity::ActivityEvent,
    credentials::TokenStatus,
    model::{AppStatus, GameEntry, ModEntry, ModProvider, ModStatus},
    nav::Route,
    notices::{Notice, NoticeId},
    settings::{SettingChange, TextField},
};
use reclaw_config::WindowPrefs;

/// Every way the shared state can change. `Send`: the supervisor, a downloader or the gamepad
/// thread can build one and hand it to the [`StoreFeed`](super::StoreFeed).
#[derive(Clone, PartialEq, Debug)]
pub enum AppAction {
    // What the host reports.
    SetGames(Vec<GameEntry>),
    SetProjects(Vec<ProjectInfo>),
    SetMods(Vec<ModEntry>),
    /// The installed games whose mods can be browsed, by id.
    SetModdable(Vec<u32>),
    /// The catalog loader's progress and findings.
    Catalog(super::status::CatalogStatus),
    /// What is known about one service's access token.
    Credentials {
        provider: Provider,
        status: TokenStatus,
    },
    /// Tell the user something (a notice from the host that is not about one download).
    Notify(Notice),
    SetRun {
        id: u32,
        run: RunState,
    },
    SetStatus {
        id: u32,
        status: AppStatus,
    },
    SetModStatus {
        provider: ModProvider,
        id: String,
        status: ModStatus,
    },
    Activity(ActivityEvent),
    SetController(Option<ControllerInfo>),
    SetKeyboardInset(f32),
    SetDisplay(DisplayEnvironment),
    /// A page the host wants shown (a deep link, a clicked notification). The UI takes it with `Open(None)`.
    Open(Option<Route>),

    // What the user changed.
    ToggleFavorite(u32),
    Setting(SettingChange),
    SettingText {
        app: Option<u32>,
        field: TextField,
        value: String,
    },
    /// Set or (`None`) clear a launch setting, for a game or the defaults.
    LaunchSetting {
        app: Option<u32>,
        key: SettingKey,
        value: Option<SettingValue>,
    },
    MarkModInstalling {
        game: u32,
        provider: ModProvider,
        id: String,
    },
    DismissNotice(NoticeId),
    DismissAllNotices,
    Window(WindowPrefs),
}
