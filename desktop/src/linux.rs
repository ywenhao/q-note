use crate::host::{self, Dock};
use gtk::{gdk, glib, prelude::*};
use nsh_platform_linux::{AppOptions, AppView, Appearance, Limits, gtk4 as gtk};
use q_note_native::{
    app_state,
    geometry::{self, Rect},
};
use serde_json::json;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::{
            self, AtomEnum, ClientMessageData, ClientMessageEvent, ConfigureWindowAux,
            ConnectionExt as _, EventMask,
        },
    },
    rust_connection::RustConnection,
};

struct Window {
    native: gtk::ApplicationWindow,
    view: Rc<AppView>,
}
struct Host {
    app: gtk::Application,
    windows: HashMap<String, Window>,
    dock: Dock,
    x11: Option<(RustConnection, usize)>,
    last_frame: Option<Rect>,
    changed: Option<Instant>,
    tray: Option<ksni::blocking::Handle<Tray>>,
    cursor: Option<(f64, f64)>,
}

pub fn run() -> anyhow::Result<()> {
    nsh_platform_linux::set_ui_sets(vec![
        nsh_ui_mobile::registry(),
        q_note_native::__nsh_native_module(),
    ]);
    glib::set_prgname(Some("com.win11.q-note"));
    let app = gtk::Application::builder()
        .application_id("com.win11.q-note")
        .build();
    let running: Rc<RefCell<Option<Host>>> = Rc::new(RefCell::new(None));
    let active = running.clone();
    app.connect_activate(move |app| {
        if let Some(host)=active.borrow_mut().as_mut(){host.show_main();return;}
        let css=gtk::CssProvider::new();css.load_from_string("window.q-note { background: transparent; border: none; box-shadow: none; } window.q-note > box { background: transparent; }");
        if let Some(display)=gdk::Display::default(){gtk::style_context_add_provider_for_display(&display,&css,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);}
        let x11=x11rb::connect(None).ok();
        let mut host=Host {app:app.clone(),windows:HashMap::new(),dock:Dock::default(),x11,last_frame:None,changed:None,tray:None,cursor:None};
        let frame=geometry::main_frame(app_state::settings().window.as_ref(),host.area(None));
        if let Err(error)=host.open("main",frame){eprintln!("{error}");app.quit();return;}
        host.tray=make_tray();
        if app_state::has_editor(){let _=host.edit();}
        *active.borrow_mut()=Some(host);
        app_state::start_update_schedule();
    });
    let ticks = running.clone();
    glib::timeout_add_local(Duration::from_millis(16), move || {
        if let Some(host) = ticks.borrow_mut().as_mut() {
            host.tick();
        }
        glib::ControlFlow::Continue
    });
    if let Some(ms) = std::env::var("Q_NOTE_TEST_EXIT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    {
        let app = app.clone();
        glib::timeout_add_local_once(Duration::from_millis(ms), move || app.quit());
    }
    let shots = running.clone();
    if let Ok(path) = std::env::var("Q_NOTE_SCREENSHOT") {
        glib::timeout_add_local_once(Duration::from_millis(1500), move || {
            if let Some(host) = shots.borrow().as_ref() {
                if let Some(window) = host.windows.get("main") {
                    screenshot(&window.native, &path);
                }
            }
        });
    }
    app.run_with_args::<&str>(&[]);
    if let Some(host) = running.borrow_mut().take() {
        for window in host.windows.values() {
            window.view.stop();
        }
    }
    Ok(())
}

