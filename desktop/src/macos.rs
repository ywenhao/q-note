use crate::host::{self, Dock};
use block2::RcBlock;
use nsh_platform_macos::{AppOptions, AppView, Appearance, Limits};
use objc2::{
    MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSApplicationTerminateReply, NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSEventType,
    NSPasteboard, NSScreen, NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    NSArray, NSObject, NSObjectProtocol, NSPoint, NSRect, NSRunLoop, NSRunLoopCommonModes, NSSize,
    NSString, NSTimer,
};
use objc2_quartz_core::CALayer;
use q_note_native::{
    app_state,
    geometry::{self, Rect},
};
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ptr::NonNull,
    rc::Rc,
    time::{Duration, Instant},
};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

struct Window {
    native: Retained<QNoteWindow>,
    view: Rc<AppView>,
}
struct Host {
    mtm: MainThreadMarker,
    app: Retained<NSApplication>,
    delegate: Retained<Delegate>,
    windows: HashMap<String, Window>,
    dock: Dock,
    tray: TrayIcon,
    items: Vec<MenuItem>,
    last_frame: Option<Rect>,
    changed: Option<Instant>,
    cursor: Option<NSPoint>,
}
thread_local! {static HOST:RefCell<Option<Host>>=const{RefCell::new(None)};static QUITTING:Cell<bool>=const{Cell::new(false)};}

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind=MainThreadOnly]
    #[name="QNoteNativeWindow"]
    struct QNoteWindow;
    unsafe impl NSObjectProtocol for QNoteWindow {}
    impl QNoteWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self)->bool{true}
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self)->bool{true}
        #[unsafe(method(draggingEntered:))]
        fn dragging_entered(&self,_sender:&AnyObject)->usize{1}
        #[unsafe(method(performDragOperation:))]
        fn perform_drag(&self,sender:&AnyObject)->bool{
            unsafe{
                let pasteboard:Retained<NSPasteboard>=msg_send![sender,draggingPasteboard];
                let list:Option<Retained<NSArray<NSString>>>=msg_send![&*pasteboard,propertyListForType:objc2_foundation::ns_string!("NSFilenamesPboardType")];
                if let Some(list)=list{app_state::append_paths(list.iter().map(|path|path.to_string()).collect());true}else{false}
            }
        }
    }
);
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind=MainThreadOnly]
    #[name = "QNoteNativeDelegate"]
    struct Delegate;
    unsafe impl NSObjectProtocol for Delegate {}
    unsafe impl NSApplicationDelegate for Delegate {
        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn after_last(&self, _app: &NSApplication) -> bool {
            false
        }
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _app: &NSApplication) -> NSApplicationTerminateReply {
            if QUITTING.get() {
                NSApplicationTerminateReply::TerminateNow
            } else {
                let _ = app_state::invoke("quit", json!({}));
                NSApplicationTerminateReply::TerminateCancel
            }
        }
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            let _ = app_state::invoke("show", json!({"window":"main"}));
            true
        }
    }
    unsafe impl NSWindowDelegate for Delegate {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, window: &NSWindow) -> bool {
            if let Some(role) = role(window.windowNumber()) {
                let _ = app_state::invoke("close", json!({"window":role}));
            }
            false
        }
    }
);

