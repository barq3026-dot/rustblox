// Release builds must not spawn a console window: it steals keyboard focus and
// the mouse gets captured by the wrong window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod atlas;
#[allow(dead_code)]
mod auth;
mod blocks;
mod crafting;
mod inventory;
mod items;
mod menu;
mod player;
mod world;

use macroquad::prelude::*;
use std::io::Write;

use blocks::Block;
use crafting::Station;
use inventory::{draw_hearts, draw_hotbar, draw_ui, Inv};
use items::{Item, Stack};
use player::{raycast, Player};
use world::{Mode, World};

const REACH: f32 = 6.0;
const SKY: Color = Color::new(0.55, 0.74, 0.94, 1.0);

fn window_conf() -> Conf {
    Conf {
        window_title: format!("Rustblox {}", env!("CARGO_PKG_VERSION")),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shot_path = args
        .iter()
        .position(|a| a == "--shot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let shot_mode = shot_path.is_some();
    let log_path = args
        .iter()
        .position(|a| a == "--input-log")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let cam_arg = args
        .iter()
        .position(|a| a == "--cam")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| {
            let v: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
            if v.len() == 5 {
                Some((v[0], v[1], v[2], v[3], v[4]))
            } else {
                None
            }
        });

    let seed_arg = args
        .iter()
        .position(|a| a == "--seed")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1337);

    println!("Building atlas...");
    let atlas = atlas::build_atlas();
    println!("Generating preview world...");
    let mut world = World::generate(seed_arg, atlas.clone(), world::Size::Small);
    let mut player = Player::new(world.spawn_point() + vec3(0.0, 0.2, 0.0));

    if shot_mode {
        player.pos.y += 22.0;
        player.yaw = 0.9;
        player.pitch = -0.5;
        player.flying = true;
    }
    if let Some((x, y, z, yaw, pitch)) = cam_arg {
        player.pos = vec3(x, y, z);
        player.yaw = yaw;
        player.pitch = pitch;
        player.flying = true;
    }
    if args.iter().any(|a| a == "--glass") {
        build_glass_demo(&mut world, &mut player);
    }

    let mut inv = Inv::new();
    let mut mode = Mode::Creative;
    let mut playing = false;
    let mut frames: u32 = 0;
    let no_grab = args.iter().any(|a| a == "--no-grab");
    let autoplay = args.iter().any(|a| a == "--play");
    let mut log_file = log_path.as_ref().and_then(|p| std::fs::File::create(p).ok());

    let mut settings = menu::Settings::load();
    if settings.fullscreen {
        macroquad::window::set_fullscreen(true);
    }
    let mut menu = menu::Menu::new();

    enum Pending {
        Create {
            seed: u32,
            size: world::Size,
            name: String,
            mode: Mode,
        },
        Load(String),
        Recreate {
            path: String,
            seed: u32,
            size: world::Size,
            name: String,
            mode: Mode,
        },
    }
    let mut pending: Option<Pending> = None;
    let mut save_path: Option<String> = None;
    let mut world_name = String::new();
    let mut dirty = false;
    let mut since_save = 0.0f32;
    let mut breaking: Option<(i32, i32, i32)> = None;
    let mut break_progress = 0.0f32;
    let mut death_timer = 0.0f32;
    let mut regen_timer = 0.0f32;

    if autoplay {
        playing = true;
        let survival_test = args.iter().any(|a| a == "--survival");
        if survival_test {
            mode = Mode::Survival;
            player.survival = true;
        } else {
            inv.fill_creative();
        }
        player.reset_look();
        if !no_grab {
            let _ = set_cursor_grab(true);
            show_mouse(false);
        }
    } else if !shot_mode {
        let _ = set_cursor_grab(false);
        show_mouse(true);
    }
    if shot_mode {
        inv.fill_creative();
    }
    let _ = std::fs::create_dir_all(world::WORLDS_DIR);

    // Debug scene for `--shot ... --book`: survival inventory with the recipe
    // book open and a few materials to try the auto-fill on.
    if args.iter().any(|a| a == "--book") {
        mode = Mode::Survival;
        player.survival = true;
        inv = Inv::new();
        inv.open = true;
        inv.book_open = true;
        inv.station = Station::Table;
        inv.slots[0] = Stack::new(Item::Block(Block::Planks), 24);
        inv.slots[1] = Stack::new(Item::Stick, 8);
        inv.slots[2] = Stack::new(Item::Block(Block::Cobble), 12);
        inv.slots[3] = Stack::new(Item::Block(Block::Log), 3);
        inv.slots[4] = Stack::new(Item::Coal, 5);
        inv.slots[5] = Stack::new(Item::RawIron, 2);
        inv.slots[6] = Stack::new(Item::Block(Block::Sand), 4);
    }

    loop {
        let dt = get_frame_time().min(0.05);
        let mut pause_guard = false;

        if let Some(job) = pending.take() {
            clear_background(Color::new(0.02, 0.03, 0.05, 1.0));
            let msg = match &job {
                Pending::Create { .. } => "Generating world...",
                Pending::Recreate { .. } => "Regenerating world...",
                Pending::Load(_) => "Loading world...",
            };
            let m = measure_text(msg, None, 36, 1.0);
            draw_text(
                msg,
                (screen_width() - m.width) * 0.5,
                screen_height() * 0.5,
                36.0,
                WHITE,
            );
            next_frame().await;

            let result = match job {
                Pending::Create {
                    seed,
                    size,
                    name,
                    mode: new_mode,
                } => {
                    let w = World::generate(seed, atlas.clone(), size);
                    let p = new_player(w.spawn_point(), new_mode);
                    let mut fresh = Inv::new();
                    if new_mode == Mode::Creative {
                        fresh.fill_creative();
                    }
                    let path = unique_world_path();
                    let st = build_state(&p, &fresh, &name, new_mode);
                    let _ = w.save(&path, &st);
                    Some((w, p, fresh, new_mode, path, name))
                }
                Pending::Recreate {
                    path,
                    seed,
                    size,
                    name,
                    mode: new_mode,
                } => {
                    let w = World::generate(seed, atlas.clone(), size);
                    let p = new_player(w.spawn_point(), new_mode);
                    let mut fresh = Inv::new();
                    if new_mode == Mode::Creative {
                        fresh.fill_creative();
                    }
                    let st = build_state(&p, &fresh, &name, new_mode);
                    let _ = w.save(&path, &st);
                    Some((w, p, fresh, new_mode, path, name))
                }
                Pending::Load(path) => match World::load(&path, atlas.clone()) {
                    Ok((w, st)) => {
                        let mut p = Player::new(st.pos);
                        p.yaw = st.yaw;
                        p.pitch = st.pitch;
                        p.survival = st.mode == Mode::Survival;
                        p.health = if st.health <= 0.0 {
                            player::MAX_HEALTH
                        } else {
                            st.health
                        };
                        let mut loaded = Inv::new();
                        loaded.slots = st.inventory;
                        loaded.selected = st.selected;
                        if st.mode == Mode::Creative && loaded.slots.iter().all(|s| s.is_empty()) {
                            loaded.fill_creative();
                        }
                        Some((w, p, loaded, st.mode, path, st.name))
                    }
                    Err(_) => None,
                },
            };

            if let Some((w, p, loaded, new_mode, path, name)) = result {
                world = w;
                player = p;
                inv = loaded;
                mode = new_mode;
                save_path = Some(path);
                world_name = name;
                playing = true;
                player.reset_look();
                dirty = false;
                since_save = 0.0;
                death_timer = 0.0;
                if !no_grab {
                    let _ = set_cursor_grab(true);
                    show_mouse(false);
                }
            }
            continue;
        }

        let survival = mode == Mode::Survival;

        // ---- input: pause, inventory, death ----
        if playing && !shot_mode {
            if inv.open {
                if is_key_pressed(KeyCode::Escape) {
                    if inv.book_open {
                        inv.book_open = false;
                    } else {
                        close_inventory(&mut inv, &mut player, no_grab);
                        dirty = true;
                    }
                } else if is_key_pressed(KeyCode::E) {
                    close_inventory(&mut inv, &mut player, no_grab);
                    dirty = true;
                }
            } else if player.dead {
                // Controls are ignored until the respawn timer runs out.
            } else if is_key_pressed(KeyCode::Escape) {
                flush_save(&world, &player, &inv, &world_name, &save_path, mode, &mut dirty);
                playing = false;
                let _ = set_cursor_grab(false);
                show_mouse(true);
                menu.open(menu::Page::Pause);
                pause_guard = true;
            } else if is_key_pressed(KeyCode::E) {
                inv.open = true;
                inv.station = Station::Hand;
                if !no_grab {
                    let _ = set_cursor_grab(false);
                }
                show_mouse(true);
            }
        }

        if !playing && !shot_mode {
            menu.tick(dt);
        }

        if playing && !shot_mode && !inv.open && !player.dead {
            player.update(&world, dt, settings.sensitivity);

            for i in 0..9 {
                let key = match i {
                    0 => KeyCode::Key1,
                    1 => KeyCode::Key2,
                    2 => KeyCode::Key3,
                    3 => KeyCode::Key4,
                    4 => KeyCode::Key5,
                    5 => KeyCode::Key6,
                    6 => KeyCode::Key7,
                    7 => KeyCode::Key8,
                    _ => KeyCode::Key9,
                };
                if is_key_pressed(key) {
                    inv.selected = i;
                }
            }
            let (_, wheel) = mouse_wheel();
            if wheel > 0.0 {
                inv.selected = (inv.selected + 8) % 9;
            } else if wheel < 0.0 {
                inv.selected = (inv.selected + 1) % 9;
            }
        }

        let eye = player.eye();
        let mut look = player.look();
        let menu_active = !playing && !shot_mode;
        if menu_active && menu.page == menu::Page::Main {
            // Slow camera pan behind the main menu.
            let yaw = player.yaw + menu.time() * 0.06;
            let cp = player.pitch.cos();
            look = vec3(yaw.sin() * cp, player.pitch.sin(), -yaw.cos() * cp);
        }
        let target = raycast(&world, eye, look, REACH);

        // ---- block interaction ----
        let can_act = playing && !shot_mode && !inv.open && !player.dead;
        if can_act {
            if let Some((hit, normal)) = target {
                let block = world.get(hit.0, hit.1, hit.2);
                let unbreakable = matches!(block, Block::Bedrock | Block::Water | Block::Air);

                if is_mouse_button_down(MouseButton::Left) && !unbreakable {
                    if !survival {
                        if is_mouse_button_pressed(MouseButton::Left)
                            && break_block(&mut world, &mut inv, hit, false)
                        {
                            dirty = true;
                        }
                    } else {
                        if breaking != Some(hit) {
                            breaking = Some(hit);
                            break_progress = 0.0;
                        }
                        break_progress += dt;
                        if break_progress >= block.hardness() {
                            if break_block(&mut world, &mut inv, hit, true) {
                                dirty = true;
                            }
                            breaking = None;
                            break_progress = 0.0;
                        }
                    }
                } else {
                    breaking = None;
                    break_progress = 0.0;
                }

                // Middle click copies the block in creative mode.
                if !survival && is_mouse_button_pressed(MouseButton::Middle) {
                    if block != Block::Air && block != Block::Water {
                        inv.pick_block(block);
                    }
                }

                if is_mouse_button_pressed(MouseButton::Right) {
                    let held = inv.held();
                    if survival && held == Some(Item::Apple) {
                        player.health = (player.health + 4.0).min(player::MAX_HEALTH);
                        inv.consume_held();
                        dirty = true;
                    } else if block.is_station() && normal != (0, 0, 0) {
                        // Right-clicking a table or furnace opens the craft UI.
                        inv.station = if block == Block::CraftingTable {
                            Station::Table
                        } else {
                            Station::Furnace
                        };
                        inv.open = true;
                        if !no_grab {
                            let _ = set_cursor_grab(false);
                        }
                        show_mouse(true);
                    } else if normal != (0, 0, 0) {
                        let cell = (hit.0 + normal.0, hit.1 + normal.1, hit.2 + normal.2);
                        let current = world.get(cell.0, cell.1, cell.2);
                        let place = inv.held().and_then(|i| i.block());
                        if let Some(new_block) = place {
                            if (current == Block::Air || current == Block::Water)
                                && !player.intersects_cell(cell)
                            {
                                world.set(cell.0, cell.1, cell.2, new_block);
                                dirty = true;
                                if survival {
                                    inv.consume_held();
                                }
                            }
                        }
                    }
                }
            } else {
                breaking = None;
                break_progress = 0.0;
            }
        } else {
            breaking = None;
            break_progress = 0.0;
        }

        // ---- death and slow regeneration ----
        if playing && player.dead {
            death_timer += dt;
            if death_timer > 2.5 {
                let spawn = world.spawn_point() + vec3(0.0, 0.2, 0.0);
                player.respawn(spawn);
                death_timer = 0.0;
                dirty = true;
            }
        }
        if survival && playing && !player.dead && player.health < player::MAX_HEALTH {
            regen_timer += dt;
            if regen_timer > 6.0 {
                regen_timer = 0.0;
                player.health = (player.health + 1.0).min(player::MAX_HEALTH);
            }
        } else {
            regen_timer = 0.0;
        }

        // Autosave while playing.
        if playing && !shot_mode && dirty {
            since_save += dt;
            if since_save > 15.0 {
                flush_save(&world, &player, &inv, &world_name, &save_path, mode, &mut dirty);
                since_save = 0.0;
            }
        }

        // ---- 3D pass ----
        clear_background(SKY);
        let mut cam = Camera3D::default();
        cam.fovy = settings.fov.to_radians();
        cam.z_near = 0.05;
        cam.z_far = 500.0;
        cam.projection = Projection::Perspective;
        cam.position = eye;
        cam.target = eye + look;
        cam.up = vec3(0.0, 1.0, 0.0);
        set_camera(&cam);

        world.draw(eye, settings.view_distance);
        if can_act {
            if let Some((hit, _)) = target {
                draw_block_outline(hit, Color::new(0.05, 0.05, 0.05, 0.85));
            }
            if let Some(hit) = breaking {
                let strength = (break_progress / world.get(hit.0, hit.1, hit.2).hardness()).min(1.0);
                let c = Color::new(0.05, 0.05, 0.05, 0.55 + 0.4 * strength);
                draw_block_cracks(hit, strength, c);
            }
        }

        set_default_camera();

        // Underwater tint.
        if player.in_water {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.15, 0.35, 0.75, 0.35),
            );
        }

        if playing || shot_mode {
            draw_hud(
                &player,
                &atlas,
                &inv,
                target,
                &world,
                &world_name,
                mode,
                if breaking.is_some() {
                    Some(break_progress)
                } else {
                    None
                },
            );
        }

        if player.hurt_flash() > 0.0 && survival {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.7, 0.05, 0.05, 0.28),
            );
        }

        if inv.open {
            draw_ui(&mut inv, &atlas, !survival);
        }

        if player.dead && playing {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.30, 0.02, 0.02, 0.55),
            );
            let title = "YOU DIED";
            let m = measure_text(title, None, 64, 1.0);
            draw_text(
                title,
                (screen_width() - m.width) * 0.5,
                screen_height() * 0.45,
                64.0,
                Color::new(0.96, 0.82, 0.82, 1.0),
            );
            let sub = "Respawning...";
            let m = measure_text(sub, None, 24, 1.0);
            draw_text(
                sub,
                (screen_width() - m.width) * 0.5,
                screen_height() * 0.45 + 40.0,
                24.0,
                Color::new(0.9, 0.88, 0.88, 0.85),
            );
        }

        if menu_active {
            match menu.frame(&mut settings, pause_guard) {
                menu::Action::Resume => {
                    playing = true;
                    player.reset_look();
                    if !no_grab {
                        let _ = set_cursor_grab(true);
                        show_mouse(false);
                    }
                }
                menu::Action::LoadWorld(path) => pending = Some(Pending::Load(path)),
                menu::Action::CreateWorld {
                    seed,
                    size,
                    name,
                    mode: new_mode,
                } => pending = Some(Pending::Create {
                    seed,
                    size,
                    name,
                    mode: new_mode,
                }),
                menu::Action::Recreate {
                    path,
                    seed,
                    size,
                    name,
                    mode: new_mode,
                } => pending = Some(Pending::Recreate {
                    path,
                    seed,
                    size,
                    name,
                    mode: new_mode,
                }),
                menu::Action::QuitToMenu => {
                    flush_save(&world, &player, &inv, &world_name, &save_path, mode, &mut dirty);
                    menu.open(menu::Page::Main);
                }
                menu::Action::Quit => {
                    flush_save(&world, &player, &inv, &world_name, &save_path, mode, &mut dirty);
                    std::process::exit(0);
                }
                menu::Action::None => {}
            }
        }

        if let Some(f) = log_file.as_mut() {
            let mp = mouse_position();
            let d = mouse_delta_position();
            let _ = writeln!(
                f,
                "f={} playing={} mp=({:.1},{:.1}) d=({:.2},{:.2}) yaw={:.3} pitch={:.3} cursor={:?}",
                frames,
                playing,
                mp.0,
                mp.1,
                d.x,
                d.y,
                player.yaw,
                player.pitch,
                mouse_position()
            );
        }

        if let Some(path) = &shot_path {
            frames += 1;
            if frames >= 12 {
                let img = get_screen_data();
                img.export_png(path);
                println!("Saved screenshot to {path}");
                std::process::exit(0);
            }
        }

        next_frame().await;
    }
}

