// Procedurally generated 16x16 tile atlas, Minecraft-inspired but 100% original pixels.
use macroquad::prelude::*;

pub const TILE: u32 = 16;
const GRID: u32 = 16;
pub const ATLAS_TILES: u32 = GRID;
pub const ATLAS_PX: u32 = TILE * GRID;

// Tile indices inside the atlas (column, row) -> linear index.
pub const GRASS_TOP: u8 = 0;
pub const GRASS_SIDE: u8 = 1;
pub const DIRT: u8 = 2;
pub const STONE: u8 = 3;
pub const COBBLE: u8 = 4;
pub const SAND: u8 = 5;
pub const LOG_SIDE: u8 = 6;
pub const LOG_TOP: u8 = 7;
pub const LEAVES: u8 = 8;
pub const PLANKS: u8 = 9;
pub const WATER: u8 = 10;
pub const GLASS: u8 = 11;
pub const BEDROCK: u8 = 12;
pub const COAL_ORE: u8 = 13;
pub const IRON_ORE: u8 = 14;
pub const SNOW: u8 = 15;
pub const GOLD_ORE: u8 = 16;
pub const DIAMOND_ORE: u8 = 17;
pub const TABLE_TOP: u8 = 18;
pub const TABLE_SIDE: u8 = 19;
pub const FURNACE_TOP: u8 = 20;
pub const FURNACE_SIDE: u8 = 21;
pub const STONE_BRICKS: u8 = 22;

// Item icons (materials are drawn as flat sprites, not blocks).
pub const ITEM_COAL: u8 = 23;
pub const ITEM_RAW_IRON: u8 = 24;
pub const ITEM_RAW_GOLD: u8 = 25;
pub const ITEM_DIAMOND: u8 = 26;
pub const ITEM_IRON_INGOT: u8 = 27;
pub const ITEM_GOLD_INGOT: u8 = 28;
pub const ITEM_STICK: u8 = 29;
pub const ITEM_APPLE: u8 = 30;

/// UV rectangle of a tile: (u0, v0, u1, v1).
pub fn tile_uv(tile: u8) -> (f32, f32, f32, f32) {
    let tx = (tile as u32 % GRID) as f32;
    let ty = (tile as u32 / GRID) as f32;
    let step = 1.0 / GRID as f32;
    // Half-texel inset keeps nearest-neighbour sampling inside the tile.
    let inset = 0.5 / ATLAS_PX as f32;
    (
        tx * step + inset,
        ty * step + inset,
        (tx + 1.0) * step - inset,
        (ty + 1.0) * step - inset,
    )
}

struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed | 1)
    }
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    fn int(&mut self, a: i32, b: i32) -> i32 {
        a + (self.next() % ((b - a + 1) as u32)) as i32
    }
}

struct Tile<'a> {
    img: &'a mut Image,
    ox: i32,
    oy: i32,
}

impl Tile<'_> {
    fn set(&mut self, x: i32, y: i32, c: Color) {
        if !(0..TILE as i32).contains(&x) || !(0..TILE as i32).contains(&y) {
            return;
        }
        self.img
            .set_pixel((self.ox + x) as u32, (self.oy + y) as u32, c);
    }

    fn fill(&mut self, c: Color) {
        for y in 0..TILE as i32 {
            for x in 0..TILE as i32 {
                self.set(x, y, c);
            }
        }
    }

    /// Base color with per-pixel brightness noise.
    fn speckle(&mut self, base: (u8, u8, u8), spread: i32, alpha: u8, rng: &mut Rng) {
        for y in 0..TILE as i32 {
            for x in 0..TILE as i32 {
                let d = rng.int(-spread, spread);
                let c = Color::from_rgba(
                    (base.0 as i32 + d).clamp(0, 255) as u8,
                    (base.1 as i32 + d).clamp(0, 255) as u8,
                    (base.2 as i32 + d).clamp(0, 255) as u8,
                    alpha,
                );
                self.set(x, y, c);
            }
        }
    }

    fn blob(&mut self, c: Color, w: i32, h: i32, rng: &mut Rng) {
        let x0 = rng.int(0, TILE as i32 - w);
        let y0 = rng.int(0, TILE as i32 - h);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.set(x, y, c);
            }
        }
    }
}

fn painter(img: &mut Image, tile: u8) -> Tile<'_> {
    Tile {
        img,
        ox: (tile as u32 % GRID) as i32 * TILE as i32,
        oy: (tile as u32 / GRID) as i32 * TILE as i32,
    }
}

