pub mod dock_window;
pub mod editor_window;
pub mod main_window;
pub mod style;
pub mod theme;
pub mod widgets;

use gpui::App;

pub fn init(cx: &mut App) {
    theme::apply_q_note_theme(cx);
    widgets::init(cx);
}
