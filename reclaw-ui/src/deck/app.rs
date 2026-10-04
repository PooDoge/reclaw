use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;
use reclaw_input::{Action, ActionMap, ControllerInfo, ControllerKind, FocusId};

use super::{
    Backdrop, DeckView, HintBar, LastInput, MainMenu, Overlay, PanelSide, QuickAccess, Screen,
    SectionTabs, SlidePanel,
    pages::{DownloadsPage, EmptyPage, GamePage, HomePage},
    state::{DeckState, Effect, Section},
};
use crate::{metrics::*, prelude::*};

/// Where gamepad actions arrive. The receiving end is taken once, by the app, when it mounts; the
/// type compares equal to itself so it can sit in props.
#[derive(Clone)]
pub struct ActionFeed {
    rx: Rc<RefCell<Option<UnboundedReceiver<Action>>>>,
    tx: UnboundedSender<Action>,
}

impl ActionFeed {
    pub fn new() -> (UnboundedSender<Action>, Self) {
        let (tx, rx) = unbounded();
        (
            tx.clone(),
            Self {
                rx: Rc::new(RefCell::new(Some(rx))),
                tx,
            },
        )
    }

    /// A sender for this feed, for code that only holds the feed (the host root, tests).
    pub fn sender(&self) -> UnboundedSender<Action> {
        self.tx.clone()
    }

    fn take(&self) -> Option<UnboundedReceiver<Action>> {
        self.rx.borrow_mut().take()
    }
}

impl PartialEq for ActionFeed {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Deck mode root. Owns the [`DeckState`], turns gamepad and keyboard input into actions, runs
/// them through the reducer and hands the resulting [`Effect`]s to the host.
///
/// The host owns the data (`games`, `downloads`, `controller`) and the processes; this component
/// only reads them, so a run state change from the supervisor shows up the moment the host writes it.
#[derive(Clone, PartialEq)]
pub struct DeckApp {
    pub games: State<Vec<GameEntry>>,
    pub downloads: State<Vec<Download>>,
    pub controller: State<Option<ControllerInfo>>,
    pub feed: ActionFeed,
    pub on_effect: EventHandler<Effect>,
    pub map: ActionMap,
    /// Actions applied once at startup (gallery and snapshot scenarios). Their effects are dropped.
    pub script: Vec<Action>,
}

fn key_action(key: &Key, shift: bool) -> Option<Action> {
    use reclaw_input::Direction::*;
    Some(match key {
        Key::Named(NamedKey::ArrowUp) => Action::Navigate(Up),
        Key::Named(NamedKey::ArrowDown) => Action::Navigate(Down),
        Key::Named(NamedKey::ArrowLeft) => Action::Navigate(Left),
        Key::Named(NamedKey::ArrowRight) => Action::Navigate(Right),
        Key::Named(NamedKey::Enter) => Action::Confirm,
        Key::Named(NamedKey::Escape) => Action::Back,
        Key::Named(NamedKey::PageUp) => Action::PageUp,
        Key::Named(NamedKey::PageDown) => Action::PageDown,
        Key::Named(NamedKey::Tab) if shift => Action::QuickAccess,
        Key::Named(NamedKey::Tab) => Action::MainMenu,
        Key::Character(c) if c == "[" => Action::PrevSection,
        Key::Character(c) if c == "]" => Action::NextSection,
        _ => return None,
    })
}

impl Component for DeckApp {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let games = self.games;
        let downloads = self.downloads;
        let controller = self.controller;
        let on_effect = self.on_effect.clone();

        let mut deck = {
            let script = self.script.clone();
            use_state(move || {
                let (g, d) = (games.read(), downloads.read());
                let view = DeckView {
                    games: &g,
                    downloads: &d,
                };
                let mut state = DeckState::new(&view);
                for action in script {
                    state.apply(action, &view);
                }
                state
            })
        };
        let mut window = use_state(|| (1280.0f32, 800.0f32));
        let mut was_active = use_state(|| games.read().iter().any(|g| g.run.is_active()));

        // Gamepad actions: drained by one task for the lifetime of the app.
        use_hook({
            let feed = self.feed.clone();
            let on_effect = on_effect.clone();
            move || {
                spawn(async move {
                    let Some(mut rx) = feed.take() else { return };
                    while let Some(action) = rx.next().await {
                        let kind = controller
                            .read()
                            .as_ref()
                            .map_or(ControllerKind::Generic, |c| c.kind);
                        let effects = {
                            let (g, d) = (games.read(), downloads.read());
                            let view = DeckView {
                                games: &g,
                                downloads: &d,
                            };
                            let mut state = deck.write();
                            state.set_last_input(LastInput::Gamepad(kind));
                            state.apply(action, &view)
                        };
                        effects.into_iter().for_each(|e| on_effect.call(e));
                    }
                })
            }
        });

        // Host data changed: repair focus and report an ownership change.
        use_side_effect({
            let on_effect = on_effect.clone();
            move || {
                let effects = {
                    let (g, d) = (games.read(), downloads.read());
                    let view = DeckView {
                        games: &g,
                        downloads: &d,
                    };
                    let active = view.active_game().is_some();
                    let ended = *was_active.peek() && !active;
                    was_active.set_if_modified(active);
                    // An app that just ended brings the launcher forward; otherwise just repair.
                    if ended {
                        deck.write().on_app_ended(&view)
                    } else {
                        deck.write().sync(&view)
                    }
                };
                effects.into_iter().for_each(|e| on_effect.call(e));
            }
        });

