// Chunked voxel world: terrain generation, block access and mesh building.
use std::cell::OnceCell;

use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;

use crate::atlas;
use crate::blocks::Block;
use crate::items::Stack;

pub const CHUNK: i32 = 16;
pub const HEIGHT: i32 = 64;
pub const SEA_LEVEL: i32 = 27;

/// Folder that holds one file per world.
pub const WORLDS_DIR: &str = "worlds";

/// Save file layout: magic, seed, chunk count, name, player state, RLE blocks.
const SAVE_MAGIC: &[u8; 4] = b"RBW3";
const SAVE_MAGIC_V2: &[u8; 4] = b"RBW2";
/// 36 slots, each stored as u16 item id plus u8 count.
const INV_BYTES: usize = 36 * 3;

/// Creative or survival rules.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Creative,
    Survival,
}

impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Mode::Creative => "Creative   fly, infinite blocks",
            Mode::Survival => "Survival   health, crafting, ores",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Mode::Creative => "Creative",
            Mode::Survival => "Survival",
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            Mode::Creative => 0,
            Mode::Survival => 1,
        }
    }

    fn from_u8(v: u8) -> Mode {
        match v {
            1 => Mode::Survival,
            _ => Mode::Creative,
        }
    }
}

/// Square world footprint, measured in chunks per side.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Small,
    Normal,
    Large,
}

impl Size {
    pub fn chunks(self) -> i32 {
        match self {
            Size::Small => 8,
            Size::Normal => 12,
            Size::Large => 16,
        }
    }

    pub fn blocks(self) -> i32 {
        self.chunks() * CHUNK
    }

    pub fn title(self) -> &'static str {
        match self {
            Size::Small => "Small   128 x 128",
            Size::Normal => "Normal   192 x 192",
            Size::Large => "Large   256 x 256",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Size::Small => "128x128",
            Size::Normal => "192x192",
            Size::Large => "256x256",
        }
    }

    pub fn from_chunks(n: i32) -> Self {
        match n {
            0..=9 => Size::Small,
            10..=13 => Size::Normal,
            _ => Size::Large,
        }
    }
}

/// Player state and metadata stored next to the blocks.
pub struct SaveState {
    pub name: String,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub selected: usize,
    pub mode: Mode,
    pub health: f32,
    pub inventory: [Stack; 36],
}

/// Short description of a saved world, read without loading the blocks.
#[derive(Clone)]
pub struct WorldInfo {
    pub path: String,
    pub name: String,
    pub seed: u32,
    pub size: Size,
    pub mode: Mode,
    /// Seconds since the Unix epoch, or 0 when unknown.
    pub modified: u64,
}

const DIRS: [(i32, i32, i32); 6] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
// Fake directional lighting baked into vertex colours (top faces are brightest).
const SHADE: [u8; 6] = [188, 188, 255, 112, 152, 152];

/// Alpha-tested glass shader: texels below 50% alpha are dropped entirely.
const GLASS_VERT: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
uniform mat4 Model;
uniform mat4 Projection;
varying lowp vec2 uv;
varying lowp vec4 color;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
}
"#;

const GLASS_FRAG: &str = r#"#version 100
precision mediump float;
uniform sampler2D Texture;
varying lowp vec2 uv;
varying lowp vec4 color;
void main() {
    vec4 tex = texture2D(Texture, uv);
    if (tex.a < 0.5) discard;
    gl_FragColor = tex * color;
}
"#;

pub struct Chunk {
    blocks: Vec<u8>,
    pub meshes: Vec<Mesh>,
    pub glass: Vec<Mesh>,
    pub water: Vec<Mesh>,
}

impl Chunk {
    fn new() -> Self {
        Self {
            blocks: vec![Block::Air.id(); (CHUNK * CHUNK * HEIGHT) as usize],
            meshes: Vec::new(),
            glass: Vec::new(),
            water: Vec::new(),
        }
    }
}

