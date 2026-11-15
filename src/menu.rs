// Main menu, world selection, world creation, pause menu and settings.
use macroquad::prelude::*;

use crate::world::{self, Mode, Size, WorldInfo};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Main,
    Worlds,
    Create,
    Pause,
    Settings,
}

pub enum Action {
    None,
    Resume,
    LoadWorld(String),
    CreateWorld {
        seed: u32,
        size: Size,
        name: String,
        mode: Mode,
    },
    Recreate {
        path: String,
        seed: u32,
        size: Size,
        name: String,
        mode: Mode,
    },
    QuitToMenu,
    Quit,
}

const CFG_PATH: &str = "rustblox.cfg";
const SIZES: [Size; 3] = [Size::Small, Size::Normal, Size::Large];

fn green() -> Color {
    Color::new(0.55, 0.86, 0.42, 1.0)
}

fn text_color() -> Color {
    Color::new(0.93, 0.95, 0.97, 1.0)
}

fn muted() -> Color {
    Color::new(0.72, 0.77, 0.82, 0.85)
}

const LETTER_KEYS: [(KeyCode, char); 26] = [
    (KeyCode::A, 'a'),
    (KeyCode::B, 'b'),
    (KeyCode::C, 'c'),
    (KeyCode::D, 'd'),
    (KeyCode::E, 'e'),
    (KeyCode::F, 'f'),
    (KeyCode::G, 'g'),
    (KeyCode::H, 'h'),
    (KeyCode::I, 'i'),
    (KeyCode::J, 'j'),
    (KeyCode::K, 'k'),
    (KeyCode::L, 'l'),
    (KeyCode::M, 'm'),
    (KeyCode::N, 'n'),
    (KeyCode::O, 'o'),
    (KeyCode::P, 'p'),
    (KeyCode::Q, 'q'),
    (KeyCode::R, 'r'),
    (KeyCode::S, 's'),
    (KeyCode::T, 't'),
    (KeyCode::U, 'u'),
    (KeyCode::V, 'v'),
    (KeyCode::W, 'w'),
    (KeyCode::X, 'x'),
    (KeyCode::Y, 'y'),
    (KeyCode::Z, 'z'),
];

const DIGIT_KEYS: [(KeyCode, char); 20] = [
    (KeyCode::Key0, '0'),
    (KeyCode::Key1, '1'),
    (KeyCode::Key2, '2'),
    (KeyCode::Key3, '3'),
    (KeyCode::Key4, '4'),
    (KeyCode::Key5, '5'),
    (KeyCode::Key6, '6'),
    (KeyCode::Key7, '7'),
    (KeyCode::Key8, '8'),
    (KeyCode::Key9, '9'),
    (KeyCode::Kp0, '0'),
    (KeyCode::Kp1, '1'),
    (KeyCode::Kp2, '2'),
    (KeyCode::Kp3, '3'),
    (KeyCode::Kp4, '4'),
    (KeyCode::Kp5, '5'),
    (KeyCode::Kp6, '6'),
    (KeyCode::Kp7, '7'),
    (KeyCode::Kp8, '8'),
    (KeyCode::Kp9, '9'),
];

struct TextInput {
    text: String,
    max: usize,
    digits_only: bool,
}

impl TextInput {
    fn new(digits_only: bool, max: usize) -> Self {
        Self {
            text: String::new(),
            max,
            digits_only,
        }
    }

    fn set(&mut self, text: &str) {
        self.text = text.chars().take(self.max).collect();
    }

    fn value(&self) -> &str {
        &self.text
    }

    fn push(&mut self, c: char) {
        if self.text.chars().count() < self.max {
            self.text.push(c);
        }
    }

    /// Reads typing keys. Returns true when the text changed.
    fn edit(&mut self, active: bool) -> bool {
        if !active {
            return false;
        }
        let mut changed = false;
        if is_key_pressed(KeyCode::Backspace) && self.text.pop().is_some() {
            changed = true;
        }
        if !self.digits_only {
            for (code, ch) in LETTER_KEYS {
                if is_key_pressed(code) {
                    self.push(ch);
                    changed = true;
                }
            }
            if is_key_pressed(KeyCode::Space) {
                self.push(' ');
                changed = true;
            }
        }
        for (code, ch) in DIGIT_KEYS {
            if is_key_pressed(code) {
                self.push(ch);
                changed = true;
            }
        }
        changed
    }
}

