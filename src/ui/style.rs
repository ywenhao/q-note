//! Visual tokens mirrored from the existing Vue `App.css` / `types.ts`.

use gpui::{Hsla, Rgba, rgb, rgba};

pub const APP_BG: u32 = 0xffd150;
pub const EDITOR_BG: u32 = 0xfff9df;
pub const TEXT: u32 = 0x1f2328;
pub const ACCENT: u32 = 0x007aff;
pub const DANGER: u32 = 0xff3b30;
pub const WINDOW_RADIUS: f32 = 12.0;
pub const CARD_RADIUS: f32 = 14.0;
pub const TOOLBAR_RADIUS: f32 = 8.0;
pub const LINE_HEIGHT: f32 = 22.0;

pub fn color(hex: u32) -> Rgba {
    rgb(hex)
}

/// `hex` is 0xRRGGBB; alpha is 0.0–1.0.
pub fn color_alpha(hex: u32, alpha: f32) -> Rgba {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    rgba((hex << 8) | a)
}

pub fn hsla_from_hex(hex: u32) -> Hsla {
    color(hex).into()
}

pub fn parse_note_color(hex: &str) -> Rgba {
    let h = hex.trim_start_matches('#');
    let value = u32::from_str_radix(h, 16).unwrap_or(0xfff9db);
    color(value)
}

pub fn toolbar_chip() -> Rgba {
    color_alpha(0xffffff, 0.14)
}

pub fn chrome_chip() -> Rgba {
    color_alpha(0xffffff, 0.26)
}

pub fn card_shadow() -> Vec<gpui::BoxShadow> {
    vec![gpui::BoxShadow {
        color: color_alpha(0x1f2328, 0.10).into(),
        offset: gpui::point(gpui::px(0.), gpui::px(4.)),
        blur_radius: gpui::px(14.),
        spread_radius: gpui::px(0.),
    }]
}

pub fn card_shadow_hover() -> Vec<gpui::BoxShadow> {
    vec![gpui::BoxShadow {
        color: color_alpha(0x1f2328, 0.14).into(),
        offset: gpui::point(gpui::px(0.), gpui::px(8.)),
        blur_radius: gpui::px(20.),
        spread_radius: gpui::px(0.),
    }]
}
