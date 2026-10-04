use freya::prelude::*;
use reclaw_media::{ImageKind, Kind, Want};

use super::ArtPlaceholder;
use crate::media::{RemoteFile, use_remote_file};

/// A picture from the internet in the place of a placeholder: the placeholder shows until the
/// picture is on disk, and stays if it never arrives (offline, no art, downloads switched off).
///
/// The picture fills the box and is cropped to it, like cover art. Pass it a size through the
/// placeholder, which also names the art's role.
#[derive(Clone, PartialEq)]
pub struct RemoteArt {
    url: Option<String>,
    placeholder: ArtPlaceholder,
    width: Size,
    height: Size,
    key: DiffKey,
}

impl KeyExt for RemoteArt {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl RemoteArt {
    pub fn new(url: Option<String>, placeholder: ArtPlaceholder, width: Size, height: Size) -> Self {
        Self { url, placeholder, width, height, key: DiffKey::None }
    }
}

impl Component for RemoteArt {
    fn render(&self) -> impl IntoElement {
        let file = use_remote_file(self.url.as_deref(), Want::Image);
        let RemoteFile::Ready(cached) = file else { return self.placeholder.clone().into_element() };
        let Kind::Image(kind) = cached.kind else { return self.placeholder.clone().into_element() };
        let source = ImageSource::Path(cached.path);
        let fallback = self.placeholder.clone();
        match kind {
            ImageKind::Svg => SvgViewer::new(source)
                .width(self.width.clone())
                .height(self.height.clone())
                .error_renderer(move |_: String| fallback.clone().into_element())
                .into_element(),
            _ => ImageViewer::new(source)
                .width(self.width.clone())
                .height(self.height.clone())
                .image_cover(ImageCover::Center)
                .aspect_ratio(AspectRatio::Max)
                .loading_placeholder(self.placeholder.clone())
                .error_renderer(move |_: String| fallback.clone().into_element())
                .into_element(),
        }
    }

    fn render_key(&self) -> DiffKey {
        // A different address is a different picture, with its own fetch.
        self.key.clone().or(DiffKey::from(&self.url))
    }
}
