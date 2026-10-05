use freya::prelude::*;
use reclaw_media::{
    ImageKind, Kind, Want,
    readme::{banner_candidates, fits_banner},
};

use crate::{
    banner,
    media::{RemoteFile, use_remote_file},
    model::Art,
    prelude::*,
    readme::use_readme,
};

/// How many README pictures are tried, after the catalog's own banner, before giving up on pictures.
const README_TRIES: usize = 3;
/// Blur radius of the icon laid behind a generated banner.
const BACKDROP_BLUR: f32 = 28.;

/// The wide picture at the top of a game's page. It uses the best picture there is, in this order:
///
/// 1. the banner the catalog names (`reclaw.heroUrl`), which is trusted as it is;
/// 2. a picture from the project's README that is wide enough to crop ([`fits_banner`]), tried best
///    first, a few of them;
/// 3. a generated banner: the project's own colour (steady from run to run) with its icon, blurred,
///    behind the icon itself. This one needs no network, so a game is never left with an empty box.
///
/// Whatever fails to arrive (offline, a dead link, a picture that is not an image) falls through to
/// the next step. Pictures are cropped to the box like cover art; the generated banner keeps its
/// icon clear of the title strip that covers the bottom `strip` pixels.
#[derive(Clone, PartialEq)]
pub struct BannerArt {
    art: Art,
    seed: String,
    height: f32,
    strip: f32,
    key: DiffKey,
}

impl KeyExt for BannerArt {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl BannerArt {
    /// `seed` is what the generated colour is taken from (the game's title).
    pub fn new(art: Art, seed: impl Into<String>, height: f32, strip: f32) -> Self {
        Self { art, seed: seed.into(), height, strip, key: DiffKey::None }
    }
}

impl Component for BannerArt {
    fn render(&self) -> impl IntoElement {
        let readme = use_readme(self.art.repo.as_ref());
        // (address, whether the picture must prove it is wide enough once its size is known)
        let mut tries: Vec<(String, bool)> = Vec::new();
        if let Some(hero) = &self.art.hero {
            tries.push((hero.clone(), false));
        }
        if let Some(document) = &readme.document {
            tries.extend(
                banner_candidates(document, README_TRIES)
                    .into_iter()
                    .filter(|url| Some(url) != self.art.hero.as_ref())
                    .map(|url| (url, true)),
            );
        }
        BannerStep {
            tries,
            fallback: GeneratedBanner {
                seed: self.seed.clone(),
                icon: self.art.capsule.clone(),
                height: self.height,
                strip: self.strip,
                key: DiffKey::None,
            },
            height: self.height,
            key: DiffKey::None,
        }
    }

    fn render_key(&self) -> DiffKey {
        // A different game is a different banner, with its own fetches.
        self.key.clone().or(DiffKey::from(&(&self.seed, &self.art.hero)))
    }
}

/// One attempt of the chain: show the first address if it arrives and fits, otherwise hand over to
/// the rest. Each attempt is its own component, so each calls its hook once, in the same order.
#[derive(Clone, PartialEq)]
struct BannerStep {
    tries: Vec<(String, bool)>,
    fallback: GeneratedBanner,
    height: f32,
    key: DiffKey,
}

impl KeyExt for BannerStep {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for BannerStep {
    fn render(&self) -> impl IntoElement {
        let current = self.tries.first();
        let file = use_remote_file(current.map(|(url, _)| url.as_str()), Want::Image);
        let Some((_, must_fit)) = current else { return self.fallback.clone().into_element() };
        match file {
            // Waiting, or switched off: the generated banner, so the box is never blank and never a label.
            RemoteFile::Off | RemoteFile::Loading => self.fallback.clone().into_element(),
            RemoteFile::Failed(_) => self.next(),
            RemoteFile::Ready(cached) => {
                let Kind::Image(kind) = cached.kind else { return self.next() };
                if *must_fit && !fits_banner(cached.size) {
                    return self.next();
                }
                let source = ImageSource::Path(cached.path);
                let (next, height) = (self.next_step(), self.height);
                match kind {
                    ImageKind::Svg => SvgViewer::new(source)
                        .width(Size::fill())
                        .height(Size::px(height))
                        .error_renderer(move |_: String| next.clone().into_element())
                        .into_element(),
                    _ => ImageViewer::new(source)
                        .width(Size::fill())
                        .height(Size::px(height))
                        .image_cover(ImageCover::Center)
                        .aspect_ratio(AspectRatio::Max)
                        .loading_placeholder(self.fallback.clone())
                        .error_renderer(move |_: String| next.clone().into_element())
                        .into_element(),
                }
            }
        }
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(DiffKey::from(&self.tries.first().map(|(url, _)| url.clone())))
    }
}

impl BannerStep {
    fn next_step(&self) -> BannerStep {
        BannerStep {
            tries: self.tries.iter().skip(1).cloned().collect(),
            fallback: self.fallback.clone(),
            height: self.height,
            key: DiffKey::None,
        }
    }

    fn next(&self) -> Element {
        self.next_step().into_element()
    }
}

/// The last step: a colour taken from the game's name, its icon blurred into a backdrop, and the
/// icon itself clear of the title strip. Works with no network (then it is just the colour).
#[derive(Clone, PartialEq)]
struct GeneratedBanner {
    seed: String,
    icon: Option<String>,
    height: f32,
    strip: f32,
    key: DiffKey,
}

impl KeyExt for GeneratedBanner {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for GeneratedBanner {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let icon = use_remote_file(self.icon.as_deref(), Want::Image);
        let (strong, faint) = banner::gradient(&self.seed, (t.bg_base.r(), t.bg_base.g(), t.bg_base.b()));
        let wash = LinearGradient::new()
            .angle(135.)
            .stop((Color::from_rgb(strong.0, strong.1, strong.2), 0.))
            .stop((Color::from_rgb(faint.0, faint.1, faint.2), 100.));

        // The icon, when it arrived and is a picture the toolkit can draw as one.
        let picture = match icon {
            RemoteFile::Ready(cached) => match cached.kind {
                Kind::Image(kind) if kind != ImageKind::Svg => Some(cached.path),
                _ => None,
            },
            _ => None,
        };

        let room = (self.height - self.strip).max(0.);
        let size = (room * 0.62).clamp(48., 132.);
        let top = (room - size) / 2.;
        rect()
            .width(Size::fill())
            .height(Size::px(self.height))
            .overflow(Overflow::Clip)
            .background(wash)
            .maybe_child(picture.clone().map(|path| {
                ImageViewer::new(ImageSource::Path(path))
                    .width(Size::fill())
                    .height(Size::px(self.height))
                    .image_cover(ImageCover::Center)
                    .aspect_ratio(AspectRatio::Max)
                    .blur(BACKDROP_BLUR)
                    .opacity(0.32)
            }))
            .maybe_child(picture.map(|path| {
                rect().position(Position::new_absolute().top(top).left(0.)).width(Size::fill()).cross_align(Alignment::Center).child(
                    ImageViewer::new(ImageSource::Path(path))
                        .width(Size::px(size))
                        .height(Size::px(size))
                        .image_cover(ImageCover::Center)
                        .aspect_ratio(AspectRatio::Max)
                        .corner_radius(CornerRadius::new_all(RADIUS_LG)),
                )
            }))
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(DiffKey::from(&(&self.seed, &self.icon)))
    }
}
