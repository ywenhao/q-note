use std::{path::Path, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use gpui::{Image, ImageFormat, ImageSource};

use crate::models::{AttachmentSource, NoteAttachment};

pub(crate) fn attachment_image_source(attachment: &NoteAttachment) -> Option<ImageSource> {
    match attachment.source {
        AttachmentSource::Path => Some(Path::new(&attachment.value).to_path_buf().into()),
        AttachmentSource::Url => Some(attachment.value.clone().into()),
        AttachmentSource::Data => image_from_data_url(&attachment.value)
            .map(Arc::new)
            .map(ImageSource::from),
    }
}

pub(crate) fn image_to_data_url(image: &Image) -> String {
    format!(
        "data:{};base64,{}",
        image.format.mime_type(),
        STANDARD.encode(&image.bytes)
    )
}

pub(crate) fn file_to_data_url(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    let format = image_format_for_path(path)
        .ok_or_else(|| anyhow::anyhow!("unsupported image format: {}", path.display()))?;
    Ok(format!(
        "data:{};base64,{}",
        format.mime_type(),
        STANDARD.encode(bytes)
    ))
}

fn image_from_data_url(value: &str) -> Option<Image> {
    let (header, encoded) = value.split_once(',')?;
    let mime = header
        .strip_prefix("data:")?
        .split(';')
        .next()
        .unwrap_or_default();
    let format = ImageFormat::from_mime_type(mime)?;
    let bytes = STANDARD.decode(encoded.trim()).ok()?;
    Some(Image::from_bytes(format, bytes))
}

fn image_format_for_path(path: &Path) -> Option<ImageFormat> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::Webp),
        "gif" => Some(ImageFormat::Gif),
        "svg" => Some(ImageFormat::Svg),
        "bmp" => Some(ImageFormat::Bmp),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        _ => None,
    }
}
