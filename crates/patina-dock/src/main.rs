//! Patina dock: the floating vertical dock on the left of every output.
//!
//! As the design system's `Dock`: app icons (`--dock-icon`, 48 px) in a column with 10 px
//! gaps, 10 px padding, `--radius-xl`, `--surface-panel` with the panel blur (drawn by the
//! compositor) and a 1 px `--border-subtle`, 14 px from the left edge and centred in the
//! area below the panel. A 4 px dot left of an icon marks a running app (`--tide-300` when it
//! has focus, `--ink-500` otherwise); hovering scales an icon to 1.08 over `--dur` with
//! `--ease`. As in the prototype the dock sits below windows.
//!
//! Clicking an app that has focus minimises it; clicking one that is running brings it back;
//! clicking one that is not running starts it. The right-click menu has Open, Keep in dock and
//! Close all windows, as in the prototype.

mod pins;

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use iced::{
    futures::{SinkExt, Stream},
    widget::{button, column, container, mouse_area, Space},
    window::Id,
    Alignment, Background, Border, Color, Element, Length, Point, Subscription, Task,
};
use iced_layershell::{
    actions::IcedNewPopupSettings,
    build_pattern::daemon,
    reexport::{
        Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption, PopupAnchor,
        PopupGravity,
    },
    settings::{LayerShellSettings, Settings, StartMode},
    to_layer_message,
};
use patina_apps::App;
use patina_link::{Command, Update, WindowInfo};
use patina_tokens as tok;
use patina_ui::{self as ui, c, motion::CubicBezier, Icon};

/// Logical sizes from the design system's Dock.
const ICON: f32 = tok::DOCK_ICON;
const GAP: f32 = 10.0;
const PAD: f32 = 10.0;
const BORDER: f32 = 1.0;
/// 14 px from the screen edge (`dockLeft: 14px`).
const EDGE: i32 = 14;
const HOVER_SCALE: f32 = 1.08;
/// Icons are loaded at twice their size, so they stay sharp up to 200%.
const ICON_PX: u32 = (ICON as u32) * 2;
/// Menu: 200 px wide; rows 28 px with 2 px gaps inside 4 px padding (prototype `menuItems`).
const MENU_W: u32 = 200;

/// One icon in the dock.
#[derive(Debug, Clone)]
struct Item {
    /// The desktop file id, or the app id of a window no installed app claims.
    key: String,
    app: Option<App>,
    windows: Vec<WindowInfo>,
}

impl Item {
    fn focused(&self) -> Option<&WindowInfo> {
        self.windows.iter().find(|w| w.activated && !w.minimized)
    }
}

/// An icon's hover animation: scale moving from `from` to `to` since `start`.
#[derive(Debug, Clone, Copy)]
struct Hover {
    from: f32,
    to: f32,
    start: Instant,
}