pub struct World {
    chunks: Vec<Chunk>,
    pub atlas: Texture2D,
    pub seed: u32,
    /// Width and depth of the world in blocks.
    pub width: i32,
    chunks_n: i32,
    heights: Vec<i32>,
    /// Alpha-test material for glass, created on the first draw.
    glass_material: OnceCell<Option<Material>>,
}

impl World {
    pub fn generate(seed: u32, atlas: Texture2D, size: Size) -> Self {
        let n = size.chunks();
        let width = size.blocks();
        let _ = size;
        let mut chunks = Vec::with_capacity((n * n) as usize);
        for _ in 0..n * n {
            chunks.push(Chunk::new());
        }
        let mut world = World {
            chunks,
            atlas,
            seed,
            width,
            chunks_n: n,
            heights: vec![0; (width * width) as usize],
            glass_material: OnceCell::new(),
        };

        // Terrain columns.
        for z in 0..width {
            for x in 0..width {
                let h = terrain_height(x, z, seed);
                world.heights[(z * width + x) as usize] = h;

                let (top, filler) = if h > 41 {
                    (Block::Snow, Block::Stone)
                } else if h <= SEA_LEVEL + 1 {
                    (Block::Sand, Block::Sand)
                } else {
                    (Block::Grass, Block::Dirt)
                };

                for y in 0..=h {
                    let block = if y == 0 {
                        Block::Bedrock
                    } else if y == h {
                        top
                    } else if y >= h - 3 {
                        filler
                    } else {
                        ore_at(x, y, z, seed)
                    };
                    world.set_raw(x, y, z, block);
                }

                for y in (h + 1)..=SEA_LEVEL {
                    world.set_raw(x, y, z, Block::Water);
                }
            }
        }

        // Trees on dry grass, on a jittered grid so they never touch each other.
        let mut tx = 3;
        while tx < width - 3 {
            let mut tz = 3;
            while tz < width - 3 {
                let jx = (hash2(tx, tz, 777) * 3.0) as i32;
                let jz = (hash2(tx, tz, 778) * 3.0) as i32;
                let x = tx + jx;
                let z = tz + jz;
                let h = world.heights[(z * width + x) as usize];
                let dry = h > SEA_LEVEL + 1 && h <= 41;
                if dry && hash2(tx, tz, 999) > 0.52 {
                    world.plant_tree(x, h + 1, z, seed, tx, tz);
                }
                tz += 9;
            }
            tx += 9;
        }

        for cz in 0..n {
            for cx in 0..n {
                world.rebuild(cx, cz);
            }
        }
        world
    }

