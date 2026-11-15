// Player inventory: 36 slots, a small craft grid and the inventory screen.
use macroquad::prelude::*;

use crate::atlas::{ATLAS_TILES, TILE};
use crate::blocks::{Block, ALL_BLOCKS, HOTBAR};
use crate::crafting::{self, Station};
use crate::items::{Item, Stack};

pub const SLOTS: usize = 36;
pub const HOTBAR_LEN: usize = 9;

const SLOT: f32 = 46.0;
const GAP: f32 = 6.0;
const COL: f32 = SLOT + GAP;

pub struct Inv {
    pub slots: [Stack; SLOTS],
    pub selected: usize,
    /// Stack attached to the mouse while the inventory is open.
    pub cursor: Stack,
    /// 3x3 craft cells; the 2x2 grid uses 0,1,3,4.
    pub craft: [Stack; 9],
    pub station: Station,
    pub open: bool,
    /// Recipe book overlay inside the inventory screen.
    pub book_open: bool,
    pub book_page: usize,
    /// Short feedback line shown in the book: text and the time it expires.
    pub book_note: Option<(String, f64)>,
}

impl Inv {
    pub fn new() -> Self {
        Self {
            slots: [Stack::EMPTY; SLOTS],
            selected: 0,
            cursor: Stack::EMPTY,
            craft: [Stack::EMPTY; 9],
            station: Station::Hand,
            open: false,
            book_open: false,
            book_page: 0,
            book_note: None,
        }
    }

    /// Creative mode starts with the classic palette in the hotbar.
    pub fn fill_creative(&mut self) {
        for (i, block) in HOTBAR.iter().enumerate() {
            self.slots[i] = Stack::new(Item::Block(*block), 64);
        }
    }

    pub fn craft_size(&self) -> usize {
        match self.station {
            Station::Hand => 2,
            _ => 3,
        }
    }

    pub fn held(&self) -> Option<Item> {
        self.slots
            .get(self.selected)
            .and_then(|s| s.item)
            .filter(|_| self.slots[self.selected].count > 0)
    }

    pub fn consume_held(&mut self) {
        if let Some(slot) = self.slots.get_mut(self.selected) {
            if slot.count > 0 {
                slot.count -= 1;
                if slot.count == 0 {
                    slot.item = None;
                }
            }
        }
    }

    pub fn pick_block(&mut self, block: Block) {
        if let Some(slot) = self.slots.get_mut(self.selected) {
            *slot = Stack::new(Item::Block(block), 64);
        }
    }

    /// Adds items to the first free stacks; returns how many did not fit.
    pub fn add(&mut self, item: Item, count: u8) -> u8 {
        let mut left = count;
        for slot in self.slots.iter_mut() {
            if left == 0 {
                break;
            }
            if slot.item == Some(item) && slot.count < item.max_stack() {
                let moved = (item.max_stack() - slot.count).min(left);
                slot.count += moved;
                left -= moved;
            }
        }
        for slot in self.slots.iter_mut() {
            if left == 0 {
                break;
            }
            if slot.is_empty() {
                let moved = left.min(item.max_stack());
                *slot = Stack::new(item, moved);
                left -= moved;
            }
        }
        left
    }

    pub fn craft_result(&self) -> Option<Stack> {
        crafting::result(&self.craft, self.craft_size(), self.station)
    }

    /// Consumes one item from every occupied craft cell.
    pub fn consume_craft(&mut self) {
        let n = self.craft_size();
        for y in 0..n {
            for x in 0..n {
                let slot = &mut self.craft[y * n + x];
                if slot.count > 0 {
                    slot.count -= 1;
                    if slot.count == 0 {
                        slot.item = None;
                    }
                }
            }
        }
    }

    /// Puts craft-grid leftovers back into the inventory.
    pub fn spill_craft(&mut self) {
        for i in 0..9 {
            if !self.craft[i].is_empty() {
                let stack = std::mem::replace(&mut self.craft[i], Stack::EMPTY);
                if let Some(item) = stack.item {
                    self.add(item, stack.count);
                }
            }
        }
    }

    pub fn item_count(&self, item: Item) -> u32 {
        self.slots
            .iter()
            .filter(|s| s.item == Some(item))
            .map(|s| s.count as u32)
            .sum()
    }