pub fn run() -> anyhow::Result<()> {
    let mtm = MainThreadMarker::new().expect("AppKit must run on the main thread");
    nsh_platform_macos::set_ui_sets(vec![
        nsh_ui_mobile::registry(),
        q_note_native::__nsh_native_module(),
    ]);
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let delegate: Retained<Delegate> = unsafe { msg_send![Delegate::alloc(mtm), init] };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    let menu = Menu::new();
    let items: Vec<_> = host::tray_labels(false)
        .iter()
        .map(|label| MenuItem::new(label, true, None))
        .collect();
    for item in &items {
        menu.append(item)?;
    }
    let image = image::load_from_memory(include_bytes!("../../app-icon.png"))?
        .resize_exact(32, 32, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let tray = TrayIconBuilder::new()
        .with_tooltip("Q Note")
        .with_icon(Icon::from_rgba(image.into_raw(), 32, 32)?)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()?;
    let mut host = Host {
        mtm,
        app: app.clone(),
        delegate,
        windows: HashMap::new(),
        dock: Dock::default(),
        tray,
        items,
        last_frame: None,
        changed: None,
        cursor: None,
    };
    host.open(
        "main",
        geometry::main_frame(app_state::settings().window.as_ref(), area(mtm, None)),
    )?;
    if app_state::has_editor() {
        host.edit()?;
    }
    HOST.with(|state| *state.borrow_mut() = Some(host));
    let tick = RcBlock::new(|_: NonNull<NSTimer>| {
        HOST.with(|state| {
            if let Some(host) = state.borrow_mut().as_mut() {
                host.tick();
            }
        })
    });
    let timer =
        unsafe { NSTimer::scheduledTimerWithTimeInterval_repeats_block(0.016, true, &tick) };
    unsafe {
        NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes);
    }
    let events = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        let event = unsafe { event.as_ref() };
        if let Some(window) = event.window(mtm) {
            if let Some(role) = role(window.windowNumber()) {
                match event.r#type() {
                    NSEventType::RightMouseUp => {
                        let p = event.locationInWindow();
                        app_state::event(
                            &role,
                            json!({"kind":"context","x":p.x,"y":window.frame().size.height-p.y}),
                        );
                        return std::ptr::null_mut();
                    }
                    NSEventType::RightMouseDown => return std::ptr::null_mut(),
                    NSEventType::ScrollWheel if app_state::preview_open(&role) => {
                        app_state::event(
                            &role,
                            json!({"kind":"wheel","delta":-event.scrollingDeltaY()}),
                        );
                        return std::ptr::null_mut();
                    }
                    _ => {}
                }
            }
        }
        event as *const NSEvent as *mut NSEvent
    });
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            NSEventMask::RightMouseDown | NSEventMask::RightMouseUp | NSEventMask::ScrollWheel,
            &events,
        )
    };
    app_state::start_update_schedule();
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    app.run();
    timer.invalidate();
    if let Some(monitor) = monitor {
        unsafe {
            NSEvent::removeMonitor(&monitor);
        }
    }
    HOST.with(|state| {
        if let Some(host) = state.borrow_mut().take() {
            for window in host.windows.values() {
                window.view.stop();
            }
            drop(host.tray);
        }
    });
    Ok(())
}

