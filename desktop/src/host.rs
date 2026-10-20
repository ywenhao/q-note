use q_note_native::{
    app_state,
    geometry::{self, Rect},
    models::DockEdge,
};
use std::time::{Duration, Instant};

pub fn bundle(role: &str) -> anyhow::Result<nsh_engine::Bundle> {
    let code = match role {
        "main" => include_str!(concat!(env!("OUT_DIR"), "/main.js")),
        "editor" => include_str!(concat!(env!("OUT_DIR"), "/editor.js")),
        "dock" => include_str!(concat!(env!("OUT_DIR"), "/dock.js")),
        _ => anyhow::bail!("Unknown window role {role}"),
    };
    let mut bundle = nsh_engine::Bundle::source(code.to_owned());
    bundle.policy = nsh_engine::Policy::default();
    bundle.policy.declared = Some(nsh_core::access::Capabilities::from_json(&serde_json::json!({
        "network": { "hosts": ["*"], "http": [{"host":"*", "schemes":["http","https"], "methods":["GET"]}] },
        "clipboard": true
    })).map_err(|error| anyhow::anyhow!(error))?);
    Ok(bundle)
}

pub fn title(role: &str) -> String {
    let zh = app_state::settings().language == q_note_native::models::Language::Zh;
    if role == "editor" {
        format!(
            "Q Note - {}",
            match (zh, app_state::editing_existing()) {
                (true, true) => "编辑便签",
                (true, false) => "新建便签",
                (false, true) => "Edit note",
                (false, false) => "New note",
            }
        )
    } else {
        "Q Note".into()
    }
}

pub struct Dock {
    pub full: Rect,
    pub frame: Rect,
    pub edge: Option<DockEdge>,
    pub active: bool,
    pub dragging: bool,
    pub retained: Option<Instant>,
    pub hide_after: Option<Instant>,
    pub animation: Option<(Rect, Rect, Instant)>,
}

impl Default for Dock {
    fn default() -> Self {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: 30.0,
            height: 30.0,
        };
        Self {
            full: frame,
            frame,
            edge: None,
            active: false,
            dragging: false,
            retained: None,
            hide_after: None,
            animation: None,
        }
    }
}

impl Dock {
    pub fn anchor(&self) -> Option<Rect> {
        self.retained
            .filter(|time| time.elapsed() < Duration::from_secs(300))
            .map(|_| self.full)
    }
    pub fn open(&mut self, main: Rect, area: Rect) {
        let retained = self.anchor().is_some();
        if !retained {
            self.full = Rect {
                x: main.x + main.width - 30.0,
                y: main.y + main.height - 30.0,
                width: 30.0,
                height: 30.0,
            }
            .clamp(area);
            self.edge = None;
        }
        self.full = self.full.clamp(area);
        if let Some(edge) = self.edge {
            let (full, current) = geometry::snap(self.full, area);
            if current == Some(edge) {
                self.full = full;
            } else {
                self.edge = None;
            }
        }
        self.frame = self.full;
        self.active = true;
        self.dragging = false;
        self.animation = None;
        self.hide_after = Some(Instant::now() + Duration::from_millis(500));
    }
    pub fn restore(&mut self) {
        if self.active {
            self.retained = Some(Instant::now());
        }
        self.active = false;
        self.animation = None;
        self.hide_after = None;
    }
    pub fn hover(&mut self, inside: bool) {
        if self.dragging {
            return;
        }
        self.hide_after = None;
        if let Some(edge) = self.edge {
            self.animate(if inside {
                self.full
            } else {
                geometry::hidden(self.full, edge)
            });
        }
    }
    pub fn drag(&mut self, dx: f64, dy: f64, area: Rect) {
        if !self.dragging {
            self.animation = None;
            self.frame = self.full;
            self.dragging = true;
            self.hide_after = None;
        }
        self.full = Rect {
            x: self.full.x + dx,
            y: self.full.y + dy,
            ..self.full
        }
        .clamp(area);
        self.frame = self.full;
        self.edge = None;
    }
    pub fn release(&mut self, area: Rect) {
        self.dragging = false;
        let (full, edge) = geometry::snap(self.full, area);
        self.full = full;
        self.frame = full;
        self.edge = edge;
        self.hide_after = Some(Instant::now() + Duration::from_millis(500));
    }
    fn animate(&mut self, target: Rect) {
        self.animation = Some((self.frame, target, Instant::now()));
    }
    pub fn tick(&mut self) -> bool {
        if !self.active {
            return false;
        }
        if self.hide_after.is_some_and(|time| Instant::now() >= time) {
            self.hide_after = None;
            if let Some(edge) = self.edge {
                self.animate(geometry::hidden(self.full, edge));
            }
        }
        if let Some((from, to, start)) = self.animation {
            let t = (start.elapsed().as_secs_f64() / 0.130).min(1.0);
            let eased = 1.0 - (1.0 - t).powi(3);
            self.frame = Rect {
                x: from.x + (to.x - from.x) * eased,
                y: from.y + (to.y - from.y) * eased,
                ..to
            };
            if t >= 1.0 {
                self.animation = None;
            }
            return true;
        }
        false
    }
}

pub fn tray_labels(docked: bool) -> [String; 4] {
    let settings = app_state::settings();
    let zh = settings.language == q_note_native::models::Language::Zh;
    [
        (match (zh, settings.always_on_top) {
            (true, true) => "取消置顶",
            (true, false) => "置顶",
            (false, true) => "Unpin window",
            (false, false) => "Pin window",
        })
        .into(),
        if zh {
            "切换语言"
        } else {
            "Switch language"
        }
        .into(),
        match (zh, docked) {
            (true, true) => "切换主界面",
            (true, false) => "切换悬浮球",
            (false, true) => "Switch to main window",
            (false, false) => "Switch to Q icon",
        }
        .into(),
        if zh { "退出" } else { "Quit" }.into(),
    ]
}

pub fn tray_action(index: usize) {
    let command = ["topmost", "language", "dock", "quit"][index];
    let _ = app_state::invoke(command, serde_json::json!({"window":"main"}));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn area() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1600.0,
            height: 1000.0,
        }
    }
    fn main_frame() -> Rect {
        Rect {
            x: 1260.0,
            y: 275.0,
            width: 300.0,
            height: 450.0,
        }
    }
    #[test]
    fn restoring_before_first_dock_does_not_retain_the_origin() {
        let mut dock = Dock::default();
        dock.restore();
        assert!(dock.anchor().is_none());
        dock.open(main_frame(), area());
        assert_eq!((dock.full.x, dock.full.y), (1530.0, 695.0));
    }
    #[test]
    fn retained_edge_survives_main_monitor_changes_and_expires_after_five_minutes() {
        let mut dock = Dock::default();
        dock.open(main_frame(), area());
        dock.drag(40.0, -210.0, area());
        dock.release(area());
        assert_eq!(dock.edge, Some(DockEdge::Right));
        dock.restore();
        let anchor = dock.anchor().unwrap();
        dock.open(
            Rect {
                x: 1900.0,
                ..main_frame()
            },
            area(),
        );
        assert_eq!(dock.full, anchor);
        assert_eq!(dock.edge, Some(DockEdge::Right));
        dock.restore();
        dock.retained = Some(Instant::now() - Duration::from_secs(301));
        assert!(dock.anchor().is_none());
        dock.open(main_frame(), area());
        assert_eq!((dock.full.x, dock.full.y), (1530.0, 695.0));
        assert_eq!(dock.edge, None);
    }
}
