use freya::prelude::*;
use reclaw_input::FocusId;

use super::{
    LaunchButton, NowPlayingBanner, Shelf,
    focus::FocusFrame,
    state::{DeckView, ShelfSpec, ids, shelf_top, shelves},
};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Library home: Now Playing banner (when an app is active) over the "Continue" and "All apps"
/// shelves. The page snaps vertically to the focused shelf, like Big Picture.
#[derive(Clone, PartialEq)]
pub struct HomePage {
    games: Vec<GameEntry>,
    focus: FocusId,
    ring_visible: bool,
    viewport_w: f32,
    on_click: EventHandler<FocusId>,
}

impl HomePage {
    pub fn new(
        games: Vec<GameEntry>,
        focus: FocusId,
        ring_visible: bool,
        viewport_w: f32,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self {
            games,
            focus,
            ring_visible,
            viewport_w,
            on_click,
        }
    }
}

impl Component for HomePage {
    fn render(&self) -> impl IntoElement {
        let view = DeckView {
            games: &self.games,
            downloads: &[],
        };
        let active = view.active_game().cloned();
        let banner = active.is_some();
        let specs: Vec<ShelfSpec> = shelves(&view);

        let focused_shelf = ids::tile_shelf(self.focus);
        let offset_y = match focused_shelf {
            Some(i) if i > 0 => shelf_top(i, banner),
            _ => 0.,
        };

        let shelf_elements = specs.iter().enumerate().map(|(i, spec)| {
            let games: Vec<GameEntry> = spec
                .games
                .iter()
                .filter_map(|id| self.games.iter().find(|g| g.id == *id).cloned())
                .collect();
            Shelf::new(
                spec.title,
                i,
                games,
                self.focus,
                self.ring_visible,
                self.viewport_w,
                self.on_click.clone(),
            )
            .key(i)
            .into_element()
        });

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .overflow(Overflow::Clip)
            .child(
                rect()
                    .vertical()
                    .width(Size::fill())
                    .offset_y(-offset_y)
                    .padding(Gaps::new(8., 0., 0., 0.))
                    .maybe_child(active.map(|game| {
                        rect()
                            .padding(Gaps::new(0., 0., 24., 0.))
                            .child(NowPlayingBanner::new(
                                game,
                                self.focus,
                                self.ring_visible,
                                ids::BANNER_RESUME,
                                ids::BANNER_STOP,
                                self.on_click.clone(),
                            ))
                    }))
                    .children(shelf_elements),
            )
    }
}

/// Game page: project, title, badges, the launch verb(s), and the secondary actions.
#[derive(Clone, PartialEq)]
pub struct GamePage {
    game: GameEntry,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl GamePage {
    pub fn new(
        game: GameEntry,
        focus: FocusId,
        ring_visible: bool,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self {
            game,
            focus,
            ring_visible,
            on_click,
        }
    }
}

impl Component for GamePage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let g = &self.game;
        let (a, b) = (self.on_click.clone(), self.on_click.clone());
        let badge = if g.run.is_active() {
            StatusBadge::running()
        } else {
            StatusBadge::new(g.status)
        };

        rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::px(720.))
            .padding(Gaps::new(SPACE_6, 0., 0., 0.))
            .child(TypeStyle::Eyebrow.text(g.project.clone(), t.accent))
            .child(TypeStyle::DeckTitle.text(g.title.clone(), t.ink))
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_4)
                    .child(badge)
                    .maybe(!g.version.is_empty(), |el| {
                        el.child(TypeStyle::Mono.text(g.version.clone(), t.ink_muted))
                    })
                    .child(TypeStyle::DeckMeta.text(g.source.host(), t.ink_muted)),
            )
            .child(rect().height(Size::px(SPACE_5)))
            .child(LaunchButton::new(
                g.clone(),
                self.focus,
                self.ring_visible,
                ids::GAME_PRIMARY,
                ids::GAME_STOP,
                self.on_click.clone(),
            ))
            .child(
                rect()
                    .horizontal()
                    .spacing(SPACE_4)
                    .child(FocusFrame::new(
                        ActionButton::new(ButtonVariant::Secondary)
                            .icon(IconName::Folder)
                            .label("Open folder")
                            .size(ButtonSize::Controller)
                            .on_press(move |_| a.call(ids::GAME_FOLDER)),
                        self.ring_visible && self.focus == ids::GAME_FOLDER,
                    ))
                    .child(FocusFrame::new(
                        ActionButton::new(ButtonVariant::Secondary)
                            .icon(IconName::Settings)
                            .label("Manage")
                            .size(ButtonSize::Controller)
                            .on_press(move |_| b.call(ids::GAME_MANAGE)),
                        self.ring_visible && self.focus == ids::GAME_MANAGE,
                    )),
            )
    }
}

/// Download queue: one `DownloadItem` per entry with its Cancel as the focus target.
#[derive(Clone, PartialEq)]
pub struct DownloadsPage {
    downloads: Vec<Download>,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl DownloadsPage {
    pub fn new(
        downloads: Vec<Download>,
        focus: FocusId,
        ring_visible: bool,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self {
            downloads,
            focus,
            ring_visible,
            on_click,
        }
    }
}

impl Component for DownloadsPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        if self.downloads.is_empty() {
            return EmptyPage::new("Nothing in the queue", "Installs and updates show up here.")
                .into_element();
        }
        let items = self.downloads.iter().map(|d| {
            let id = ids::download_cancel(d.app_id);
            let on_click = self.on_click.clone();
            DownloadItem::new(d.clone())
                .controller(true, self.ring_visible && self.focus == id)
                .on_cancel(move |_| on_click.call(id))
                .key(d.app_id)
                .into_element()
        });
        rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::fill())
            .child(TypeStyle::DeckHeading.text("Downloads", t.ink))
            .children(items)
            .into_element()
    }
}

/// Sections that have no content yet say so plainly.
#[derive(Clone, PartialEq)]
pub struct EmptyPage {
    title: &'static str,
    detail: &'static str,
}

impl EmptyPage {
    pub fn new(title: &'static str, detail: &'static str) -> Self {
        Self { title, detail }
    }
}

impl Component for EmptyPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        rect()
            .vertical()
            .center()
            .spacing(SPACE_2)
            .width(Size::fill())
            .height(Size::fill())
            .child(TypeStyle::DeckHeading.text(self.title, t.ink))
            .child(TypeStyle::DeckBody.text(self.detail, t.ink_muted))
    }
}