fn new_player(spawn: Vec3, mode: Mode) -> Player {
    let mut p = Player::new(spawn + vec3(0.0, 0.2, 0.0));
    p.survival = mode == Mode::Survival;
    p
}

/// Debug scene for the `--glass` screenshot flag: a glass wall with blocks
/// right behind it, so clear panes and depth ordering are easy to judge.
fn build_glass_demo(world: &mut World, player: &mut Player) {
    let c = world.width / 2;
    // Put the scene above the highest column in the area so nothing pokes through.
    let mut ground = 0;
    for x in (c - 4)..=(c + 4) {
        for z in (c - 6)..=(c + 8) {
            ground = ground.max(world.height_at(x, z));
        }
    }
    let base = ground.max(world::SEA_LEVEL) + 2;
    let top = base + 4;

    // Backdrop wall, three blocks behind the panes.
    for x in (c - 3)..=(c + 3) {
        for y in base..top {
            world.set(x, y, c - 3, Block::StoneBricks);
            world.set(x, y, c - 4, Block::DiamondOre);
        }
    }
    // Blocks floating between the glass and the backdrop.
    world.set(c - 2, base, c - 1, Block::Cobble);
    world.set(c + 2, base + 1, c - 1, Block::Planks);
    world.set(c + 1, base, c - 2, Block::GoldOre);
    world.set(c - 1, base + 2, c - 2, Block::CraftingTable);
    world.set(c, base + 1, c - 1, Block::Furnace);

    // Glass wall with a two-block doorway in the middle.
    for x in (c - 3)..=(c + 3) {
        for y in base..top {
            if x == c && y < base + 2 {
                continue;
            }
            world.set(x, y, c, Block::Glass);
        }
    }

    player.pos = vec3(c as f32 + 0.5, (base + 1) as f32, (c + 7) as f32 + 0.5);
    player.yaw = 0.0;
    player.pitch = -0.10;
    player.flying = true;
}

