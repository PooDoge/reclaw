use super::*;

fn png_header(w: u32, h: u32) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b.extend([8, 6, 0, 0, 0]);
    b
}

#[test]
fn png_reads_the_header_chunk() {
    assert_eq!(dimensions(ImageKind::Png, &png_header(1920, 1080)), Some((1920, 1080)));
}

#[test]
fn gif_reads_the_logical_screen() {
    let mut b = b"GIF89a".to_vec();
    b.extend(320u16.to_le_bytes());
    b.extend(200u16.to_le_bytes());
    assert_eq!(dimensions(ImageKind::Gif, &b), Some((320, 200)));
}

#[test]
fn jpeg_skips_segments_to_the_frame_header() {
    // SOI, an APP0 segment of 16 bytes, a DQT-like segment, then SOF0 with 480x640.
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    b.extend([0u8; 14]);
    b.extend([0xFF, 0xDB, 0x00, 0x04, 0, 0]);
    b.extend([0xFF, 0xC0, 0x00, 0x11, 0x08]);
    b.extend(640u16.to_be_bytes()); // height
    b.extend(480u16.to_be_bytes()); // width
    assert_eq!(dimensions(ImageKind::Jpeg, &b), Some((480, 640)));
}

#[test]
fn jpeg_tables_that_share_the_marker_range_are_not_frames() {
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xC4, 0x00, 0x04, 0, 0];
    b.extend([0xFF, 0xC2, 0x00, 0x11, 0x08, 0x00, 0x64, 0x00, 0xC8]);
    assert_eq!(dimensions(ImageKind::Jpeg, &b), Some((200, 100)), "progressive frames count; the Huffman table does not");
}

#[test]
fn jpeg_with_no_frame_before_the_scan_has_no_size() {
    assert_eq!(dimensions(ImageKind::Jpeg, &[0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02]), None);
}

fn riff(chunk: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b = b"RIFF\0\0\0\0WEBP".to_vec();
    b.extend(chunk);
    b.extend((body.len() as u32).to_le_bytes());
    b.extend(body);
    b
}

#[test]
fn webp_lossy_lossless_and_extended() {
    let mut lossy = vec![0u8; 6]; // frame tag and start code
    lossy.extend(800u16.to_le_bytes());
    lossy.extend(600u16.to_le_bytes());
    assert_eq!(dimensions(ImageKind::Webp, &riff(b"VP8 ", &lossy)), Some((800, 600)));

    let (w, h) = (1023u32, 511u32);
    let bits = (w - 1) | ((h - 1) << 14);
    let mut lossless = vec![0x2f];
    lossless.extend(bits.to_le_bytes());
    assert_eq!(dimensions(ImageKind::Webp, &riff(b"VP8L", &lossless)), Some((1023, 511)));

    let mut extended = vec![0u8; 4];
    extended.extend(&(4000u32 - 1).to_le_bytes()[..3]);
    extended.extend(&(3000u32 - 1).to_le_bytes()[..3]);
    assert_eq!(dimensions(ImageKind::Webp, &riff(b"VP8X", &extended)), Some((4000, 3000)));
}

#[test]
fn svg_uses_its_size_or_its_view_box() {
    assert_eq!(dimensions(ImageKind::Svg, br#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40"></svg>"#), Some((120, 40)));
    assert_eq!(dimensions(ImageKind::Svg, br#"<?xml version="1.0"?><svg viewBox="0 0 24 24" xmlns="x"/>"#), Some((24, 24)));
    assert_eq!(
        dimensions(ImageKind::Svg, br#"<svg width="100%" height="100%" viewBox="0 0 300 150"/>"#),
        Some((300, 150)),
        "percent is not a size"
    );
    assert_eq!(dimensions(ImageKind::Svg, br#"<svg width='64px' height='32px'/>"#), Some((64, 32)));
    assert_eq!(dimensions(ImageKind::Svg, b"<svg/>"), None);
}

#[test]
fn truncated_and_empty_files_have_no_size_and_do_not_panic() {
    for kind in [ImageKind::Png, ImageKind::Gif, ImageKind::Jpeg, ImageKind::Webp, ImageKind::Svg] {
        assert_eq!(dimensions(kind, b""), None, "{kind:?}");
        assert_eq!(dimensions(kind, &[0xFF; 7]), None, "{kind:?}");
    }
    let full = png_header(10, 10);
    for cut in 0..full.len() {
        let _ = dimensions(ImageKind::Png, &full[..cut]);
    }
}

#[test]
fn a_size_of_zero_is_not_a_size() {
    assert_eq!(dimensions(ImageKind::Png, &png_header(0, 100)), None);
}