    fn take_items(&mut self, item: Item, mut count: u32) {
        for slot in self.slots.iter_mut() {
            if count == 0 {
                break;
            }
            if slot.item == Some(item) {
                let take = (slot.count as u32).min(count);
                slot.count -= take as u8;
                count -= take;
                if slot.count == 0 {
                    slot.item = None;
                }
            }
        }
    }

    /// Clears the craft grid back into the inventory and lays out a recipe
    /// from the items the player has. On failure nothing is moved.
    pub fn auto_fill(&mut self, recipe: &crafting::Recipe) -> Result<(), Vec<(Item, u32)>> {
        let missing: Vec<(Item, u32)> = recipe
            .ingredients()
            .into_iter()
            .filter(|(item, count)| self.item_count(*item) < *count)
            .collect();
        if !missing.is_empty() {
            return Err(missing);
        }

        self.spill_craft();
        let n = self.craft_size();
        for (item, count) in recipe.ingredients() {
            self.take_items(item, count);
        }
        for (y, line) in recipe.pattern.iter().enumerate() {
            if y >= n {
                break;
            }
            for (x, ch) in line.chars().enumerate() {
                if x >= n || ch == ' ' {
                    continue;
                }
                if let Some((_, item)) = recipe.key.iter().find(|(c, _)| *c == ch) {
                    self.craft[y * n + x] = Stack::new(*item, 1);
                }
            }
        }
        Ok(())
    }
}

fn atlas_source(tile: u8) -> Rect {
    let tx = (tile as u32 % ATLAS_TILES) as f32 * TILE as f32;
    let ty = (tile as u32 / ATLAS_TILES) as f32 * TILE as f32;
    Rect::new(tx, ty, TILE as f32, TILE as f32)
}

pub fn draw_icon(atlas: &Texture2D, rect: Rect, item: Item) {
    draw_texture_ex(
        atlas,
        rect.x,
        rect.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(rect.w, rect.h)),
            source: Some(atlas_source(item.tile())),
            ..Default::default()
        },
    );
}

fn slot_frame(rect: Rect, hot: bool) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hot {
            Color::new(0.30, 0.34, 0.28, 0.98)
        } else {
            Color::new(0.13, 0.14, 0.18, 0.95)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if hot {
            Color::new(0.62, 0.90, 0.48, 1.0)
        } else {
            Color::new(0.0, 0.0, 0.0, 0.65)
        },
    );
}

fn draw_stack(atlas: &Texture2D, rect: Rect, stack: &Stack, hot: bool) {
    slot_frame(rect, hot);
    if let Some(item) = stack.item {
        draw_icon(atlas, Rect::new(rect.x + 5.0, rect.y + 5.0, rect.w - 10.0, rect.h - 10.0), item);
        if stack.count > 1 {
            let txt = format!("{}", stack.count);
            let m = measure_text(&txt, None, 18, 1.0);
            draw_text(
                &txt,
                rect.x + rect.w - m.width - 3.0,
                rect.y + rect.h - 4.0,
                18.0,
                Color::new(1.0, 1.0, 1.0, 0.95),
            );
        }
    }
}

/// Vanilla-ish click handling: pick up, place, merge or swap.
fn click_slot(slot: &mut Stack, cursor: &mut Stack, right: bool) {
    let hovered = slot.item;
    if right {
        if cursor.is_empty() {
            if let Some(_item) = hovered {
                let half = (slot.count + 1) / 2;
                *cursor = slot.take(half);
            }
        } else if slot.is_empty() {
            let item = cursor.item.unwrap();
            *slot = Stack::new(item, 1);
            cursor.count -= 1;
            if cursor.count == 0 {
                cursor.item = None;
            }
        } else if slot.same(cursor) && slot.count < slot.item.unwrap().max_stack() {
            slot.count += 1;
            cursor.count -= 1;
            if cursor.count == 0 {
                cursor.item = None;
            }
        } else {
            std::mem::swap(slot, cursor);
        }
        return;
    }

    if cursor.is_empty() {
        std::mem::swap(slot, cursor);
    } else if slot.is_empty() {
        *slot = *cursor;
        *cursor = Stack::EMPTY;
    } else if slot.same(cursor) {
        let moved = slot.space().min(cursor.count);
        slot.count += moved;
        cursor.count -= moved;
        if cursor.count == 0 {
            cursor.item = None;
        }
    } else {
        std::mem::swap(slot, cursor);
    }
}