fn close_inventory(inv: &mut Inv, player: &mut Player, no_grab: bool) {
    inv.spill_craft();
    if !inv.cursor.is_empty() {
        let stack: Stack = std::mem::replace(&mut inv.cursor, Stack::EMPTY);
        if let Some(item) = stack.item {
            inv.add(item, stack.count);
        }
    }
    inv.station = Station::Hand;
    inv.open = false;
    inv.book_open = false;
    inv.book_note = None;
    if !no_grab {
        let _ = set_cursor_grab(true);
    }
    show_mouse(false);
    player.reset_look();
}

/// Removes a block and gives the survival player its drop.
fn break_block(world: &mut World, inv: &mut Inv, cell: (i32, i32, i32), survival: bool) -> bool {
    let block = world.get(cell.0, cell.1, cell.2);
    if matches!(block, Block::Air | Block::Bedrock | Block::Water) {
        return false;
    }
    world.set(cell.0, cell.1, cell.2, Block::Air);
    if survival {
        if let Some(item) = block.drop() {
            let left = inv.add(item, 1);
            if left > 0 {
                println!("Inventory full, lost {}", item.name());
            }
        }
    }
    true
}

fn build_state(player: &Player, inv: &Inv, name: &str, mode: Mode) -> world::SaveState {
    world::SaveState {
        name: name.to_string(),
        pos: player.pos,
        yaw: player.yaw,
        pitch: player.pitch,
        selected: inv.selected,
        mode,
        health: player.health,
        inventory: inv.slots,
    }
}

