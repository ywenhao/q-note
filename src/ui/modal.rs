//! Settings / confirm overlays matching the Tauri Vue dialogs.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, Div, InteractiveElement, IntoElement, ParentElement,
    SharedString, Stateful, Styled, div, px, relative, svg,
};
use gpui_component::{
    IconName, IconNamed as _, Sizable as _, animation::cubic_bezier, h_flex, spinner::Spinner,
    v_flex,
};

use super::style::{
    ACCENT, DANGER, SWITCH_ON, WINDOW_RADIUS, color, color_alpha, confirm_panel_bg,
    confirm_panel_shadow, modal_overlay_bg, modal_panel_bg, modal_panel_shadow, settings_group_bg,
    settings_row_bg, settings_row_bg_hover,
};

pub const SETTINGS_DIALOG_WIDTH: f32 = 276.;
pub const CONFIRM_DIALOG_WIDTH: f32 = 420.;
pub const OVERLAY_MS: u64 = 120;
pub const PANEL_MS: u64 = 140;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppModal {
    Settings,
    ConfirmDeleteAll,
}

pub(crate) fn overlay_ease() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.)
}

pub(crate) fn overlay_animation() -> Animation {
    Animation::new(Duration::from_millis(OVERLAY_MS)).with_easing(overlay_ease())
}

pub(crate) fn panel_animation() -> Animation {
    Animation::new(Duration::from_millis(PANEL_MS)).with_easing(overlay_ease())
}

pub(crate) fn modal_layer() -> Stateful<Div> {
    overlay_layer("modal-layer")
}

pub(crate) fn modal_backdrop() -> Stateful<Div> {
    overlay_backdrop("modal-backdrop")
}

pub(crate) fn animate_overlay(
    name: &'static str,
    overlay: Stateful<Div>,
    closing: bool,
    generation: u64,
) -> impl IntoElement {
    overlay.with_animation(
        SharedString::from(format!(
            "{name}-overlay-{}-{generation}",
            if closing { "out" } else { "in" }
        )),
        overlay_animation(),
        move |this, delta| {
            let t = if closing { 1. - delta } else { delta };
            this.opacity(t)
        },
    )
}

pub(crate) fn animate_panel(
    name: &'static str,
    panel: impl Styled + IntoElement + 'static,
    closing: bool,
    generation: u64,
) -> impl IntoElement {
    panel.with_animation(
        SharedString::from(format!(
            "{name}-panel-{}-{generation}",
            if closing { "out" } else { "in" }
        )),
        panel_animation(),
        move |this, delta| {
            let t = if closing { 1. - delta } else { delta };
            this.opacity(t).mt(px(8. * (1. - t)))
        },
    )
}

pub(crate) fn overlay_layer(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .rounded(px(WINDOW_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .p(px(18.))
        .occlude()
}

pub(crate) fn overlay_backdrop(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(modal_overlay_bg())
        .occlude()
}

pub(crate) fn settings_shell(lang_key: u64) -> Stateful<Div> {
    v_flex()
        .id(("settings-dialog", lang_key))
        .w(px(SETTINGS_DIALOG_WIDTH))
        .max_w(relative(1.))
        .gap(px(7.))
        .p(px(8.))
        .rounded(px(10.))
        .border_1()
        .border_color(color_alpha(0xffffff, 0.58))
        .bg(modal_panel_bg())
        .shadow(modal_panel_shadow())
}

pub(crate) fn confirm_shell(id: &'static str, lang_key: u64) -> Stateful<Div> {
    v_flex()
        .id((id, lang_key))
        .w_full()
        .max_w(px(CONFIRM_DIALOG_WIDTH))
        .p(px(18.))
        .rounded(px(8.))
        .border_1()
        .border_color(color_alpha(0xffffff, 0.52))
        .bg(confirm_panel_bg())
        .shadow(confirm_panel_shadow())
}

pub(crate) fn settings_group() -> Div {
    v_flex()
        .w_full()
        .gap(px(4.))
        .py(px(2.))
        .overflow_hidden()
        .rounded(px(9.))
        .bg(settings_group_bg())
}

pub(crate) fn settings_row(id: impl Into<SharedString>) -> Stateful<Div> {
    h_flex()
        .id(id.into())
        .w_full()
        .h(px(34.))
        .px(px(9.))
        .items_center()
        .justify_between()
        .gap(px(8.))
        .flex_none()
        .bg(settings_row_bg())
        .text_color(color(0x1d1d1f))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(settings_row_bg_hover()))
}

pub(crate) fn settings_action(id: impl Into<SharedString>) -> Stateful<Div> {
    h_flex()
        .id(id.into())
        .flex_1()
        .min_w_0()
        .h(px(34.))
        .px(px(8.))
        .items_center()
        .justify_center()
        .gap(px(6.))
        .bg(settings_row_bg())
        .text_color(color(0x1d1d1f))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(settings_row_bg_hover()))
}

