use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::models::{AttachmentSource, NoteAttachment};

pub fn attachment_image(attachment: &NoteAttachment) -> Image {
    match attachment.source {
        AttachmentSource::Path => image_from_path(Path::new(&attachment.value)),
        AttachmentSource::Data => image_from_data_url(&attachment.value),
        // Remote images are filled asynchronously by the application bridge.
        AttachmentSource::Url => None,
    }
    .unwrap_or_default()
}

pub fn file_to_data_url(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    let mime = mime_for_path(path)
        .ok_or_else(|| anyhow::anyhow!("unsupported image format: {}", path.display()))?;
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

pub fn rgba_bytes_to_data_url(width: u32, height: u32, bytes: &[u8]) -> anyhow::Result<String> {
    let encoded = image::RgbaImage::from_raw(width, height, bytes.to_vec())
        .ok_or_else(|| anyhow::anyhow!("invalid RGBA clipboard image"))?;
    let mut output = std::io::Cursor::new(Vec::new());
    encoded.write_to(&mut output, image::ImageFormat::Png)?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(output.into_inner())
    ))
}

fn image_from_path(path: &Path) -> Option<Image> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
    {
        return std::fs::read(path)
            .ok()
            .and_then(|bytes| Image::load_from_svg_data(&bytes).ok());
    }
    Image::load_from_path(path).ok()
}

fn image_from_data_url(value: &str) -> Option<Image> {
    let (header, encoded) = value.split_once(',')?;
    let mime = header
        .strip_prefix("data:")?
        .split(';')
        .next()
        .unwrap_or_default();
    let bytes = STANDARD.decode(encoded.trim()).ok()?;
    if mime == "image/svg+xml" {
        return Image::load_from_svg_data(&bytes).ok();
    }
    raster_image(&bytes)
}

pub fn raster_image(bytes: &[u8]) -> Option<Image> {
    let decoded = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = decoded.dimensions();
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        decoded.as_raw().as_slice(),
        width,
        height,
    );
    Some(Image::from_rgba8(buffer))
}

pub fn downloaded_image(url: &str, bytes: &[u8]) -> Option<Image> {
    let clean = url.split(['?', '#']).next().unwrap_or(url);
    if clean.to_ascii_lowercase().ends_with(".svg")
        || bytes
            .iter()
            .copied()
            .skip_while(u8::is_ascii_whitespace)
            .take(4)
            .eq(b"<svg".iter().copied())
    {
        return Image::load_from_svg_data(bytes).ok();
    }
    raster_image(bytes)
}

fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "svg" => Some("image/svg+xml"),
        "bmp" => Some("image/bmp"),
        "tif" | "tiff" => Some("image/tiff"),
        _ => None,
    }
}