pub struct Settings {
    pub sensitivity: f32,
    pub fov: f32,
    pub view_distance: f32,
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sensitivity: 0.0022,
            fov: 72.0,
            view_distance: 220.0,
            fullscreen: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let mut s = Self::default();
        if let Ok(txt) = std::fs::read_to_string(CFG_PATH) {
            for line in txt.lines() {
                let mut parts = line.splitn(2, '=');
                let key = parts.next().unwrap_or("").trim();
                let raw = parts.next().unwrap_or("").trim();
                let value = raw.parse::<f32>().unwrap_or(f32::NAN);
                if !value.is_finite() {
                    continue;
                }
                match key {
                    "sensitivity" => s.sensitivity = value.clamp(0.0005, 0.0060),
                    "fov" => s.fov = value.clamp(55.0, 100.0),
                    "view_distance" => s.view_distance = value.clamp(80.0, 320.0),
                    "fullscreen" => s.fullscreen = value > 0.5,
                    _ => {}
                }
            }
        }
        s
    }

    pub fn save(&self) {
        let txt = format!(
            "sensitivity={}\nfov={}\nview_distance={}\nfullscreen={}\n",
            self.sensitivity,
            self.fov,
            self.view_distance,
            if self.fullscreen { 1 } else { 0 }
        );
        let _ = std::fs::write(CFG_PATH, txt);
    }
}

struct Slider {
    label: &'static str,
    min: f32,
    max: f32,
    step: f32,
}

const SLIDERS: [Slider; 3] = [
    Slider {
        label: "Mouse sensitivity",
        min: 0.0008,
        max: 0.0055,
        step: 0.0002,
    },
    Slider {
        label: "Field of view",
        min: 55.0,
        max: 100.0,
        step: 2.0,
    },
    Slider {
        label: "View distance",
        min: 80.0,
        max: 320.0,
        step: 10.0,
    },
];

fn slider_value(s: &Settings, i: usize) -> f32 {
    match i {
        0 => s.sensitivity,
        1 => s.fov,
        _ => s.view_distance,
    }
}

fn set_slider_value(s: &mut Settings, i: usize, v: f32) {
    match i {
        0 => s.sensitivity = v,
        1 => s.fov = v,
        _ => s.view_distance = v,
    }
}

fn slider_text(i: usize, v: f32) -> String {
    match i {
        0 => format!("{:.1}", v * 1000.0),
        _ => format!("{:.0}", v),
    }
}

