use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Duration,
};

pub fn request(source: String, value: String, window: String, request: String) {
    std::thread::spawn(move || {
        let result = resolve(&source, &value);
        if let Err(error) = &result {
            eprintln!("Image unavailable: {error}");
        }
        crate::app_state::event(
            &window,
            serde_json::json!({"kind":"imageReady","request":request,"value":result.unwrap_or_default()}),
        );
    });
}

fn resolve(source: &str, value: &str) -> Result<String> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(value) {
        return Ok(cached.clone());
    }
    let bytes = if source == "path" {
        let path = std::path::Path::new(value);
        if path.metadata()?.len() > 64 * 1024 * 1024 {
            bail!("Image exceeds 64 MiB");
        }
        std::fs::read(path)?
    } else if value.starts_with("data:") {
        let (header, data) = value.split_once(',').context("Invalid data URI")?;
        if header.ends_with(";base64") {
            base64::engine::general_purpose::STANDARD.decode(data)?
        } else {
            percent_encoding::percent_decode_str(data).collect()
        }
    } else {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .new_agent()
            .get(value)
            .call()?
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_vec()?
    };
    let svg = value.to_ascii_lowercase().starts_with("data:image/svg")
        || value
            .split(['?', '#'])
            .next()
            .unwrap_or(value)
            .to_ascii_lowercase()
            .ends_with(".svg");
    let (bytes, mime) = if svg {
        let mut options = resvg::usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        let tree = resvg::usvg::Tree::from_data(&bytes, &options)?;
        let size = tree.size();
        let scale = (4096.0 / size.width().max(size.height())).min(1.0);
        let width = (size.width() * scale).ceil() as u32;
        let height = (size.height() * scale).ceil() as u32;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1))
            .ok_or_else(|| anyhow!("Invalid SVG dimensions"))?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        (pixmap.encode_png()?, "image/png")
    } else {
        let path = value
            .split(['?', '#'])
            .next()
            .unwrap_or(value)
            .to_ascii_lowercase();
        let mime = if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg"
        } else if path.ends_with(".gif") {
            "image/gif"
        } else if path.ends_with(".webp") {
            "image/webp"
        } else if path.ends_with(".bmp") {
            "image/bmp"
        } else {
            "image/png"
        };
        (bytes, mime)
    };
    let result = format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    let mut cache = cache.lock().unwrap();
    if cache.len() >= 32
        || cache.values().map(String::len).sum::<usize>() + result.len() > 64 * 1024 * 1024
    {
        cache.clear();
    }
    if result.len() < 32 * 1024 * 1024 {
        cache.insert(value.into(), result.clone());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svg_data_uri_is_rendered_to_png_without_changing_the_source() {
        let value = "data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%20width=%2220%22%20height=%2210%22%3E%3Crect%20width=%2220%22%20height=%2210%22%20fill=%22red%22/%3E%3C/svg%3E";
        let result = resolve("data", value).unwrap();
        let png = base64::engine::general_purpose::STANDARD
            .decode(result.split_once(',').unwrap().1)
            .unwrap();
        let image = image::load_from_memory(&png).unwrap();
        assert_eq!((image.width(), image.height()), (20, 10));
    }
}
