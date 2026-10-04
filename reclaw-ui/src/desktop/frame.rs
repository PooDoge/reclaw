use freya::prelude::*;

use super::{
    dialogs::{GameDialogs, GameDialogsLayer},
    pages::library::Filter,
    ui::{DesktopEnv, DesktopUi},
};
use crate::{
    components::{Nav as NavBar, NavMode, NavTarget},
    nav::{Route, RouteStage, Section, use_nav},
    prelude::*,
    shell::use_shell,
    store::{use_activity, use_games, use_keyboard_inset},
    typography::TypeStyle,
};

/// The navigation target a route belongs under. Pages without a tab of their own (a game, a form)
/// stay under the tab the user came from, passed as `last`.
fn target_for(route: &Route, last: NavTarget) -> NavTarget {
    match route.section() {
        Some(section) => NavTarget::Tab(section),
        None if matches!(route, Route::Settings {} | Route::SettingsSection { .. }) => NavTarget::Settings,
        None => last,
    }
}

fn route_for(target: NavTarget) -> Route {
    match target {
        NavTarget::Tab(section) => Route::of_section(section),
        NavTarget::Settings => Route::Settings {},
    }
}

/// The desktop interface's persistent part: navigation chrome at the layout class the window calls
/// for, the status bar, the dialogs layer and the stage that draws and animates the pages.
#[derive(PartialEq)]
pub struct DesktopFrame {}

impl Component for DesktopFrame {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, nav) = (use_shell(), use_nav());
        let config = *shell.transitions.read();
        let mode = shell.model.read().mode;

        let (games, activity, keyboard_inset) = (use_games(), use_activity(), use_keyboard_inset());
        let window = *shell.window.read();
        let class = shell.dev.layout.unwrap_or_else(|| LayoutClass::from_width(window.0));
        let measured = DesktopEnv { class, density: shell.dev.density.unwrap_or_else(|| class.default_density()), window, keyboard_inset };
        let mut env = use_state(|| measured);
        env.set_if_modified(measured);

        let dialogs = GameDialogs::use_new();
        let search = use_state(String::new);
        let filter = use_state(|| Filter::All);
        let selected = use_state(|| None::<u32>);
        let platform = use_state(|| None);
        let provider = use_state(|| None);
        use_provide_context(move || DesktopUi { env, search, filter, selected, platform, provider, dialogs });

        let mut last = use_state(|| NavTarget::Tab(Section::Library));
        let active = target_for(&nav.current(), *last.peek());
        last.set_if_modified(active);

        let downloads = activity.counts().running as u32;
        let on_select = EventHandler::new(move |target: NavTarget| nav.open(route_for(target)));
        let bar = |mode: NavMode| NavBar::new(mode, active).downloads(downloads).on_select(on_select.clone());

        let stage = rect().width(Size::flex(1.)).height(Size::flex(1.)).child(RouteStage { config, mode });
        let body = match class {
            LayoutClass::Wide => {
                let mut toggle = shell.model;
                let deck = ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Gamepad)
                    .label("Deck mode")
                    .size(ButtonSize::Md)
                    .on_press(move |_| toggle.write().toggle_mode());
                let updates = games.iter().filter(|g| g.status == AppStatus::UpdateReady).count();
                let status = rect()
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .width(Size::fill())
                    .height(Size::px(28.))
                    .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
                    .background(t.bg_deep)
                    .child(rect().width(Size::flex(1.)).child(TypeStyle::Meta.text("Library synced", t.ink_subtle)))
                    .child(TypeStyle::Mono.text(
                        format!("{} apps  {updates} {}", games.len(), if updates == 1 { "update" } else { "updates" }),
                        t.ink_subtle,
                    ));
                rect()
                    .vertical()
                    .content(Content::Flex)
                    .expanded()
                    .child(bar(NavMode::Top).actions(deck).trailing(SearchField::new(search)))
                    .child(stage)
                    .child(status)
            }
            LayoutClass::Compact => rect().horizontal().content(Content::Flex).expanded().child(bar(NavMode::Rail)).child(stage),
            LayoutClass::Phone => rect().vertical().content(Content::Flex).expanded().child(stage).child(bar(NavMode::Bottom)),
        };
        rect().expanded().background(t.bg_base).child(body).child(GameDialogsLayer { dialogs })
    }
}