impl Hover {
    fn scale(&self, now: Instant) -> f32 {
        let t = now.saturating_duration_since(self.start).as_secs_f32() / tok::DUR.as_secs_f32();
        let p = CubicBezier::PATINA.ease(t.min(1.0));
        self.from + (self.to - self.from) * p
    }
    fn running(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start) < tok::DUR
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Surface {
    Dock,
    Menu,
}

struct Dock {
    apps: Vec<App>,
    icons: HashMap<String, Option<iced::widget::image::Handle>>,
    finder: patina_apps::icons::IconFinder,
    pins: Vec<String>,
    windows: Vec<WindowInfo>,
    /// Dock surfaces by output name, and what each window id is.
    docks: HashMap<String, Id>,
    surfaces: HashMap<Id, Surface>,
    link: Option<std::sync::mpsc::Sender<Command>>,
    hovers: HashMap<String, Hover>,
    /// Pointer position over each item, relative to the item (for placing the menu).
    pointer: Point,
    /// The open menu: its popup id and the item it is for.
    menu: Option<(Id, String)>,
    now: Instant,
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    Link(std::sync::mpsc::Sender<Command>),
    Compositor(Update),
    Click(String),
    Hover(String, bool),
    Pointer(Point),
    OpenMenu(Id, usize),
    MenuOpen(String),
    MenuPin(String),
    MenuCloseAll(String),
    Frame(Instant),
    Closed(Id),
}

impl Dock {
    fn new() -> (Self, Task<Message>) {
        let apps = patina_apps::all_apps();
        let pins = pins::load(&apps);
        (
            Self {
                apps,
                icons: HashMap::new(),
                finder: patina_apps::icons::IconFinder::new(),
                pins,
                windows: Vec::new(),
                docks: HashMap::new(),
                surfaces: HashMap::new(),
                link: None,
                hovers: HashMap::new(),
                pointer: Point::ORIGIN,
                menu: None,
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    /// Pinned apps in order, then running apps that are not pinned, in window order.
    fn items(&self) -> Vec<Item> {
        let mut items: Vec<Item> = self
            .pins
            .iter()
            .filter_map(|id| self.apps.iter().find(|a| &a.id == id))
            .map(|a| Item {
                key: a.id.clone(),
                app: Some(a.clone()),
                windows: Vec::new(),
            })
            .collect();
        for w in &self.windows {
            let app = patina_apps::app_for_window(&self.apps, &w.app_id);
            let key = app
                .map(|a| a.id.clone())
                .unwrap_or_else(|| w.app_id.clone());
            if key.is_empty() {
                continue;
            }
            match items.iter_mut().find(|i| i.key == key) {
                Some(i) => i.windows.push(w.clone()),
                None => items.push(Item {
                    key,
                    app: app.cloned(),
                    windows: vec![w.clone()],
                }),
            }
        }
        items
    }

    fn size(&self) -> (u32, u32) {
        let n = self.items().len().max(1) as f32;
        let w = ICON + 2.0 * (PAD + BORDER);
        let h = n * ICON + (n - 1.0) * GAP + 2.0 * (PAD + BORDER);
        (w as u32, h as u32)
    }

    fn icon(&mut self, item: &Item) -> Option<iced::widget::image::Handle> {
        if let Some(h) = self.icons.get(&item.key) {
            return h.clone();
        }
        let names: Vec<String> = item
            .app
            .as_ref()
            .and_then(|a| a.icon.clone())
            .into_iter()
            .chain([item.key.clone(), "application-x-executable".into()])
            .collect();
        let handle = names
            .iter()
            .find_map(|n| self.finder.load_rgba(n, ICON_PX, 1))
            .map(|rgba| ui::rgba_handle(&rgba));
        self.icons.insert(item.key.clone(), handle.clone());
        handle
    }

    fn send(&self, command: Command) {
        if let Some(link) = &self.link {
            let _ = link.send(command);
        }
    }

    /// Makes every output have one dock, and every dock the right size.
    fn sync_surfaces(&mut self, outputs: Option<Vec<String>>) -> Task<Message> {
        let mut tasks = Vec::new();
        if let Some(outputs) = outputs {
            // Docks of outputs that went away are closed by the compositor; forget them.
            self.docks.retain(|name, _| outputs.contains(name));
            for name in outputs {
                if self.docks.contains_key(&name) {
                    continue;
                }
                let id = Id::unique();
                self.docks.insert(name.clone(), id);
                self.surfaces.insert(id, Surface::Dock);
                tasks.push(Task::done(Message::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some(self.size()),
                        layer: Layer::Bottom,
                        anchor: Anchor::Left,
                        exclusive_zone: Some(0),
                        margin: Some((0, 0, 0, EDGE)),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        output_option: OutputOption::OutputName(name),
                        events_transparent: false,
                        namespace: Some("patina-dock".into()),
                    },
                    id,
                }));
            }
        }
        let size = self.size();
        for id in self.docks.values() {
            tasks.push(Task::done(Message::SizeChange { id: *id, size }));
        }
        Task::batch(tasks)
    }

    fn hover(&mut self, key: &str, on: bool) {
        let now = Instant::now();
        let current = self.hovers.get(key).map(|h| h.scale(now)).unwrap_or(1.0);
        self.now = now;
        self.hovers.insert(
            key.to_string(),
            Hover {
                from: current,
                to: if on { HOVER_SCALE } else { 1.0 },
                start: now,
            },
        );
    }

    fn animating(&self) -> bool {
        let now = Instant::now();
        self.hovers.values().any(|h| h.running(now))
    }
}

fn namespace() -> String {
    String::from("patina-dock")
}

fn close_window(id: Id) -> Task<Message> {
    iced_runtime::task::effect(iced_runtime::Action::Window(
        iced_runtime::window::Action::Close(id),
    ))
}

fn update(dock: &mut Dock, message: Message) -> Task<Message> {
    match message {
        Message::Link(link) => dock.link = Some(link),
        Message::Compositor(Update::Outputs(outputs)) => return dock.sync_surfaces(Some(outputs)),
        Message::Compositor(Update::Windows(windows)) => {
            // A window of an app we do not know: maybe it was just installed.
            if windows.iter().any(|w| {
                !w.app_id.is_empty() && patina_apps::app_for_window(&dock.apps, &w.app_id).is_none()
            }) {
                dock.apps = patina_apps::all_apps();
            }
            let before = dock.items().len();
            dock.windows = windows;
            if dock.items().len() != before {
                return dock.sync_surfaces(None);
            }
        }
        Message::Compositor(_) => {}
        Message::Click(key) => {
            let Some(item) = dock.items().into_iter().find(|i| i.key == key) else {
                return Task::none();
            };
            if let Some(w) = item.focused() {
                dock.send(Command::Minimize(w.id));
            } else if let Some(w) = item
                .windows
                .iter()
                .rev()
                .find(|w| !w.minimized)
                .or(item.windows.last())
            {
                dock.send(Command::Activate(w.id));
            } else if let Some(app) = &item.app {
                if let Err(e) = patina_apps::launch(app) {
                    eprintln!("patina-dock: cannot start {}: {e}", app.id);
                }
            }
        }
        Message::Hover(key, on) => dock.hover(&key, on),
        Message::Pointer(p) => dock.pointer = p,
        Message::OpenMenu(parent, index) => {
            let Some(item) = dock.items().into_iter().nth(index) else {
                return Task::none();
            };
            let mut tasks = Vec::new();
            if let Some((old, _)) = dock.menu.take() {
                tasks.push(close_window(old));
            }
            let id = Id::unique();
            dock.surfaces.insert(id, Surface::Menu);
            dock.menu = Some((id, item.key.clone()));
            // The item's corner in the dock, plus where the pointer is on it.
            let y0 = BORDER + PAD + index as f32 * (ICON + GAP);
            let (x, y) = (
                (BORDER + PAD + dock.pointer.x) as i32,
                (y0 + dock.pointer.y) as i32,
            );
            tasks.push(Task::done(Message::NewPopUp {
                // Down and right from the pointer, as the prototype places menus.
                settings: IcedNewPopupSettings::new(parent, (MENU_W, menu_height()), (x, y, 1, 1))
                    .anchor(PopupAnchor::TopLeft)
                    .gravity(PopupGravity::BottomRight),
                id,
            }));
            return Task::batch(tasks);
        }
        Message::MenuOpen(key) => {
            if let Some(app) = dock.apps.iter().find(|a| a.id == key) {
                if let Err(e) = patina_apps::launch(app) {
                    eprintln!("patina-dock: cannot start {}: {e}", app.id);
                }
            }
            return close_menu(dock);
        }
        Message::MenuPin(key) => {
            if dock.pins.contains(&key) {
                dock.pins.retain(|p| p != &key);
            } else {
                dock.pins.push(key);
            }
            pins::save(&dock.pins);
            let t = dock.sync_surfaces(None);
            return Task::batch([close_menu(dock), t]);
        }
        Message::MenuCloseAll(key) => {
            if let Some(item) = dock.items().into_iter().find(|i| i.key == key) {
                for w in &item.windows {
                    dock.send(Command::Close(w.id));
                }
            }
            return close_menu(dock);
        }
        Message::Frame(now) => dock.now = now,
        Message::Closed(id) => {
            dock.surfaces.remove(&id);
            dock.docks.retain(|_, d| *d != id);
            if dock.menu.as_ref().is_some_and(|(m, _)| *m == id) {
                dock.menu = None;
            }
        }
        _ => {}
    }
    Task::none()
}

fn close_menu(dock: &mut Dock) -> Task<Message> {
    match dock.menu.take() {
        Some((id, _)) => close_window(id),
        None => Task::none(),
    }
}

fn menu_height() -> u32 {
    // 4 padding, three 28 px rows with 2 px gaps, and the separator before the last row
    // (1 px plus 4 px above and below, and its 2 px gap), 4 padding.
    (4 + 28 + 2 + 28 + 2 + (4 + 1 + 4 + 2) + 28 + 4) as u32
}

fn compositor_updates() -> impl Stream<Item = Message> {
    iced::stream::channel(8, async |mut output| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let commands = patina_link::spawn(tx, "patina-dock");
        let _ = output.send(Message::Link(commands)).await;
        while let Some(update) = rx.recv().await {
            let _ = output.send(Message::Compositor(update)).await;
        }
        std::future::pending::<()>().await;
    })
}

fn subscription(dock: &Dock) -> Subscription<Message> {
    let frames = if dock.animating() {
        iced::time::every(Duration::from_millis(8)).map(Message::Frame)
    } else {
        Subscription::none()
    };
    Subscription::batch([
        frames,
        Subscription::run(compositor_updates),
        iced::window::close_events().map(Message::Closed),
    ])
}

fn view(dock: &Dock, id: Id) -> Element<'_, Message> {
    match dock.surfaces.get(&id) {
        Some(Surface::Menu) => menu_view(dock),
        _ => dock_view(dock, id),
    }
}

