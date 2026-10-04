//! Deck mode's brain: which screen, which overlay, what is focused, who owns the pad, and what the
//! host should do. Pure data in, pure data out: no Freya types, so it is tested like any logic.
use std::collections::HashMap;

use reclaw_input::{
    Action, ControllerKind, Direction, FocusId, FocusNode, InputOwner, Rect, next_focus,
};

use super::launch::{LaunchVerb, launch_verb, shows_stop_pair};
use crate::{metrics::*, model::*};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Section {
    Library,
    Catalog,
    Downloads,
    Mods,
}

impl Section {
    pub const ALL: [Section; 4] = [Self::Library, Self::Catalog, Self::Downloads, Self::Mods];

    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Catalog => "Catalog",
            Self::Downloads => "Downloads",
            Self::Mods => "Mods",
        }
    }

    fn step(self, forward: bool) -> Self {
        let i = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        let n = Self::ALL.len();
        Self::ALL[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Home,
    Game(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Overlay {
    None,
    MainMenu,
    QuickAccess,
}

/// Drives which hint glyphs show and whether the focus ring is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LastInput {
    Gamepad(ControllerKind),
    Keyboard,
    Pointer,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Scope {
    Home(Section),
    Game(u32),
    MainMenu,
    QuickAccess,
}

/// What the host must do. The state never touches processes or windows itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    Launch(u32),
    /// Bring the running app's window forward and give it the pad.
    Resume(u32),
    /// Graceful quit; pressed again while stopping, the supervisor force-kills.
    Stop(u32),
    Install(u32),
    OpenFolder(u32),
    Manage(u32),
    CancelDownload(u32),
    Search,
    SwitchToDesktop,
    BringLauncherToFront,
    SendLauncherToBack,
    InputOwner(InputOwner),
}

pub struct DeckView<'a> {
    pub games: &'a [GameEntry],
    pub downloads: &'a [Download],
}

impl DeckView<'_> {
    pub fn game(&self, id: u32) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.id == id)
    }

    /// The app shown in the Now Playing banner: the first one that is starting, running or stopping.
    pub fn active_game(&self) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.run.is_active())
    }
}

/// Focus ids. Tiles encode (shelf, game); everything else is a small fixed id.
pub mod ids {
    use reclaw_input::FocusId;

    pub const BANNER_RESUME: FocusId = FocusId(1);
    pub const BANNER_STOP: FocusId = FocusId(2);
    pub const GAME_PRIMARY: FocusId = FocusId(10);
    pub const GAME_STOP: FocusId = FocusId(11);
    pub const GAME_FOLDER: FocusId = FocusId(12);
    pub const GAME_MANAGE: FocusId = FocusId(13);
    pub const QA_RESUME: FocusId = FocusId(20);
    pub const QA_STOP: FocusId = FocusId(21);
    pub const QA_DOWNLOADS: FocusId = FocusId(22);

    pub fn tile(shelf: usize, game: u32) -> FocusId {
        FocusId(0x1000_0000 | ((shelf as u32) << 20) | game)
    }

    pub fn menu(index: usize) -> FocusId {
        FocusId(0x2000_0000 | index as u32)
    }

    pub fn download_cancel(app: u32) -> FocusId {
        FocusId(0x4000_0000 | app)
    }

    /// Game id out of a tile id, if it is one.
    pub fn tile_game(id: FocusId) -> Option<u32> {
        (id.0 & 0xF000_0000 == 0x1000_0000).then_some(id.0 & 0x000F_FFFF)
    }

    pub fn tile_shelf(id: FocusId) -> Option<usize> {
        (id.0 & 0xF000_0000 == 0x1000_0000).then_some(((id.0 >> 20) & 0xFF) as usize)
    }

    pub fn menu_index(id: FocusId) -> Option<usize> {
        (id.0 & 0xF000_0000 == 0x2000_0000).then_some((id.0 & 0xFFFF) as usize)
    }

    pub fn cancel_app(id: FocusId) -> Option<u32> {
        (id.0 & 0xF000_0000 == 0x4000_0000).then_some(id.0 & 0x0FFF_FFFF)
    }
}

/// Entries of the main menu, top to bottom.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuEntry {
    Section(Section),
    Settings,
    SwitchToDesktop,
}