impl Host {
    fn open(&mut self, role: &str, frame: Rect) -> anyhow::Result<()> {
        if let Some(window) = self.windows.get(role) {
            window
                .native
                .setTitle(&NSString::from_str(&host::title(role)));
            window.native.makeKeyAndOrderFront(None);
            return Ok(());
        }
        let style = if role == "dock" {
            NSWindowStyleMask::Borderless
        } else {
            NSWindowStyleMask::Borderless
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Miniaturizable
        };
        let native: Retained<QNoteWindow> = unsafe {
            msg_send![QNoteWindow::alloc(self.mtm),initWithContentRect:nsrect(self.mtm,frame),styleMask:style,backing:NSBackingStoreType::Buffered,defer:false]
        };
        unsafe {
            native.setReleasedWhenClosed(false);
        }
        native.setOpaque(false);
        native.setBackgroundColor(Some(&NSColor::clearColor()));
        native.setHasShadow(role != "dock");
        native.setTitle(&NSString::from_str(&host::title(role)));
        native.setDelegate(Some(ProtocolObject::from_ref(&*self.delegate)));
        if role != "dock" {
            native.setContentMinSize(NSSize::new(
                if role == "editor" { 420.0 } else { 300.0 },
                if role == "editor" { 520.0 } else { 450.0 },
            ));
            native.setContentMaxSize(NSSize::new(1000.0, 800.0));
        } else {
            native.setContentMinSize(NSSize::new(30.0, 30.0));
            native.setContentMaxSize(NSSize::new(30.0, 30.0));
        }
        if role == "editor" {
            native.registerForDraggedTypes(&NSArray::from_slice(&[objc2_foundation::ns_string!(
                "NSFilenamesPboardType"
            )]));
        }
        let view = AppView::new(
            self.mtm,
            AppOptions {
                title: host::title(role),
                size: (frame.width as i32, frame.height as i32),
                appearance: Appearance::Light,
                limits: Limits::standalone_desktop(),
                ..Default::default()
            },
        );
        native.setContentView(Some(&view.view()));
        view.load(host::bundle(role)?.into());
        native.makeKeyAndOrderFront(None);
        self.windows.insert(role.into(), Window { native, view });
        if role == "dock" {
            self.clip_dock();
        }
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
        from_ns(self.mtm, window.native.frame())
    }
    fn set_frame(&self, role: &str, frame: Rect) {
        if let Some(window) = self.windows.get(role) {
            window
                .native
                .setFrame_display(nsrect(self.mtm, frame), true);
        }
        if role == "dock" {
            self.clip_dock();
        }
    }
    fn clip_dock(&self) {
        if let Some(window) = self.windows.get("dock") {
            let frame = self.dock.frame;
            let work = area(self.mtm, Some(self.dock.full));
            let left = (work.x - frame.x).clamp(0.0, 30.0);
            let top = (work.y - frame.y).clamp(0.0, 30.0);
            let right = (work.x + work.width - frame.x).clamp(0.0, 30.0);
            let bottom = (work.y + work.height - frame.y).clamp(0.0, 30.0);
            let view = window.view.view();
            view.setWantsLayer(true);
            if let Some(layer) = view.layer() {
                let mask = CALayer::new();
                mask.setFrame(NSRect::new(
                    NSPoint::new(left, top),
                    NSSize::new((right - left).max(0.0), (bottom - top).max(0.0)),
                ));
                mask.setBackgroundColor(Some(&NSColor::whiteColor().CGColor()));
                unsafe {
                    layer.setMask(Some(&mask));
                }
            }
        }
    }
    fn topmost(&self, role: &str) {
        if let Some(window) = self.windows.get(role) {
            window
                .native
                .setLevel(if app_state::settings().always_on_top {
                    3
                } else {
                    0
                });
            window
                .native
                .setTitle(&NSString::from_str(&host::title(role)));
        }
    }
    fn show_main(&mut self) {
        self.dock.restore();
        if let Some(window) = self.windows.get("dock") {
            window.native.orderOut(None);
        }
        if let Some(window) = self.windows.get("main") {
            window.native.deminiaturize(None);
            window.native.makeKeyAndOrderFront(None);
        }
        #[allow(deprecated)]
        self.app.activateIgnoringOtherApps(true);
        self.update_tray();
    }
    fn edit(&mut self) -> anyhow::Result<()> {
        self.show_main();
        let main = self.frame("main");
        self.open(
            "editor",
            geometry::editor_frame(main, area(self.mtm, Some(main))),
        )
    }
    fn toggle_dock(&mut self) -> anyhow::Result<()> {
        if self.dock.active {
            self.show_main();
            return Ok(());
        }
        self.save_window()?;
        let main = self.frame("main");
        self.dock
            .open(main, area(self.mtm, self.dock.anchor().or(Some(main))));
        self.open("dock", self.dock.frame)?;
        self.set_frame("dock", self.dock.frame);
        if let Some(window) = self.windows.get("main") {
            window.native.orderOut(None);
        }
        app_state::event("dock", json!({"kind":"showDock"}));
        self.update_tray();
        Ok(())
    }
    fn save_window(&self) -> anyhow::Result<()> {
        app_state::save_window(self.frame("main").state())
    }
    fn update_tray(&self) {
        for (item, label) in self.items.iter().zip(host::tray_labels(self.dock.active)) {
            item.set_text(label);
        }
    }
    fn tick(&mut self) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(index) = self.items.iter().position(|item| item.id() == &event.id) {
                host::tray_action(index);
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                self.show_main();
            }
        }
        for command in app_state::take_commands() {
            match command.action.as_str() {
                "show" => self.show_main(),
                "edit" => {
                    if let Err(error) = self.edit() {
                        eprintln!("{error}");
                    }
                }
                "dock" => {
                    if let Err(error) = self.toggle_dock() {
                        eprintln!("{error}");
                    }
                }
                "close" => {
                    if command.window == "main" {
                        app_state::report_save_error(self.save_window());
                    }
                    if let Some(window) = self.windows.get(&command.window) {
                        window.native.orderOut(None);
                    }
                }
                "minimize" => {
                    if let Some(window) = self.windows.get(&command.window) {
                        window.native.miniaturize(None);
                    }
                }
                "settings" => {
                    for role in self.windows.keys() {
                        self.topmost(role);
                    }
                    self.update_tray();
                }
                "saveWindow" => app_state::window_saved(self.save_window()),
                "quit" => {
                    let saved = self.save_window().and_then(|_| app_state::flush_for_exit());
                    if saved.is_ok() {
                        QUITTING.set(true);
                        self.app.terminate(None);
                    } else {
                        app_state::report_save_error(saved);
                    }
                }
                "hoverDock" => self
                    .dock
                    .hover(command.payload["inside"].as_bool().unwrap_or(false)),
                "beginDockDrag" => self.cursor = Some(NSEvent::mouseLocation()),
                "dragDock" => {
                    let point = NSEvent::mouseLocation();
                    if let Some(previous) = self.cursor.replace(point) {
                        let work = area(
                            self.mtm,
                            Some(Rect {
                                x: point.x,
                                y: desktop_top(self.mtm) - point.y,
                                width: 0.0,
                                height: 0.0,
                            }),
                        );
                        self.dock
                            .drag(point.x - previous.x, previous.y - point.y, work);
                        self.set_frame("dock", self.dock.frame);
                    } else {
                        self.dock.drag(
                            command.payload["dx"].as_f64().unwrap_or(0.0),
                            command.payload["dy"].as_f64().unwrap_or(0.0),
                            area(self.mtm, Some(self.dock.full)),
                        );
                    }
                }
                "endDockDrag" => {
                    self.cursor = None;
                    if command.payload["dragged"].as_bool().unwrap_or(false) {
                        self.dock.release(area(self.mtm, Some(self.dock.full)));
                        self.set_frame("dock", self.dock.frame);
                    }
                }
                _ => {}
            }
        }
        if self.dock.tick() {
            self.set_frame("dock", self.dock.frame);
        }
        if self
            .windows
            .get("main")
            .is_some_and(|window| window.native.isVisible() && !window.native.isMiniaturized())
        {
            let frame = self.frame("main");
            if self.last_frame != Some(frame) {
                self.last_frame = Some(frame);
                self.changed = Some(Instant::now());
            }
            if self
                .changed
                .is_some_and(|time| time.elapsed() >= Duration::from_millis(300))
            {
                app_state::report_save_error(self.save_window());
                self.changed = None;
            }
        }
    }
}