    fn plant_tree(&mut self, x: i32, y: i32, z: i32, seed: u32, tx: i32, tz: i32) {
        let trunk = 4 + (hash2(tx, tz, seed ^ 0xabc) * 3.0) as i32;
        for i in 0..trunk {
            self.set_raw(x, y + i, z, Block::Log);
        }
        let top = y + trunk;
        for dy in -2i32..=1 {
            let radius: i32 = if dy <= -1 { 2 } else { 1 };
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    if dx.abs() == radius && dz.abs() == radius {
                        continue;
                    }
                    let bx = x + dx;
                    let by = top + dy;
                    let bz = z + dz;
                    if self.get(bx, by, bz) == Block::Air {
                        self.set_raw(bx, by, bz, Block::Leaves);
                    }
                }
            }
        }
        self.set_raw(x, top, z, Block::Leaves);
    }

    fn set_raw(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if !(0..self.width).contains(&x) || !(0..self.width).contains(&z) || !(0..HEIGHT).contains(&y)
        {
            return;
        }
        let idx = self.chunk_index(x / CHUNK, z / CHUNK);
        let lx = (x % CHUNK) as usize;
        let ly = y as usize;
        let lz = (z % CHUNK) as usize;
        let i = (ly * CHUNK as usize + lz) * CHUNK as usize + lx;
        self.chunks[idx].blocks[i] = block.id();
    }

    fn chunk_index(&self, cx: i32, cz: i32) -> usize {
        (cz * self.chunks_n + cx) as usize
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 {
            return Block::Bedrock;
        }
        if y >= HEIGHT || x < 0 || z < 0 || x >= self.width || z >= self.width {
            return Block::Air;
        }
        let ch = &self.chunks[self.chunk_index(x / CHUNK, z / CHUNK)];
        let lx = (x % CHUNK) as usize;
        let ly = y as usize;
        let lz = (z % CHUNK) as usize;
        Block::from_id(ch.blocks[(ly * CHUNK as usize + lz) * CHUNK as usize + lx])
    }

    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.get(x, y, z).is_solid()
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if !(0..self.width).contains(&x) || !(0..self.width).contains(&z) || !(0..HEIGHT).contains(&y)
        {
            return;
        }
        self.set_raw(x, y, z, block);
        let cx = x / CHUNK;
        let cz = z / CHUNK;
        self.rebuild(cx, cz);
        // Border edits also change the neighbouring chunk's visible faces.
        if x % CHUNK == 0 && cx > 0 {
            self.rebuild(cx - 1, cz);
        }
        if x % CHUNK == CHUNK - 1 && cx < self.chunks_n - 1 {
            self.rebuild(cx + 1, cz);
        }
        if z % CHUNK == 0 && cz > 0 {
            self.rebuild(cx, cz - 1);
        }
        if z % CHUNK == CHUNK - 1 && cz < self.chunks_n - 1 {
            self.rebuild(cx, cz + 1);
        }
    }

    pub fn rebuild(&mut self, cx: i32, cz: i32) {
        if !(0..self.chunks_n).contains(&cx) || !(0..self.chunks_n).contains(&cz) {
            return;
        }
        let (opaque, glass, water) = self.mesh_chunk(cx, cz);
        let atlas = self.atlas.clone();
        let idx = self.chunk_index(cx, cz);
        let ch = &mut self.chunks[idx];
        ch.meshes = opaque.into_meshes(atlas.clone());
        ch.glass = glass.into_meshes(atlas.clone());
        ch.water = water.into_meshes(atlas);
    }

    fn mesh_chunk(&self, cx: i32, cz: i32) -> (MeshData, MeshData, MeshData) {
        let mut opaque = MeshData::default();
        let mut glass = MeshData::default();
        let mut water = MeshData::default();
        let base_x = cx * CHUNK;
        let base_z = cz * CHUNK;

        for y in 0..HEIGHT {
            for z in 0..CHUNK {
                for x in 0..CHUNK {
                    let wx = base_x + x;
                    let wz = base_z + z;
                    let block = self.get(wx, y, wz);
                    if block == Block::Air {
                        continue;
                    }
                    let target = match block {
                        Block::Water => &mut water,
                        Block::Glass => &mut glass,
                        _ => &mut opaque,
                    };
                    let def = block.def();
                    for (dir, &(dx, dy, dz)) in DIRS.iter().enumerate() {
                        let neighbor = self.get(wx + dx, y + dy, wz + dz);
                        if !should_draw(neighbor, block) {
                            continue;
                        }
                        let tile = match dir {
                            2 => def.tiles.0,
                            3 => def.tiles.1,
                            _ => def.tiles.2,
                        };
                        let (u0, v0, u1, v1) = atlas::tile_uv(tile);
                        let (x0, x1) = (wx as f32, wx as f32 + 1.0);
                        let (y0, y1) = (y as f32, y as f32 + 1.0);
                        let (z0, z1) = (wz as f32, wz as f32 + 1.0);

                        let corners: [Vec3; 4] = match dir {
                            0 => [vec3(x1, y0, z1), vec3(x1, y0, z0), vec3(x1, y1, z0), vec3(x1, y1, z1)],
                            1 => [vec3(x0, y0, z0), vec3(x0, y0, z1), vec3(x0, y1, z1), vec3(x0, y1, z0)],
                            2 => [vec3(x0, y1, z1), vec3(x1, y1, z1), vec3(x1, y1, z0), vec3(x0, y1, z0)],
                            3 => [vec3(x0, y0, z0), vec3(x1, y0, z0), vec3(x1, y0, z1), vec3(x0, y0, z1)],
                            4 => [vec3(x0, y0, z1), vec3(x1, y0, z1), vec3(x1, y1, z1), vec3(x0, y1, z1)],
                            _ => [vec3(x1, y0, z0), vec3(x0, y0, z0), vec3(x0, y1, z0), vec3(x1, y1, z0)],
                        };
                        let uvs = [(u0, v1), (u1, v1), (u1, v0), (u0, v0)];
                        let n = vec3(dx as f32, dy as f32, dz as f32);
                        target.quad(corners, uvs, SHADE[dir], n);
                    }
                }
            }
        }
        (opaque, glass, water)
    }

    /// Alpha-test material: transparent texels of the glass tile are discarded
    /// instead of blended, so panes stay clear and never write depth over the
    /// blocks behind them.
    fn glass_material(&self) -> Option<&Material> {
        self.glass_material
            .get_or_init(|| {
                let params = MaterialParams {
                    pipeline_params: PipelineParams {
                        // The shader discards clear texels, so no blending is
                        // needed: glass draws as opaque pixels with depth.
                        depth_write: true,
                        depth_test: Comparison::LessOrEqual,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                // "Texture" is reserved: macroquad binds the mesh's own texture.
                match load_material(
                    ShaderSource::Glsl {
                        vertex: GLASS_VERT,
                        fragment: GLASS_FRAG,
                    },
                    params,
                ) {
                    Ok(mat) => Some(mat),
                    Err(e) => {
                        println!("Glass material failed: {e}");
                        None
                    }
                }
            })
            .as_ref()
    }

    pub fn draw(&self, cam: Vec3, view_distance: f32) {
        let limit = view_distance * view_distance;
        let visible = |cx: i32, cz: i32| {
            let center = vec3(
                (cx * CHUNK) as f32 + 8.0,
                cam.y,
                (cz * CHUNK) as f32 + 8.0,
            );
            (center - cam).length_squared() <= limit
        };
        let chunks = || {
            (0..self.chunks_n).flat_map(move |cz| (0..self.chunks_n).map(move |cx| (cx, cz)))
        };

        // Opaque blocks first (they fill the depth buffer).
        for (cx, cz) in chunks() {
            if !visible(cx, cz) {
                continue;
            }
            let ch = &self.chunks[self.chunk_index(cx, cz)];
            for m in &ch.meshes {
                draw_mesh(m);
            }
        }

        // Glass panes: only the frame survives the alpha test.
        let material = self.glass_material();
        if let Some(mat) = material {
            gl_use_material(mat);
            for (cx, cz) in chunks() {
                if !visible(cx, cz) {
                    continue;
                }
                let ch = &self.chunks[self.chunk_index(cx, cz)];
                for m in &ch.glass {
                    draw_mesh(m);
                }
            }
            gl_use_default_material();
        }

        // Water last so it blends over everything else.
        for (cx, cz) in chunks() {
            if !visible(cx, cz) {
                continue;
            }
            let ch = &self.chunks[self.chunk_index(cx, cz)];
            for m in &ch.water {
                draw_mesh(m);
            }
        }
    }

    /// Column height of the terrain (highest non-water block).
    pub fn height_at(&self, x: i32, z: i32) -> i32 {
        if !(0..self.width).contains(&x) || !(0..self.width).contains(&z) {
            return SEA_LEVEL;
        }
        self.heights[(z * self.width + x) as usize]
    }

    pub fn spawn_point(&self) -> Vec3 {
        let c = self.width / 2;
        for radius in 0..self.width / 2 {
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                let x = c + dx * radius;
                let z = c + dz * radius;
                let h = self.height_at(x, z);
                if h > SEA_LEVEL + 1 && h < 41 {
                    return vec3(x as f32 + 0.5, (h + 1) as f32, z as f32 + 0.5);
                }
            }
        }
        vec3(c as f32, 45.0, c as f32)
    }

    pub fn save(&self, path: &str, st: &SaveState) -> std::io::Result<()> {
        let total = (CHUNK * CHUNK * HEIGHT) as usize * self.chunks.len();
        let mut out = Vec::with_capacity(total / 8 + 64);
        push_header(&mut out, self.seed, self.chunks_n, st);
        write_runs(&mut out, &self.chunks);
        let _ = std::fs::create_dir_all(WORLDS_DIR);
        std::fs::write(path, out)
    }

    pub fn load(path: &str, atlas: Texture2D) -> std::io::Result<(World, SaveState)> {
        let data = std::fs::read(path)?;
        let (seed, n, mut st, off) = parse_header(&data)?;

        let size = Size::from_chunks(n);
        let width = size.blocks();
        let mut chunks = Vec::with_capacity((n * n) as usize);
        for _ in 0..n * n {
            chunks.push(Chunk::new());
        }
        let mut world = World {
            chunks,
            atlas,
            seed,
            width,
            chunks_n: n,
            heights: vec![0; (width * width) as usize],
            glass_material: OnceCell::new(),
        };

        read_runs(&data, off, &mut world.chunks)?;

        // Heights are a pure function of the seed, so they are not stored.
        for z in 0..width {
            for x in 0..width {
                world.heights[(z * width + x) as usize] = terrain_height(x, z, seed);
            }
        }
        for cz in 0..n {
            for cx in 0..n {
                world.rebuild(cx, cz);
            }
        }

        st.pos = vec3(
            st.pos.x.clamp(0.5, width as f32 - 0.5),
            st.pos.y.clamp(0.0, HEIGHT as f32 + 8.0),
            st.pos.z.clamp(0.5, width as f32 - 0.5),
        );
        st.pitch = st.pitch.clamp(-1.54, 1.54);
        Ok((world, st))
    }
}

