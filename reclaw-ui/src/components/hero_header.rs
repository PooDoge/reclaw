use freya::prelude::*;

use super::{ArtPlaceholder, PressHandler, StatusBadge, pointer_cursor};
use crate::{
    launch::{LaunchVerb, launch_verb},
    metrics::*,
    nav::use_page_motion,
    prelude::*,
    typography::TypeStyle,
};

/// Game page top: banner, project eyebrow, title, and one action bar holding the single install
/// verb, version, source and secondary actions.
///
/// `narrow` (compact and phone) shrinks the banner, uses the page title size and drops the
/// source column; secondary actions are icon-only at every width.
#[derive(Clone, PartialEq)]
pub struct HeroHeader {
    game: GameEntry,
    narrow: bool,
    density: Density,
    on_primary: Option<PressHandler>,
    on_open_folder: Option<PressHandler>,
    on_manage: Option<PressHandler>,
    on_open: Option<PressHandler>,
    on_favorite: Option<PressHandler>,
    favorite: bool,
    on_settings: Option<PressHandler>,
}

impl HeroHeader {
    pub fn new(game: GameEntry) -> Self {
        Self {
            game,
            narrow: false,
            density: Density::Pointer,
            on_primary: None,
            on_open_folder: None,
            on_manage: None,
            on_open: None,
            on_favorite: None,
            favorite: false,
            on_settings: None,
        }
    }

    pub fn narrow(mut self, narrow: bool) -> Self {
        self.narrow = narrow;
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }

    pub fn on_primary(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_primary = Some(handler.into());
        self
    }

    pub fn on_open_folder(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_open_folder = Some(handler.into());
        self
    }

    pub fn on_manage(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_manage = Some(handler.into());
        self
    }

    /// Open the game's own page: pressing the banner or the Details button.
    pub fn on_open(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_open = Some(handler.into());
        self
    }

    /// A star button. `favorite` is the current state; the handler flips it.
    pub fn on_favorite(mut self, favorite: bool, handler: impl Into<PressHandler>) -> Self {
        self.favorite = favorite;
        self.on_favorite = Some(handler.into());
        self
    }

    /// A cog button, for the game's own settings page.
    pub fn on_settings(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_settings = Some(handler.into());
        self
    }

    /// (label, icon, enabled) of the one state-changing verb, from install and run state.
    fn primary(&self) -> (&'static str, IconName, bool) {
        let verb = launch_verb(self.game.status, &self.game.run, false);
        let icon = match verb {
            LaunchVerb::Play | LaunchVerb::Resume => IconName::Play,
            LaunchVerb::Update | LaunchVerb::Install => IconName::Download,
            LaunchVerb::Installing | LaunchVerb::Starting => IconName::Queue,
            LaunchVerb::Stop | LaunchVerb::ForceQuit => IconName::Stop,
            LaunchVerb::Retry => IconName::Refresh,
        };
        (verb.label(), icon, verb.enabled())
    }
}

impl Component for HeroHeader {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let motion = use_page_motion();
        let narrow = self.narrow;
        let art_height = if narrow { 200. } else { HERO_H };
        let (verb, verb_icon, verb_enabled) = self.primary();
        let size = if self.density == Density::Touch { ButtonSize::Touch } else { ButtonSize::Lg };
        let strip_height = if narrow { 74. } else { 82. };
        let icon_size = if self.density == Density::Touch { ButtonSize::Touch } else { ButtonSize::Md };
        let (title_style, pad_x) = if narrow { (TypeStyle::TitlePage, SPACE_4) } else { (TypeStyle::TitleHero, SPACE_6) };

        // A hero entrance (the game page arriving) eases the banner in and lifts the strip and the bar
        // after it; at rest every value is neutral.
        let banner_zoom = motion.settle(0., 0.8, 0.08);
        let (strip_opacity, strip_dy) = motion.rise(0.2, 0.9, 16.);
        let (bar_opacity, bar_dy) = motion.rise(0.35, 1., 24.);