pub struct Menu {
    pub page: Page,
    sel: usize,
    time: f32,
    back: Page,
    pub worlds: Vec<WorldInfo>,
    scroll: usize,
    /// Row that Play / Delete / Recreate act on.
    focus: usize,
    /// Last clicked row, used to detect a double click.
    click_row: Option<usize>,
    click_at: f32,
    confirm_delete: Option<String>,
    name_input: TextInput,
    seed_input: TextInput,
    size_idx: usize,
    mode_idx: usize,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            page: Page::Main,
            sel: 0,
            time: 0.0,
            back: Page::Main,
            worlds: Vec::new(),
            scroll: 0,
            focus: 0,
            click_row: None,
            click_at: 0.0,
            confirm_delete: None,
            name_input: TextInput::new(false, 24),
            seed_input: TextInput::new(true, 10),
            size_idx: 1,
            mode_idx: 0,
        }
    }

    pub fn open(&mut self, page: Page) {
        match page {
            Page::Worlds => {
                self.worlds = world::list_worlds();
                self.scroll = 0;
            }
            Page::Create => {
                self.worlds = world::list_worlds();
                let n = self.worlds.len() + 1;
                self.name_input.set(&format!("World {}", n));
                self.seed_input.set("");
                self.size_idx = 1;
            }
            Page::Settings => {
                self.back = if self.page == Page::Settings {
                    self.back
                } else {
                    self.page
                };
            }
            _ => {}
        }
        self.page = page;
        self.sel = 0;
        self.focus = 0;
        self.confirm_delete = None;
    }

    pub fn open_settings(&mut self) {
        if self.page != Page::Settings {
            self.back = self.page;
        }
        self.page = Page::Settings;
        self.sel = 0;
    }

    pub fn refresh_worlds(&mut self) {
        self.worlds = world::list_worlds();
        if self.sel > self.worlds.len() {
            self.sel = 0;
        }
        if self.focus >= self.worlds.len() {
            self.focus = 0;
        }
        let max_scroll = self.worlds.len().saturating_sub(5);
        self.scroll = self.scroll.min(max_scroll);
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
    }

    pub fn frame(&mut self, s: &mut Settings, ignore_escape: bool) -> Action {
        let (sw, sh) = (screen_width(), screen_height());
        let mp: Vec2 = mouse_position().into();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let esc = !ignore_escape && is_key_pressed(KeyCode::Escape);

        let veil = match self.page {
            Page::Main => 0.45,
            Page::Pause => 0.60,
            _ => 0.80,
        };
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.02, 0.03, 0.05, veil));
        draw_rectangle(0.0, 0.0, sw, sh * 0.35, Color::new(0.0, 0.01, 0.03, 0.40));
        draw_rectangle(0.0, sh * 0.65, sw, sh * 0.35, Color::new(0.0, 0.01, 0.03, 0.45));

        match self.page {
            Page::Main => self.draw_main(mp, clicked),
            Page::Worlds => self.draw_worlds(mp, clicked, esc),
            Page::Create => self.draw_create(mp, clicked, esc),
            Page::Pause => self.draw_pause(mp, clicked, esc),
            Page::Settings => self.draw_settings(s, mp, clicked, esc),
        }
    }

    fn draw_main(&mut self, mp: Vec2, clicked: bool) -> Action {
        let (sw, sh) = (screen_width(), screen_height());
        let bob = (self.time * 1.6).sin() * 3.0;
        draw_title("RUSTBLOX", sw * 0.5, sh * 0.20 + bob, 96.0);
        draw_centered(
            "voxel sandbox in Rust",
            sw * 0.5,
            sh * 0.20 + bob + 36.0,
            24.0,
            Color::new(0.86, 0.90, 0.94, 0.85),
        );

        let items = ["Play", "Settings", "Quit"];
        if let Some(i) = self.buttons(&items, sh * 0.42, mp, clicked, 56.0) {
            match i {
                0 => {
                    self.open(Page::Worlds);
                    self.sel = 0;
                    return Action::None;
                }
                1 => {
                    self.open_settings();
                    return Action::None;
                }
                _ => return Action::Quit,
            }
        }
        draw_footer("Arrows + Enter, or mouse", sw, sh);
        Action::None
    }

    fn draw_worlds(&mut self, mp: Vec2, clicked: bool, esc: bool) -> Action {
        let (sw, sh) = (screen_width(), screen_height());
        draw_title("SELECT WORLD", sw * 0.5, sh * 0.11, 56.0);

        let w = 760.0_f32.min(sw - 80.0);
        let x = (sw - w) * 0.5;
        let row_h = 64.0;
        let gap = 8.0;
        let visible = 5usize;
        let top = sh * 0.19;

        let rows = self.worlds.len();
        if rows == 0 && self.sel < 1 {
            self.sel = 0;
        }
        let total = rows + 6;
        if is_key_pressed(KeyCode::Down) {
            self.sel = (self.sel + 1) % total;
        }
        if is_key_pressed(KeyCode::Up) {
            self.sel = (self.sel + total - 1) % total;
        }
        if is_key_pressed(KeyCode::Right) && self.sel >= rows {
            self.sel = rows + (self.sel - rows + 1) % 6;
        }
        if is_key_pressed(KeyCode::Left) && self.sel >= rows {
            self.sel = rows + (self.sel - rows + 5) % 6;
        }

        let max_scroll = rows.saturating_sub(visible);
        let (_, wheel) = mouse_wheel();
        if wheel > 0.0 {
            self.scroll = self.scroll.saturating_sub(1);
        } else if wheel < 0.0 {
            self.scroll = (self.scroll + 1).min(max_scroll);
        }
        if self.sel < rows {
            if self.sel < self.scroll {
                self.scroll = self.sel;
            }
            if self.sel >= self.scroll + visible {
                self.scroll = self.sel + 1 - visible;
            }
        }

        let mut action = Action::None;
        let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
        let delete_key = is_key_pressed(KeyCode::Delete);

        if rows == 0 {
            draw_centered(
                "No worlds yet. Create one!",
                sw * 0.5,
                top + 120.0,
                24.0,
                muted(),
            );
        }

        for i in 0..visible {
            let idx = self.scroll + i;
            if idx >= rows {
                break;
            }
            let info = self.worlds[idx].clone();
            let rect = Rect::new(x, top + i as f32 * (row_h + gap), w, row_h);
            let hovered = rect.contains(mp);
            if hovered {
                self.sel = idx;
            }
            let selected = self.sel == idx;
            if selected {
                self.focus = idx;
            }
            panel(rect, selected, hovered);

            // Little world swatch instead of a rendered thumbnail.
            let tint = match info.size {
                Size::Small => Color::new(0.30, 0.52, 0.26, 1.0),
                Size::Normal => Color::new(0.34, 0.46, 0.56, 1.0),
                Size::Large => Color::new(0.46, 0.40, 0.58, 1.0),
            };
            draw_rectangle(rect.x + 10.0, rect.y + 10.0, 44.0, 44.0, tint);
            draw_rectangle_lines(
                rect.x + 10.0,
                rect.y + 10.0,
                44.0,
                44.0,
                1.0,
                Color::new(0.0, 0.0, 0.0, 0.5),
            );

            draw_text(&info.name, rect.x + 66.0, rect.y + 27.0, 24.0, text_color());
            let meta = format!(
                "seed {}    {}    {}    {}",
                info.seed,
                info.mode.short(),
                info.size.short(),
                ago(info.modified)
            );
            draw_text(&meta, rect.x + 66.0, rect.y + 49.0, 16.0, muted());

            // Per-row delete button.
            let del = Rect::new(rect.x + rect.w - 84.0, rect.y + (row_h - 28.0) * 0.5, 74.0, 28.0);
            let confirming = self.confirm_delete.as_deref() == Some(info.path.as_str());
            let del_hovered = del.contains(mp);
            draw_rectangle(
                del.x,
                del.y,
                del.w,
                del.h,
                if confirming {
                    Color::new(0.45, 0.14, 0.13, 0.95)
                } else if del_hovered {
                    Color::new(0.28, 0.16, 0.17, 0.95)
                } else {
                    Color::new(0.16, 0.13, 0.15, 0.9)
                },
            );
            let label = if confirming { "Sure?" } else { "Delete" };
            let m = measure_text(label, None, 16, 1.0);
            draw_text(
                label,
                del.x + (del.w - m.width) * 0.5,
                del.y + 19.0,
                16.0,
                if confirming { text_color() } else { muted() },
            );

            if clicked && del_hovered {
                if confirming {
                    let _ = std::fs::remove_file(&info.path);
                    self.confirm_delete = None;
                    self.refresh_worlds();
                    self.sel = 0;
                    return Action::None;
                }
                self.confirm_delete = Some(info.path.clone());
                return Action::None;
            }
            if clicked && hovered {
                // Single click selects the row, a quick second click plays it.
                let double =
                    self.click_row == Some(idx) && self.time - self.click_at < 0.45;
                self.click_row = Some(idx);
                self.click_at = self.time;
                if double {
                    return Action::LoadWorld(info.path.clone());
                }
            }
            if enter && selected {
                return Action::LoadWorld(info.path.clone());
            }
            if delete_key && selected {
                if self.confirm_delete.as_deref() == Some(info.path.as_str()) {
                    let _ = std::fs::remove_file(&info.path);
                    self.confirm_delete = None;
                    self.refresh_worlds();
                    self.sel = 0;
                    return Action::None;
                }
                self.confirm_delete = Some(info.path.clone());
            }
        }

        // Bottom buttons.
        let list_bottom = top + visible as f32 * (row_h + gap);
        let half = (w - 14.0) * 0.5;
        let play = Rect::new(x, list_bottom + 12.0, half, 48.0);
        let create = Rect::new(x + half + 14.0, list_bottom + 12.0, half, 48.0);
        let quarter = (w - 30.0) / 4.0;
        let y2 = list_bottom + 70.0;
        let settings = Rect::new(x, y2, quarter, 44.0);
        let delete = Rect::new(x + quarter + 10.0, y2, quarter, 44.0);
        let recreate = Rect::new(x + 2.0 * (quarter + 10.0), y2, quarter, 44.0);
        let back = Rect::new(x + 3.0 * (quarter + 10.0), y2, quarter, 44.0);

        let rects = [play, create, settings, delete, recreate, back];
        let labels = [
            "Play selected",
            "Create new world",
            "Settings",
            "Delete",
            "Recreate",
            "Back",
        ];
        for (i, rect) in rects.iter().enumerate() {
            let hovered = rect.contains(mp);
            if hovered {
                self.sel = rows + i;
            }
            let selected = self.sel == rows + i;
            let enabled = rows > 0 || !matches!(i, 0 | 3 | 4);
            button(*rect, labels[i], selected, hovered, enabled, 20.0);
            if clicked && hovered && enabled {
                self.sel = rows + i;
                action = self.worlds_button(i);
            }
        }

        if enter && self.sel >= rows {
            action = self.worlds_button(self.sel - rows);
        }
        if esc {
            self.page = Page::Main;
            self.sel = 0;
            return Action::None;
        }

        draw_footer("Enter or double-click - play    Del - delete    Esc - back", sw, sh);
        action
    }

    fn worlds_button(&mut self, i: usize) -> Action {
        match i {
            0 => {
                if let Some(info) = self.worlds.get(self.focus).cloned() {
                    return Action::LoadWorld(info.path);
                }
                Action::None
            }
            1 => {
                self.open(Page::Create);
                Action::None
            }
            2 => {
                self.open_settings();
                Action::None
            }
            3 => {
                if self.focus < self.worlds.len() {
                    let path = self.worlds[self.focus].path.clone();
                    if self.confirm_delete.as_deref() == Some(path.as_str()) {
                        let _ = std::fs::remove_file(&path);
                        self.confirm_delete = None;
                        self.refresh_worlds();
                        self.sel = 0;
                    } else {
                        self.confirm_delete = Some(path);
                    }
                }
                Action::None
            }
            4 => {
                if let Some(info) = self.worlds.get(self.focus).cloned() {
                    return Action::Recreate {
                        path: info.path,
                        seed: info.seed,
                        size: info.size,
                        name: info.name,
                        mode: info.mode,
                    };
                }
                Action::None
            }
            _ => {
                self.page = Page::Main;
                self.sel = 0;
                Action::None
            }
        }
    }

    fn draw_create(&mut self, mp: Vec2, clicked: bool, esc: bool) -> Action {
        let (sw, sh) = (screen_width(), screen_height());
        draw_title("CREATE WORLD", sw * 0.5, sh * 0.09, 56.0);

        let w = 600.0_f32.min(sw - 80.0);
        let x = (sw - w) * 0.5;
        let row_h = 64.0;
        let gap = 8.0;
        let top = sh * 0.17;

        let name_rect = Rect::new(x, top, w, row_h);
        let seed_rect = Rect::new(x, top + row_h + gap, w, row_h);
        let size_rect = Rect::new(x, top + 2.0 * (row_h + gap), w, row_h);
        let mode_rect = Rect::new(x, top + 3.0 * (row_h + gap), w, row_h);
        let create_rect = Rect::new(x, top + 4.0 * (row_h + gap) + 6.0, w, 50.0);
        let back_rect = Rect::new(x, create_rect.y + 58.0, w, 44.0);

        let total = 6;
        if is_key_pressed(KeyCode::Down) {
            self.sel = (self.sel + 1) % total;
        }
        if is_key_pressed(KeyCode::Up) {
            self.sel = (self.sel + total - 1) % total;
        }
        for (i, rect) in [
            name_rect,
            seed_rect,
            size_rect,
            mode_rect,
            create_rect,
            back_rect,
        ]
        .iter()
        .enumerate()
        {
            if rect.contains(mp) && clicked {
                self.sel = i;
            }
        }

        self.name_input.edit(self.sel == 0);
        self.seed_input.edit(self.sel == 1);

        if self.sel == 2 {
            if is_key_pressed(KeyCode::Left) {
                self.size_idx = (self.size_idx + 2) % 3;
            }
            if is_key_pressed(KeyCode::Right) {
                self.size_idx = (self.size_idx + 1) % 3;
            }
        }
        if self.sel == 3 {
            if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Right) {
                self.mode_idx = 1 - self.mode_idx;
            }
        }

        draw_text_field(
            name_rect,
            "World name",
            self.name_input.value(),
            self.sel == 0,
            self.time,
        );
        draw_text_field(
            seed_rect,
            "Seed (empty = random)",
            self.seed_input.value(),
            self.sel == 1,
            self.time,
        );

        // Random seed shortcut inside the seed row.
        let random_rect = Rect::new(
            seed_rect.x + seed_rect.w - 100.0,
            seed_rect.y + seed_rect.h - 38.0,
            86.0,
            26.0,
        );
        let random_hovered = random_rect.contains(mp);
        draw_rectangle(
            random_rect.x,
            random_rect.y,
            random_rect.w,
            random_rect.h,
            if random_hovered {
                Color::new(0.18, 0.28, 0.16, 0.95)
            } else {
                Color::new(0.12, 0.15, 0.13, 0.9)
            },
        );
        let m = measure_text("Random", None, 16, 1.0);
        draw_text(
            "Random",
            random_rect.x + (random_rect.w - m.width) * 0.5,
            random_rect.y + 18.0,
            16.0,
            green(),
        );
        if clicked && random_hovered {
            self.seed_input.set(&format!("{}", macroquad::rand::gen_range(1u32, 1_000_000_000)));
            self.sel = 1;
        }

        // Size row.
        panel(size_rect, self.sel == 2, size_rect.contains(mp));
        draw_text(
            "World size (arrows)",
            size_rect.x + 16.0,
            size_rect.y + 24.0,
            16.0,
            muted(),
        );
        let size_label = SIZES[self.size_idx].title();
        let m = measure_text(size_label, None, 22, 1.0);
        draw_text(
            size_label,
            size_rect.x + size_rect.w - 16.0 - m.width,
            size_rect.y + 50.0,
            22.0,
            text_color(),
        );
        if clicked && size_rect.contains(mp) {
            self.size_idx = (self.size_idx + 1) % 3;
            self.sel = 2;
        }

        // Mode row.
        panel(mode_rect, self.sel == 3, mode_rect.contains(mp));
        draw_text(
            "Game mode (arrows)",
            mode_rect.x + 16.0,
            mode_rect.y + 24.0,
            16.0,
            muted(),
        );
        let mode = if self.mode_idx == 0 {
            Mode::Creative
        } else {
            Mode::Survival
        };
        let m = measure_text(mode.title(), None, 20, 1.0);
        draw_text(
            mode.title(),
            mode_rect.x + mode_rect.w - 16.0 - m.width,
            mode_rect.y + 50.0,
            20.0,
            if mode == Mode::Survival {
                Color::new(0.95, 0.72, 0.42, 1.0)
            } else {
                green()
            },
        );
        if clicked && mode_rect.contains(mp) {
            self.mode_idx = 1 - self.mode_idx;
            self.sel = 3;
        }

        button(
            create_rect,
            "Create",
            self.sel == 4,
            create_rect.contains(mp),
            true,
            24.0,
        );
        button(
            back_rect,
            "Back",
            self.sel == 5,
            back_rect.contains(mp),
            true,
            22.0,
        );

        let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
        if (clicked && create_rect.contains(mp)) || (enter && self.sel == 4) {
            return Action::CreateWorld {
                seed: self.resolved_seed(),
                size: SIZES[self.size_idx],
                name: self.resolved_name(),
                mode,
            };
        }
        if (clicked && back_rect.contains(mp)) || (enter && self.sel == 5) || esc {
            self.open(Page::Worlds);
            return Action::None;
        }

        draw_footer("Type to edit    |    Enter - create    Esc - back", sw, sh);
        Action::None
    }

    fn resolved_seed(&self) -> u32 {
        match self.seed_input.value().trim().parse::<u32>() {
            Ok(0) | Err(_) => macroquad::rand::gen_range(1u32, 1_000_000_000),
            Ok(v) => v,
        }
    }

    fn resolved_name(&self) -> String {
        let name: String = self
            .name_input
            .value()
            .chars()
            .filter(|c| c.is_ascii() && !c.is_control())
            .collect();
        let name = name.trim();
        if name.is_empty() {
            format!("World {}", self.worlds.len() + 1)
        } else {
            name.to_string()
        }
    }

    fn draw_pause(&mut self, mp: Vec2, clicked: bool, esc: bool) -> Action {
        let (sw, sh) = (screen_width(), screen_height());
        draw_title("PAUSED", sw * 0.5, sh * 0.24, 72.0);

        let items = ["Resume", "Settings", "Quit to menu"];
        if let Some(i) = self.buttons(&items, sh * 0.42, mp, clicked, 56.0) {
            match i {
                0 => return Action::Resume,
                1 => {
                    self.open_settings();
                    return Action::None;
                }
                _ => {
                    self.page = Page::Main;
                    self.sel = 0;
                    return Action::QuitToMenu;
                }
            }
        }
        if esc {
            return Action::Resume;
        }
        draw_footer("Esc - back to the world", sw, sh);
        Action::None
    }

    fn draw_settings(&mut self, s: &mut Settings, mp: Vec2, clicked: bool, esc: bool) -> Action {
        let (sw, sh) = (screen_width(), screen_height());
        draw_title("SETTINGS", sw * 0.5, sh * 0.08, 56.0);

        let total = SLIDERS.len() + 2; // sliders + fullscreen + Back
        if is_key_pressed(KeyCode::Down) {
            self.sel = (self.sel + 1) % total;
        }
        if is_key_pressed(KeyCode::Up) {
            self.sel = (self.sel + total - 1) % total;
        }

        let w = 620.0_f32.min(sw - 80.0);
        let x = (sw - w) * 0.5;
        let row_h = 64.0;
        let gap = 6.0;
        let top = sh * 0.18;

        for (i, spec) in SLIDERS.iter().enumerate() {
            let rect = Rect::new(x, top + i as f32 * (row_h + gap), w, row_h);
            let hovered = rect.contains(mp);
            if hovered && clicked {
                self.sel = i;
            }
            let selected = self.sel == i;
            panel(rect, selected, hovered);
            if selected {
                draw_text(">", rect.x + 12.0, rect.y + 30.0, 22.0, green());
            }

            let mut value = slider_value(s, i);
            draw_text(spec.label, rect.x + 30.0, rect.y + 28.0, 22.0, text_color());
            let txt = slider_text(i, value);
            let m = measure_text(&txt, None, 22, 1.0);
            draw_text(
                &txt,
                rect.x + rect.w - 18.0 - m.width,
                rect.y + 28.0,
                22.0,
                if selected { green() } else { text_color() },
            );

            let bar = Rect::new(rect.x + 30.0, rect.y + row_h - 20.0, rect.w - 60.0, 6.0);
            draw_rectangle(bar.x, bar.y, bar.w, bar.h, Color::new(0.0, 0.0, 0.0, 0.55));
            let ratio = ((value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0);
            draw_rectangle(bar.x, bar.y, bar.w * ratio, bar.h, green());
            let handle_x = bar.x + bar.w * ratio;
            draw_rectangle(
                handle_x - 3.0,
                bar.y - 4.0,
                6.0,
                bar.h + 8.0,
                Color::new(1.0, 1.0, 1.0, 0.85),
            );

            let mut changed = false;
            if selected {
                if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                    value -= spec.step;
                    changed = true;
                }
                if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                    value += spec.step;
                    changed = true;
                }
            }
            let hit = Rect::new(bar.x, bar.y - 10.0, bar.w, bar.h + 20.0);
            if clicked && hit.contains(mp) {
                let r = ((mp.x - bar.x) / bar.w).clamp(0.0, 1.0);
                value = spec.min + r * (spec.max - spec.min);
                changed = true;
            }
            if changed {
                let clamped = value.clamp(spec.min, spec.max);
                set_slider_value(s, i, clamped);
                s.save();
            }
        }

        // Fullscreen toggle.
        let fs_rect = Rect::new(
            x,
            top + SLIDERS.len() as f32 * (row_h + gap),
            w,
            row_h,
        );
        let fs_hovered = fs_rect.contains(mp);
        if fs_hovered && clicked {
            self.sel = SLIDERS.len();
        }
        let fs_selected = self.sel == SLIDERS.len();
        panel(fs_rect, fs_selected, fs_hovered);
        if fs_selected {
            draw_text(">", fs_rect.x + 12.0, fs_rect.y + 30.0, 22.0, green());
        }
        draw_text(
            "Fullscreen",
            fs_rect.x + 30.0,
            fs_rect.y + 30.0,
            22.0,
            text_color(),
        );
        let on = s.fullscreen;
        let label = if on { "On" } else { "Off" };
        let m = measure_text(label, None, 22, 1.0);
        draw_text(
            label,
            fs_rect.x + fs_rect.w - 18.0 - m.width,
            fs_rect.y + 30.0,
            22.0,
            if fs_selected { green() } else { text_color() },
        );
        let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
        if (clicked && fs_hovered) || (enter && fs_selected) {
            s.fullscreen = !s.fullscreen;
            macroquad::window::set_fullscreen(s.fullscreen);
            s.save();
            self.sel = SLIDERS.len();
        }

        // Back button.
        let back_rect = Rect::new(
            x,
            top + (SLIDERS.len() + 1) as f32 * (row_h + gap) + 8.0,
            w,
            50.0,
        );
        let back_hovered = back_rect.contains(mp);
        if back_hovered && clicked {
            self.sel = SLIDERS.len() + 1;
        }
        let back_selected = self.sel == SLIDERS.len() + 1;
        button(back_rect, "Back", back_selected, back_hovered, true, 22.0);

        if (back_hovered && clicked) || (enter && back_selected) || esc {
            self.page = self.back;
            self.sel = 0;
            return Action::None;
        }

        draw_footer("Arrows + Enter, or mouse    |    Esc - back", sw, sh);
        Action::None
    }

    fn buttons(
        &mut self,
        items: &[&str],
        top: f32,
        mp: Vec2,
        clicked: bool,
        height: f32,
    ) -> Option<usize> {
        let sw = screen_width();
        let w = 360.0_f32.min(sw - 80.0);
        let x = (sw - w) * 0.5;

        if is_key_pressed(KeyCode::Down) {
            self.sel = (self.sel + 1) % items.len();
        }
        if is_key_pressed(KeyCode::Up) {
            self.sel = (self.sel + items.len() - 1) % items.len();
        }

        let mut activated = None;
        for (i, label) in items.iter().enumerate() {
            let rect = Rect::new(x, top + i as f32 * (height + 14.0), w, height);
            let hovered = rect.contains(mp);
            if hovered {
                self.sel = i;
            }
            let selected = self.sel == i;
            button(rect, label, selected, hovered, true, 26.0);
            if hovered && clicked {
                activated = Some(i);
            }
        }

        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            activated = Some(self.sel.min(items.len() - 1));
        }
        activated
    }
}