/// Reads only the save header so the world list stays cheap.
pub fn list_worlds() -> Vec<WorldInfo> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir(WORLDS_DIR) else {
        return out;
    };
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rbw") {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        if let Some(mut info) = peek(&path_str) {
            info.modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            out.push(info);
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

fn peek(path: &str) -> Option<WorldInfo> {
    use std::io::Read;
    let f = std::fs::File::open(path).ok()?;
    let mut head = Vec::with_capacity(512);
    f.take(512).read_to_end(&mut head).ok()?;
    let (seed, n, st, _) = parse_header(&head).ok()?;
    Some(WorldInfo {
        path: path.to_string(),
        name: st.name,
        seed,
        size: Size::from_chunks(n),
        mode: st.mode,
        modified: 0,
    })
}

fn bad(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg.to_string())
}

fn push_header(out: &mut Vec<u8>, seed: u32, chunks_n: i32, st: &SaveState) {
    out.extend_from_slice(SAVE_MAGIC);
    out.extend_from_slice(&seed.to_le_bytes());
    out.extend_from_slice(&(chunks_n as u32).to_le_bytes());
    // Names are stored as plain ASCII so the bitmap font can render them.
    let name: String = st.name.chars().filter(|c| c.is_ascii()).take(24).collect();
    out.push(name.len() as u8);
    out.extend_from_slice(name.as_bytes());
    for v in [st.pos.x, st.pos.y, st.pos.z, st.yaw, st.pitch] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&(st.selected as u32).to_le_bytes());
    out.push(st.mode.to_u8());
    out.extend_from_slice(&st.health.to_le_bytes());
    for stack in st.inventory.iter() {
        let id = stack.item.map(|i| i.id()).unwrap_or(0);
        out.extend_from_slice(&id.to_le_bytes());
        out.push(if id == 0 { 0 } else { stack.count });
    }
}

