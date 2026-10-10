use freya::prelude::*;

use super::{
    actions::press_point,
    dialogs::{GameDialogs, GameDialogsLayer, OpenPicker},
    pages::library::Filter,
    search::{DesktopSearch, OPEN_WIDTH, button_size},
    ui::{DesktopEnv, DesktopUi},
};
use crate::{
    components::{Nav as NavBar, NavMode, NavTarget},
    effect::Effect,
    mod_games::{chosen, mod_games},
    nav::{Route, RouteStage, Section, use_nav},
    prelude::*,
    search::{SearchModel, SearchScope},
    shell::use_shell,
    store::{now_secs, use_activity, use_catalog_status, use_games, use_keyboard_inset, use_moddable, use_mods},
    typography::TypeStyle,
    window::{Frame, Titlebar},
};

/// The tab a navigation target searches.
fn scope_for(target: NavTarget) -> Option<SearchScope> {
    match target {
        NavTarget::Tab(section) => SearchScope::of_section(section),
        NavTarget::Settings => Some(SearchScope::Settings),
    }
}

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

        let (games, activity, keyboard_inset, mods, moddable) =
            (use_games(), use_activity(), use_keyboard_inset(), use_mods(), use_moddable());
        let catalog = use_catalog_status();
        let window = *shell.window.read();
        let class = shell.dev.layout.unwrap_or_else(|| LayoutClass::from_width(window.0));
        let measured = DesktopEnv { class, density: shell.dev.density.unwrap_or_else(|| class.default_density()), window, keyboard_inset };
        let mut env = use_state(|| measured);
        env.set_if_modified(measured);

        let dialogs = GameDialogs::use_new();
        let search = use_state(SearchModel::default);
        let search_field = use_state(String::new);
        let mod_shelf = use_state(|| None);
        let mod_results_page = use_state(|| 0);
        let filter = use_state(|| Filter::All);
        let selected = use_state(|| None::<u32>);
        let platform = use_state(|| None);
        let system = use_state(|| None);
        let provider = use_state(|| None);
        let mod_game = use_state(|| None::<u32>);
        use_provide_context(move || DesktopUi {
            env,
            search,
            search_field,
            mod_shelf,
            mod_results_page,
            filter,
            selected,
            system,
            platform,
            provider,
            mod_game,
            dialogs,
        });

        let mut last = use_state(|| NavTarget::Tab(Section::Library));
        let active = target_for(&nav.current(), *last.peek());
        last.set_if_modified(active);

        let downloads = activity.counts().running as u32;
        let on_select = EventHandler::new(move |target: NavTarget| nav.open(route_for(target)));
        let bar = |mode: NavMode| NavBar::new(mode, active).downloads(downloads).on_select(on_select.clone());

        // The search box searches the tab that is showing, and in Mods, the game it shows.
        let scope = scope_for(active);
        let within = (scope == Some(SearchScope::Mods))
            .then(|| chosen(*mod_game.read(), &mod_games(&games, &moddable, &mods)).map(|g| g.title.clone()))
            .flatten();
        let search_size = button_size(class, measured.density);
        // On a narrow window the box floats at the top right of a tab's own page, over the page, so opening it moves nothing.
        // A page under a tab (a game, a mod) has its own buttons there; the box waits for the tab's page.
        let floating = (class != LayoutClass::Wide).then(|| SearchScope::root_of(&nav.current())).flatten().map(|scope| {
            let gutter = SPACE_4;
            let stage_w = window.0 - if class == LayoutClass::Compact { RAIL_W } else { 0. };
            rect()
                .position(Position::new_absolute().top(gutter).left(stage_w - gutter - search_size))
                .width(Size::px(search_size))
                .height(Size::px(search_size))
                .child(DesktopSearch {
                    scope: Some(scope),
                    within: within.clone(),
                    expanded: (stage_w - 2. * gutter).max(search_size),
                    size: search_size,
                })
        });
        let stage = rect().width(Size::flex(1.)).height(Size::flex(1.)).child(RouteStage { config, mode }).maybe_child(floating);
        let body =
            match class {
                LayoutClass::Wide => {
                    let mut toggle = shell.model;
                    let deck = ActionButton::new(ButtonVariant::Ghost)
                        .icon(IconName::Gamepad)
                        .label("Deck mode")
                        .size(ButtonSize::Md)
                        .on_press(move |_| toggle.write().toggle_mode());
                    // The pages visited lately, to jump back to: browsers have this under the back button.
                    let recent = {
                        let recents = nav.recents();
                        let (games, mods) = (games.clone(), mods.clone());
                        ActionButton::new(ButtonVariant::Ghost)
                            .icon(IconName::Clock)
                            .label("Recent")
                            .size(ButtonSize::Md)
                            .enabled(!recents.is_empty())
                            .on_press(move |e: Event<PressEventData>| {
                                let labels = recents.iter().map(|r| crate::nav::title(r, &games, &mods)).collect();
                                let routes = recents.clone();
                                let on_pick = EventHandler::new(move |i: usize| {
                                    if let Some(route) = routes.get(i) {
                                        nav.open(route.clone());
                                    }
                                });
                                dialogs.pick(OpenPicker::new("Recent pages", labels, 0, press_point(&e, (480., 60.)), on_pick));
                            })
                    };
                    let updates = games.iter().filter(|g| g.status == AppStatus::UpdateReady).count();
                    let status = rect()
                        .horizontal()
                        .content(Content::Flex)
                        .cross_align(Alignment::Center)
                        .width(Size::fill())
                        .height(Size::px(28.))
                        .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
                        .background(t.bg_deep)
                        .child(rect().width(Size::flex(1.)).child(TypeStyle::Meta.text(catalog.strip(now_secs()), t.ink_subtle)))
                        .child(TypeStyle::Mono.text(
                            format!("{} apps  {updates} {}", games.len(), if updates == 1 { "update" } else { "updates" }),
                            t.ink_subtle,
                        ));
                    rect()
                        .vertical()
                        .content(Content::Flex)
                        .expanded()
                        .child(
                            bar(NavMode::Top)
                                .actions(rect().horizontal().spacing(SPACE_2).child(recent).child(deck))
                                .trailing(DesktopSearch { scope, within, expanded: OPEN_WIDTH, size: search_size }),
                        )
                        .child(stage)
                        .child(status)
                }
                LayoutClass::Compact => rect().horizontal().content(Content::Flex).expanded().child(bar(NavMode::Rail)).child(stage),
                LayoutClass::Phone => rect().vertical().content(Content::Flex).expanded().child(stage).child(bar(NavMode::Bottom)),
            };
        // With no native border the window's title bar is ours. It is part of the desktop interface only:
        // Deck mode fills the screen and has no use for window buttons.
        let titlebar = (shell.services.window.frame == Frame::Custom).then(|| {
            let on_effect = shell.on_effect.clone();
            let page = nav.current().label();
            Titlebar::new(EventHandler::new(move |command| on_effect.call(Effect::Window(command))), shell.services.window.attached, page)
                .into_element()
        });
        rect()
            .vertical()
            .content(Content::Flex)
            .expanded()
            .background(t.bg_base)
            .maybe_child(titlebar)
            .child(rect().width(Size::fill()).height(Size::flex(1.)).child(body))
            .child(GameDialogsLayer { dialogs })
    }
}
