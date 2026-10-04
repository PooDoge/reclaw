//! A picture's size in pixels, read from the start of the file, so a page can reserve the right
//! amount of room before the picture has been decoded (and so lay it out with its true proportions
//! instead of a guess). Reads headers only; it never decodes anything.
use crate::sniff::ImageKind;

/// How much of a file's start is enough: a JPEG can carry a large preview before its size.
pub const HEAD_BYTES: usize = 256 * 1024;

/// The pixel size of the image `bytes` begin, or `None` if the header is cut short or unreadable.
/// A size of zero in either direction is not a size.
pub fn dimensions(kind: ImageKind, bytes: &[u8]) -> Option<(u32, u32)> {
    let (w, h) = match kind {
        ImageKind::Png => png(bytes),
        ImageKind::Gif => gif(bytes),
        ImageKind::Jpeg => jpeg(bytes),
        ImageKind::Webp => webp(bytes),
        ImageKind::Svg => svg(bytes),
    }?;
    (w > 0 && h > 0).then_some((w, h))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn le16(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?)))
}

fn le24(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 3)?;
    Some(u32::from(s[0]) | u32::from(s[1]) << 8 | u32::from(s[2]) << 16)
}

/// The IHDR chunk comes first: width and height are the first two numbers in it.
fn png(b: &[u8]) -> Option<(u32, u32)> {
    (b.get(12..16)? == b"IHDR").then_some(())?;
    Some((be32(b, 16)?, be32(b, 20)?))
}

fn gif(b: &[u8]) -> Option<(u32, u32)> {
    Some((le16(b, 6)?, le16(b, 8)?))
}

/// Walk the segments until a start-of-frame marker, which holds the size.
fn jpeg(b: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = b[i + 1];
        match marker {
            // Fill bytes and markers with no length.
            0xFF => i += 1,
            0x01 | 0xD0..=0xD8 => i += 2,
            // Start of frame, but not the tables that share the range: DHT (C4), JPG (C8), DAC (CC).
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                let height = u32::from(u16::from_be_bytes(b.get(i + 5..i + 7)?.try_into().ok()?));
                let width = u32::from(u16::from_be_bytes(b.get(i + 7..i + 9)?.try_into().ok()?));
                return Some((width, height));
            }
            // Start of scan: the size would have come before it.
            0xDA | 0xD9 => return None,
            _ => {
                let len = usize::from(u16::from_be_bytes(b.get(i + 2..i + 4)?.try_into().ok()?));
                i += 2 + len.max(2);
            }
        }
    }
    None
}

/// Three flavors of WebP, told apart by the first chunk's name.
fn webp(b: &[u8]) -> Option<(u32, u32)> {
    match b.get(12..16)? {
        b"VP8 " => Some((le16(b, 26)? & 0x3fff, le16(b, 28)? & 0x3fff)),
        b"VP8L" => {
            (*b.get(20)? == 0x2f).then_some(())?;
            let bits = u32::from_le_bytes(b.get(21..25)?.try_into().ok()?);
            Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
        }
        b"VP8X" => Some((le24(b, 24)? + 1, le24(b, 27)? + 1)),
        _ => None,
    }
}

/// The root element's `width` and `height`, or failing that its `viewBox`. Lengths in units other
/// than pixels, or in percent, are not a size; the viewBox then stands in.
fn svg(b: &[u8]) -> Option<(u32, u32)> {
    let text = std::str::from_utf8(b.get(..b.len().min(4096))?).ok().or_else(|| std::str::from_utf8(&b[..b.len().min(4096) - 1]).ok())?;
    let start = text.to_ascii_lowercase().find("<svg")?;
    let tag = &text[start..];
    let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
    let attr = |name: &str| -> Option<String> {
        let lower = tag.to_ascii_lowercase();
        let at = lower.find(&format!(" {name}="))? + name.len() + 2;
        let quote = tag[at..].chars().next().filter(|c| matches!(c, '"' | '\''))?;
        let rest = &tag[at + 1..];
        Some(rest[..rest.find(quote)?].to_string())
    };
    let pixels = |value: String| -> Option<u32> {
        let number = value.trim().trim_end_matches("px");
        (number.chars().all(|c| c.is_ascii_digit() || c == '.')).then(|| number.parse::<f32>().ok().map(|n| n.round() as u32))?
    };
    if let (Some(w), Some(h)) = (attr("width").and_then(pixels), attr("height").and_then(pixels)) {
        return Some((w, h));
    }
    let view_box = attr("viewbox")?;
    let numbers: Vec<f32> =
        view_box.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
    (numbers.len() == 4).then(|| (numbers[2].round() as u32, numbers[3].round() as u32))
}

#[cfg(test)]
mod tests;