pub const MENU: [MenuEntry; 6] = [
    MenuEntry::Section(Section::Library),
    MenuEntry::Section(Section::Catalog),
    MenuEntry::Section(Section::Downloads),
    MenuEntry::Section(Section::Mods),
    MenuEntry::Settings,
    MenuEntry::SwitchToDesktop,
];

pub struct ShelfSpec {
    pub title: &'static str,
    pub games: Vec<u32>,
}

/// "Continue" (installed or active) then "All apps". An empty Continue shelf is left out.
pub fn shelves(view: &DeckView) -> Vec<ShelfSpec> {
    let cont: Vec<u32> = view
        .games
        .iter()
        .filter(|g| g.status.is_installed() || g.run.is_active())
        .map(|g| g.id)
        .collect();
    let mut out = Vec::new();
    if !cont.is_empty() {
        out.push(ShelfSpec {
            title: "Continue",
            games: cont,
        });
    }
    out.push(ShelfSpec {
        title: "All apps",
        games: view.games.iter().map(|g| g.id).collect(),
    });
    out
}

pub const BANNER_H: f32 = 72.;
pub const BANNER_BLOCK: f32 = BANNER_H + 24.;
pub const SHELF_TITLE_BLOCK: f32 = 44.;
pub const SHELF_H: f32 = SHELF_TITLE_BLOCK + DECK_TILE_H + DECK_CAPTION_H + 32.;

pub fn shelf_top(index: usize, banner: bool) -> f32 {
    (if banner { BANNER_BLOCK } else { 0. }) + index as f32 * SHELF_H
}

pub fn tile_rect(shelf: usize, tile: usize, banner: bool) -> Rect {
    Rect::new(
        tile as f32 * (DECK_TILE_W + DECK_TILE_GAP),
        shelf_top(shelf, banner) + SHELF_TITLE_BLOCK,
        DECK_TILE_W,
        DECK_TILE_H + DECK_CAPTION_H,
    )
}

#[derive(Clone, Debug)]
pub struct DeckState {
    section: Section,
    screen: Screen,
    overlay: Overlay,
    focus: FocusId,
    memory: HashMap<Scope, FocusId>,
    in_front: bool,
    /// The overlay was opened over a running app (via Guide): closing it should send us back.
    came_from_game: bool,
    last_input: LastInput,
    owner: InputOwner,
}

impl DeckState {
    pub fn new(view: &DeckView) -> Self {
        let mut s = Self {
            section: Section::Library,
            screen: Screen::Home,
            overlay: Overlay::None,
            focus: FocusId(0),
            memory: HashMap::new(),
            in_front: true,
            came_from_game: false,
            last_input: LastInput::Gamepad(ControllerKind::Generic),
            owner: InputOwner::Launcher,
        };
        s.focus = s.default_focus(s.scope(), view);
        s.owner = s.input_owner(view);
        s
    }