/// Parses the header and returns it together with the offset of the block data.
fn parse_header(data: &[u8]) -> std::io::Result<(u32, i32, SaveState, usize)> {
    if data.len() < 13 {
        return Err(bad("not a rustblox save"));
    }
    let v3 = &data[0..4] == SAVE_MAGIC;
    if !v3 && &data[0..4] != SAVE_MAGIC_V2 {
        return Err(bad("not a rustblox save"));
    }
    let seed = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let n = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as i32;
    let name_len = data[12] as usize;
    if !(1..=64).contains(&n) || name_len > 32 {
        return Err(bad("bad save header"));
    }
    let mut off = 13;
    if data.len() < off + name_len + 24 {
        return Err(bad("truncated save header"));
    }
    let name = String::from_utf8_lossy(&data[off..off + name_len]).to_string();
    off += name_len;

    let mut floats = [0f32; 5];
    for f in floats.iter_mut() {
        let mut b = [0u8; 4];
        b.copy_from_slice(&data[off..off + 4]);
        *f = f32::from_le_bytes(b);
        off += 4;
    }
    let mut sel = [0u8; 4];
    sel.copy_from_slice(&data[off..off + 4]);
    off += 4;

    let mut inventory = [Stack::EMPTY; 36];
    let (mode, health) = if v3 {
        if data.len() < off + 5 + INV_BYTES {
            return Err(bad("truncated save header"));
        }
        let mode = Mode::from_u8(data[off]);
        off += 1;
        let mut hb = [0u8; 4];
        hb.copy_from_slice(&data[off..off + 4]);
        off += 4;
        for slot in inventory.iter_mut() {
            let id = u16::from_le_bytes([data[off], data[off + 1]]);
            let count = data[off + 2];
            off += 3;
            if let (Some(item), true) = (crate::items::Item::from_id(id), count > 0) {
                *slot = Stack::new(item, count);
            }
        }
        (mode, f32::from_le_bytes(hb).clamp(0.0, 20.0))
    } else {
        (Mode::Creative, 20.0)
    };

    let st = SaveState {
        name,
        pos: vec3(floats[0], floats[1], floats[2]),
        yaw: floats[3],
        pitch: floats[4],
        selected: (u32::from_le_bytes(sel) as usize).min(8),
        mode,
        health,
        inventory,
    };
    Ok((seed, n, st, off))
}