fn hit(rect: Rect, mp: Vec2) -> bool {
    rect.contains(mp)
}

/// Draws the inventory screen and handles mouse clicks. Returns the hovered item name.
pub fn draw_ui(inv: &mut Inv, atlas: &Texture2D, creative: bool) {
    let (sw, sh) = (screen_width(), screen_height());

    if inv.book_open {
        if is_key_pressed(KeyCode::R) || is_key_pressed(KeyCode::Escape) {
            inv.book_open = false;
        }
        draw_recipe_book(inv, atlas);
        draw_cursor(inv, atlas);
        return;
    }

    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.02, 0.03, 0.05, 0.75));

    let pw = 522.0;
    let ph = 486.0;
    let px = (sw - pw) * 0.5;
    let py = (sh - ph) * 0.5;
    draw_rectangle(px, py, pw, ph, Color::new(0.07, 0.08, 0.10, 0.96));
    draw_rectangle_lines(px, py, pw, ph, 2.0, Color::new(0.30, 0.42, 0.28, 0.9));

    let title = if creative {
        "ITEMS (creative)"
    } else {
        match inv.station {
            Station::Hand => "INVENTORY",
            Station::Table => "CRAFTING TABLE",
            Station::Furnace => "FURNACE",
        }
    };
    draw_text(title, px + 24.0, py + 32.0, 26.0, Color::new(0.62, 0.90, 0.48, 1.0));

    let mp: Vec2 = mouse_position().into();
    let clicked = is_mouse_button_pressed(MouseButton::Left);
    let right = is_mouse_button_pressed(MouseButton::Right);
    let mut hovered_name: Option<Item> = None;

    // Recipe book button (survival only; creative has every block already).
    if !creative {
        let btn = Rect::new(px + pw - 156.0, py + 12.0, 132.0, 30.0);
        let hot = btn.contains(mp);
        draw_rectangle(
            btn.x,
            btn.y,
            btn.w,
            btn.h,
            if hot {
                Color::new(0.24, 0.36, 0.22, 1.0)
            } else {
                Color::new(0.15, 0.20, 0.15, 1.0)
            },
        );
        draw_rectangle_lines(
            btn.x,
            btn.y,
            btn.w,
            btn.h,
            1.0,
            Color::new(0.62, 0.90, 0.48, 0.85),
        );
        draw_text(
            "Recipes (R)",
            btn.x + 12.0,
            btn.y + 21.0,
            18.0,
            Color::new(0.86, 0.94, 0.82, 1.0),
        );
        if (clicked && hot) || is_key_pressed(KeyCode::R) {
            inv.book_open = true;
            inv.book_page = 0;
        }
    }

    let craft_x = px + 24.0;
    let craft_y = py + 58.0;

    if creative {
        // Palette: every block, click to grab a full stack.
        draw_text(
            "Creative blocks",
            craft_x,
            craft_y - 8.0,
            18.0,
            Color::new(0.78, 0.82, 0.86, 0.9),
        );
        for (i, block) in ALL_BLOCKS.iter().enumerate() {
            let cx = craft_x + (i % 9) as f32 * COL;
            let cy = craft_y + (i / 9) as f32 * COL;
            let rect = Rect::new(cx, cy, SLOT, SLOT);
            let hot = hit(rect, mp);
            draw_stack(atlas, rect, &Stack::new(Item::Block(*block), 1), hot);
            if hot {
                hovered_name = Some(Item::Block(*block));
                if clicked {
                    inv.cursor = Stack::new(Item::Block(*block), 64);
                }
            }
        }
    } else {
        let n = inv.craft_size();
        for y in 0..n {
            for x in 0..n {
                let rect = Rect::new(craft_x + x as f32 * COL, craft_y + y as f32 * COL, SLOT, SLOT);
                let idx = y * n + x;
                let hot = hit(rect, mp);
                let stack = inv.craft[idx];
                draw_stack(atlas, rect, &stack, hot);
                if hot {
                    if let Some(item) = stack.item {
                        hovered_name = Some(item);
                    }
                    if clicked || right {
                        click_slot(&mut inv.craft[idx], &mut inv.cursor, right);
                    }
                }
            }
        }

        // Arrow and result slot.
        let out_x = craft_x + n as f32 * COL + 26.0;
        let out_y = craft_y + (n as f32 * COL - GAP) * 0.5 - SLOT * 0.5;
        let mid_y = out_y + SLOT * 0.5;
        draw_text(">", out_x - 22.0, mid_y + 8.0, 26.0, Color::new(0.7, 0.75, 0.8, 0.9));
        let out_rect = Rect::new(out_x, out_y, SLOT, SLOT);
        let preview = inv.craft_result();
        let out_hot = hit(out_rect, mp);
        match &preview {
            Some(stack) => draw_stack(atlas, out_rect, stack, out_hot),
            None => slot_frame(out_rect, false),
        }
        if out_hot {
            if let Some(stack) = &preview {
                hovered_name = stack.item;
            }
            if clicked {
                if let Some(out) = preview {
                    let fits = inv.cursor.is_empty()
                        || (inv.cursor.same(&out)
                            && inv.cursor.count + out.count <= out.item.unwrap().max_stack());
                    if fits {
                        if inv.cursor.is_empty() {
                            inv.cursor = out;
                        } else {
                            inv.cursor.count += out.count;
                        }
                        inv.consume_craft();
                    }
                }
            }
        }
    }

    // Main grid (3 rows of 9).
    let main_y = craft_y + 3.0 * COL + 22.0;
    for i in 0..27 {
        let x = craft_x + (i % 9) as f32 * COL;
        let y = main_y + (i / 9) as f32 * COL;
        let idx = HOTBAR_LEN + i;
        let rect = Rect::new(x, y, SLOT, SLOT);
        let hot = hit(rect, mp);
        let stack = inv.slots[idx];
        draw_stack(atlas, rect, &stack, hot);
        if hot {
            if let Some(item) = stack.item {
                hovered_name = Some(item);
            }
            if clicked || right {
                click_slot(&mut inv.slots[idx], &mut inv.cursor, right);
            }
        }
    }

    // Hotbar.
    let hot_y = main_y + 3.0 * COL + 14.0;
    for i in 0..HOTBAR_LEN {
        let rect = Rect::new(craft_x + i as f32 * COL, hot_y, SLOT, SLOT);
        let hot = hit(rect, mp);
        let stack = inv.slots[i];
        let selected = i == inv.selected;
        draw_stack(atlas, rect, &stack, hot);
        if selected {
            draw_rectangle_lines(
                rect.x - 2.0,
                rect.y - 2.0,
                rect.w + 4.0,
                rect.h + 4.0,
                2.0,
                Color::new(1.0, 1.0, 1.0, 0.9),
            );
        }
        if hot {
            if let Some(item) = stack.item {
                hovered_name = Some(item);
            }
            if clicked {
                inv.selected = i;
            }
            if clicked || right {
                click_slot(&mut inv.slots[i], &mut inv.cursor, right);
            }
        }
    }

    // Hovered item name and help line.
    let help = if creative {
        "E - close    LMB - grab stack    RMB - place one"
    } else {
        "E - close    LMB - take/put    RMB - split    Craft in the grid"
    };
    if let Some(item) = hovered_name {
        let m = measure_text(item.name(), None, 20, 1.0);
        draw_text(
            item.name(),
            px + (pw - m.width) * 0.5,
            py + ph - 34.0,
            20.0,
            Color::new(1.0, 1.0, 1.0, 0.95),
        );
    }
    let m = measure_text(help, None, 16, 1.0);
    draw_text(
        help,
        px + (pw - m.width) * 0.5,
        py + ph - 12.0,
        16.0,
        Color::new(0.76, 0.80, 0.84, 0.75),
    );

    // Stack held by the mouse.
    draw_cursor(inv, atlas);
}