fn panel(rect: Rect, selected: bool, hovered: bool) {
    let fill = if selected {
        Color::new(0.20, 0.33, 0.19, 0.92)
    } else if hovered {
        Color::new(0.14, 0.17, 0.20, 0.88)
    } else {
        Color::new(0.09, 0.11, 0.14, 0.82)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if selected {
            green()
        } else {
            Color::new(0.0, 0.0, 0.0, 0.55)
        },
    );
}

fn button(rect: Rect, label: &str, selected: bool, hovered: bool, enabled: bool, size: f32) {
    let fill = if !enabled {
        Color::new(0.08, 0.09, 0.11, 0.6)
    } else if selected {
        Color::new(0.22, 0.38, 0.20, 0.94)
    } else if hovered {
        Color::new(0.16, 0.19, 0.22, 0.90)
    } else {
        Color::new(0.10, 0.12, 0.15, 0.84)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        2.0,
        if selected && enabled {
            green()
        } else {
            Color::new(1.0, 1.0, 1.0, 0.08)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if selected && enabled { 2.0 } else { 1.0 },
        if selected && enabled {
            green()
        } else {
            Color::new(0.0, 0.0, 0.0, 0.60)
        },
    );

    let m = measure_text(label, None, size as u16, 1.0);
    let color = if !enabled {
        Color::new(0.55, 0.58, 0.62, 0.6)
    } else if selected {
        Color::new(1.0, 1.0, 1.0, 1.0)
    } else {
        text_color()
    };
    draw_text(
        label,
        rect.x + (rect.w - m.width) * 0.5,
        rect.y + rect.h * 0.5 + m.height * 0.35,
        size,
        color,
    );
    if selected && enabled {
        draw_text(
            ">",
            rect.x + 14.0,
            rect.y + rect.h * 0.5 + m.height * 0.35,
            size,
            green(),
        );
    }
}

fn draw_text_field(rect: Rect, label: &str, value: &str, selected: bool, time: f32) {
    panel(rect, selected, false);
    draw_text(label, rect.x + 16.0, rect.y + 24.0, 16.0, muted());
    let baseline = rect.y + rect.h - 14.0;
    draw_text(value, rect.x + 16.0, baseline, 24.0, text_color());
    if selected {
        let m = measure_text(value, None, 24, 1.0);
        if (time * 2.0) as i32 % 2 == 0 {
            draw_rectangle(
                rect.x + 18.0 + m.width,
                baseline - 19.0,
                2.0,
                22.0,
                green(),
            );
        }
    }
}

fn draw_title(text: &str, cx: f32, y: f32, size: f32) {
    let m = measure_text(text, None, size as u16, 1.0);
    let x = cx - m.width * 0.5;
    draw_text(text, x + 5.0, y + 5.0, size, Color::new(0.0, 0.0, 0.0, 0.45));
    draw_text(text, x, y, size, green());
}

fn draw_centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    let m = measure_text(text, None, size as u16, 1.0);
    draw_text(text, cx - m.width * 0.5, y, size, color);
}

fn draw_footer(text: &str, sw: f32, sh: f32) {
    let m = measure_text(text, None, 18, 1.0);
    draw_text(
        text,
        (sw - m.width) * 0.5,
        sh - 42.0,
        18.0,
        Color::new(0.80, 0.84, 0.88, 0.65),
    );
}

fn ago(modified: u64) -> String {
    if modified == 0 {
        return String::new();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let d = now.saturating_sub(modified);
    if d < 60 {
        "just now".to_string()
    } else if d < 3600 {
        format!("{} min ago", d / 60)
    } else if d < 86_400 {
        format!("{} h ago", d / 3600)
    } else {
        format!("{} d ago", d / 86_400)
    }
}