        let banner = rect()
            .width(Size::fill())
            .map(self.on_open.clone(), |el, h| pointer_cursor(el).on_press(h))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(art_height))
                    .overflow(Overflow::Clip)
                    .child(rect().width(Size::fill()).height(Size::fill()).scale(banner_zoom).child(ArtPlaceholder::hero(art_height))),
            )
            .child(
                // The title sits on a solid strip so it holds 4.5:1 over any banner image.
                // Absolute `bottom` does not anchor as CSS does in torin, so place by `top` with a fixed height.
                rect()
                    .position(Position::new_absolute().top(art_height - strip_height).left(0.))
                    .width(Size::fill())
                    .height(Size::px(strip_height))
                    .opacity(strip_opacity)
                    .offset_y(strip_dy)
                    .vertical()
                    .padding(Gaps::new(SPACE_4, pad_x, SPACE_4, pad_x))
                    .background(t.bg_base)
                    .child(TypeStyle::Eyebrow.text(self.game.project.clone(), t.accent))
                    .child(title_style.text(self.game.title.clone(), t.ink)),
            );

        let kv = |name: &'static str, value: String| {
            rect().vertical().spacing(2.).child(TypeStyle::Eyebrow.text(name, t.ink_subtle)).child(TypeStyle::Mono.text(value, t.ink))
        };

        let bar = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(if narrow { SPACE_3 } else { SPACE_5 })
            .width(Size::fill())
            .padding(Gaps::new(SPACE_4, pad_x, SPACE_4, pad_x))
            .background(t.bg_panel)
            .opacity(bar_opacity)
            .offset_y(bar_dy)
            .border(Border::new().fill(t.line).width(BorderWidth { top: 1., ..Default::default() }))
            .child(
                ActionButton::install()
                    .size(size)
                    .icon(verb_icon)
                    .label(verb)
                    .enabled(verb_enabled)
                    .map(self.on_primary.clone(), |b, h| b.on_press(h)),
            )
            .child(kv("Version", self.game.version.to_string()))
            .maybe(!narrow, |el| el.child(kv("Source", self.game.source.host().to_string())))
            .child(StatusBadge::new(self.game.status))
            .child(rect().width(Size::flex(1.)))
            .maybe_child(self.on_favorite.clone().map(|h| {
                // Named when there is room; the star alone says "favorite" to most people but not to a screen reader.
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(if self.favorite { IconName::StarFilled } else { IconName::Star })
                    .alt(if self.favorite { "Remove from favorites" } else { "Add to favorites" })
                    .map((!narrow).then_some(if self.favorite { "Favorited" } else { "Favorite" }), |b, label| b.label(label))
                    .size(icon_size)
                    .on_press(h)
            }))
            .maybe_child(self.on_settings.clone().map(|h| {
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Settings)
                    .alt("Game settings")
                    .map((!narrow).then_some("Settings"), |b, label| b.label(label))
                    .size(icon_size)
                    .on_press(h)
            }))
            .maybe_child(self.on_open.clone().map(|h| {
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Chevron)
                    .label("Details")
                    .size(if self.density == Density::Touch { ButtonSize::Touch } else { ButtonSize::Md })
                    .on_press(h)
            }))
            .child(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Folder)
                    .size(if self.density == Density::Touch { ButtonSize::Touch } else { ButtonSize::Md })
                    .map(self.on_open_folder.clone(), |b, h| b.on_press(h)),
            )
            .child(
                // Icon-only where space is short; the label names the button for everyone else.
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::More)
                    .map((!narrow).then_some("Manage"), |b, label| b.label(label))
                    .size(if self.density == Density::Touch { ButtonSize::Touch } else { ButtonSize::Md })
                    .map(self.on_manage.clone(), |b, h| b.on_press(h)),
            );

        rect()
            .vertical()
            .width(Size::fill())
            .background(t.bg_base)
            .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
            .corner_radius(RADIUS_MD)
            .overflow(Overflow::Clip)
            .child(banner)
            .child(bar)
    }
}