fn draw_cursor(inv: &Inv, atlas: &Texture2D) {
    if inv.cursor.is_empty() {
        return;
    }
    let mp: Vec2 = mouse_position().into();
    let rect = Rect::new(mp.x - SLOT * 0.5, mp.y - SLOT * 0.5, SLOT, SLOT);
    if let Some(item) = inv.cursor.item {
        draw_icon(
            atlas,
            Rect::new(rect.x + 4.0, rect.y + 4.0, rect.w - 8.0, rect.h - 8.0),
            item,
        );
    }
    if inv.cursor.count > 1 {
        let txt = format!("{}", inv.cursor.count);
        draw_text(
            &txt,
            rect.x + rect.w - 16.0,
            rect.y + rect.h - 2.0,
            18.0,
            Color::new(1.0, 1.0, 1.0, 0.95),
        );
    }
}

fn station_label(station: Station) -> &'static str {
    match station {
        Station::Hand => "2x2 grid",
        Station::Table => "Crafting table",
        Station::Furnace => "Furnace",
    }
}

fn can_craft(inv: &Inv, recipe: &crafting::Recipe) -> bool {
    recipe
        .ingredients()
        .iter()
        .all(|(item, count)| inv.item_count(*item) >= *count)
}

/// Full-screen recipe list: click a row to lay the ingredients into the grid.
fn draw_recipe_book(inv: &mut Inv, atlas: &Texture2D) {
    let (sw, sh) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.02, 0.03, 0.05, 0.82));

    let bw = 600.0_f32.min(sw - 60.0);
    let bh = 540.0_f32.min(sh - 60.0);
    let bx = (sw - bw) * 0.5;
    let by = (sh - bh) * 0.5;
    draw_rectangle(bx, by, bw, bh, Color::new(0.07, 0.08, 0.10, 0.98));
    draw_rectangle_lines(bx, by, bw, bh, 2.0, Color::new(0.30, 0.42, 0.28, 0.9));

    draw_text(
        "RECIPE BOOK",
        bx + 24.0,
        by + 36.0,
        28.0,
        Color::new(0.62, 0.90, 0.48, 1.0),
    );
    let sub = format!(
        "{}    click a recipe to fill the grid    R / Esc - close",
        station_label(inv.station)
    );
    draw_text(&sub, bx + 24.0, by + 60.0, 15.0, Color::new(0.76, 0.80, 0.84, 0.8));

    let recipes: Vec<&crafting::Recipe> = crafting::RECIPES
        .iter()
        .filter(|r| crafting::allowed(r.station, inv.station))
        .collect();
    let row_h = 54.0;
    let gap = 4.0;
    let list_top = by + 74.0;
    let footer = 64.0;
    let per_page = (((bh - 74.0 - footer) / (row_h + gap)).floor() as usize).max(1);
    let pages = ((recipes.len() + per_page - 1) / per_page).max(1);
    if inv.book_page >= pages {
        inv.book_page = 0;
    }

    let mp: Vec2 = mouse_position().into();
    let clicked = is_mouse_button_pressed(MouseButton::Left);
    let (_, wheel) = mouse_wheel();
    if wheel > 0.0 && pages > 1 {
        inv.book_page = (inv.book_page + pages - 1) % pages;
    } else if wheel < 0.0 && pages > 1 {
        inv.book_page = (inv.book_page + 1) % pages;
    }

    let rw = bw - 40.0;
    let start = inv.book_page * per_page;
    for (row, recipe) in recipes.iter().skip(start).take(per_page).enumerate() {
        let ry = list_top + row as f32 * (row_h + gap);
        let rect = Rect::new(bx + 20.0, ry, rw, row_h);
        let hot = rect.contains(mp);
        let ready = can_craft(inv, recipe);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if hot {
                Color::new(0.17, 0.23, 0.16, 1.0)
            } else {
                Color::new(0.11, 0.12, 0.14, 0.95)
            },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            if ready {
                Color::new(0.42, 0.62, 0.34, 0.9)
            } else {
                Color::new(0.24, 0.26, 0.30, 0.9)
            },
        );

        // Result icon.
        let icon = Rect::new(rect.x + 9.0, rect.y + 9.0, 36.0, 36.0);
        draw_rectangle(icon.x - 1.0, icon.y - 1.0, icon.w + 2.0, icon.h + 2.0, Color::new(0.0, 0.0, 0.0, 0.45));
        draw_icon(atlas, icon, recipe.out.0);

        let name = if recipe.out.1 > 1 {
            format!("{} x{}", recipe.out.0.name(), recipe.out.1)
        } else {
            recipe.out.0.name().to_string()
        };
        draw_text(
            &name,
            rect.x + 58.0,
            rect.y + 24.0,
            19.0,
            if ready {
                Color::new(0.95, 0.97, 0.93, 1.0)
            } else {
                Color::new(0.72, 0.74, 0.72, 0.9)
            },
        );

        let need = recipe
            .ingredients()
            .iter()
            .map(|(item, count)| format!("{} x{}", item.name(), count))
            .collect::<Vec<_>>()
            .join("  +  ");
        draw_text(
            &need,
            rect.x + 58.0,
            rect.y + 43.0,
            14.0,
            Color::new(0.70, 0.76, 0.66, 0.9),
        );

        let tag = station_label(recipe.station);
        let m = measure_text(tag, None, 13, 1.0);
        draw_text(
            tag,
            rect.x + rect.w - m.width - 10.0,
            rect.y + 22.0,
            13.0,
            Color::new(0.60, 0.66, 0.58, 0.8),
        );

        if hot && clicked {
            match inv.auto_fill(recipe) {
                Ok(()) => {
                    inv.book_note = Some((
                        format!("Grid filled for {}", recipe.out.0.name()),
                        get_time() + 2.5,
                    ));
                    inv.book_open = false;
                }
                Err(missing) => {
                    let text = missing
                        .iter()
                        .map(|(item, count)| {
                            let have = inv.item_count(*item).min(*count);
                            format!("{} x{}", item.name(), *count - have)
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    inv.book_note = Some((format!("Missing: {text}"), get_time() + 2.5));
                }
            }
        }
    }

    // Paging.
    let footer_y = by + bh - 40.0;
    if pages > 1 {
        let prev = Rect::new(bx + 20.0, footer_y - 18.0, 34.0, 28.0);
        let next = Rect::new(bx + bw - 54.0, footer_y - 18.0, 34.0, 28.0);
        for (rect, label, delta) in [(prev, "<", pages - 1), (next, ">", 1)] {
            let hot = rect.contains(mp);
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                if hot {
                    Color::new(0.24, 0.34, 0.22, 1.0)
                } else {
                    Color::new(0.13, 0.16, 0.14, 1.0)
                },
            );
            draw_text(label, rect.x + 12.0, rect.y + 20.0, 20.0, Color::new(0.86, 0.94, 0.82, 1.0));
            if hot && clicked {
                inv.book_page = (inv.book_page + delta) % pages;
            }
        }
        let label = format!("Page {} / {}", inv.book_page + 1, pages);
        let m = measure_text(&label, None, 15, 1.0);
        draw_text(
            &label,
            bx + (bw - m.width) * 0.5,
            footer_y,
            15.0,
            Color::new(0.72, 0.76, 0.72, 0.85),
        );
    }

    if let Some((text, until)) = &inv.book_note {
        if get_time() < *until {
            let m = measure_text(text, None, 17, 1.0);
            draw_text(
                text,
                bx + (bw - m.width) * 0.5,
                by + bh - 12.0,
                17.0,
                Color::new(0.95, 0.86, 0.55, 0.95),
            );
        }
    }
    if inv.book_note.as_ref().is_some_and(|(_, until)| get_time() >= *until) {
        inv.book_note = None;
    }
}