/// Appends every block as (run length, block id) pairs.
fn write_runs(out: &mut Vec<u8>, chunks: &[Chunk]) {
    let mut run_id = 0u8;
    let mut run_len = 0u32;
    for ch in chunks {
        for &b in ch.blocks.iter() {
            if run_len == 0 {
                run_id = b;
                run_len = 1;
            } else if b == run_id && run_len < u32::from(u16::MAX) {
                run_len += 1;
            } else {
                out.extend_from_slice(&(run_len as u16).to_le_bytes());
                out.push(run_id);
                run_id = b;
                run_len = 1;
            }
        }
    }
    if run_len > 0 {
        out.extend_from_slice(&(run_len as u16).to_le_bytes());
        out.push(run_id);
    }
}

/// Fills chunks with the block data that starts at `off`.
fn read_runs(data: &[u8], mut off: usize, chunks: &mut [Chunk]) -> std::io::Result<()> {
    let per_chunk = (CHUNK * CHUNK * HEIGHT) as usize;
    let total = per_chunk * chunks.len();
    let mut filled = 0usize;
    while off + 3 <= data.len() && filled < total {
        let run = u16::from_le_bytes([data[off], data[off + 1]]) as usize;
        let id = data[off + 2];
        off += 3;
        if run == 0 {
            return Err(bad("empty save run"));
        }
        for _ in 0..run.min(total - filled) {
            chunks[filled / per_chunk].blocks[filled % per_chunk] = id;
            filled += 1;
        }
    }
    if filled != total {
        return Err(bad("truncated save"));
    }
    Ok(())
}

fn should_draw(neighbor: Block, me: Block) -> bool {
    if neighbor == Block::Air {
        return true;
    }
    if !neighbor.is_transparent() {
        return false;
    }
    // Transparent-but-different blocks still show their shared face; identical
    // transparent blocks hide it so water bodies and glass panes look clean.
    neighbor != me
}