fn flush_save(
    world: &World,
    player: &Player,
    inv: &Inv,
    name: &str,
    path: &Option<String>,
    mode: Mode,
    dirty: &mut bool,
) {
    if !*dirty {
        return;
    }
    if let Some(path) = path.as_deref() {
        let st = build_state(player, inv, name, mode);
        let _ = world.save(path, &st);
        *dirty = false;
    }
}

fn unique_world_path() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{}/world_{}.rbw", world::WORLDS_DIR, millis)
}

fn draw_hud(
    player: &Player,
    atlas: &Texture2D,
    inv: &Inv,
    target: Option<((i32, i32, i32), (i32, i32, i32))>,
    world: &World,
    world_name: &str,
    mode: Mode,
    break_progress: Option<f32>,
) {
    let (sw, sh) = (screen_width(), screen_height());

    // Crosshair.
    let cx = sw * 0.5;
    let cy = sh * 0.5;
    let c = Color::new(1.0, 1.0, 1.0, 0.85);
    draw_line(cx - 9.0, cy, cx - 2.0, cy, 2.0, c);
    draw_line(cx + 2.0, cy, cx + 9.0, cy, 2.0, c);
    draw_line(cx, cy - 9.0, cx, cy - 2.0, 2.0, c);
    draw_line(cx, cy + 2.0, cx, cy + 9.0, 2.0, c);

    draw_hotbar(inv, atlas, mode == Mode::Creative);

    let held = inv
        .held()
        .map(|i| i.name())
        .unwrap_or("");
    if !held.is_empty() {
        let m = measure_text(held, None, 22, 1.0);
        draw_text(
            held,
            (sw - m.width) * 0.5,
            sh - 96.0,
            22.0,
            Color::new(1.0, 1.0, 1.0, 0.95),
        );
    }

    if mode == Mode::Survival {
        draw_hearts(player.health);
        if player.air < 15.0 {
            let bw = 120.0f32;
            let x = sw * 0.5 - bw * 0.5;
            let y = sh - 104.0;
            draw_rectangle(x, y, bw, 6.0, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_rectangle(
                x,
                y,
                bw * (player.air / 15.0),
                6.0,
                Color::new(0.40, 0.72, 1.0, 0.9),
            );
        }
    }

    let info = format!(
        "FPS {}\n{}\nseed {}\n{}\nXYZ {:.1} {:.1} {:.1}",
        get_fps(),
        if world_name.is_empty() { "-" } else { world_name },
        world.seed,
        mode.short(),
        player.pos.x,
        player.pos.y,
        player.pos.z
    );
    let mut y = 24.0;
    for line in info.lines() {
        draw_text(line, 12.0, y, 18.0, Color::new(1.0, 1.0, 1.0, 0.85));
        y += 20.0;
    }

    if let Some((hit, _)) = target {
        let b = world.get(hit.0, hit.1, hit.2);
        if b != Block::Air && b != Block::Water {
            let label = format!("{}  ({:.1}s)", b.name(), b.hardness());
            let m = measure_text(&label, None, 18, 1.0);
            draw_text(
                &label,
                sw * 0.5 - m.width * 0.5,
                sh * 0.5 + 34.0,
                18.0,
                Color::new(1.0, 1.0, 1.0, 0.8),
            );
        }
    }

    if let Some(p) = break_progress {
        let bw = 90.0f32;
        let x = sw * 0.5 - bw * 0.5;
        let y = sh * 0.5 + 48.0;
        draw_rectangle(x - 1.0, y - 1.0, bw + 2.0, 8.0, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_rectangle(
            x,
            y,
            bw * p.clamp(0.0, 1.0),
            6.0,
            Color::new(0.72, 0.92, 0.5, 0.95),
        );
    }
}

fn draw_block_outline(cell: (i32, i32, i32), color: Color) {
    let (x, y, z) = (cell.0 as f32, cell.1 as f32, cell.2 as f32);
    let e = 0.004;
    let (x0, x1) = (x - e, x + 1.0 + e);
    let (y0, y1) = (y - e, y + 1.0 + e);
    let (z0, z1) = (z - e, z + 1.0 + e);
    let p = [
        vec3(x0, y0, z0),
        vec3(x1, y0, z0),
        vec3(x1, y0, z1),
        vec3(x0, y0, z1),
        vec3(x0, y1, z0),
        vec3(x1, y1, z0),
        vec3(x1, y1, z1),
        vec3(x0, y1, z1),
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (a, b) in edges {
        draw_line_3d(p[a], p[b], color);
    }
}

/// Draws a shrinking wire box on the block being mined.
fn draw_block_cracks(cell: (i32, i32, i32), strength: f32, color: Color) {
    let (x, y, z) = (cell.0 as f32, cell.1 as f32, cell.2 as f32);
    let e = 0.006;
    let inset = 0.5 * (1.0 - strength.clamp(0.0, 1.0));
    let (x0, x1) = (x + inset - e, x + 1.0 - inset + e);
    let (y0, y1) = (y + inset - e, y + 1.0 - inset + e);
    let (z0, z1) = (z + inset - e, z + 1.0 - inset + e);
    let p = [
        vec3(x0, y0, z0),
        vec3(x1, y0, z0),
        vec3(x1, y0, z1),
        vec3(x0, y0, z1),
        vec3(x0, y1, z0),
        vec3(x1, y1, z0),
        vec3(x1, y1, z1),
        vec3(x0, y1, z1),
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (a, b) in edges {
        draw_line_3d(p[a], p[b], color);
    }
}