/// Hotbar drawn over the world; creative slots have no counts.
pub fn draw_hotbar(inv: &Inv, atlas: &Texture2D, creative: bool) {
    let (sw, sh) = (screen_width(), screen_height());
    let slot = 54.0;
    let gap = 4.0;
    let total = HOTBAR_LEN as f32 * slot + (HOTBAR_LEN - 1) as f32 * gap;
    let x0 = (sw - total) * 0.5;
    let y0 = sh - slot - 18.0;

    for i in 0..HOTBAR_LEN {
        let x = x0 + i as f32 * (slot + gap);
        let is_sel = i == inv.selected;
        draw_rectangle(
            x - 2.0,
            y0 - 2.0,
            slot + 4.0,
            slot + 4.0,
            if is_sel {
                Color::new(1.0, 1.0, 1.0, 0.9)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.55)
            },
        );
        draw_rectangle(x, y0, slot, slot, Color::new(0.12, 0.12, 0.14, 0.75));
        let stack = inv.slots[i];
        if let Some(item) = stack.item {
            draw_icon(
                atlas,
                Rect::new(x + 3.0, y0 + 3.0, slot - 6.0, slot - 6.0),
                item,
            );
            if !creative && stack.count > 1 {
                let txt = format!("{}", stack.count);
                let m = measure_text(&txt, None, 18, 1.0);
                draw_text(
                    &txt,
                    x + slot - m.width - 4.0,
                    y0 + slot - 4.0,
                    18.0,
                    Color::new(1.0, 1.0, 1.0, 0.95),
                );
            }
        }
        draw_text(
            &format!("{}", i + 1),
            x + 4.0,
            y0 + 14.0,
            16.0,
            Color::new(1.0, 1.0, 1.0, 0.8),
        );
    }
}

