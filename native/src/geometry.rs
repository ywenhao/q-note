use crate::models::{DockEdge, WindowState};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn clamp(self, area: Rect) -> Rect {
        Rect {
            x: self
                .x
                .clamp(area.x, (area.x + area.width - self.width).max(area.x)),
            y: self
                .y
                .clamp(area.y, (area.y + area.height - self.height).max(area.y)),
            ..self
        }
    }

    pub fn state(self) -> WindowState {
        WindowState {
            width: self.width as f32,
            height: self.height as f32,
            x: self.x as f32,
            y: self.y as f32,
        }
    }
}

pub fn main_frame(saved: Option<&WindowState>, area: Rect) -> Rect {
    let width = saved
        .map_or(300.0, |s| f64::from(s.width))
        .clamp(300.0, 1000.0);
    let height = saved
        .map_or(450.0, |s| f64::from(s.height))
        .clamp(450.0, 800.0);
    Rect {
        x: area.x + area.width - width - 40.0,
        y: area.y + (area.height - height) / 2.0,
        width,
        height,
    }
    .clamp(area)
}

pub fn editor_frame(main: Rect, area: Rect) -> Rect {
    Rect {
        x: main.x - 532.0,
        y: main.y + (main.height - 640.0) / 2.0,
        width: 520.0,
        height: 640.0,
    }
    .clamp(area)
}

pub fn snap(frame: Rect, area: Rect) -> (Rect, Option<DockEdge>) {
    let frame = frame.clamp(area);
    let candidates = [
        (frame.x - area.x, DockEdge::Left),
        (area.x + area.width - frame.x - 30.0, DockEdge::Right),
        (frame.y - area.y, DockEdge::Top),
        (area.y + area.height - frame.y - 30.0, DockEdge::Bottom),
    ];
    let (distance, edge) = candidates
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap();
    if distance > 28.0 {
        return (frame, None);
    }
    let mut result = frame;
    match edge {
        DockEdge::Left => result.x = area.x,
        DockEdge::Right => result.x = area.x + area.width - 30.0,
        DockEdge::Top => result.y = area.y,
        DockEdge::Bottom => result.y = area.y + area.height - 30.0,
    }
    (result, Some(edge))
}

pub fn hidden(frame: Rect, edge: DockEdge) -> Rect {
    let mut result = frame;
    match edge {
        DockEdge::Left => result.x -= 15.0,
        DockEdge::Right => result.x += 15.0,
        DockEdge::Top => result.y -= 15.0,
        DockEdge::Bottom => result.y += 15.0,
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snap_threshold_and_negative_monitor_coordinates() {
        let area = Rect {
            x: -1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1040.0,
        };
        let frame = Rect {
            x: -1892.0,
            y: 300.0,
            width: 30.0,
            height: 30.0,
        };
        let (snapped, edge) = snap(frame, area);
        assert_eq!(edge, Some(DockEdge::Left));
        assert_eq!(hidden(snapped, edge.unwrap()).x, -1935.0);
        assert!(
            snap(
                Rect {
                    x: -1891.0,
                    ..frame
                },
                area
            )
            .1
            .is_none()
        );
    }
}
