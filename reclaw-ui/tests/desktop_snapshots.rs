//! Headless renders of the Library page at each layout class and theme, written to
//! target/snapshots/*.png for visual review. Also asserts the right layout class was selected.
mod common;

use common::*;
use reclaw_ui::{metrics::Density, shell::DevOverrides, theme::ThemeKind};

fn snapshot(name: &str, width: f32, height: f32, theme: ThemeKind, density: Option<Density>) {
    let mut s = Mount::desktop().size(width, height).theme(theme).dev(DevOverrides { density, ..DevOverrides::default() }).start();
    s.snapshot(name);
}

#[test]
fn library_midnight_all_classes() {
    snapshot("wide-midnight", 1100., 700., ThemeKind::Midnight, None);
    snapshot("compact-midnight", 860., 640., ThemeKind::Midnight, None);
    snapshot("phone-midnight", 390., 780., ThemeKind::Midnight, None);
    // A handheld: wide layout, touch density.
    snapshot("handheld-midnight", 1280., 800., ThemeKind::Midnight, Some(Density::Touch));
}

#[test]
fn library_daylight_all_classes() {
    snapshot("wide-daylight", 1100., 700., ThemeKind::Daylight, None);
    snapshot("phone-daylight", 390., 780., ThemeKind::Daylight, None);
}

/// States that need a pointer in the real app, rendered directly.
#[test]
fn component_states() {
    use freya::prelude::*;
    use freya_core::element::AppComponent;
    use freya_testing::prelude::*;
    use reclaw_ui::{prelude::*, sample::sample_games};
    let games = sample_games();
    let state_sheet = {
        let g = games.clone();
        move || {
            use_init_reclaw(ThemeKind::Midnight);
            let t = use_reclaw();
            rect()
                .vertical()
                .spacing(16.)
                .padding(16.)
                .background(t.bg_base)
                .child(
                    rect()
                        .horizontal()
                        .spacing(12.)
                        .child(GameCapsule::new(g[0].clone()))
                        .child(GameCapsule::new(g[0].clone()).hovered(true))
                        .child(GameCapsule::new(g[2].clone()).selected(true))
                        .child(GameCapsule::new(g[3].clone())),
                )
                .child(
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .child(ActionButton::install().icon(IconName::Play).label("Play").size(ButtonSize::Lg))
                        .child(ActionButton::new(ButtonVariant::Primary).label("Check for updates"))
                        .child(ActionButton::new(ButtonVariant::Secondary).label("Manage"))
                        .child(ActionButton::new(ButtonVariant::Ghost).icon(IconName::Folder).label("Open folder"))
                        .child(ActionButton::new(ButtonVariant::Danger).label("Uninstall"))
                        .child(ActionButton::install().label("Disabled").enabled(false)),
                )
                .child(
                    rect().horizontal().spacing(8.).children(
                        [
                            AppStatus::Installed,
                            AppStatus::UpdateReady,
                            AppStatus::Installing,
                            AppStatus::Failed,
                            AppStatus::Available,
                            AppStatus::NeedsFile,
                        ]
                        .map(|s| StatusBadge::new(s).into_element()),
                    ),
                )
        }
    };
    let (mut runner, ()) = TestingRunner::new(AppComponent::from(state_sheet), (860., 420.).into(), |_| {}, 1.);
    for _ in 0..3 {
        runner.sync_and_update();
    }
    runner.render_to_file(out_dir().join("states-midnight.png"));
}
