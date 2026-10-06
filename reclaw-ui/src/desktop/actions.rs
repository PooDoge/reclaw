//! What pressing the common desktop buttons does, shared by the pages that have them.
use freya::prelude::*;

use super::dialogs::GameDialogs;
use crate::{
    activity::FailureHint,
    effect::Effect,
    launch::{VerbCommand, launch_verb},
    model::GameEntry,
};

/// Press the game's one verb button (Install, Play, Update, Stop ...): send the command, or open the
/// install form.
pub fn press_verb(game: &GameEntry, dialogs: GameDialogs, on_effect: &EventHandler<Effect>) {
    let verb = launch_verb(game.status, &game.run, false);
    match verb.command(game.id, game.status.is_installed()) {
        VerbCommand::Send(effect) => on_effect.call(effect),
        VerbCommand::OpenInstall => dialogs.install(game.id),
        VerbCommand::Nothing => {}
    }
}

/// Where a press happened, for anchoring a menu to it. A keyboard press has no position, so the
/// caller's fallback is used.
pub fn press_point(data: &PressEventData, fallback: (f32, f32)) -> (f32, f32) {
    match data {
        PressEventData::Mouse(m) => (m.global_location.x as f32, m.global_location.y as f32),
        PressEventData::Touch(t) => (t.global_location.x as f32, t.global_location.y as f32),
        PressEventData::Keyboard(_) => fallback,
    }
}

/// Press a "Failed" label or badge: the job's report and log, or the log folder when the job is from an earlier run (the board
/// does not survive a restart; the log file does).
pub fn press_failure(hint: &FailureHint, dialogs: GameDialogs, on_effect: &EventHandler<Effect>) {
    match hint.activity {
        Some(id) => dialogs.failure_log(id),
        None => on_effect.call(Effect::OpenLogFolder),
    }
}
