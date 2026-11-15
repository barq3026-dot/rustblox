// First-person player: movement, AABB collision and voxel ray picking.
use macroquad::prelude::*;

use crate::blocks::Block;
use crate::world::World;

pub const HALF_WIDTH: f32 = 0.3;
pub const BODY_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;

const GRAVITY: f32 = 26.0;
const JUMP_SPEED: f32 = 8.4;
const WALK_SPEED: f32 = 5.2;
const SPRINT_SPEED: f32 = 8.4;
const FLY_SPEED: f32 = 14.0;
const SWIM_SPEED: f32 = 3.4;

pub struct Player {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub flying: bool,
    pub in_water: bool,
    /// Drop the next mouse delta (used right after grabbing the cursor).
    pub skip_look: bool,
    /// Survival rules: fall damage, drowning, no flying.
    pub survival: bool,
    pub health: f32,
    /// Seconds of breath left under water.
    pub air: f32,
    pub dead: bool,
    hurt_flash: f32,
    fall_from: f32,
}

pub const MAX_HEALTH: f32 = 20.0;
const MAX_AIR: f32 = 15.0;

impl Player {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            flying: false,
            in_water: false,
            skip_look: true,
            survival: false,
            health: MAX_HEALTH,
            air: MAX_AIR,
            dead: false,
            hurt_flash: 0.0,
            fall_from: pos.y,
        }
    }

    pub fn hurt_flash(&self) -> f32 {
        self.hurt_flash
    }

    pub fn hurt(&mut self, amount: f32) {
        if !self.survival || self.dead || amount <= 0.0 {
            return;
        }
        self.health = (self.health - amount).max(0.0);
        self.hurt_flash = 0.35;
        if self.health <= 0.0 {
            self.dead = true;
        }
    }

    /// Resets the player after death or on load.
    pub fn respawn(&mut self, pos: Vec3) {
        self.pos = pos;
        self.vel = Vec3::ZERO;
        self.health = MAX_HEALTH;
        self.air = MAX_AIR;
        self.dead = false;
        self.hurt_flash = 0.0;
        self.fall_from = pos.y;
        self.flying = false;
    }

    /// Ignore the next mouse movement, so the camera does not snap.
    pub fn reset_look(&mut self) {
        self.skip_look = true;
    }

    pub fn eye(&self) -> Vec3 {
        self.pos + vec3(0.0, EYE_HEIGHT, 0.0)
    }

    pub fn look(&self) -> Vec3 {
        let cp = self.pitch.cos();
        vec3(self.yaw.sin() * cp, self.pitch.sin(), -self.yaw.cos() * cp)
    }

    pub fn update(&mut self, world: &World, dt: f32, sensitivity: f32) {
        // mouse_delta_position() is in local (-1..1 over the window) units and
        // reversed (last - current). Convert it to pixels so sensitivity does not
        // depend on the window size, then apply it.
        let local = mouse_delta_position();
        let mut px = vec2(
            -local.x * screen_width() * 0.5,
            -local.y * screen_height() * 0.5,
        );
        if self.skip_look {
            px = Vec2::ZERO;
            self.skip_look = false;
        }
        // Ignore absurd jumps (e.g. cursor warped by the OS or alt-tab).
        px = px.clamp_length_max(220.0);
        self.yaw += px.x * sensitivity;
        self.pitch = (self.pitch - px.y * sensitivity).clamp(-1.54, 1.54);

        if is_key_pressed(KeyCode::F) && !self.survival {
            self.flying = !self.flying;
            self.vel.y = 0.0;
        }
        if self.survival {
            self.flying = false;
        }
        self.hurt_flash = (self.hurt_flash - dt).max(0.0);

        let forward = vec3(self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = vec3(self.yaw.cos(), 0.0, self.yaw.sin());

        let mut wish = Vec3::ZERO;
        if is_key_down(KeyCode::W) {
            wish += forward;
        }
        if is_key_down(KeyCode::S) {
            wish -= forward;
        }
        if is_key_down(KeyCode::D) {
            wish += right;
        }
        if is_key_down(KeyCode::A) {
            wish -= right;
        }
        if wish.length_squared() > 0.0 {
            wish = wish.normalize();
        }

        let feet = self.pos;
        self.in_water = world.get(feet.x.floor() as i32, feet.y.floor() as i32, feet.z.floor() as i32)
            == Block::Water
            || world.get(
                feet.x.floor() as i32,
                (feet.y + 0.9).floor() as i32,
                feet.z.floor() as i32,
            ) == Block::Water;

        let sprinting = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let speed = if self.flying {
            FLY_SPEED
        } else if self.in_water {
            SWIM_SPEED
        } else if sprinting {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };

        let accel = if self.flying { 18.0 } else { 14.0 };
        let target = wish * speed;
        self.vel.x += (target.x - self.vel.x) * (accel * dt).min(1.0);
        self.vel.z += (target.z - self.vel.z) * (accel * dt).min(1.0);

        if self.flying {
            let mut vy = 0.0;
            if is_key_down(KeyCode::Space) {
                vy += speed;
            }
            if is_key_down(KeyCode::LeftControl) {
                vy -= speed;
            }
            self.vel.y += (vy - self.vel.y) * (accel * dt).min(1.0);
        } else if self.in_water {
            self.vel.y -= GRAVITY * 0.25 * dt;
            self.vel.y = self.vel.y.max(-4.0);
            if is_key_down(KeyCode::Space) {
                self.vel.y = 3.6;
            }
        } else {
            self.vel.y -= GRAVITY * dt;
            self.vel.y = self.vel.y.max(-55.0);
            if self.on_ground && is_key_down(KeyCode::Space) {
                self.vel.y = JUMP_SPEED;
                self.on_ground = false;
            }
        }

        let was_ground = self.on_ground;
        self.on_ground = false;
        let dx = self.vel.x * dt;
        let dy = self.vel.y * dt;
        let dz = self.vel.z * dt;
        self.move_axis(world, 0, dx);
        self.move_axis(world, 2, dz);
        self.move_axis(world, 1, dy);

        if self.survival {
            // Remember the highest point since leaving the ground.
            if self.pos.y > self.fall_from {
                self.fall_from = self.pos.y;
            }
            if was_ground {
                self.fall_from = self.pos.y;
            }
            if self.on_ground && !was_ground && !self.in_water {
                let fell = self.fall_from - self.pos.y;
                if fell > 3.5 {
                    self.hurt((fell - 3.0).floor());
                }
                self.fall_from = self.pos.y;
            }

            // Breath drains while the eyes are under water.
            let eye = self.pos + vec3(0.0, EYE_HEIGHT, 0.0);
            let under = world.get(
                eye.x.floor() as i32,
                eye.y.floor() as i32,
                eye.z.floor() as i32,
            ) == Block::Water;
            if under {
                self.air = (self.air - dt).max(0.0);
                if self.air <= 0.0 {
                    self.hurt(2.0 * dt);
                }
            } else {
                self.air = MAX_AIR;
            }
        }

        // Never let the player fall out of the world.
        if self.pos.y < -20.0 {
            let spawn = world.spawn_point();
            self.respawn(spawn);
        }
    }

    fn move_axis(&mut self, world: &World, axis: usize, amount: f32) {
        if amount.abs() < 1e-6 {
            return;
        }
        let steps = (amount.abs() / 0.2).ceil() as i32;
        let step = amount / steps as f32;
        for _ in 0..steps {
            let mut next = self.pos;
            match axis {
                0 => next.x += step,
                1 => next.y += step,
                _ => next.z += step,
            }
            if self.box_free(world, next) {
                self.pos = next;
            } else {
                match axis {
                    0 => self.vel.x = 0.0,
                    1 => {
                        if step < 0.0 {
                            self.on_ground = true;
                        }
                        self.vel.y = 0.0;
                    }
                    _ => self.vel.z = 0.0,
                }
                break;
            }
        }
    }

    fn box_free(&self, world: &World, p: Vec3) -> bool {
        let x0 = (p.x - HALF_WIDTH).floor() as i32;
        let x1 = (p.x + HALF_WIDTH).floor() as i32;
        let y0 = p.y.floor() as i32;
        let y1 = (p.y + BODY_HEIGHT - 0.001).floor() as i32;
        let z0 = (p.z - HALF_WIDTH).floor() as i32;
        let z1 = (p.z + HALF_WIDTH).floor() as i32;
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    if world.is_solid(x, y, z) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// True if placing a full block at `cell` would intersect the player.
    pub fn intersects_cell(&self, cell: (i32, i32, i32)) -> bool {
        let (x, y, z) = cell;
        let px0 = self.pos.x - HALF_WIDTH;
        let px1 = self.pos.x + HALF_WIDTH;
        let py0 = self.pos.y;
        let py1 = self.pos.y + BODY_HEIGHT;
        let pz0 = self.pos.z - HALF_WIDTH;
        let pz1 = self.pos.z + HALF_WIDTH;
        !(px1 <= x as f32 || px0 >= x as f32 + 1.0)
            && !(py1 <= y as f32 || py0 >= y as f32 + 1.0)
            && !(pz1 <= z as f32 || pz0 >= z as f32 + 1.0)
    }
}

/// Voxel DDA (Amanatides & Woo). Returns the hit block and the face normal.
pub fn raycast(
    world: &World,
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
) -> Option<((i32, i32, i32), (i32, i32, i32))> {
    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;

    let step_x = if dir.x > 0.0 { 1 } else { -1 };
    let step_y = if dir.y > 0.0 { 1 } else { -1 };
    let step_z = if dir.z > 0.0 { 1 } else { -1 };

    let delta_x = if dir.x.abs() < 1e-6 { f32::INFINITY } else { 1.0 / dir.x.abs() };
    let delta_y = if dir.y.abs() < 1e-6 { f32::INFINITY } else { 1.0 / dir.y.abs() };
    let delta_z = if dir.z.abs() < 1e-6 { f32::INFINITY } else { 1.0 / dir.z.abs() };

    let boundary = |v: f32, dir: f32| {
        if dir > 0.0 {
            (v.floor() + 1.0 - v) / dir
        } else if dir < 0.0 {
            (v - v.floor()) / -dir
        } else {
            f32::INFINITY
        }
    };
    let mut max_x = boundary(origin.x, dir.x);
    let mut max_y = boundary(origin.y, dir.y);
    let mut max_z = boundary(origin.z, dir.z);

    let mut normal = (0, 0, 0);
    let mut traveled = 0.0f32;

    if world.is_solid(x, y, z) {
        return Some(((x, y, z), normal));
    }

    while traveled <= max_distance {
        if max_x < max_y && max_x < max_z {
            if !max_x.is_finite() {
                break;
            }
            x += step_x;
            traveled = max_x;
            max_x += delta_x;
            normal = (-step_x, 0, 0);
        } else if max_y < max_z {
            if !max_y.is_finite() {
                break;
            }
            y += step_y;
            traveled = max_y;
            max_y += delta_y;
            normal = (0, -step_y, 0);
        } else {
            if !max_z.is_finite() {
                break;
            }
            z += step_z;
            traveled = max_z;
            max_z += delta_z;
            normal = (0, 0, -step_z);
        }
        let block = world.get(x, y, z);
        if block.is_solid() {
            return Some(((x, y, z), normal));
        }
    }
    None
}
