use freya::prelude::*;
use reclaw_games::project::Media;

use super::ctx::Ctx;
use crate::{
    components::{hoverable, pointer_cursor},
    desktop::pages::common::heading,
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

const TILE_W: f32 = 256.;
const TILE_H: f32 = 144.;

/// Screenshots and videos in one strip that scrolls sideways. A video opens in the system player or
/// browser; Freya 0.5 has no video widget. Remote art is a placeholder until the catalog cache
/// supplies files (see `ArtPlaceholder`).
pub(super) fn view(c: &Ctx) -> Option<Element> {
    let project = c.view.project.as_ref().filter(|p| !p.media.is_empty())?;
    let open = c.open_url();
    let tiles = project
        .media
        .iter()
        .enumerate()
        .map(|(i, media)| MediaTile { media: media.clone(), open: open.clone(), key: DiffKey::None }.key(i).into_element());
    Some(
        rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(heading(&c.t, "Screenshots and videos"))
            .child(
                ScrollView::new()
                    .direction(Direction::Horizontal)
                    .show_scrollbar(false)
                    .height(Size::px(TILE_H + 28.))
                    .child(rect().horizontal().spacing(SPACE_3).children(tiles)),
            )
            .into_element(),
    )
}

#[derive(Clone, PartialEq)]
struct MediaTile {
    media: Media,
    open: EventHandler<String>,
    key: DiffKey,
}

impl KeyExt for MediaTile {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for MediaTile {
    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }

    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let (caption, tag, link) = match &self.media {
            Media::Screenshot { url, caption } => (caption.clone().unwrap_or_default(), "SCREENSHOT", url.clone()),
            Media::Video { url, title, .. } => (title.clone().unwrap_or_else(|| "Video".to_string()), "VIDEO", url.clone()),
        };
        let is_video = matches!(self.media, Media::Video { .. });
        let open = self.open.clone();

        let art = rect()
            .width(Size::px(TILE_W))
            .height(Size::px(TILE_H))
            .background(t.bg_raised)
            .border(Border::new().fill(if hovering() { t.accent } else { t.line }).width(1.).alignment(BorderAlignment::Inner))
            .corner_radius(RADIUS_MD)
            .center()
            .child(if is_video {
                rect()
                    .width(Size::px(44.))
                    .height(Size::px(44.))
                    .center()
                    .corner_radius(22.)
                    .background(t.bg_base)
                    .child(icon(IconName::Play, 20., t.ink))
                    .into_element()
            } else {
                TypeStyle::Mono.text(tag, t.ink_subtle).into_element()
            });
        let tile = rect()
            .vertical()
            .spacing(SPACE_1)
            .width(Size::px(TILE_W))
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(format!("{tag}: {caption}"))
            .on_press(move |_| open.call(link.clone()))
            .child(art)
            .child(TypeStyle::Meta.text(caption, t.ink_muted).max_lines(1).text_overflow(TextOverflow::Ellipsis));
        pointer_cursor(hoverable(tile, hovering))
    }
}
