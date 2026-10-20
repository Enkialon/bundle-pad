pub const MAX_ASSET_BYTES: u64 = 40 * 1024 * 1024;

pub const MARKDOWN_UTI: &str = "net.daringfireball.markdown";

pub fn sniff_image(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if is_bmp(bytes) {
        return Some("image/bmp");
    }
    if is_avif(bytes) {
        return Some("image/avif");
    }
    None
}

pub fn extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        "image/avif" => Some("avif"),
        _ => None,
    }
}

pub fn sniff_for_display(bytes: &[u8], reference: &str) -> Option<String> {
    if let Some(mime) = sniff_image(bytes) {
        return Some(mime.to_string());
    }
    if reference.to_ascii_lowercase().ends_with(".svg") && looks_like_svg(bytes) {
        return Some("image/svg+xml".to_string());
    }
    None
}

fn is_bmp(bytes: &[u8]) -> bool {
    if bytes.len() < 14 || !bytes.starts_with(b"BM") {
        return false;
    }
    let declared = u32::from_le_bytes(bytes[2..6].try_into().unwrap()) as usize;
    declared == bytes.len()
}

fn is_avif(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return false;
    }
    matches!(&bytes[8..12], b"avif" | b"avis")
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_start().to_ascii_lowercase();
    let starts = trimmed.starts_with("<svg") || trimmed.starts_with("<?xml") || trimmed.starts_with("<!doctype svg");
    starts && trimmed.contains("<svg") && !trimmed.contains("<script") && !trimmed.contains("javascript:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_png_and_rejects_text() {
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
        assert_eq!(sniff_image(b"not an image"), None);
        assert_eq!(sniff_image(b"BM this is not a bitmap"), None);
    }

    #[test]
    fn svg_display_rejects_script() {
        assert_eq!(
            sniff_for_display(br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#, "assets/a.svg").as_deref(),
            Some("image/svg+xml")
        );
        assert_eq!(sniff_for_display(br#"<svg><script>alert(1)</script></svg>"#, "assets/a.svg"), None);
    }
}