impl Host {
    fn open(&mut self, role: &str, frame: Rect) -> anyhow::Result<()> {
        if self.windows.contains_key(role) {
            let window = &self.windows[role].native;
            window.set_title(Some(&host::title(role)));
            window.present();
            return Ok(());
        }
        let window = gtk::ApplicationWindow::builder()
            .application(&self.app)
            .title(host::title(role))
            .default_width(frame.width as i32)
            .default_height(frame.height as i32)
            .decorated(false)
            .resizable(role != "dock")
            .build();
        window.add_css_class("q-note");
        if role != "dock" {
            window.set_size_request(
                if role == "editor" { 420 } else { 300 },
                if role == "editor" { 520 } else { 450 },
            );
        }
        let view = AppView::new(AppOptions {
            app_id: "com.win11.q-note".into(),
            title: host::title(role),
            size: (frame.width as i32, frame.height as i32),
            appearance: Appearance::Light,
            limits: Limits::standalone_desktop(),
            ..Default::default()
        });
        window.set_child(Some(&view.widget()));
        view.load(host::bundle(role)?.into());
        let close_role = role.to_owned();
        window.connect_close_request(move |_| {
            let _ = app_state::invoke("close", json!({"window":close_role}));
            glib::Propagation::Stop
        });
        let context = gtk::GestureClick::new();
        context.set_button(3);
        context.set_propagation_phase(gtk::PropagationPhase::Capture);
        let target = role.to_owned();
        context.connect_pressed(move |gesture, _, x, y| {
            app_state::event(&target, json!({"kind":"context","x":x,"y":y}));
            gesture.set_state(gtk::EventSequenceState::Claimed);
        });
        window.add_controller(context);
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        wheel.set_propagation_phase(gtk::PropagationPhase::Capture);
        let target = role.to_owned();
        wheel.connect_scroll(move |_, _, dy| {
            if app_state::preview_open(&target) {
                app_state::event(&target, json!({"kind":"wheel","delta":dy}));
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(wheel);
        if role == "editor" {
            let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
            drop.set_propagation_phase(gtk::PropagationPhase::Capture);
            drop.connect_drop(|_, value, _, _| {
                if let Ok(files) = value.get::<gdk::FileList>() {
                    let paths: Vec<_> = files
                        .files()
                        .into_iter()
                        .filter_map(|f| f.path().map(|p| p.to_string_lossy().into_owned()))
                        .collect();
                    app_state::event("editor", json!({"kind":"files","paths":paths}));
                    true
                } else {
                    false
                }
            });
            window.add_controller(drop);
        }
        window.present();
        self.windows.insert(
            role.into(),
            Window {
                native: window,
                view,
            },
        );
        self.set_frame(role, frame);
        self.topmost(role);
        Ok(())
    }
    fn frame(&self, role: &str) -> Rect {
        let Some(window) = self.windows.get(role) else {
            return Rect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 450.0,
            };
        };
        let scale = window.native.scale_factor() as f64;
        let mut frame = Rect {
            x: 0.0,
            y: 0.0,
            width: window.native.width() as f64,
            height: window.native.height() as f64,
        };
        if let (Some((conn, screen)), Some(xid)) = (&self.x11, xid(&window.native)) {
            let root = conn.setup().roots[*screen].root;
            if let Ok(cookie) = conn.translate_coordinates(xid, root, 0, 0) {
                if let Ok(reply) = cookie.reply() {
                    frame.x = reply.dst_x as f64 / scale;
                    frame.y = reply.dst_y as f64 / scale;
                }
            }
        }
        frame
    }
    fn set_frame(&self, role: &str, frame: Rect) {
        let Some(window) = self.windows.get(role) else {
            return;
        };
        let scale = window.native.scale_factor() as f64;
        if let (Some((conn, _)), Some(xid)) = (&self.x11, xid(&window.native)) {
            let _ = conn.configure_window(
                xid,
                &ConfigureWindowAux::new()
                    .x((frame.x * scale).round() as i32)
                    .y((frame.y * scale).round() as i32)
                    .width((frame.width * scale).round() as u32)
                    .height((frame.height * scale).round() as u32),
            );
            let _ = conn.flush();
        }
        window
            .native
            .set_default_size(frame.width as i32, frame.height as i32);
    }
    fn area(&self, role: Option<&str>) -> Rect {
        let frame = role.map(|r| self.frame(r));
        self.area_near(frame)
    }
    fn area_near(&self, frame: Option<Rect>) -> Rect {
        if let Some(display) = gdk::Display::default() {
            let monitors = display.monitors();
            let mut selected = None;
            for index in 0..monitors.n_items() {
                if let Some(monitor) = monitors.item(index).and_downcast::<gdk::Monitor>() {
                    let g = monitor.geometry();
                    let area = Rect {
                        x: g.x() as f64,
                        y: g.y() as f64,
                        width: g.width() as f64,
                        height: g.height() as f64,
                    };
                    if selected.is_none() {
                        selected = Some(area);
                    }
                    if frame.is_some_and(|f| {
                        f.x + f.width / 2.0 >= area.x
                            && f.x + f.width / 2.0 < area.x + area.width
                            && f.y + f.height / 2.0 >= area.y
                            && f.y + f.height / 2.0 < area.y + area.height
                    }) {
                        selected = Some(area);
                        break;
                    }
                }
            }
            if let Some(mut area) = selected {
                if let Some((conn, screen)) = &self.x11 {
                    let root = conn.setup().roots[*screen].root;
                    if let Ok(atom) = conn.intern_atom(false, b"_NET_WORKAREA") {
                        if let Ok(atom) = atom.reply() {
                            if let Ok(value) =
                                conn.get_property(false, root, atom.atom, AtomEnum::CARDINAL, 0, 4)
                            {
                                if let Ok(value) = value.reply() {
                                    if let Some(mut values) = value.value32() {
                                        let x = values.next().unwrap_or(0) as i32 as f64;
                                        let y = values.next().unwrap_or(0) as i32 as f64;
                                        let w = values.next().unwrap_or(area.width as u32) as f64;
                                        let h = values.next().unwrap_or(area.height as u32) as f64;
                                        let right = (area.x + area.width).min(x + w);
                                        let bottom = (area.y + area.height).min(y + h);
                                        area.x = area.x.max(x);
                                        area.y = area.y.max(y);
                                        area.width = (right - area.x).max(30.0);
                                        area.height = (bottom - area.y).max(30.0);
                                    }
                                }
                            }
                        }
                    }
                }
                return area;
            }
        }
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        }
    }
    fn topmost(&self, role: &str) {
        if let (Some((conn, screen)), Some(window)) = (&self.x11, self.windows.get(role)) {
            if let Some(xid) = xid(&window.native) {
                let atom = |name: &[u8]| {
                    conn.intern_atom(false, name)
                        .ok()
                        .and_then(|c| c.reply().ok())
                        .map(|r| r.atom)
                };
                if let (Some(state), Some(above)) =
                    (atom(b"_NET_WM_STATE"), atom(b"_NET_WM_STATE_ABOVE"))
                {
                    let event = ClientMessageEvent::new(
                        32,
                        xid,
                        state,
                        ClientMessageData::from([
                            u32::from(app_state::settings().always_on_top),
                            above,
                            0,
                            1,
                            0,
                        ]),
                    );
                    let _ = conn.send_event(
                        false,
                        conn.setup().roots[*screen].root,
                        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                        event,
                    );
                    let _ = conn.flush();
                }
            }
        }
    }
    fn show_main(&mut self) {
        self.dock.restore();
        if let Some(window) = self.windows.get("dock") {
            window.native.set_visible(false);
        }
        if let Some(window) = self.windows.get("main") {
            window.native.present();
        }
        self.update_tray();
    }
    fn edit(&mut self) -> anyhow::Result<()> {
        self.show_main();
        let frame = geometry::editor_frame(self.frame("main"), self.area(Some("main")));
        self.open("editor", frame)
    }
    fn toggle_dock(&mut self) -> anyhow::Result<()> {
        if self.dock.active {
            self.show_main();
            return Ok(());
        }
        self.save_window()?;
        let main = self.frame("main");
        self.dock
            .open(main, self.area_near(self.dock.anchor().or(Some(main))));
        self.open("dock", self.dock.frame)?;
        if let Some(window) = self.windows.get("main") {
            window.native.set_visible(false);
        }
        app_state::event("dock", json!({"kind":"showDock"}));
        self.clip_dock();
        self.update_tray();
        Ok(())
    }
    fn save_window(&self) -> anyhow::Result<()> {
        app_state::save_window(self.frame("main").state())
    }
    fn cursor(&self) -> Option<(f64, f64)> {
        let (conn, screen) = self.x11.as_ref()?;
        let reply = conn
            .query_pointer(conn.setup().roots[*screen].root)
            .ok()?
            .reply()
            .ok()?;
        let scale = self
            .windows
            .get("dock")
            .map(|window| window.native.scale_factor() as f64)
            .unwrap_or(1.0);
        Some((reply.root_x as f64 / scale, reply.root_y as f64 / scale))
    }
    fn update_tray(&self) {
        if let Some(tray) = &self.tray {
            let labels = tray_labels(self.dock.active);
            tray.update(|tray| tray.labels = labels);
        }
    }
    fn tick(&mut self) {
        for command in app_state::take_commands() {
            match command.action.as_str() {
                "edit" => {
                    let _ = self.edit();
                }
                "show" => self.show_main(),
                "dock" => {
                    app_state::report_save_error(self.toggle_dock());
                }
                "close" => {
                    if command.window == "main" {
                        app_state::report_save_error(self.save_window());
                    }
                    if let Some(window) = self.windows.get(&command.window) {
                        window.native.set_visible(false);
                    }
                }
                "minimize" => {
                    if let Some(window) = self.windows.get(&command.window) {
                        window.native.minimize();
                    }
                }
                "quit" => {
                    let saved = self.save_window().and_then(|_| app_state::flush_for_exit());
                    if saved.is_ok() {
                        self.app.quit();
                    } else {
                        app_state::report_save_error(saved);
                    }
                }
                "settings" => {
                    for (role, window) in &self.windows {
                        self.topmost(role);
                        window.native.set_title(Some(&host::title(role)));
                    }
                    self.update_tray();
                }
                "saveWindow" => app_state::window_saved(self.save_window()),
                "hoverDock" => self
                    .dock
                    .hover(command.payload["inside"].as_bool().unwrap_or(false)),
                "beginDockDrag" => self.cursor = self.cursor(),
                "dragDock" => {
                    let point = self.cursor();
                    let previous = self.cursor;
                    self.cursor = point;
                    let (dx, dy) = match (point, previous) {
                        (Some((x, y)), Some((px, py))) => (x - px, y - py),
                        _ => (
                            command.payload["dx"].as_f64().unwrap_or(0.0),
                            command.payload["dy"].as_f64().unwrap_or(0.0),
                        ),
                    };
                    let area = self.area_near(point.map(|(x, y)| Rect {
                        x,
                        y,
                        width: 0.0,
                        height: 0.0,
                    }));
                    self.dock.drag(dx, dy, area);
                    self.set_frame("dock", self.dock.frame);
                    self.clip_dock();
                }
                "endDockDrag" => {
                    self.cursor = None;
                    if command.payload["dragged"].as_bool().unwrap_or(false) {
                        self.dock.release(self.area(Some("dock")));
                        self.set_frame("dock", self.dock.frame);
                        self.clip_dock();
                    }
                }
                _ => {}
            }
        }
        if self.dock.tick() {
            self.set_frame("dock", self.dock.frame);
            self.clip_dock();
        }
        if self
            .windows
            .get("main")
            .is_some_and(|w| w.native.is_visible())
        {
            let frame = self.frame("main");
            if self.last_frame != Some(frame) {
                self.last_frame = Some(frame);
                self.changed = Some(Instant::now());
            }
            if frame.width > 1000.0 || frame.height > 800.0 {
                self.set_frame(
                    "main",
                    Rect {
                        width: frame.width.min(1000.0),
                        height: frame.height.min(800.0),
                        ..frame
                    },
                );
            }
            if self
                .changed
                .is_some_and(|time| time.elapsed() >= Duration::from_millis(300))
            {
                app_state::report_save_error(self.save_window());
                self.changed = None;
            }
        }
        if let Some(window) = self.windows.get("editor") {
            if window.native.width() > 1000 || window.native.height() > 800 {
                self.set_frame(
                    "editor",
                    Rect {
                        width: (window.native.width() as f64).min(1000.0),
                        height: (window.native.height() as f64).min(800.0),
                        ..self.frame("editor")
                    },
                );
            }
        }
    }
    fn clip_dock(&self) {
        let Some((conn, _)) = &self.x11 else {
            return;
        };
        let Some(window) = self.windows.get("dock") else {
            return;
        };
        let Some(xid) = xid(&window.native) else {
            return;
        };
        let scale = window.native.scale_factor() as f64;
        let size = (30.0 * scale) as i32;
        let area = self.area_near(Some(self.dock.full));
        let frame = self.dock.frame;
        let mut rects = vec![];
        for y in 0..size {
            let dy = (y as f64 + 0.5 - size as f64 / 2.0) / scale;
            if frame.y + y as f64 / scale < area.y
                || frame.y + y as f64 / scale >= area.y + area.height
            {
                continue;
            }
            let half = (225.0 - dy * dy).max(0.0).sqrt();
            let x0 = ((15.0 - half).max(area.x - frame.x) * scale)
                .ceil()
                .max(0.0) as i32;
            let x1 = ((15.0 + half).min(area.x + area.width - frame.x) * scale)
                .floor()
                .min(size as f64) as i32;
            if x1 > x0 {
                rects.push(xproto::Rectangle {
                    x: x0 as i16,
                    y: y as i16,
                    width: (x1 - x0) as u16,
                    height: 1,
                });
            }
        }
        let _ = conn.shape_rectangles(
            SO::SET,
            SK::BOUNDING,
            xproto::ClipOrdering::UNSORTED,
            xid,
            0,
            0,
            &rects,
        );
        let _ = conn.flush();
    }
}

fn xid(window: &gtk::ApplicationWindow) -> Option<u32> {
    let surface = window.surface()?;
    if !surface.type_().name().contains("X11") {
        return None;
    }
    unsafe extern "C" {
        fn gdk_x11_surface_get_xid(surface: *mut gdk::ffi::GdkSurface) -> std::ffi::c_ulong;
    }
    Some(unsafe { gdk_x11_surface_get_xid(surface.as_ptr()) } as u32)
}

struct Tray {
    labels: [String; 4],
    icon: Vec<u8>,
}
fn tray_labels(docked: bool) -> [String; 4] {
    host::tray_labels(docked)
}
impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "q-note".into()
    }
    fn title(&self) -> String {
        "Q Note".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: 32,
            height: 32,
            data: self.icon.clone(),
        }]
    }
    fn activate(&mut self, _: i32, _: i32) {
        let _ = app_state::invoke("show", json!({"window":"main"}));
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        self.labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                ksni::menu::StandardItem {
                    label: label.clone(),
                    activate: Box::new(move |_| host::tray_action(index)),
                    ..Default::default()
                }
                .into()
            })
            .collect()
    }
}
fn make_tray() -> Option<ksni::blocking::Handle<Tray>> {
    use ksni::blocking::TrayMethods;
    let icon = image::load_from_memory(include_bytes!("../../app-icon.png"))
        .ok()?
        .resize_exact(32, 32, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let bytes = icon
        .pixels()
        .flat_map(|p| [p[3], p[0], p[1], p[2]])
        .collect();
    match (Tray {
        labels: tray_labels(false),
        icon: bytes,
    })
    .disable_dbus_name(true)
    .spawn()
    {
        Ok(handle) => Some(handle),
        Err(error) => {
            eprintln!("Tray unavailable: {error}");
            None
        }
    }
}

fn screenshot(window: &gtk::ApplicationWindow, path: &str) {
    let snapshot = gtk::Snapshot::new();
    let paintable = gtk::WidgetPaintable::new(Some(window));
    paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
    if let (Some(node), Some(renderer)) = (snapshot.to_node(), window.renderer()) {
        let texture = renderer.render_texture(&node, None);
        if let Err(error) = texture.save_to_png(path) {
            eprintln!("Screenshot: {error}");
        }
    }
}