/// Small pixel heart used by the health bar.
pub fn draw_hearts(health: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    let full = (health / 2.0).ceil().max(0.0) as i32;
    let x0 = sw * 0.5 - 5.0 * 26.0;
    let y = sh - 92.0;
    // 8x7 heart bitmap, drawn as 3px cells.
    const HEART: [&str; 7] = [
        ".XX.XX..",
        "XXXXXXXX",
        "XXXXXXXX",
        "XXXXXXXX",
        ".XXXXXX.",
        "..XXXX..",
        "...XX...",
    ];
    for i in 0..10 {
        let bx = x0 + i as f32 * 26.0;
        let filled = i < full;
        for (row, line) in HEART.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let color = if ch == '.' {
                    continue;
                } else if filled {
                    Color::new(0.86, 0.20, 0.18, 0.98)
                } else {
                    Color::new(0.22, 0.10, 0.11, 0.85)
                };
                draw_rectangle(bx + col as f32 * 3.0, y + row as f32 * 3.0, 3.0, 3.0, color);
            }
        }
        if filled {
            draw_rectangle(bx + 6.0, y + 6.0, 3.0, 3.0, Color::new(0.98, 0.56, 0.52, 0.9));
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn recipe_for(item: Item) -> &'static crafting::Recipe {
        crafting::RECIPES.iter().find(|r| r.out.0 == item).unwrap()
    }

    #[test]
    fn auto_fill_lays_out_recipe_and_takes_items() {
        let mut inv = Inv::new();
        inv.add(Item::Block(Block::Planks), 10);
        inv.add(Item::Stick, 4);

        let table = recipe_for(Item::Block(Block::CraftingTable));
        assert!(inv.auto_fill(table).is_ok());
        assert_eq!(inv.craft[0].item, Some(Item::Block(Block::Planks)));
        assert_eq!(inv.craft[1].item, Some(Item::Block(Block::Planks)));
        assert_eq!(inv.craft[2].item, Some(Item::Block(Block::Planks)));
        assert_eq!(inv.craft[3].item, Some(Item::Block(Block::Planks)));
        assert_eq!(inv.item_count(Item::Block(Block::Planks)), 6);
        assert_eq!(inv.craft_result().map(|s| s.item), Some(Some(Item::Block(Block::CraftingTable))));
    }

    #[test]
    fn auto_fill_reports_missing_items() {
        let mut inv = Inv::new();
        inv.add(Item::Block(Block::Cobble), 3);
        let furnace = recipe_for(Item::Block(Block::Furnace));
        assert_eq!(inv.auto_fill(furnace), Err(vec![(Item::Block(Block::Cobble), 8)]));
        // Nothing moved on failure.
        assert_eq!(inv.item_count(Item::Block(Block::Cobble)), 3);
        assert!(inv.craft.iter().all(|s| s.is_empty()));
    }

    #[test]
    fn auto_fill_returns_previous_grid_to_inventory() {
        let mut inv = Inv::new();
        inv.add(Item::Block(Block::Planks), 8);
        inv.craft[0] = Stack::new(Item::Block(Block::Planks), 1);
        inv.craft[1] = Stack::new(Item::Block(Block::Planks), 1);
        let table = recipe_for(Item::Block(Block::CraftingTable));
        assert!(inv.auto_fill(table).is_ok());
        assert_eq!(inv.item_count(Item::Block(Block::Planks)), 6);
        assert_eq!(inv.craft.iter().filter(|s| !s.is_empty()).count(), 4);
    }
}