#[derive(Default)]
struct MeshData {
    verts: Vec<Vertex>,
    idx: Vec<u16>,
}

impl MeshData {
    fn quad(&mut self, p: [Vec3; 4], uv: [(f32, f32); 4], shade: u8, normal: Vec3) {
        if self.verts.len() + 4 > u16::MAX as usize {
            return;
        }
        let base = self.verts.len() as u16;
        for i in 0..4 {
            self.verts.push(Vertex {
                position: p[i],
                uv: vec2(uv[i].0, uv[i].1),
                color: [shade, shade, shade, 255],
                normal: normal.extend(0.0),
            });
        }
        self.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// macroquad issues one draw call per mesh and caps it at 10k vertices /
    /// 5k indices, so split big chunks into parts that fit.
    fn into_meshes(self, texture: Texture2D) -> Vec<Mesh> {
        const QUADS_PER_PART: usize = 800;
        let mut out = Vec::new();
        let quads = self.verts.len() / 4;
        let mut start = 0usize;
        while start < quads {
            let end = (start + QUADS_PER_PART).min(quads);
            let mut vertices = Vec::with_capacity((end - start) * 4);
            for v in &self.verts[start * 4..end * 4] {
                vertices.push(Vertex {
                    position: v.position,
                    uv: v.uv,
                    color: v.color,
                    normal: v.normal,
                });
            }
            let mut indices = Vec::with_capacity((end - start) * 6);
            for q in start..end {
                let b = (q * 4) as u16;
                indices.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
            }
            out.push(Mesh {
                vertices,
                indices,
                texture: Some(texture.clone()),
            });
            start = end;
        }
        out
    }
}

fn hash2(x: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x27d4_eb2d)
        ^ (z as u32).wrapping_mul(0x1656_67b1)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_215.0
}

fn smoother(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let x0 = x.floor();
    let z0 = z.floor();
    let fx = smoother(x - x0);
    let fz = smoother(z - z0);
    let (xi, zi) = (x0 as i32, z0 as i32);
    let a = hash2(xi, zi, seed);
    let b = hash2(xi + 1, zi, seed);
    let c = hash2(xi, zi + 1, seed);
    let d = hash2(xi + 1, zi + 1, seed);
    let top = a + (b - a) * fx;
    let bottom = c + (d - c) * fx;
    top + (bottom - top) * fz
}

