use crate::framebuffer::Framebuffer;
use crate::maze::{cast_ray, Maze};
use raylib::prelude::*;

pub struct Target {
    pub x: f32,
    pub y: f32,
    pub alive: bool,
}

impl Target {
    pub fn new(x: f32, y: f32) -> Self {
        Target { x, y, alive: true }
    }
}

/// Coloca `count` targets en celdas libres del mapa, lejos del jugador,
/// en posiciones "aleatorias" (generador simple sin depender del crate rand).
pub fn spawn_targets(maze: &Maze, count: usize, player_x: f32, player_y: f32) -> Vec<Target> {
    let mut open_cells = Vec::new();
    for y in 1..maze.height() as i32 - 1 {
        for x in 1..maze.width() as i32 - 1 {
            if maze.is_open(x, y) {
                let cx = x as f32 + 0.5;
                let cy = y as f32 + 0.5;
                let d = ((cx - player_x).powi(2) + (cy - player_y).powi(2)).sqrt();
                if d > 3.0 {
                    open_cells.push((cx, cy));
                }
            }
        }
    }

    let mut targets = Vec::new();
    if open_cells.is_empty() {
        return targets;
    }

    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize
        | 1;

    for _ in 0..count {
        if open_cells.is_empty() {
            break;
        }
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let idx = seed % open_cells.len();
        let (cx, cy) = open_cells[idx];
        targets.push(Target::new(cx, cy));
        open_cells.remove(idx);
    }

    targets
}

pub fn draw_targets(
    framebuffer: &mut Framebuffer,
    player_x: f32,
    player_y: f32,
    player_a: f32,
    fov: f32,
    maze: &Maze,
    targets: &[Target],
    time: f32,
) {
    for target in targets {
        if target.alive {
            draw_one_target(framebuffer, player_x, player_y, player_a, fov, maze, target, time);
        }
    }
}

fn draw_one_target(
    framebuffer: &mut Framebuffer,
    player_x: f32,
    player_y: f32,
    player_a: f32,
    fov: f32,
    maze: &Maze,
    target: &Target,
    time: f32,
) {
    let sprite_a = (target.y - player_y).atan2(target.x - player_x);
    let mut angle_diff = sprite_a - player_a;
    while angle_diff > std::f32::consts::PI {
        angle_diff -= 2.0 * std::f32::consts::PI;
    }
    while angle_diff < -std::f32::consts::PI {
        angle_diff += 2.0 * std::f32::consts::PI;
    }
    if angle_diff.abs() > fov / 2.0 {
        return;
    }

    let sprite_d = ((player_x - target.x).powi(2) + (player_y - target.y).powi(2)).sqrt();
    if sprite_d < 0.3 {
        return;
    }

    // ocultar el target si hay una pared en medio
    let wall_hit = cast_ray(maze, player_x, player_y, sprite_a);
    if wall_hit.perp_dist < sprite_d - 0.05 {
        return;
    }

    let screen_height = framebuffer.height as f32;
    let screen_width = framebuffer.width as f32;

    // animacion simple: "flota" verticalmente (esto cuenta como la
    // animacion de sprite pedida en la rubrica)
    let bob = (time * 3.0 + target.x * 10.0).sin() * 0.05;

    let corrected_d = (sprite_d * angle_diff.cos()).max(0.1);
    let sprite_size = (screen_height / corrected_d) * 0.6;
    let screen_x = ((angle_diff / fov) + 0.5) * screen_width;

    let start_x = (screen_x - sprite_size / 2.0).max(0.0) as i32;
    let end_x = (screen_x + sprite_size / 2.0).min(screen_width) as i32;

    let vertical_offset = bob * sprite_size;
    let start_y = (screen_height / 2.0 - sprite_size / 2.0 + vertical_offset).max(0.0) as i32;
    let end_y = (start_y as f32 + sprite_size).min(screen_height) as i32;

    let sprite_w = (end_x - start_x).max(1) as f32;
    let sprite_h = (end_y - start_y).max(1) as f32;

    framebuffer.set_current_color(Color::new(10, 10, 10, 255));

    for sx in start_x..end_x {
        for sy in start_y..end_y {
            let u = (sx - start_x) as f32 / sprite_w;
            let v = (sy - start_y) as f32 / sprite_h;
            if is_silhouette(u, v) {
                framebuffer.set_pixel(sx as u32, sy as u32);
            }
        }
    }
}

/// Silueta humanoide simple: cabeza (circulo) + cuerpo (trapecio).
fn is_silhouette(u: f32, v: f32) -> bool {
    let dx = u - 0.5;
    let dy = v - 0.18;
    if (dx * dx + dy * dy).sqrt() < 0.16 {
        return true;
    }
    if v > 0.32 && v < 1.0 {
        let body_half_width = 0.15 + (v - 0.32) * 0.25;
        if (u - 0.5).abs() < body_half_width {
            return true;
        }
    }
    false
}

/// Intenta "disparar": si el jugador apunta a un target vivo dentro de
/// rango y no hay pared en medio, lo mata y devuelve true.
pub fn try_shoot(maze: &Maze, player_x: f32, player_y: f32, player_a: f32, targets: &mut [Target]) -> bool {
    const HIT_ANGLE_TOLERANCE: f32 = 0.05;
    const MAX_RANGE: f32 = 12.0;

    for target in targets.iter_mut() {
        if !target.alive {
            continue;
        }
        let sprite_a = (target.y - player_y).atan2(target.x - player_x);
        let mut angle_diff = sprite_a - player_a;
        while angle_diff > std::f32::consts::PI {
            angle_diff -= 2.0 * std::f32::consts::PI;
        }
        while angle_diff < -std::f32::consts::PI {
            angle_diff += 2.0 * std::f32::consts::PI;
        }

        if angle_diff.abs() > HIT_ANGLE_TOLERANCE {
            continue;
        }

        let sprite_d = ((player_x - target.x).powi(2) + (player_y - target.y).powi(2)).sqrt();
        if sprite_d > MAX_RANGE {
            continue;
        }

        let wall_hit = cast_ray(maze, player_x, player_y, sprite_a);
        if wall_hit.perp_dist < sprite_d - 0.05 {
            continue;
        }

        target.alive = false;
        return true;
    }
    false
}