    pub fn section(&self) -> Section {
        self.section
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn overlay(&self) -> Overlay {
        self.overlay
    }

    pub fn focus(&self) -> FocusId {
        self.focus
    }

    pub fn last_input(&self) -> LastInput {
        self.last_input
    }

    pub fn set_last_input(&mut self, last: LastInput) {
        self.last_input = last;
    }

    pub fn in_front(&self) -> bool {
        self.in_front
    }

    /// The ring is drawn only while the gamepad or keyboard is driving.
    pub fn focus_visible(&self) -> bool {
        !matches!(self.last_input, LastInput::Pointer)
    }

    /// Who should get the gamepad right now.
    pub fn input_owner(&self, view: &DeckView) -> InputOwner {
        if view.active_game().is_some() && !self.in_front {
            InputOwner::App
        } else {
            InputOwner::Launcher
        }
    }

    fn scope(&self) -> Scope {
        match (self.overlay, self.screen) {
            (Overlay::MainMenu, _) => Scope::MainMenu,
            (Overlay::QuickAccess, _) => Scope::QuickAccess,
            (Overlay::None, Screen::Game(id)) => Scope::Game(id),
            (Overlay::None, Screen::Home) => Scope::Home(self.section),
        }
    }

    // ---- declared focus layout ------------------------------------------------------------

    pub fn nodes(&self, view: &DeckView) -> Vec<FocusNode> {
        self.nodes_for(self.scope(), view)
    }

    fn nodes_for(&self, scope: Scope, view: &DeckView) -> Vec<FocusNode> {
        let node = |id, x, y, w, h| FocusNode {
            id,
            rect: Rect::new(x, y, w, h),
        };
        match scope {
            Scope::MainMenu => (0..MENU.len())
                .map(|i| {
                    node(
                        ids::menu(i),
                        0.,
                        i as f32 * DECK_ROW_H,
                        DECK_PANEL_W,
                        DECK_ROW_H,
                    )
                })
                .collect(),
            Scope::QuickAccess => {
                let mut v = Vec::new();
                if view.active_game().is_some() {
                    v.push(node(ids::QA_RESUME, 0., 0., 190., DECK_TARGET_MIN));
                    v.push(node(ids::QA_STOP, 206., 0., 150., DECK_TARGET_MIN));
                }
                v.push(node(ids::QA_DOWNLOADS, 0., 200., DECK_PANEL_W, DECK_ROW_H));
                v
            }
            Scope::Game(id) => {
                let mut v = vec![node(ids::GAME_PRIMARY, 0., 0., 220., DECK_TARGET_MIN)];
                let controller = true;
                if view
                    .game(id)
                    .is_some_and(|g| shows_stop_pair(&g.run, controller))
                {
                    v.push(node(ids::GAME_STOP, 236., 0., 150., DECK_TARGET_MIN));
                }
                v.push(node(ids::GAME_FOLDER, 0., 80., 220., DECK_TARGET_MIN));
                v.push(node(ids::GAME_MANAGE, 236., 80., 150., DECK_TARGET_MIN));
                v
            }
            Scope::Home(Section::Library) => {
                let banner = view.active_game().is_some();
                let mut v = Vec::new();
                if banner {
                    v.push(node(ids::BANNER_RESUME, 0., 0., 190., BANNER_H));
                    v.push(node(ids::BANNER_STOP, 206., 0., 150., BANNER_H));
                }
                for (s, shelf) in shelves(view).iter().enumerate() {
                    for (t, game) in shelf.games.iter().enumerate() {
                        v.push(FocusNode {
                            id: ids::tile(s, *game),
                            rect: tile_rect(s, t, banner),
                        });
                    }
                }
                v
            }
            Scope::Home(Section::Downloads) => view
                .downloads
                .iter()
                .enumerate()
                .map(|(i, d)| {
                    node(
                        ids::download_cancel(d.app_id),
                        0.,
                        i as f32 * 96.,
                        DECK_TARGET_MIN,
                        DECK_TARGET_MIN,
                    )
                })
                .collect(),
            Scope::Home(_) => Vec::new(),
        }
    }

    fn default_focus(&self, scope: Scope, view: &DeckView) -> FocusId {
        let nodes = self.nodes_for(scope, view);
        let first = nodes.first().map(|n| n.id).unwrap_or(FocusId(0));
        match scope {
            Scope::MainMenu => ids::menu(
                MENU.iter()
                    .position(|m| *m == MenuEntry::Section(self.section))
                    .unwrap_or(0),
            ),
            Scope::Home(_) if view.active_game().is_some() => {
                nodes.first().map(|n| n.id).unwrap_or(first)
            }
            Scope::Home(_) => first,
            _ => first,
        }
    }

    fn remembered(&self, scope: Scope, view: &DeckView) -> FocusId {
        let nodes = self.nodes_for(scope, view);
        match self.memory.get(&scope) {
            Some(id) if nodes.iter().any(|n| n.id == *id) => *id,
            _ => self.default_focus(scope, view),
        }
    }

    fn enter_scope(&mut self, view: &DeckView) {
        self.focus = self.remembered(self.scope(), view);
    }

    fn leave_scope(&mut self) {
        self.memory.insert(self.scope(), self.focus);
    }

    // ---- input ----------------------------------------------------------------------------

    pub fn apply(&mut self, action: Action, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.apply_inner(action, view, &mut fx);
        self.sync_owner(view, &mut fx);
        fx
    }

    /// Pointer click on a focus target: focus it and confirm.
    pub fn click(&mut self, id: FocusId, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.last_input = LastInput::Pointer;
        if self.nodes(view).iter().any(|n| n.id == id) {
            self.focus = id;
            self.activate(id, view, &mut fx);
        }
        self.sync_owner(view, &mut fx);
        fx
    }

    /// Call when the host's data changed (a run state moved, a download finished). Repairs focus
    /// that points at something that no longer exists and reports an ownership change.
    pub fn sync(&mut self, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        if matches!(self.screen, Screen::Game(id) if view.game(id).is_none()) {
            self.screen = Screen::Home;
        }
        if !self.nodes(view).iter().any(|n| n.id == self.focus) {
            self.focus = self.remembered(self.scope(), view);
        }
        self.sync_owner(view, &mut fx);
        fx
    }

    /// The running app ended: the launcher is the foreground again and owns the pad.
    pub fn on_app_ended(&mut self, view: &DeckView) -> Vec<Effect> {
        self.in_front = true;
        self.came_from_game = false;
        let mut fx = vec![Effect::BringLauncherToFront];
        fx.extend(self.sync(view));
        fx
    }

    fn sync_owner(&mut self, view: &DeckView, fx: &mut Vec<Effect>) {
        let owner = self.input_owner(view);
        if owner != self.owner {
            self.owner = owner;
            fx.push(Effect::InputOwner(owner));
        }
    }

    fn apply_inner(&mut self, action: Action, view: &DeckView, fx: &mut Vec<Effect>) {
        match action {
            Action::Navigate(dir) => self.navigate(dir, view),
            Action::PageUp => (0..4).for_each(|_| self.navigate(Direction::Left, view)),
            Action::PageDown => (0..4).for_each(|_| self.navigate(Direction::Right, view)),
            Action::Confirm => {
                let id = self.focus;
                self.activate(id, view, fx);
            }
            Action::Back => self.back(view, fx),
            Action::MainMenu => self.toggle_overlay(Overlay::MainMenu, view, fx),
            Action::QuickAccess => self.toggle_overlay(Overlay::QuickAccess, view, fx),
            Action::PrevSection | Action::NextSection => {
                if self.overlay == Overlay::None && self.screen == Screen::Home {
                    self.leave_scope();
                    self.section = self.section.step(action == Action::NextSection);
                    self.enter_scope(view);
                }
            }
            Action::Secondary => {
                let target = match (self.screen, ids::tile_game(self.focus)) {
                    (Screen::Game(id), _) => Some(id),
                    (Screen::Home, Some(id)) => Some(id),
                    _ => None,
                };
                if let (Overlay::None, Some(id)) = (self.overlay, target) {
                    fx.push(Effect::Manage(id));
                }
            }
            Action::Tertiary => fx.push(Effect::Search),
            // Reserved: the context menu is specified but not built.
            Action::Options => {}
        }
    }

    fn navigate(&mut self, dir: Direction, view: &DeckView) {
        if let Some(next) = next_focus(&self.nodes(view), self.focus, dir) {
            self.focus = next;
        }
    }

    fn back(&mut self, view: &DeckView, fx: &mut Vec<Effect>) {
        if self.overlay != Overlay::None {
            self.close_overlay(view, fx);
        } else if matches!(self.screen, Screen::Game(_)) {
            self.leave_scope();
            self.screen = Screen::Home;
            self.enter_scope(view);
        }
    }

    fn toggle_overlay(&mut self, which: Overlay, view: &DeckView, fx: &mut Vec<Effect>) {
        if self.overlay == which {
            self.close_overlay(view, fx);
            return;
        }
        if self.overlay != Overlay::None {
            // Switching straight from one panel to the other.
            self.leave_scope();
            self.overlay = which;
            self.enter_scope(view);
            return;
        }
        if view.active_game().is_some() && !self.in_front {
            self.in_front = true;
            self.came_from_game = true;
            fx.push(Effect::BringLauncherToFront);
        }
        self.leave_scope();
        self.overlay = which;
        self.enter_scope(view);
    }

    fn close_overlay(&mut self, view: &DeckView, fx: &mut Vec<Effect>) {
        self.leave_scope();
        self.overlay = Overlay::None;
        self.enter_scope(view);
        if self.came_from_game && view.active_game().is_some() {
            self.came_from_game = false;
            self.in_front = false;
            fx.push(Effect::SendLauncherToBack);
        }
    }

    fn activate(&mut self, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        if let Some(app) = view.active_game().map(|g| g.id) {
            if id == ids::BANNER_RESUME || id == ids::QA_RESUME {
                return self.resume(app, view, fx);
            }
            if id == ids::BANNER_STOP || id == ids::QA_STOP {
                return fx.push(Effect::Stop(app));
            }
        }
        if let Some(game) = ids::tile_game(id) {
            self.leave_scope();
            self.screen = Screen::Game(game);
            self.enter_scope(view);
            return;
        }
        if let Some(index) = ids::menu_index(id) {
            return self.activate_menu(MENU.get(index).copied(), view, fx);
        }
        if let Some(app) = ids::cancel_app(id) {
            return fx.push(Effect::CancelDownload(app));
        }
        if id == ids::QA_DOWNLOADS {
            return self.go_to_section(Section::Downloads, view, fx);
        }
        if let Screen::Game(game_id) = self.screen {
            let Some(game) = view.game(game_id) else {
                return;
            };
            match id {
                i if i == ids::GAME_PRIMARY => self.primary(game, view, fx),
                i if i == ids::GAME_STOP => fx.push(Effect::Stop(game_id)),
                i if i == ids::GAME_FOLDER => fx.push(Effect::OpenFolder(game_id)),
                i if i == ids::GAME_MANAGE => fx.push(Effect::Manage(game_id)),
                _ => {}
            }
        }
    }

    fn primary(&mut self, game: &GameEntry, view: &DeckView, fx: &mut Vec<Effect>) {
        match launch_verb(game.status, &game.run, true) {
            LaunchVerb::Play => {
                self.in_front = false;
                fx.push(Effect::Launch(game.id));
            }
            LaunchVerb::Retry if game.status.is_installed() => {
                self.in_front = false;
                fx.push(Effect::Launch(game.id));
            }
            LaunchVerb::Retry | LaunchVerb::Install | LaunchVerb::Update => {
                fx.push(Effect::Install(game.id))
            }
            LaunchVerb::Resume | LaunchVerb::Stop => self.resume(game.id, view, fx),
            LaunchVerb::ForceQuit => fx.push(Effect::Stop(game.id)),
            LaunchVerb::Installing | LaunchVerb::Starting => {}
        }
    }

    fn resume(&mut self, app: u32, view: &DeckView, fx: &mut Vec<Effect>) {
        if self.overlay != Overlay::None {
            self.leave_scope();
            self.overlay = Overlay::None;
            self.enter_scope(view);
        }
        self.came_from_game = false;
        self.in_front = false;
        fx.push(Effect::Resume(app));
    }

    fn activate_menu(&mut self, entry: Option<MenuEntry>, view: &DeckView, fx: &mut Vec<Effect>) {
        match entry {
            Some(MenuEntry::Section(s)) => self.go_to_section(s, view, fx),
            Some(MenuEntry::SwitchToDesktop) => fx.push(Effect::SwitchToDesktop),
            Some(MenuEntry::Settings) | None => {}
        }
    }

    fn go_to_section(&mut self, section: Section, view: &DeckView, fx: &mut Vec<Effect>) {
        // Leaving for another page means the user is browsing, not returning to the game.
        self.came_from_game = false;
        let _ = fx;
        self.leave_scope();
        self.overlay = Overlay::None;
        self.screen = Screen::Home;
        self.section = section;
        self.enter_scope(view);
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use reclaw_runtime::Outcome;

    use super::*;
    use crate::sample::{sample_downloads, sample_games};

    fn running() -> RunState {
        RunState::Running {
            pid: 7,
            since: SystemTime::now(),
        }
    }

    struct Fixture {
        games: Vec<GameEntry>,
        downloads: Vec<Download>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                games: sample_games(),
                downloads: sample_downloads(),
            }
        }

        fn view(&self) -> DeckView<'_> {
            DeckView {
                games: &self.games,
                downloads: &self.downloads,
            }
        }