pub(crate) fn settings_label(icon: impl Into<SharedString>, text: impl Into<SharedString>) -> Div {
    h_flex()
        .min_w_0()
        .items_center()
        .gap(px(6.))
        .child(settings_icon(icon))
        .child(text.into())
}

pub(crate) fn settings_icon(path: impl Into<SharedString>) -> gpui::Svg {
    svg()
        .size(px(12.))
        .flex_none()
        .text_color(color_alpha(0x1d1d1f, 0.72))
        .path(path)
}

pub(crate) fn settings_close_button() -> Stateful<Div> {
    h_flex()
        .id("settings-close")
        .size(px(24.))
        .flex_none()
        .justify_center()
        .rounded_full()
        .bg(color_alpha(0x787880, 0.12))
        .text_color(color_alpha(0x1d1d1f, 0.7))
        .cursor_pointer()
        .hover(|style| style.bg(color_alpha(0x787880, 0.18)))
        .child(
            svg()
                .size(px(14.))
                .flex_none()
                .text_color(color_alpha(0x1d1d1f, 0.7))
                .path(IconName::Close.path()),
        )
}

pub(crate) fn launch_switch(on: bool) -> impl IntoElement {
    div()
        .relative()
        .w(px(38.))
        .h(px(22.))
        .flex_none()
        .rounded_full()
        .bg(if on {
            color(SWITCH_ON)
        } else {
            color_alpha(0x787880, 0.24)
        })
        .child(
            div()
                .absolute()
                .top(px(3.))
                .left(px(if on { 19. } else { 3. }))
                .size(px(16.))
                .rounded_full()
                .bg(color(0xffffff))
                .shadow(vec![gpui::BoxShadow {
                    color: color_alpha(0x1f2328, 0.18).into(),
                    offset: gpui::point(px(0.), px(3.)),
                    blur_radius: px(8.),
                    spread_radius: px(0.),
                }]),
        )
}

pub(crate) fn update_dot() -> Div {
    div()
        .size(px(6.))
        .flex_none()
        .rounded_full()
        .bg(color(DANGER))
        .shadow(vec![gpui::BoxShadow {
            color: color_alpha(0xffffff, 0.82).into(),
            offset: gpui::point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: px(2.),
        }])
}

pub(crate) fn check_update_icon(checking: bool) -> impl IntoElement {
    if checking {
        Spinner::new()
            .icon(IconName::LoaderCircle)
            .xsmall()
            .color(color_alpha(0x1d1d1f, 0.72).into())
            .into_any_element()
    } else {
        settings_icon(crate::REFRESH_ICON_PATH).into_any_element()
    }
}

pub(crate) fn version_button() -> Stateful<Div> {
    div()
        .id("settings-version")
        .rounded_full()
        .px(px(6.))
        .py(px(1.))
        .text_xs()
        .text_color(color_alpha(0x3c3c43, 0.72))
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(color_alpha(0xffffff, 0.45))
                .text_color(color(ACCENT))
        })
}

pub(crate) fn text_button(id: impl Into<SharedString>) -> Stateful<Div> {
    h_flex()
        .id(id.into())
        .h(px(28.))
        .px(px(10.))
        .items_center()
        .justify_center()
        .rounded(px(8.))
        .border_1()
        .border_color(color_alpha(0x3c3c43, 0.13))
        .bg(color_alpha(0xffffff, 0.72))
        .text_color(color(0x1d1d1f))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(color_alpha(0xffffff, 0.84)))
}

pub(crate) fn danger_button(id: impl Into<SharedString>) -> Stateful<Div> {
    h_flex()
        .id(id.into())
        .h(px(28.))
        .px(px(10.))
        .items_center()
        .justify_center()
        .rounded(px(8.))
        .border_1()
        .border_color(color(DANGER))
        .bg(color(DANGER))
        .text_color(color(0xfff5f5))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(color(0xfd5f57)))
}

pub(crate) fn primary_button(id: impl Into<SharedString>) -> Stateful<Div> {
    h_flex()
        .id(id.into())
        .h(px(28.))
        .px(px(10.))
        .items_center()
        .justify_center()
        .rounded(px(8.))
        .border_1()
        .border_color(color_alpha(0x1d1d1f, 0.18))
        .bg(color_alpha(0x1d1d1f, 0.92))
        .text_color(color(0xffffff))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(color_alpha(0x1d1d1f, 0.75)))
}
