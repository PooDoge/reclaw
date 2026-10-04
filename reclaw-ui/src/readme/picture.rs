use freya::prelude::*;
use reclaw_media::{
    ImageKind, Kind, Want,
    readme::{Badge, Pic},
};

use crate::{
    components::{ArtPlaceholder, pointer_cursor},
    media::{RemoteFile, use_remote_file},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

/// The tallest a README picture is drawn; a tall screenshot is scaled down to this.
const MAX_HEIGHT: f32 = 720.;
/// Width used for the placeholder of a picture whose size is not known (yet).
const PLACEHOLDER_W: f32 = 480.;

/// A status badge as a two-part chip: the label on grey, the message on the badge's color.
#[derive(Clone, PartialEq)]
pub struct BadgeChip {
    badge: Badge,
    link: Option<String>,
    on_open: EventHandler<String>,
    key: DiffKey,
}

impl KeyExt for BadgeChip {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl BadgeChip {
    pub fn new(badge: Badge, link: Option<String>, on_open: EventHandler<String>) -> Self {
        Self { badge, link, on_open, key: DiffKey::None }
    }
}

impl Component for BadgeChip {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let color = self.badge.color;
        let fill = Color::from_rgb(color.r, color.g, color.b);
        let ink = if color.wants_dark_text() { Color::from_rgb(0x11, 0x14, 0x18) } else { Color::WHITE };
        let part = |text: String, background: Color, foreground: Color| {
            rect()
                .center()
                .height(Size::px(20.))
                .padding(Gaps::new(0., 6., 0., 6.))
                .background(background)
                .child(TypeStyle::Mono.text(text, foreground).max_lines(1))
        };
        let mut chip = rect().horizontal().height(Size::px(20.)).corner_radius(3.).overflow(Overflow::Clip);
        if let Some(label) = self.badge.label.clone() {
            chip = chip.child(part(label, t.bg_raised, t.ink_muted));
        }
        chip = chip.child(part(self.badge.message.clone(), fill, ink));
        match self.link.clone() {
            Some(link) => {
                let on_open = self.on_open.clone();
                pointer_cursor(chip.a11y_role(AccessibilityRole::Link).on_press(move |_| on_open.call(link.clone()))).into_element()
            }
            None => chip.into_element(),
        }
    }
}

/// One picture of a README, scaled to fit the room it has with its own proportions, and pressable
/// when the README wrapped it in a link. A placeholder (with the picture's alt text) stands in until
/// it arrives and if it never does.
#[derive(Clone, PartialEq)]
pub struct FitPicture {
    pic: Pic,
    on_open: EventHandler<String>,
    key: DiffKey,
}

impl KeyExt for FitPicture {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl FitPicture {
    pub fn new(pic: Pic, on_open: EventHandler<String>) -> Self {
        Self { pic, on_open, key: DiffKey::None }
    }
}

impl Component for FitPicture {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let file = use_remote_file(self.pic.src.as_deref(), Want::Image);
        // How much room there is, learned from the layout: the first frame has none and draws nothing.
        let mut room = use_state(|| 0.0_f32);

        let body: Element = match file {
            RemoteFile::Ready(cached) if matches!(cached.kind, Kind::Image(_)) => {
                let (natural_w, natural_h) = cached.size.map_or((PLACEHOLDER_W, PLACEHOLDER_W * 9. / 16.), |(w, h)| (w as f32, h as f32));
                // The README's own width wins over the file's; its height too, when both are given.
                let wanted_w = self.pic.width.map_or(natural_w, |w| w as f32);
                let ratio = match (self.pic.width, self.pic.height) {
                    (Some(w), Some(h)) => h as f32 / w as f32,
                    _ => natural_h / natural_w,
                };
                let mut width = wanted_w.min(room()).max(0.);
                if width * ratio > MAX_HEIGHT {
                    width = MAX_HEIGHT / ratio;
                }
                let (width, height) = (width.floor(), (width * ratio).floor());
                let source = ImageSource::Path(cached.path);
                let alt = self.pic.alt.clone();
                let broken = {
                    let alt = alt.clone();
                    move |_: String| missing(&t, &alt)
                };
                let picture = match cached.kind {
                    Kind::Image(ImageKind::Svg) => {
                        SvgViewer::new(source).width(Size::px(width)).height(Size::px(height)).error_renderer(broken).into_element()
                    }
                    _ => ImageViewer::new(source)
                        .width(Size::px(width))
                        .height(Size::px(height))
                        .aspect_ratio(AspectRatio::Min)
                        .image_cover(ImageCover::Center)
                        .error_renderer(broken)
                        .into_element(),
                };
                rect()
                    .a11y_role(AccessibilityRole::Image)
                    .a11y_alt(alt)
                    .corner_radius(RADIUS_SM)
                    .overflow(Overflow::Clip)
                    .child(picture)
                    .into_element()
            }
            RemoteFile::Failed(_) | RemoteFile::Off if !self.pic.alt.is_empty() => missing(&t, &self.pic.alt),
            _ => {
                let width = self.pic.width.map_or(PLACEHOLDER_W, |w| w as f32).min(room()).max(0.);
                ArtPlaceholder::new("IMAGE", Size::px(width), Size::px((width * 9. / 16.).floor())).into_element()
            }
        };

        let content = rect().width(Size::fill()).on_sized(move |e: Event<SizedEventData>| room.set_if_modified(e.area.width())).child(body);
        match self.pic.link.clone() {
            Some(link) => {
                let on_open = self.on_open.clone();
                pointer_cursor(content.a11y_role(AccessibilityRole::Link).on_press(move |_| on_open.call(link.clone()))).into_element()
            }
            None => content.into_element(),
        }
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(DiffKey::from(&self.pic.src))
    }
}

/// What stands in for a picture that is not there: its alt text, as a quiet note.
fn missing(t: &Reclaw, alt: &str) -> Element {
    TypeStyle::Meta.text(format!("[{alt}]"), t.ink_subtle).into_element()
}
