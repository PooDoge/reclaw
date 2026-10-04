//! Builds the page that fills the window for one route. Each takes the page it is for, not the
//! state's current screen, so a page that is on its way out during a transition still draws itself.
use freya::prelude::*;
use reclaw_input::Action;

use super::frame::Frame;
use crate::{
    deck::{
        Backdrop, DownloadsPage, EmptyPage, GamePage, HintBar, HomePage, InstallBody, Overlay, Section, SectionTabs, SettingsBody, ids,
        install_footer,
    },
    metrics::*,
    settings::SettingsTarget,
    surface::FullScreenPage,
};

fn hints(f: &Frame, list: &[(Action, &'static str)]) -> Element {
    HintBar::new(list.to_vec(), f.kind, f.last_input, f.map.clone()).tight(f.window.0 < 520.).into_element()
}

/// One of a tab's pages (Library, Catalog, Downloads, Mods).
pub(super) fn home(f: &Frame, section: Section) -> Element {
    chrome(f, section, None)
}

/// A game's page, under the tab the user came from.
pub(super) fn game(f: &Frame, id: u32) -> Element {
    chrome(f, f.state.section(), Some(id))
}

/// Tabs, a body, the hint bar, over the Big Art backdrop.
fn chrome(f: &Frame, section: Section, game_page: Option<u32>) -> Element {
    let (w, h) = f.window;
    let state = &f.state;
    let focused_game = match game_page {
        Some(id) => Some(id),
        None => ids::tile_game(state.focus()),
    };
    let t = crate::theme::ThemeKind::Midnight.tokens();
    let tint = if focused_game.and_then(|id| f.game(id)).is_some() { t.accent } else { t.bg_raised };

    let body: Element = match (game_page, section) {
        (Some(id), _) => match f.game(id) {
            Some(game) => GamePage::new(game.clone(), state.focus(), f.ring, f.click.clone()).into_element(),
            None => rect().into_element(),
        },
        (None, Section::Library) => {
            HomePage::new(f.games.clone(), state.focus(), f.ring, w - 2. * DECK_SAFE_X, f.click.clone()).into_element()
        }
        (None, Section::Downloads) => DownloadsPage::new(f.downloads.clone(), state.focus(), f.ring, f.click.clone()).into_element(),
        (None, Section::Catalog) => EmptyPage::new("Catalog", "Community app lists will appear here.").into_element(),
        (None, Section::Mods) => EmptyPage::new("Mods", "Mods from Thunderstore and GameBanana will appear here.").into_element(),
    };

    let list: &[(Action, &'static str)] = match (state.overlay(), game_page) {
        (Overlay::None, Some(_)) => {
            &[(Action::MainMenu, "Menu"), (Action::Options, "Options"), (Action::Confirm, "Select"), (Action::Back, "Back")]
        }
        (Overlay::None, None) => {
            &[(Action::MainMenu, "Menu"), (Action::QuickAccess, "Quick access"), (Action::Options, "Options"), (Action::Confirm, "Select")]
        }
        _ => &[(Action::Confirm, "Select"), (Action::Back, "Back")],
    };

    rect()
        .width(Size::px(w))
        .height(Size::px(h))
        .child(Backdrop::new(tint))
        .child(
            rect()
                .position(Position::new_absolute().top(0.).left(0.))
                .vertical()
                .content(Content::Flex)
                .width(Size::px(w))
                .height(Size::px(h))
                .padding(Gaps::new(DECK_SAFE_Y, DECK_SAFE_X, DECK_SAFE_Y, DECK_SAFE_X))
                .child(SectionTabs::new(section, f.kind, f.last_input, f.map.clone()))
                .child(rect().width(Size::fill()).height(Size::flex(1.)).padding(Gaps::new(SPACE_4, 0., 0., 0.)).child(body))
                .child(hints(f, list)),
        )
        .into_element()
}

pub(super) fn install(f: &Frame, app: u32) -> Element {
    let title = f.game(app).map(|g| g.title.to_string()).unwrap_or_default();
    let state = &f.state;
    let typing = state.text_entry().is_some();
    let list: &[(Action, &'static str)] =
        if typing { &[(Action::Confirm, "Done")] } else { &[(Action::Confirm, "Select"), (Action::Back, "Back")] };
    page(f)
        .title(format!("Install {title}"))
        .body(InstallBody::new(state.install_draft().clone(), f.texts, state.focus(), f.ring, f.click.clone()))
        .footer(install_footer(state.install_draft(), state.focus(), f.ring, f.click.clone()))
        .hints(hints(f, list))
        .reveal(f.reveal)
        .build()
}

pub(super) fn settings(f: &Frame, target: SettingsTarget) -> Element {
    let state = &f.state;
    // The schema of this page's target: the live one, or the one remembered from when the page was last
    // shown, so a settings page leaving during a transition still has its content.
    let Some(shown) = f.settings.as_ref().filter(|s| s.target == target) else { return rect().into_element() };
    let schema = shown.schema.clone();
    let two = state.two_pane();
    let typing = state.text_entry().is_some();
    let list: &[(Action, &'static str)] = if typing {
        &[(Action::Confirm, "Done")]
    } else if matches!(state.overlay(), Overlay::Menu(_)) {
        &[(Action::Confirm, "Select"), (Action::Back, "Back")]
    } else {
        &[(Action::MainMenu, "Menu"), (Action::Confirm, "Select"), (Action::Back, "Back")]
    };
    let viewport = FullScreenPage::body_viewport(f.window, false, true, f.keyboard_inset);
    let body = SettingsBody {
        schema: schema.clone(),
        target,
        values: state.values().clone(),
        launch_text: shown.launch_text.clone(),
        section: state.settings_section(),
        two_pane: two,
        drilled: state.drilled(),
        texts: f.texts,
        focus: state.focus(),
        ring_visible: f.ring,
        viewport,
        reveal: if two { f.reveal } else { None },
        on_click: f.click.clone(),
    };
    page(f).title(schema.title).body(body).hints(hints(f, list)).reveal(if two { None } else { f.reveal }).wide(two).fixed(two).build()
}

/// Builder for the full-screen pages, so each screen states only what differs.
struct PageSpec<'a> {
    f: &'a Frame,
    title: String,
    body: Option<Element>,
    footer: Option<Element>,
    hints: Option<Element>,
    reveal: Option<(f32, f32)>,
    wide: bool,
    fixed: bool,
}

fn page(f: &Frame) -> PageSpec<'_> {
    PageSpec { f, title: String::new(), body: None, footer: None, hints: None, reveal: None, wide: false, fixed: false }
}

impl PageSpec<'_> {
    fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }

    fn body(mut self, b: impl IntoElement) -> Self {
        self.body = Some(b.into_element());
        self
    }

    fn footer(mut self, b: Element) -> Self {
        self.footer = Some(b);
        self
    }

    fn hints(mut self, b: Element) -> Self {
        self.hints = Some(b);
        self
    }

    fn reveal(mut self, r: Option<(f32, f32)>) -> Self {
        self.reveal = r;
        self
    }

    fn wide(mut self, w: bool) -> Self {
        self.wide = w;
        self
    }

    fn fixed(mut self, w: bool) -> Self {
        self.fixed = w;
        self
    }

    fn build(self) -> Element {
        let f = self.f;
        let back = f.back.clone();
        let mut page =
            FullScreenPage::new(self.title, f.window, Density::Controller, back, self.body.unwrap_or_else(|| rect().into_element()))
                .keyboard_inset(f.keyboard_inset)
                .reveal(self.reveal)
                .wide(self.wide)
                .fixed(self.fixed);
        if let Some(footer) = self.footer {
            page = page.footer(footer);
        }
        if let Some(hints) = self.hints {
            page = page.hints(hints);
        }
        rect().width(Size::px(f.window.0)).height(Size::px(f.window.1)).child(page).into_element()
    }
}
