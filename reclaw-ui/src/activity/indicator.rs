use super::{
    board::ActivityBoard,
    format::{format_eta, format_rate},
    types::{Activity, Kind, Outcome, Stage},
};
use crate::model::{AppStatus, GameEntry};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndicatorKind {
    /// A newer version exists; nothing has started.
    UpdateAvailable,
    Queued,
    Downloading,
    /// Verifying, building or extracting: the files are here and being made ready.
    Installing,
    /// Finished this run.
    Done,
    Failed,
    /// Only mods are downloading for this game.
    Mods,
}

/// What a card, a row or a sidebar entry shows for one game's background work.
#[derive(Clone, PartialEq, Debug)]
pub struct Indicator {
    pub kind: IndicatorKind,
    /// 0 to 1 while a transfer with a known size runs; the bar. `None` draws no bar (or an indeterminate one).
    pub progress: Option<f32>,
    /// Short text beside the icon: "34%", "Finishing", "Updated", "Update ready", "Failed".
    pub label: String,
    /// Mod downloads running for this game, shown as "+2 mods".
    pub mods: usize,
}

impl Indicator {
    /// Whether work is happening right now, which draws the bar and the animated icon.
    pub fn is_active(&self) -> bool {
        matches!(self.kind, IndicatorKind::Queued | IndicatorKind::Downloading | IndicatorKind::Installing | IndicatorKind::Mods)
    }
}

fn running_kind(a: &Activity) -> IndicatorKind {
    match a.stage {
        Stage::Queued => IndicatorKind::Queued,
        Stage::Downloading => IndicatorKind::Downloading,
        Stage::Verifying | Stage::Extracting | Stage::Finishing => IndicatorKind::Installing,
    }
}

/// The game's own job, if any: running beats failed beats finished, and within a group the newest.
fn primary(board: &ActivityBoard, game_id: u32) -> Option<&Activity> {
    let own = || board.for_game(game_id).filter(|a| !a.is_mod());
    own()
        .filter(|a| a.is_running())
        .last()
        .or_else(|| own().filter(|a| matches!(a.outcome, Outcome::Failed { .. })).last())
        .or_else(|| own().filter(|a| a.outcome == Outcome::Finished).last())
}

/// The mod downloads running for a game, and their average progress when every size is known.
fn mod_downloads(board: &ActivityBoard, game_id: u32) -> (usize, Option<f32>) {
    let running: Vec<&Activity> = board.for_game(game_id).filter(|a| a.is_mod() && a.is_running()).collect();
    let progress = running
        .iter()
        .map(|a| a.progress())
        .collect::<Option<Vec<f32>>>()
        .filter(|p| !p.is_empty())
        .map(|p| p.iter().sum::<f32>() / p.len() as f32);
    (running.len(), progress)
}

/// What to show on a game's card or row, or `None` when nothing is going on and no update waits.
pub fn indicator_for(board: &ActivityBoard, game: &GameEntry) -> Option<Indicator> {
    let (mods, mod_progress) = mod_downloads(board, game.id);
    if let Some(a) = primary(board, game.id) {
        let (kind, label, progress) = match &a.outcome {
            Outcome::Running => {
                let kind = running_kind(a);
                let label = match (kind, a.percent()) {
                    (IndicatorKind::Installing, _) => a.stage.label().to_string(),
                    (_, Some(p)) => format!("{p}%"),
                    (IndicatorKind::Queued, None) => "Queued".to_string(),
                    (_, None) => "Downloading".to_string(),
                };
                (kind, label, a.progress())
            }
            Outcome::Finished => {
                (IndicatorKind::Done, if matches!(a.kind, Kind::Update) { "Updated" } else { "Installed" }.to_string(), None)
            }
            Outcome::Failed { .. } => (IndicatorKind::Failed, "Failed".to_string(), None),
            Outcome::Cancelled => return None,
        };
        return Some(Indicator { kind, progress, label, mods });
    }
    if game.status == AppStatus::UpdateReady {
        return Some(Indicator { kind: IndicatorKind::UpdateAvailable, progress: None, label: "Update ready".into(), mods });
    }
    (mods > 0).then(|| Indicator {
        kind: IndicatorKind::Mods,
        progress: mod_progress,
        label: if mods == 1 { "1 mod".into() } else { format!("{mods} mods") },
        mods,
    })
}

/// Which group of the sidebar an entry belongs to; the order here is the order shown.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SidebarKind {
    Active,
    Failed,
    Available,
    Done,
}

/// One game in the desktop sidebar's Updates section.
#[derive(Clone, PartialEq, Debug)]
pub struct SidebarEntry {
    pub game_id: u32,
    pub title: String,
    pub kind: SidebarKind,
    pub indicator: Indicator,
    /// The second line: "34% · 2 min left · 7.4 MB/s", "Updated to v0.9.2", "Update ready".
    pub detail: String,
}

fn detail_for(board: &ActivityBoard, game: &GameEntry, indicator: &Indicator) -> String {
    let mods = if indicator.mods > 0 && indicator.kind != IndicatorKind::Mods {
        format!(" · +{} mod{}", indicator.mods, if indicator.mods == 1 { "" } else { "s" })
    } else {
        String::new()
    };
    let main = match (indicator.kind, primary(board, game.id)) {
        (IndicatorKind::Downloading, Some(a)) => {
            let parts: Vec<String> =
                [a.percent().map(|p| format!("{p}%")), a.eta().map(format_eta), a.rate.filter(|r| *r > 0).map(format_rate)]
                    .into_iter()
                    .flatten()
                    .collect();
            if parts.is_empty() { "Downloading".to_string() } else { parts.join(" · ") }
        }
        (IndicatorKind::Queued, _) => "Waiting to start".to_string(),
        (IndicatorKind::Installing, Some(a)) => a.stage.label().to_string(),
        (IndicatorKind::Done, Some(a)) => match &a.changelog {
            Some(c) => format!("Updated to {}", c.to),
            None => {
                if matches!(a.kind, Kind::Update) {
                    "Updated".to_string()
                } else {
                    "Installed".to_string()
                }
            }
        },
        (IndicatorKind::Failed, Some(a)) => match &a.outcome {
            Outcome::Failed { reason } => reason.clone(),
            _ => "Failed".to_string(),
        },
        (IndicatorKind::UpdateAvailable, _) => "Update ready".to_string(),
        (IndicatorKind::Mods, _) => indicator.label.clone() + " downloading",
        _ => indicator.label.clone(),
    };
    main + &mods
}

/// The sidebar's Updates section: one entry per game that has an update waiting, work in progress,
/// a failure, or a result from this run. Active first, then failed, then waiting, then done; within
/// a group in the order the library lists the games. A finished update stays until the app restarts.
pub fn sidebar_entries(board: &ActivityBoard, games: &[GameEntry]) -> Vec<SidebarEntry> {
    let mut entries: Vec<SidebarEntry> = games
        .iter()
        .filter_map(|game| {
            let indicator = indicator_for(board, game)?;
            let kind = match indicator.kind {
                IndicatorKind::Queued | IndicatorKind::Downloading | IndicatorKind::Installing | IndicatorKind::Mods => SidebarKind::Active,
                IndicatorKind::Failed => SidebarKind::Failed,
                IndicatorKind::UpdateAvailable => SidebarKind::Available,
                IndicatorKind::Done => SidebarKind::Done,
            };
            let detail = detail_for(board, game, &indicator);
            Some(SidebarEntry { game_id: game.id, title: game.title.to_string(), kind, indicator, detail })
        })
        .collect();
    // Stable: equal groups keep the library's order.
    entries.sort_by_key(|e| e.kind);
    entries
}