fn role(number: isize) -> Option<String> {
    HOST.with(|state| {
        state.borrow().as_ref().and_then(|host| {
            host.windows
                .iter()
                .find(|(_, window)| window.native.windowNumber() == number)
                .map(|(role, _)| role.clone())
        })
    })
}
fn desktop_top(mtm: MainThreadMarker) -> f64 {
    NSScreen::screens(mtm)
        .firstObject()
        .map(|screen| screen.frame().origin.y + screen.frame().size.height)
        .unwrap_or(1080.0)
}
fn from_ns(mtm: MainThreadMarker, rect: NSRect) -> Rect {
    Rect {
        x: rect.origin.x,
        y: desktop_top(mtm) - rect.origin.y - rect.size.height,
        width: rect.size.width,
        height: rect.size.height,
    }
}
fn nsrect(mtm: MainThreadMarker, rect: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.x, desktop_top(mtm) - rect.y - rect.height),
        NSSize::new(rect.width, rect.height),
    )
}
fn area(mtm: MainThreadMarker, near: Option<Rect>) -> Rect {
    let screens = NSScreen::screens(mtm);
    let selected = near
        .and_then(|rect| {
            screens.iter().find(|screen| {
                let r = from_ns(mtm, screen.frame());
                rect.x + rect.width / 2.0 >= r.x
                    && rect.x + rect.width / 2.0 < r.x + r.width
                    && rect.y + rect.height / 2.0 >= r.y
                    && rect.y + rect.height / 2.0 < r.y + r.height
            })
        })
        .or_else(|| screens.firstObject());
    selected
        .map(|screen| from_ns(mtm, screen.visibleFrame()))
        .unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        })
}
