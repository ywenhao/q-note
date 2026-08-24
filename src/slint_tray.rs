use std::sync::mpsc;

use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

const MENU_TOPMOST_ID: &str = "q-note-toggle-topmost";
const MENU_LANGUAGE_ID: &str = "q-note-toggle-language";
const MENU_DOCK_ID: &str = "q-note-toggle-dock";
const MENU_QUIT_ID: &str = "q-note-quit";

#[derive(Clone, PartialEq)]
pub struct TrayLabels {
    pub topmost: String,
    pub language: String,
    pub dock: String,
    pub quit: String,
}

#[derive(Debug, Clone, Copy)]
pub enum TrayCommand {
    ShowMain,
    ToggleTopmost,
    ToggleLanguage,
    ToggleDock,
    Quit,
}

struct NativeMenu {
    menu: Menu,
    topmost: MenuItem,
    language: MenuItem,
    dock: MenuItem,
    _separator: PredefinedMenuItem,
    quit: MenuItem,
}

impl NativeMenu {
    fn new(labels: &TrayLabels) -> Self {
        let menu = Menu::new();
        let topmost = MenuItem::with_id(MENU_TOPMOST_ID, &labels.topmost, true, None);
        let language = MenuItem::with_id(MENU_LANGUAGE_ID, &labels.language, true, None);
        let dock = MenuItem::with_id(MENU_DOCK_ID, &labels.dock, true, None);
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::with_id(MENU_QUIT_ID, &labels.quit, true, None);
        let _ = menu.append(&topmost);
        let _ = menu.append(&language);
        let _ = menu.append(&dock);
        let _ = menu.append(&separator);
        let _ = menu.append(&quit);
        Self {
            menu,
            topmost,
            language,
            dock,
            _separator: separator,
            quit,
        }
    }

    fn update(&self, labels: &TrayLabels) {
        self.topmost.set_text(&labels.topmost);
        self.language.set_text(&labels.language);
        self.dock.set_text(&labels.dock);
        self.quit.set_text(&labels.quit);
    }
}

#[cfg(target_os = "linux")]
enum TrayControl {
    Update(TrayLabels),
    Hide,
}

pub struct SlintTray {
    command_rx: mpsc::Receiver<TrayCommand>,
    #[cfg(not(target_os = "linux"))]
    _tray: tray_icon::TrayIcon,
    #[cfg(not(target_os = "linux"))]
    menu: NativeMenu,
    #[cfg(target_os = "linux")]
    control_tx: mpsc::Sender<TrayControl>,
}

impl SlintTray {
    pub fn new(labels: TrayLabels) -> anyhow::Result<Self> {
        let (command_tx, command_rx) = mpsc::channel();

        #[cfg(not(target_os = "linux"))]
        {
            let menu = NativeMenu::new(&labels);
            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(menu.menu.clone()))
                .with_menu_on_left_click(false)
                .with_tooltip("Q Note")
                .with_icon(load_tray_icon())
                .build()?;
            let _ = command_tx;
            Ok(Self {
                command_rx,
                _tray: tray,
                menu,
            })
        }

        #[cfg(target_os = "linux")]
        {
            let (control_tx, control_rx) = mpsc::channel();
            std::thread::Builder::new()
                .name("q-note-tray".into())
                .spawn(move || {
                    let _ = gtk::init();
                    let menu = NativeMenu::new(&labels);
                    let tray = TrayIconBuilder::new()
                        .with_menu(Box::new(menu.menu.clone()))
                        .with_menu_on_left_click(false)
                        .with_tooltip("Q Note")
                        .with_icon(load_tray_icon())
                        .build()
                        .ok();
                    let menu_events = MenuEvent::receiver();
                    let tray_events = TrayIconEvent::receiver();
                    loop {
                        while let Ok(control) = control_rx.try_recv() {
                            match control {
                                TrayControl::Update(labels) => menu.update(&labels),
                                TrayControl::Hide => {
                                    if let Some(tray) = tray.as_ref() {
                                        let _ = tray.set_visible(false);
                                    }
                                    return;
                                }
                            }
                        }
                        while let Ok(event) = menu_events.try_recv() {
                            if let Some(command) = command_for_id(event.id.as_ref()) {
                                let _ = command_tx.send(command);
                            }
                        }
                        while let Ok(event) = tray_events.try_recv() {
                            if is_left_release(&event) {
                                let _ = command_tx.send(TrayCommand::ShowMain);
                            }
                        }
                        while gtk::events_pending() {
                            gtk::main_iteration_do(false);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                })?;
            Ok(Self {
                command_rx,
                control_tx,
            })
        }
    }

    pub fn poll(&self) -> Vec<TrayCommand> {
        let mut commands = self.command_rx.try_iter().collect::<Vec<_>>();
        #[cfg(not(target_os = "linux"))]
        {
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if let Some(command) = command_for_id(event.id.as_ref()) {
                    commands.push(command);
                }
            }
            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if is_left_release(&event) {
                    commands.push(TrayCommand::ShowMain);
                }
            }
        }
        commands
    }

    pub fn update_labels(&self, labels: TrayLabels) {
        #[cfg(not(target_os = "linux"))]
        self.menu.update(&labels);

        #[cfg(target_os = "linux")]
        let _ = self.control_tx.send(TrayControl::Update(labels));
    }

    pub fn hide(&self) {
        #[cfg(not(target_os = "linux"))]
        let _ = self._tray.set_visible(false);

        #[cfg(target_os = "linux")]
        let _ = self.control_tx.send(TrayControl::Hide);
    }
}

fn command_for_id(id: &str) -> Option<TrayCommand> {
    match id {
        MENU_TOPMOST_ID => Some(TrayCommand::ToggleTopmost),
        MENU_LANGUAGE_ID => Some(TrayCommand::ToggleLanguage),
        MENU_DOCK_ID => Some(TrayCommand::ToggleDock),
        MENU_QUIT_ID => Some(TrayCommand::Quit),
        _ => None,
    }
}

fn is_left_release(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        }
    )
}

fn load_tray_icon() -> tray_icon::Icon {
    let image = image::load_from_memory(include_bytes!("../assets/tray-icon.png"))
        .expect("tray icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    tray_icon::Icon::from_rgba(image.into_raw(), width, height).expect("tray RGBA")
}
