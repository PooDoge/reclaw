//! Headless renders of Deck mode's main scenarios, written to target/snapshots/deck-*.png.
mod common;

use common::*;
use reclaw_input::{Action::*, ControllerInfo, Direction::*};
use reclaw_runtime::{Outcome, RunState};
use reclaw_ui::sample::sample_games;

fn shot(name: &str, mount: Mount) {
    mount.start().snapshot(&format!("deck-{name}"));
}

#[test]
fn deck_home() {
    shot("home", Mount::deck());
    shot("home-second-shelf", Mount::deck().script([Navigate(Right), Navigate(Down), Navigate(Right)]));
    shot("home-tv", Mount::deck().size(1920., 1080.));
}

#[test]
fn deck_with_a_running_app() {
    let games = with_run(sample_games(), 1, running());
    shot("home-running", Mount::deck().games(games.clone()));
    shot("quick-access-running", Mount::deck().games(games.clone()).script([QuickAccess]));
    shot("game-running", Mount::deck().games(games).script([Navigate(Down), Confirm]));
}

#[test]
fn deck_overlays_and_pages() {
    shot("main-menu", Mount::deck().script([MainMenu, Navigate(Down)]));
    shot("game-installed", Mount::deck().script([Confirm]));
    shot("downloads", Mount::deck().script([NextSection, NextSection]));
    shot("quick-access-idle", Mount::deck().pad(None).script([QuickAccess]));
}

#[test]
fn deck_states_and_glyph_sets() {
    let stopping = with_run(sample_games(), 1, RunState::Stopping { pid: 1 });
    shot("game-stopping", Mount::deck().games(stopping).script([Confirm]));
    let failed =
        with_run(sample_games(), 1, RunState::Failed(Outcome::ExitedWithCode { code: 3, ran_for: std::time::Duration::from_secs(90) }));
    shot("game-failed", Mount::deck().games(failed).script([Confirm]));
    let pads: [(&str, Option<ControllerInfo>); 2] = [("home-playstation", dualsense()), ("home-nintendo", switch_pro())];
    for (name, pad) in pads {
        shot(name, Mount::deck().pad(pad));
    }
}