        let click = {
            let on_effect = on_effect.clone();
            EventHandler::new(move |id: FocusId| {
                let effects = {
                    let (g, d) = (games.read(), downloads.read());
                    let view = DeckView {
                        games: &g,
                        downloads: &d,
                    };
                    deck.write().click(id, &view)
                };
                effects.into_iter().for_each(|e| on_effect.call(e));
            })
        };
        let close_overlay = {
            let on_effect = on_effect.clone();
            EventHandler::new(move |()| {
                let effects = {
                    let (g, d) = (games.read(), downloads.read());
                    let view = DeckView {
                        games: &g,
                        downloads: &d,
                    };
                    deck.write().apply(Action::Back, &view)
                };
                effects.into_iter().for_each(|e| on_effect.call(e));
            })
        };

        let state = deck.read().clone();
        let (g, d) = (games.read().clone(), downloads.read().clone());
        let pad = controller.read().clone();
        let kind = pad.as_ref().map_or(ControllerKind::Generic, |c| c.kind);
        let ring = state.focus_visible();
        // Hints show the connected pad's glyphs; before any input the state's own guess is generic.
        let last_input = match state.last_input() {
            LastInput::Gamepad(_) => LastInput::Gamepad(kind),
            other => other,
        };
        let (win_w, win_h) = window();
        let page_w = win_w - 2. * DECK_SAFE_X;

        let active = g.iter().find(|x| x.run.is_active()).cloned();
        let focused_game = match state.screen() {
            Screen::Game(id) => g.iter().find(|x| x.id == id),
            Screen::Home => {
                reclaw_ui_tile_game(state.focus()).and_then(|id| g.iter().find(|x| x.id == id))
            }
        };
        let tint = if focused_game.is_some() {
            t.accent
        } else {
            t.bg_raised
        };

        let body: Element = match (state.screen(), state.section()) {
            (Screen::Game(id), _) => match g.iter().find(|x| x.id == id) {
                Some(game) => {
                    GamePage::new(game.clone(), state.focus(), ring, click.clone()).into_element()
                }
                None => rect().into_element(),
            },
            (Screen::Home, Section::Library) => {
                HomePage::new(g.clone(), state.focus(), ring, page_w, click.clone()).into_element()
            }
            (Screen::Home, Section::Downloads) => {
                DownloadsPage::new(d.clone(), state.focus(), ring, click.clone()).into_element()
            }
            (Screen::Home, Section::Catalog) => {
                EmptyPage::new("Catalog", "Community app lists will appear here.").into_element()
            }
            (Screen::Home, Section::Mods) => EmptyPage::new(
                "Mods",
                "Mods from Thunderstore and GameBanana will appear here.",
            )
            .into_element(),
        };

        let hints: Vec<(Action, &'static str)> = match (state.overlay(), state.screen()) {
            (Overlay::None, Screen::Home) => vec![
                (Action::Confirm, "Select"),
                (Action::PrevSection, "Previous"),
                (Action::NextSection, "Next"),
                (Action::QuickAccess, "Quick access"),
                (Action::MainMenu, "Menu"),
            ],
            (Overlay::None, Screen::Game(_)) => vec![
                (Action::Confirm, "Select"),
                (Action::Back, "Library"),
                (Action::Secondary, "Manage"),
                (Action::QuickAccess, "Quick access"),
            ],
            (_, _) => vec![(Action::Confirm, "Select"), (Action::Back, "Close")],
        };

        let map = self.map.clone();
        let mut hotkey_deck = deck;
        let hotkey_effects = on_effect.clone();

        rect()
            .expanded()
            .background(t.deck_bg)
            .on_sized(move |e: Event<SizedEventData>| {
                window.set_if_modified((e.area.width(), e.area.height()))
            })
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                let Some(action) = key_action(&e.key, e.modifiers.contains(Modifiers::SHIFT))
                else {
                    return;
                };
                let effects = {
                    let (g, d) = (games.read(), downloads.read());
                    let view = DeckView {
                        games: &g,
                        downloads: &d,
                    };
                    let mut state = hotkey_deck.write();
                    state.set_last_input(LastInput::Keyboard);
                    state.apply(action, &view)
                };
                effects.into_iter().for_each(|e| hotkey_effects.call(e));
            })
            .child(Backdrop::new(tint))
            .child(
                // Everything below the backdrop lives inside the safe zone.
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .vertical()
                    .content(Content::Flex)
                    .width(Size::px(win_w))
                    .height(Size::px(win_h))
                    .padding(Gaps::new(
                        DECK_SAFE_Y,
                        DECK_SAFE_X,
                        DECK_SAFE_Y,
                        DECK_SAFE_X,
                    ))
                    .child(SectionTabs::new(
                        state.section(),
                        kind,
                        last_input,
                        map.clone(),
                    ))
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.))
                            .padding(Gaps::new(SPACE_4, 0., 0., 0.))
                            .child(body),
                    )
                    .child(HintBar::new(hints, kind, last_input, map)),
            )
            .child(SlidePanel::new(
                PanelSide::Left,
                state.overlay() == Overlay::MainMenu,
                (win_w, win_h),
                close_overlay.clone(),
                MainMenu::new(state.section(), state.focus(), ring, click.clone()),
            ))
            .child(SlidePanel::new(
                PanelSide::Right,
                state.overlay() == Overlay::QuickAccess,
                (win_w, win_h),
                close_overlay,
                QuickAccess::new(active, pad, d.len(), state.focus(), ring, click),
            ))
    }
}

fn reclaw_ui_tile_game(id: FocusId) -> Option<u32> {
    super::ids::tile_game(id)
}
