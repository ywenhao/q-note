//! Visual tokens mirrored from the existing Vue `App.css` / `types.ts`.

use gpui::{BoxShadow, Hsla, Rgba, point, px, rgb, rgba};

use crate::models::{APP_BACKGROUND, EDITOR_BACKGROUND, parse_hex_color};

pub const APP_BG: u32 = APP_BACKGROUND;
pub const EDITOR_BG: u32 = EDITOR_BACKGROUND;
pub const TEXT: u32 = 0x1f2328;
pub const ACCENT: u32 = 0x007aff;
pub const DANGER: u32 = 0xff3b30;
pub const WINDOW_RADIUS: f32 = 12.0;
pub const CARD_RADIUS: f32 = 8.0;
pub const TOOLBAR_RADIUS: f32 = 8.0;
pub const LINE_HEIGHT: f32 = 22.0;
pub const SWITCH_ON: u32 = 0x34c759;

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
    color(parse_hex_color(hex))
}

pub fn toolbar_chip() -> Rgba {
    color_alpha(0xffffff, 0.14)
}

pub fn chrome_chip() -> Rgba {
    color_alpha(0xffffff, 0.26)
}

pub fn card_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color_alpha(0x1f2328, 0.10).into(),
        offset: point(px(0.), px(4.)),
        blur_radius: px(14.),
        spread_radius: px(0.),
    }]
}

pub fn card_shadow_hover() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color_alpha(0x1f2328, 0.14).into(),
        offset: point(px(0.), px(8.)),
        blur_radius: px(20.),
        spread_radius: px(0.),
    }]
}

pub fn modal_overlay_bg() -> Rgba {
    color_alpha(0x1d1d1f, 0.24)
}

pub fn modal_panel_bg() -> Rgba {
    color_alpha(0xfffdf4, 0.92)
}

pub fn confirm_panel_bg() -> Rgba {
    color_alpha(0xfffdf4, 0.90)
}

pub fn settings_group_bg() -> Rgba {
    color_alpha(0x3c3c43, 0.08)
}

pub fn settings_row_bg() -> Rgba {
    color_alpha(0xffffff, 0.58)
}

pub fn settings_row_bg_hover() -> Rgba {
    color_alpha(0xffffff, 0.78)
}

pub fn modal_panel_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color_alpha(0x1f2328, 0.16).into(),
        offset: point(px(0.), px(16.)),
        blur_radius: px(38.),
        spread_radius: px(0.),
    }]
}

pub fn confirm_panel_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color_alpha(0x1f2328, 0.22).into(),
        offset: point(px(0.), px(24.)),
        blur_radius: px(60.),
        spread_radius: px(0.),
    }]
}