fn dock_view(dock: &Dock, id: Id) -> Element<'_, Message> {
    let items = dock.items();
    // Icons are loaded lazily; `view` cannot mutate, so missing ones are drawn without one
    // and filled in by `ensure_icons` before the next frame.
    let mut col = column![].spacing(GAP).align_x(Alignment::Center);
    for (i, item) in items.iter().enumerate() {
        let scale = dock
            .hovers
            .get(&item.key)
            .map(|h| h.scale(dock.now))
            .unwrap_or(1.0);
        let image: Element<'_, Message> = match dock.icons.get(&item.key).cloned().flatten() {
            Some(h) => ui::app_icon(&h, ICON, scale),
            None => Space::new().width(ICON).height(ICON).into(),
        };
        let focused = item.focused().is_some();
        let dot: Element<'_, Message> = if item.windows.is_empty() {
            Space::new().width(4).height(4).into()
        } else {
            let color = if focused {
                c(tok::TIDE_300)
            } else {
                c(tok::INK_500)
            };
            container(Space::new().width(4).height(4))
                .style(move |_| container::Style {
                    background: Some(Background::Color(color)),
                    border: Border {
                        radius: 2.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into()
        };
        let key = item.key.clone();
        let b = button(image)
            .padding(0)
            .width(ICON)
            .height(ICON)
            .on_press(Message::Click(key.clone()))
            .style(|_, _| button::Style::default());
        let area = mouse_area(b)
            .on_enter(Message::Hover(key.clone(), true))
            .on_exit(Message::Hover(key.clone(), false))
            .on_move(Message::Pointer)
            .on_right_press(Message::OpenMenu(id, i));
        // Row: dot, 4 px gap, icon (the dot's 4 px + 4 px gap = the design's 8 px offset).
        col = col.push(
            iced::widget::row![container(dot).center_y(ICON), area]
                .spacing(4)
                .align_y(Alignment::Center),
        );
    }
    // The dot column eats 8 px of the left padding (10 - 8 = 2 px from the border).
    container(col)
        .padding(iced::Padding {
            top: PAD,
            bottom: PAD,
            left: PAD - 8.0,
            right: PAD,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ui::panel_glass(tok::RADIUS_XL))
        .into()
}

fn menu_view(dock: &Dock) -> Element<'_, Message> {
    let Some((_, key)) = &dock.menu else {
        return Space::new().into();
    };
    let item = dock.items().into_iter().find(|i| &i.key == key);
    let launchable = item.as_ref().is_some_and(|i| i.app.is_some());
    let pinned = dock.pins.contains(key);
    let has_windows = item.as_ref().is_some_and(|i| !i.windows.is_empty());
    let k = key.clone();
    let rows = column![
        ui::menu_row(
            Icon::ExternalLink,
            "Open".into(),
            "",
            false,
            launchable.then(|| Message::MenuOpen(k.clone())),
        ),
        ui::menu_row(
            Icon::Pin,
            if pinned {
                "Remove from dock"
            } else {
                "Keep in dock"
            }
            .into(),
            "",
            false,
            launchable.then(|| Message::MenuPin(k.clone())),
        ),
        column![
            ui::menu_separator(),
            ui::menu_row(
                Icon::X,
                "Close all windows".into(),
                "",
                true,
                has_windows.then(|| Message::MenuCloseAll(k.clone())),
            ),
        ]
        .spacing(2),
    ]
    .spacing(2);
    container(rows)
        .padding(4)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(c(tok::SURFACE_WINDOW))),
            border: Border {
                radius: tok::RADIUS_LG.into(),
                width: 1.0,
                color: c(tok::BORDER_SUBTLE),
            },
            ..container::Style::default()
        })
        .into()
}

/// Loads the icons of the current items (outside `view`, which cannot change the dock).
fn ensure_icons(dock: &mut Dock) {
    for item in dock.items() {
        if !dock.icons.contains_key(&item.key) {
            dock.icon(&item);
        }
    }
}

fn main() -> Result<(), iced_layershell::Error> {
    daemon(
        Dock::new,
        namespace,
        |dock: &mut Dock, message| {
            let task = update(dock, message);
            ensure_icons(dock);
            task
        },
        view,
    )
    .style(|_, _| iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: c(tok::TEXT_PRIMARY),
    })
    .subscription(subscription)
    .settings(Settings {
        fonts: vec![ui::SANS_BYTES.into(), ui::MONO_BYTES.into()],
        default_font: ui::sans(iced::font::Weight::Normal),
        default_text_size: tok::FS_BODY.into(),
        antialiasing: true,
        layer_settings: LayerShellSettings {
            // Surfaces are made per output once the outputs are known.
            start_mode: StartMode::Background,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}