pub fn build_atlas() -> Texture2D {
    let mut img = Image::gen_image_color(ATLAS_PX as u16, ATLAS_PX as u16, Color::new(0.0, 0.0, 0.0, 0.0));

    // grass_top
    {
        let mut t = painter(&mut img, GRASS_TOP);
        let mut rng = Rng::new(11);
        t.speckle((104, 162, 58), 9, 255, &mut rng);
        for _ in 0..14 {
            let x = rng.int(0, 15);
            let y = rng.int(0, 15);
            let g = rng.int(-14, 10);
            t.set(
                x,
                y,
                Color::from_rgba(
                    (104 + g).clamp(0, 255) as u8,
                    (162 + g).clamp(0, 255) as u8,
                    (58 + g / 2).clamp(0, 255) as u8,
                    255,
                ),
            );
        }
    }

    // grass_side: dirt with a ragged grass cap
    {
        let mut t = painter(&mut img, GRASS_SIDE);
        let mut rng = Rng::new(12);
        t.speckle((134, 96, 67), 14, 255, &mut rng);
        for x in 0..16 {
            let depth = rng.int(3, 5);
            for y in 0..depth {
                let d = rng.int(-14, 14);
                t.set(
                    x,
                    y,
                    Color::from_rgba(
                        (100 + d).clamp(0, 255) as u8,
                        (158 + d).clamp(0, 255) as u8,
                        (56 + d / 2).clamp(0, 255) as u8,
                        255,
                    ),
                );
            }
        }
    }

    // dirt
    {
        let mut t = painter(&mut img, DIRT);
        let mut rng = Rng::new(13);
        t.speckle((134, 96, 67), 16, 255, &mut rng);
        for _ in 0..14 {
            t.blob(Color::from_rgba(112, 78, 54, 255), 2, 1, &mut rng);
        }
    }

    // stone
    {
        let mut t = painter(&mut img, STONE);
        let mut rng = Rng::new(14);
        t.speckle((128, 128, 128), 12, 255, &mut rng);
        for _ in 0..8 {
            t.blob(Color::from_rgba(112, 112, 112, 255), 3, 2, &mut rng);
        }
    }

    // cobblestone
    {
        let mut t = painter(&mut img, COBBLE);
        let mut rng = Rng::new(15);
        t.speckle((122, 122, 122), 10, 255, &mut rng);
        for _ in 0..12 {
            let w = rng.int(3, 5);
            let h = rng.int(2, 4);
            t.blob(Color::from_rgba(146, 146, 146, 255), w, h, &mut rng);
            let x0 = rng.int(0, 13);
            let y0 = rng.int(0, 13);
            for i in 0..w + 1 {
                t.set(x0 + i, y0 + h, Color::from_rgba(88, 88, 88, 255));
            }
        }
    }

    // sand
    {
        let mut t = painter(&mut img, SAND);
        let mut rng = Rng::new(16);
        t.speckle((219, 207, 160), 10, 255, &mut rng);
    }

    // log_side
    {
        let mut t = painter(&mut img, LOG_SIDE);
        let mut rng = Rng::new(17);
        t.speckle((109, 82, 50), 8, 255, &mut rng);
        for x in 0..16 {
            if x % 4 == 1 {
                for y in 0..16 {
                    let d = rng.int(-10, 10);
                    t.set(
                        x,
                        y,
                        Color::from_rgba(
                            (86 + d).clamp(0, 255) as u8,
                            (64 + d).clamp(0, 255) as u8,
                            (38 + d).clamp(0, 255) as u8,
                            255,
                        ),
                    );
                }
            }
        }
    }

    // log_top: growth rings
    {
        let mut t = painter(&mut img, LOG_TOP);
        let mut rng = Rng::new(18);
        for y in 0..16i32 {
            for x in 0..16i32 {
                let dx = x as f32 - 7.5;
                let dy = y as f32 - 7.5;
                let r = (dx * dx + dy * dy).sqrt();
                let ring = ((r * 1.9).sin() * 0.5 + 0.5) * 18.0;
                let d = rng.int(-5, 5) as f32;
                t.set(
                    x,
                    y,
                    Color::from_rgba(
                        (166.0 - ring + d).clamp(0.0, 255.0) as u8,
                        (134.0 - ring + d).clamp(0.0, 255.0) as u8,
                        (86.0 - ring * 0.7 + d).clamp(0.0, 255.0) as u8,
                        255,
                    ),
                );
            }
        }
    }

    // leaves
    {
        let mut t = painter(&mut img, LEAVES);
        let mut rng = Rng::new(19);
        for y in 0..16i32 {
            for x in 0..16i32 {
                let d = rng.int(-13, 13);
                t.set(
                    x,
                    y,
                    Color::from_rgba(
                        (64 + d).clamp(0, 255) as u8,
                        (116 + d).clamp(0, 255) as u8,
                        (46 + d / 2).clamp(0, 255) as u8,
                        255,
                    ),
                );
            }
        }
        for _ in 0..16 {
            t.blob(Color::from_rgba(48, 90, 36, 255), 2, 2, &mut rng);
        }
    }

    // planks
    {
        let mut t = painter(&mut img, PLANKS);
        let mut rng = Rng::new(20);
        t.speckle((162, 130, 78), 9, 255, &mut rng);
        for y in 0..16 {
            if y % 4 == 0 {
                for x in 0..16 {
                    t.set(x, y, Color::from_rgba(122, 94, 54, 255));
                }
            }
        }
        for (j, y) in [1, 5, 9, 13].iter().enumerate() {
            let x = if j % 2 == 0 { 3 } else { 11 };
            for dy in 0..3 {
                t.set(x, y + dy, Color::from_rgba(130, 101, 58, 255));
            }
        }
    }

    // water
    {
        let mut t = painter(&mut img, WATER);
        let mut rng = Rng::new(21);
        t.speckle((54, 108, 196), 10, 205, &mut rng);
        for _ in 0..10 {
            t.blob(Color::from_rgba(92, 150, 224, 205), 3, 1, &mut rng);
        }
    }

    // glass: only the 1px frame is opaque, the inside is fully transparent.
    // The pane mesh is drawn in its own alpha-test pass, so the clear middle
    // neither blends into grey haze nor hides the blocks behind it.
    {
        let mut t = painter(&mut img, GLASS);
        t.fill(Color::new(0.0, 0.0, 0.0, 0.0));
        let frame = Color::from_rgba(222, 238, 244, 255);
        let edge = Color::from_rgba(184, 206, 216, 255);
        for i in 0..16 {
            t.set(i, 0, frame);
            t.set(0, i, frame);
            t.set(i, 15, edge);
            t.set(15, i, edge);
        }
    }

    // bedrock
    {
        let mut t = painter(&mut img, BEDROCK);
        let mut rng = Rng::new(23);
        t.speckle((72, 72, 72), 18, 255, &mut rng);
        for _ in 0..22 {
            t.blob(Color::from_rgba(38, 38, 38, 255), 2, 2, &mut rng);
        }
    }

    // coal ore
    {
        let mut t = painter(&mut img, COAL_ORE);
        let mut rng = Rng::new(24);
        t.speckle((128, 128, 128), 12, 255, &mut rng);
        for _ in 0..7 {
            t.blob(Color::from_rgba(32, 32, 32, 255), 2, 2, &mut rng);
            t.blob(Color::from_rgba(52, 52, 52, 255), 1, 1, &mut rng);
        }
    }

    // iron ore
    {
        let mut t = painter(&mut img, IRON_ORE);
        let mut rng = Rng::new(25);
        t.speckle((128, 128, 128), 12, 255, &mut rng);
        for _ in 0..7 {
            t.blob(Color::from_rgba(202, 166, 130, 255), 2, 2, &mut rng);
            t.blob(Color::from_rgba(232, 208, 178, 255), 1, 1, &mut rng);
        }
    }

    // snow
    {
        let mut t = painter(&mut img, SNOW);
        let mut rng = Rng::new(26);
        t.speckle((242, 246, 250), 6, 255, &mut rng);
    }

    // gold ore
    {
        let mut t = painter(&mut img, GOLD_ORE);
        let mut rng = Rng::new(31);
        t.speckle((128, 128, 128), 12, 255, &mut rng);
        for _ in 0..7 {
            t.blob(Color::from_rgba(226, 178, 46, 255), 2, 2, &mut rng);
            t.blob(Color::from_rgba(255, 232, 138, 255), 1, 1, &mut rng);
        }
    }

    // diamond ore
    {
        let mut t = painter(&mut img, DIAMOND_ORE);
        let mut rng = Rng::new(32);
        t.speckle((128, 128, 128), 12, 255, &mut rng);
        for _ in 0..7 {
            t.blob(Color::from_rgba(84, 214, 208, 255), 2, 2, &mut rng);
            t.blob(Color::from_rgba(198, 252, 248, 255), 1, 1, &mut rng);
        }
    }

    // stone bricks
    {
        let mut t = painter(&mut img, STONE_BRICKS);
        let mut rng = Rng::new(33);
        t.speckle((124, 124, 124), 9, 255, &mut rng);
        let mortar = Color::from_rgba(84, 84, 84, 255);
        for i in 0..16 {
            t.set(i, 7, mortar);
            t.set(i, 15, mortar);
        }
        for y in 0..7 {
            t.set(7, y, mortar);
        }
        for y in 8..15 {
            t.set(11, y, mortar);
        }
    }

    // crafting table top: planks with a grid and a dark border
    {
        let mut t = painter(&mut img, TABLE_TOP);
        let mut rng = Rng::new(34);
        t.speckle((168, 134, 82), 8, 255, &mut rng);
        let dark = Color::from_rgba(96, 70, 40, 255);
        for i in 0..16 {
            t.set(i, 0, dark);
            t.set(i, 15, dark);
            t.set(0, i, dark);
            t.set(15, i, dark);
        }
        for i in 3..13 {
            t.set(i, 7, dark);
            t.set(7, i, dark);
        }
        for i in 3..13 {
            t.set(i, 8, Color::from_rgba(124, 94, 56, 255));
            t.set(8, i, Color::from_rgba(124, 94, 56, 255));
        }
    }

    // crafting table side: planks with a dark working surface and tools
    {
        let mut t = painter(&mut img, TABLE_SIDE);
        let mut rng = Rng::new(35);
        t.speckle((150, 118, 72), 8, 255, &mut rng);
        for i in 0..16 {
            t.set(i, 4, Color::from_rgba(96, 70, 40, 255));
            t.set(i, 11, Color::from_rgba(96, 70, 40, 255));
        }
        // simple tool silhouettes
        for i in 0..5 {
            t.set(3 + i, 7, Color::from_rgba(74, 74, 78, 255));
        }
        for i in 0..3 {
            t.set(9, 6 + i, Color::from_rgba(122, 82, 44, 255));
        }
        t.set(11, 8, Color::from_rgba(74, 74, 78, 255));
    }

    // furnace top
    {
        let mut t = painter(&mut img, FURNACE_TOP);
        let mut rng = Rng::new(36);
        t.speckle((116, 116, 116), 10, 255, &mut rng);
        for i in 2..14 {
            t.set(i, 2, Color::from_rgba(92, 92, 92, 255));
            t.set(i, 13, Color::from_rgba(92, 92, 92, 255));
            t.set(2, i, Color::from_rgba(92, 92, 92, 255));
            t.set(13, i, Color::from_rgba(92, 92, 92, 255));
        }
    }

    // furnace side: cobble with a dark mouth and embers
    {
        let mut t = painter(&mut img, FURNACE_SIDE);
        let mut rng = Rng::new(37);
        t.speckle((112, 112, 112), 10, 255, &mut rng);
        for y in 6..14 {
            for x in 4..12 {
                t.set(x, y, Color::from_rgba(38, 36, 36, 255));
            }
        }
        for x in 5..11 {
            t.set(x, 13, Color::from_rgba(226, 126, 42, 255));
        }
        t.set(6, 12, Color::from_rgba(248, 186, 70, 255));
        t.set(9, 12, Color::from_rgba(248, 186, 70, 255));
        for x in 0..16 {
            t.set(x, 3, Color::from_rgba(88, 88, 88, 255));
        }
    }

    // material icons: transparent background with a small sprite
    {
        let mut t = painter(&mut img, ITEM_COAL);
        let mut rng = Rng::new(38);
        for y in 3..13 {
            for x in 3..13 {
                let dx = x as f32 - 7.5;
                let dy = y as f32 - 7.5;
                if dx * dx + dy * dy * 1.2 < 24.0 {
                    let d = rng.int(-10, 10);
                    t.set(
                        x,
                        y,
                        Color::from_rgba(
                            (48 + d).clamp(0, 255) as u8,
                            (48 + d).clamp(0, 255) as u8,
                            (52 + d).clamp(0, 255) as u8,
                            255,
                        ),
                    );
                }
            }
        }
        t.set(5, 5, Color::from_rgba(96, 96, 104, 255));
        t.set(9, 8, Color::from_rgba(80, 80, 88, 255));
    }
    for (tile, base, light, seed) in [
        (ITEM_RAW_IRON, (176, 148, 124), (222, 200, 178), 39u32),
        (ITEM_RAW_GOLD, (200, 158, 46), (250, 220, 120), 40),
    ] {
        let mut t = painter(&mut img, tile);
        let mut rng = Rng::new(seed);
        for y in 4..13 {
            for x in 3..13 {
                let edge = x == 3 || x == 12 || y == 4 || y == 12;
                if rng.int(0, 9) > 1 {
                    t.set(
                        x,
                        y,
                        Color::from_rgba(
                            (base.0 + if edge { -28 } else { 0 }).clamp(0, 255) as u8,
                            (base.1 + if edge { -24 } else { 0 }).clamp(0, 255) as u8,
                            (base.2 + if edge { -20 } else { 0 }).clamp(0, 255) as u8,
                            255,
                        ),
                    );
                }
            }
        }
        t.set(5, 6, Color::from_rgba(light.0, light.1, light.2, 255));
        t.set(9, 10, Color::from_rgba(light.0, light.1, light.2, 255));
    }
    {
        let mut t = painter(&mut img, ITEM_DIAMOND);
        let gem = Color::from_rgba(96, 226, 220, 255);
        let shine = Color::from_rgba(206, 255, 252, 255);
        for y in 0..16i32 {
            let span = 7 - (y as f32 - 6.0).abs() as i32;
            for x in 0..16i32 {
                if (x as f32 - 7.5).abs() <= span as f32 {
                    t.set(x, y, gem);
                }
            }
        }
        t.set(5, 5, shine);
        t.set(6, 5, shine);
        t.set(5, 9, Color::from_rgba(54, 174, 176, 255));
    }
    for (tile, base, light, seed) in [
        (ITEM_IRON_INGOT, (198, 198, 206), (238, 238, 244), 41u32),
        (ITEM_GOLD_INGOT, (232, 188, 58), (255, 232, 140), 42),
    ] {
        let mut t = painter(&mut img, tile);
        let mut rng = Rng::new(seed);
        for y in 5..12 {
            let inset = if y < 8 { 2 } else { 0 };
            for x in (3 + inset)..(13 - inset) {
                let edge = y == 5 || y == 11 || x == 3 + inset || x == 12 - inset;
                let d = rng.int(-6, 6);
                t.set(
                    x,
                    y,
                    Color::from_rgba(
                        (base.0 + if edge { -34 } else { d }).clamp(0, 255) as u8,
                        (base.1 + if edge { -30 } else { d }).clamp(0, 255) as u8,
                        (base.2 + if edge { -26 } else { d }).clamp(0, 255) as u8,
                        255,
                    ),
                );
            }
        }
        t.set(5, 6, Color::from_rgba(light.0, light.1, light.2, 255));
        t.set(6, 6, Color::from_rgba(light.0, light.1, light.2, 255));
    }
    {
        let mut t = painter(&mut img, ITEM_STICK);
        let wood = Color::from_rgba(146, 108, 62, 255);
        let dark = Color::from_rgba(110, 78, 44, 255);
        for i in 0..11 {
            t.set(4 + i, 11 - i, wood);
            t.set(5 + i, 11 - i, dark);
        }
    }
    {
        let mut t = painter(&mut img, ITEM_APPLE);
        let mut rng = Rng::new(43);
        for y in 4..14 {
            for x in 2..14 {
                let dx = x as f32 - 7.5;
                let dy = y as f32 - 9.0;
                if dx * dx + dy * dy < 20.0 {
                    let d = rng.int(-12, 8);
                    t.set(
                        x,
                        y,
                        Color::from_rgba(
                            (198 + d).clamp(0, 255) as u8,
                            (52 + d / 2).clamp(0, 255) as u8,
                            (48 + d / 2).clamp(0, 255) as u8,
                            255,
                        ),
                    );
                }
            }
        }
        t.set(7, 3, Color::from_rgba(94, 66, 38, 255));
        t.set(8, 2, Color::from_rgba(94, 66, 38, 255));
        t.set(9, 2, Color::from_rgba(84, 160, 62, 255));
        t.set(10, 2, Color::from_rgba(84, 160, 62, 255));
        t.set(4, 7, Color::from_rgba(246, 190, 180, 255));
    }

    let tex = Texture2D::from_image(&img);
    tex.set_filter(FilterMode::Nearest);
    tex
}
