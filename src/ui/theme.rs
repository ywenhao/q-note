//! Apply a light theme tuned to Q Note's yellow board aesthetic.
//!
//! `gpui-base` on crates.io is an empty stub and cannot help with styling.
//! We keep visual parity by customizing `gpui-component` Theme colors and
//! painting shell/card surfaces with the original CSS tokens in `style.rs`.

use gpui::App;
use gpui_component::{Theme, ThemeColor, ThemeMode};

use super::style::{ACCENT, APP_BG, DANGER, TEXT, color_alpha, hsla_from_hex};

pub fn apply_q_note_theme(cx: &mut App) {
    let theme = Theme::global_mut(cx);
    theme.mode = ThemeMode::Light;
    theme.radius = gpui::px(8.);
    theme.radius_lg = gpui::px(12.);
    theme.shadow = true;
    theme.font_size = gpui::px(14.);

    let mut colors = *ThemeColor::light();
    colors.background = hsla_from_hex(APP_BG);
    colors.foreground = hsla_from_hex(TEXT);
    colors.primary = hsla_from_hex(ACCENT);
    colors.primary_hover = hsla_from_hex(0x0066d6);
    colors.primary_active = hsla_from_hex(0x0055b3);
    colors.primary_foreground = hsla_from_hex(0xffffff);
    colors.danger = hsla_from_hex(DANGER);
    colors.danger_hover = hsla_from_hex(0xe0342b);
    colors.danger_active = hsla_from_hex(0xc42e26);
    colors.danger_foreground = hsla_from_hex(0xffffff);
    colors.accent = hsla_from_hex(0xffe08a);
    colors.accent_foreground = hsla_from_hex(TEXT);
    colors.muted = hsla_from_hex(0xffe8a0);
    colors.muted_foreground = hsla_from_hex(0x5c636a);
    colors.popover = hsla_from_hex(0xfff7e0);
    colors.popover_foreground = hsla_from_hex(TEXT);
    colors.border = hsla_from_hex(0xe6c04a);
    colors.input = hsla_from_hex(0xe6c04a);
    colors.secondary = hsla_from_hex(0xffe8a0);
    colors.secondary_foreground = hsla_from_hex(TEXT);
    colors.secondary_hover = hsla_from_hex(0xffdf80);
    colors.secondary_active = hsla_from_hex(0xffd666);
    colors.list = hsla_from_hex(APP_BG);
    colors.list_hover = hsla_from_hex(0xffe08a);
    colors.list_active = hsla_from_hex(0xffe08a);
    colors.title_bar = hsla_from_hex(APP_BG);
    colors.title_bar_border = hsla_from_hex(APP_BG);
    colors.sidebar = hsla_from_hex(APP_BG);
    colors.tab_bar = hsla_from_hex(APP_BG);
    colors.overlay = color_alpha(0x1d1d1f, 0.24).into();
    theme.colors = colors;
}