        fn set_run(&mut self, id: u32, run: RunState) {
            self.games.iter_mut().find(|g| g.id == id).unwrap().run = run;
        }
    }

    fn tile(shelf: usize, game: u32) -> FocusId {
        ids::tile(shelf, game)
    }

    #[test]
    fn starts_on_the_first_tile() {
        let f = Fixture::new();
        let s = DeckState::new(&f.view());
        assert_eq!(s.focus(), tile(0, 1));
        assert_eq!(
            (s.screen(), s.overlay(), s.section()),
            (Screen::Home, Overlay::None, Section::Library)
        );
    }

    #[test]
    fn moves_along_a_shelf_and_down_to_the_next() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::Navigate(Direction::Right), &f.view());
        assert_eq!(s.focus(), tile(0, 2));
        s.apply(Action::Navigate(Direction::Down), &f.view());
        // The "All apps" shelf lists games 1..6 in order, so game 2 is directly underneath.
        assert_eq!(ids::tile_shelf(s.focus()), Some(1));
        assert_eq!(ids::tile_game(s.focus()), Some(2));
        s.apply(Action::Navigate(Direction::Up), &f.view());
        assert_eq!(s.focus(), tile(0, 2));
    }

    #[test]
    fn edges_do_not_wrap() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::Navigate(Direction::Left), &f.view());
        s.apply(Action::Navigate(Direction::Up), &f.view());
        assert_eq!(s.focus(), tile(0, 1));
    }

    #[test]
    fn confirm_opens_the_game_and_back_restores_focus() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::Navigate(Direction::Right), &f.view());
        s.apply(Action::Confirm, &f.view());
        assert_eq!(s.screen(), Screen::Game(2));
        assert_eq!(s.focus(), ids::GAME_PRIMARY);
        s.apply(Action::Back, &f.view());
        assert_eq!(s.screen(), Screen::Home);
        assert_eq!(s.focus(), tile(0, 2), "returns to the tile it left");
    }

    #[test]
    fn back_on_home_does_nothing() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        assert!(s.apply(Action::Back, &f.view()).is_empty());
        assert_eq!(s.screen(), Screen::Home);
    }

    #[test]
    fn play_launches_and_hands_the_pad_to_the_app() {
        let mut f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::Confirm, &f.view()); // open Starfall 64 (installed)
        let fx = s.apply(Action::Confirm, &f.view());
        assert_eq!(fx, vec![Effect::Launch(1)]);
        assert!(!s.in_front());
        // The host reports the run; syncing flips ownership.
        f.set_run(1, running());
        assert_eq!(s.sync(&f.view()), vec![Effect::InputOwner(InputOwner::App)]);
        assert_eq!(s.input_owner(&f.view()), InputOwner::App);
    }

    #[test]
    fn guide_over_a_running_app_brings_us_forward_and_back_closes_it_again() {
        let mut f = Fixture::new();
        f.set_run(1, running());
        let mut s = DeckState::new(&f.view());
        s.in_front = false;
        s.owner = InputOwner::App;

        let fx = s.apply(Action::MainMenu, &f.view());
        assert_eq!(
            fx,
            vec![
                Effect::BringLauncherToFront,
                Effect::InputOwner(InputOwner::Launcher)
            ]
        );
        assert_eq!(s.overlay(), Overlay::MainMenu);

        let fx = s.apply(Action::Back, &f.view());
        assert_eq!(
            fx,
            vec![
                Effect::SendLauncherToBack,
                Effect::InputOwner(InputOwner::App)
            ]
        );
        assert_eq!(s.overlay(), Overlay::None);
    }

    #[test]
    fn resume_from_quick_access_returns_to_the_game() {
        let mut f = Fixture::new();
        f.set_run(1, running());
        let mut s = DeckState::new(&f.view());
        s.in_front = false;
        s.owner = InputOwner::App;
        s.apply(Action::QuickAccess, &f.view());
        assert_eq!(
            s.focus(),
            ids::QA_RESUME,
            "Resume is the default focus while an app runs"
        );
        let fx = s.apply(Action::Confirm, &f.view());
        assert_eq!(
            fx,
            vec![Effect::Resume(1), Effect::InputOwner(InputOwner::App)]
        );
        assert_eq!(s.overlay(), Overlay::None);
    }

    #[test]
    fn stop_and_force_stop_are_the_same_effect() {
        let mut f = Fixture::new();
        f.set_run(1, running());
        let mut s = DeckState::new(&f.view());
        s.apply(Action::Navigate(Direction::Right), &f.view());
        assert_eq!(s.focus(), ids::BANNER_STOP);
        assert_eq!(s.apply(Action::Confirm, &f.view()), vec![Effect::Stop(1)]);
        f.set_run(1, RunState::Stopping { pid: 7 });
        assert_eq!(
            s.apply(Action::Confirm, &f.view()),
            vec![Effect::Stop(1)],
            "pressed again: the supervisor force-kills"
        );
    }

    #[test]
    fn game_page_shows_resume_and_stop_for_a_running_app() {
        let mut f = Fixture::new();
        f.set_run(1, running());
        let mut s = DeckState::new(&f.view());
        // Banner is first; go down to the tile, open it.
        s.apply(Action::Navigate(Direction::Down), &f.view());
        s.apply(Action::Confirm, &f.view());
        assert_eq!(s.screen(), Screen::Game(1));
        s.apply(Action::Navigate(Direction::Right), &f.view());
        assert_eq!(s.focus(), ids::GAME_STOP);
        s.apply(Action::Navigate(Direction::Left), &f.view());
        assert_eq!(
            s.apply(Action::Confirm, &f.view()),
            vec![Effect::Resume(1), Effect::InputOwner(InputOwner::App)],
            "Resume hands the pad back to the app"
        );
    }

    #[test]
    fn app_ending_brings_the_launcher_back() {
        let mut f = Fixture::new();
        f.set_run(1, running());
        let mut s = DeckState::new(&f.view());
        s.in_front = false;
        s.owner = InputOwner::App;
        f.set_run(
            1,
            RunState::Failed(Outcome::ExitedWithCode {
                code: 2,
                ran_for: Default::default(),
            }),
        );
        let fx = s.on_app_ended(&f.view());
        assert_eq!(
            fx,
            vec![
                Effect::BringLauncherToFront,
                Effect::InputOwner(InputOwner::Launcher)
            ]
        );
    }

    #[test]
    fn bumpers_cycle_sections_and_remember_focus() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::Navigate(Direction::Right), &f.view());
        s.apply(Action::NextSection, &f.view());
        assert_eq!(s.section(), Section::Catalog);
        s.apply(Action::PrevSection, &f.view());
        assert_eq!(s.section(), Section::Library);
        assert_eq!(s.focus(), tile(0, 2));
        s.apply(Action::PrevSection, &f.view());
        assert_eq!(s.section(), Section::Mods, "wraps around the tabs");
    }

    #[test]
    fn menu_traps_focus_and_navigates() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::MainMenu, &f.view());
        assert_eq!(s.focus(), ids::menu(0));
        s.apply(Action::Navigate(Direction::Down), &f.view());
        s.apply(Action::Navigate(Direction::Down), &f.view());
        s.apply(Action::Confirm, &f.view());
        assert_eq!(
            (s.section(), s.overlay()),
            (Section::Downloads, Overlay::None)
        );
        // A tile on the Library page is not reachable from the menu: the trap held.
        s.apply(Action::MainMenu, &f.view());
        assert!(ids::menu_index(s.focus()).is_some());
    }

    #[test]
    fn switching_to_desktop_is_an_effect() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::MainMenu, &f.view());
        for _ in 0..5 {
            s.apply(Action::Navigate(Direction::Down), &f.view());
        }
        assert_eq!(
            s.apply(Action::Confirm, &f.view()),
            vec![Effect::SwitchToDesktop]
        );
    }

    #[test]
    fn downloads_cancel_and_quick_access_shortcut() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::QuickAccess, &f.view());
        s.apply(Action::Confirm, &f.view());
        assert_eq!(s.section(), Section::Downloads);
        assert_eq!(s.focus(), ids::download_cancel(2));
        assert_eq!(
            s.apply(Action::Confirm, &f.view()),
            vec![Effect::CancelDownload(2)]
        );
    }

    #[test]
    fn pointer_click_focuses_and_confirms_and_hides_the_ring() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        assert!(s.focus_visible());
        s.click(tile(1, 3), &f.view());
        assert_eq!(s.screen(), Screen::Game(3));
        assert!(!s.focus_visible(), "mouse use hides the focus ring");
        s.set_last_input(LastInput::Gamepad(ControllerKind::Xbox));
        assert!(s.focus_visible());
    }

    #[test]
    fn focus_repairs_itself_when_its_target_disappears() {
        let mut f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.apply(Action::Navigate(Direction::Right), &f.view());
        f.games.retain(|g| g.id != 2);
        s.sync(&f.view());
        assert!(s.nodes(&f.view()).iter().any(|n| n.id == s.focus()));
    }

    #[test]
    fn secondary_manages_the_focused_game() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        assert_eq!(
            s.apply(Action::Secondary, &f.view()),
            vec![Effect::Manage(1)]
        );
    }

    #[test]
    fn needs_file_routes_to_install() {
        let f = Fixture::new();
        let mut s = DeckState::new(&f.view());
        s.click(tile(1, 3), &f.view()); // Kart Ruins, NeedsFile
        assert_eq!(
            s.apply(Action::Confirm, &f.view()),
            vec![Effect::Install(3)]
        );
    }
}