fn fbm(x: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;
    for o in 0..octaves {
        sum += value_noise(x * freq, z * freq, seed.wrapping_add(o * 7919)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Stone-family block at (x, y, z): plain stone or an ore vein.
/// Veins occupy 4x4 columns and the ore type depends on depth.
fn ore_at(x: i32, y: i32, z: i32, seed: u32) -> Block {
    if y < 3 {
        return Block::Stone;
    }
    let vein = hash2(x / 4 + (y / 4) * 13, z / 4, seed ^ 0x51ce);
    let speck = hash2(x * 3 + y * 17, z * 5 + y * 11, seed ^ 0x5eed);
    if vein < 0.13 {
        if y < 22 && speck > 0.90 {
            Block::DiamondOre
        } else if y < 38 && speck > 0.78 {
            Block::GoldOre
        } else if y < 52 && speck > 0.55 {
            Block::IronOre
        } else if speck > 0.25 {
            Block::CoalOre
        } else {
            Block::Stone
        }
    } else if speck < 0.012 {
        Block::CoalOre
    } else if speck > 0.988 {
        Block::IronOre
    } else {
        Block::Stone
    }
}

fn terrain_height(x: i32, z: i32, seed: u32) -> i32 {
    let fx = x as f32;
    let fz = z as f32;
    let n1 = fbm(fx * 0.012, fz * 0.012, seed, 4);
    let n2 = fbm(fx * 0.055, fz * 0.055, seed ^ 0x1234, 2);
    let h = 16.0 + n1 * 24.0 + n2 * 8.0;
    (h as i32).clamp(4, HEIGHT - 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled_chunks(n: usize) -> Vec<Chunk> {
        let per = (CHUNK * CHUNK * HEIGHT) as usize;
        (0..n)
            .map(|c| {
                let mut ch = Chunk::new();
                for i in 0..per {
                    ch.blocks[i] = match (i / 97 + c) % 4 {
                        0 => Block::Air.id(),
                        1 => Block::Stone.id(),
                        2 => Block::Dirt.id(),
                        _ => Block::Grass.id(),
                    };
                }
                ch
            })
            .collect()
    }

    #[test]
    fn runs_round_trip() {
        let source = filled_chunks(4);
        let mut buf = Vec::new();
        write_runs(&mut buf, &source);
        let mut dest = vec![Chunk::new(), Chunk::new(), Chunk::new(), Chunk::new()];
        read_runs(&buf, 0, &mut dest).unwrap();
        for (a, b) in source.iter().zip(dest.iter()) {
            assert_eq!(a.blocks, b.blocks);
        }
    }

    #[test]
    fn runs_reject_truncated_data() {
        let source = filled_chunks(1);
        let mut buf = Vec::new();
        write_runs(&mut buf, &source);
        buf.truncate(buf.len() / 2);
        let mut dest = vec![Chunk::new()];
        assert!(read_runs(&buf, 0, &mut dest).is_err());
    }

    #[test]
    fn header_round_trip() {
        let mut inventory = [Stack::EMPTY; 36];
        inventory[0] = Stack::new(crate::items::Item::Block(Block::Planks), 32);
        inventory[35] = Stack::new(crate::items::Item::Diamond, 3);
        let st = SaveState {
            name: "My World".to_string(),
            pos: vec3(12.5, 33.25, 7.0),
            yaw: 1.5,
            pitch: -0.25,
            selected: 3,
            mode: Mode::Survival,
            health: 13.5,
            inventory,
        };
        let mut buf = Vec::new();
        push_header(&mut buf, 987_654, 12, &st);
        let (seed, n, back, off) = parse_header(&buf).unwrap();
        assert_eq!(seed, 987_654);
        assert_eq!(n, 12);
        assert_eq!(off, buf.len());
        assert_eq!(back.name, "My World");
        assert_eq!(back.pos, st.pos);
        assert_eq!(back.yaw, st.yaw);
        assert_eq!(back.pitch, st.pitch);
        assert_eq!(back.selected, 3);
        assert!(back.mode == Mode::Survival);
        assert!((back.health - 13.5).abs() < 1e-6);
        assert_eq!(back.inventory[0].item, inventory[0].item);
        assert_eq!(back.inventory[0].count, 32);
        assert_eq!(back.inventory[35].item, inventory[35].item);
        assert_eq!(back.inventory[35].count, 3);
        assert!(back.inventory[1].is_empty());
    }

    #[test]
    fn old_v2_header_loads_as_creative() {
        let mut buf = Vec::new();
        buf.extend_from_slice(SAVE_MAGIC_V2);
        buf.extend_from_slice(&123u32.to_le_bytes());
        buf.extend_from_slice(&8u32.to_le_bytes());
        buf.push(4);
        buf.extend_from_slice(b"Old1");
        for v in [10f32, 20.0, 30.0, 0.5, -0.1] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        buf.extend_from_slice(&2u32.to_le_bytes());
        let (seed, n, st, _) = parse_header(&buf).unwrap();
        assert_eq!(seed, 123);
        assert_eq!(n, 8);
        assert!(st.mode == Mode::Creative);
        assert_eq!(st.health, 20.0);
        assert_eq!(st.name, "Old1");
    }

    #[test]
    fn header_rejects_garbage() {
        assert!(parse_header(b"nope").is_err());
        let mut buf = Vec::new();
        push_header(&mut buf, 1, 8, &SaveState {
            name: String::new(),
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            selected: 0,
            mode: Mode::Creative,
            health: 20.0,
            inventory: [Stack::EMPTY; 36],
        });
        buf[0] = b'X';
        assert!(parse_header(&buf).is_err());
    }
}
