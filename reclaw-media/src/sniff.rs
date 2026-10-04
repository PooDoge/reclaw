//! What a downloaded file is, from its first bytes. A server's `Content-Type` is not trusted: a
//! wrong one is common (GitHub serves some raw files as `text/plain`) and a hostile one is possible.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
    Svg,
}

impl ImageKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Svg => "svg",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        Some(match ext {
            "png" => Self::Png,
            "jpg" => Self::Jpeg,
            "gif" => Self::Gif,
            "webp" => Self::Webp,
            "svg" => Self::Svg,
            _ => return None,
        })
    }

    /// Whether the toolkit decodes it as a bitmap. SVG is drawn from its markup instead.
    pub fn is_raster(self) -> bool {
        self != Self::Svg
    }
}

/// The image format `bytes` begin with, or `None` for anything else, including an empty file.
pub fn sniff_image(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageKind::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageKind::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(ImageKind::Gif)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(ImageKind::Webp)
    } else if looks_like_svg(bytes) {
        Some(ImageKind::Svg)
    } else {
        None
    }
}

/// SVG is text, so it is recognized by its root element, after an optional BOM, XML declaration,
/// comments and doctype.
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(2048)];
    let Ok(text) = std::str::from_utf8(head).or_else(|e| std::str::from_utf8(&head[..e.valid_up_to()])) else { return false };
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("<?").and_then(|r| r.split_once("?>")) {
            rest = after.1.trim_start();
        } else if let Some(after) = rest.strip_prefix("<!--").and_then(|r| r.split_once("-->")) {
            rest = after.1.trim_start();
        } else if let Some(after) = rest.strip_prefix("<!").and_then(|r| r.split_once('>')) {
            rest = after.1.trim_start();
        } else {
            break;
        }
    }
    rest.get(..4).is_some_and(|tag| tag.eq_ignore_ascii_case("<svg"))
}

/// Whether `bytes` are text a person could read: valid UTF-8 with no NUL bytes in the start.
pub fn looks_like_text(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(4096)];
    if head.contains(&0) {
        return false;
    }
    match std::str::from_utf8(head) {
        Ok(_) => true,
        // The window can end partway through a character, which is fine unless the file ends there too.
        Err(e) => e.error_len().is_none() && head.len() < bytes.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_common_formats_are_recognized_by_their_first_bytes() {
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\nrest"), Some(ImageKind::Png));
        assert_eq!(sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some(ImageKind::Jpeg));
        assert_eq!(sniff_image(b"GIF89a...."), Some(ImageKind::Gif));
        assert_eq!(sniff_image(b"GIF87a...."), Some(ImageKind::Gif));
        assert_eq!(sniff_image(b"RIFF\x10\0\0\0WEBPVP8 "), Some(ImageKind::Webp));
    }

    #[test]
    fn svg_is_found_past_a_declaration_comments_and_a_doctype() {
        assert_eq!(sniff_image(b"<svg xmlns='http://www.w3.org/2000/svg'/>"), Some(ImageKind::Svg));
        assert_eq!(sniff_image(b"\xef\xbb\xbf<?xml version='1.0'?>\n<!-- made by hand -->\n<!DOCTYPE svg>\n<SVG/>"), Some(ImageKind::Svg));
    }

    #[test]
    fn html_and_other_files_are_not_images() {
        for not in [
            &b"<html><body><svg/></body></html>"[..],
            b"<!DOCTYPE html><html>",
            b"just words",
            b"",
            b"\x00\x01\x02",
            b"PK\x03\x04",
            b"%PDF-1.7",
        ] {
            assert_eq!(sniff_image(not), None, "{not:?}");
        }
    }

    #[test]
    fn a_riff_file_that_is_not_webp_is_not_an_image() {
        assert_eq!(sniff_image(b"RIFF\x10\0\0\0WAVEfmt "), None);
    }

    #[test]
    fn text_is_utf8_without_nul_bytes() {
        assert!(looks_like_text("# Title\n\nSome words — with a dash.\n".as_bytes()));
        assert!(looks_like_text(b""));
        assert!(!looks_like_text(b"abc\0def"));
        assert!(!looks_like_text(&[0xFF, 0xFE, 0xFD, 0x80]));
        assert!(!looks_like_text(&[0x89, b'P', b'N', b'G', 0x00]));
    }

    #[test]
    fn text_cut_in_the_middle_of_a_character_is_still_text() {
        // Three bytes each, so the 4096-byte window ends one byte into a character.
        let bytes = "—".repeat(2000).into_bytes();
        assert!(looks_like_text(&bytes), "the window may end inside a character");
        assert!(!looks_like_text(&bytes[..4096]), "but a file that ends there is broken");
    }

    #[test]
    fn extensions_round_trip() {
        for kind in [ImageKind::Png, ImageKind::Jpeg, ImageKind::Gif, ImageKind::Webp, ImageKind::Svg] {
            assert_eq!(ImageKind::from_extension(kind.extension()), Some(kind));
        }
        assert!(ImageKind::Png.is_raster() && !ImageKind::Svg.is_raster());
    }
}
