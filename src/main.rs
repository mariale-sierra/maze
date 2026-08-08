
mod framebuffer;
mod maze;
mod texture;
mod imgutil;

use framebuffer::Framebuffer;
use maze::{cast_ray, Maze};
use raylib::prelude::*;
use std::f32::consts::PI;
use texture::TextureManager;

const SCREEN_WIDTH: i32 = 1024;
const SCREEN_HEIGHT: i32 = 600;

const FOV: f32 = PI / 3.0; // 60 grados
const MOVE_SPEED: f32 = 3.0; // celdas por segundo
const ROT_SPEED_MOUSE: f32 = 0.0025;
const PLAYER_RADIUS: f32 = 0.2;
const MAX_DEPTH: f32 = 20.0;

struct Player {
    x: f32,
    y: f32,
    a: f32,
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Backrooms Raycaster")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    rl.set_target_fps(60);
    rl.disable_cursor();

    let maze = Maze::new_backrooms();
    let texture_manager = TextureManager::new(&mut rl, &thread);

    let mut player = Player {
        x: 1.5,
        y: 1.5,
        a: 0.0,
    };

    let mut framebuffer = Framebuffer::new(SCREEN_WIDTH, SCREEN_HEIGHT, Color::new(30, 28, 15, 255));
    let mut display_texture = rl
        .load_texture_from_image(&thread, &framebuffer.image)
        .expect("failed to create framebuffer texture");

    let mut time: f32 = 0.0;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        time += dt;

        // --- input ---
        let mouse_delta = rl.get_mouse_delta();
        player.a += mouse_delta.x * ROT_SPEED_MOUSE;

        let mut move_x = 0.0;
        let mut move_y = 0.0;
        let forward = (player.a.cos(), player.a.sin());
        let right = (-player.a.sin(), player.a.cos());

        if rl.is_key_down(KeyboardKey::KEY_W) {
            move_x += forward.0;
            move_y += forward.1;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            move_x -= forward.0;
            move_y -= forward.1;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            move_x += right.0;
            move_y += right.1;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            move_x -= right.0;
            move_y -= right.1;
        }

        let len = (move_x * move_x + move_y * move_y).sqrt();
        if len > 0.0001 {
            move_x = move_x / len * MOVE_SPEED * dt;
            move_y = move_y / len * MOVE_SPEED * dt;
        }

        // mover con deslizamiento en paredes (cada eje se prueba por separado,
        // asi si chocas de frente igual puedes deslizarte a los lados)
        let try_x = player.x + move_x;
        if !maze.collides(try_x, player.y, PLAYER_RADIUS) {
            player.x = try_x;
        }
        let try_y = player.y + move_y;
        if !maze.collides(player.x, try_y, PLAYER_RADIUS) {
            player.y = try_y;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            break;
        }

        // --- render ---
        render_scene(&mut framebuffer, &maze, &texture_manager, &player, time);
        framebuffer.update_texture(&mut display_texture);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture(&display_texture, 0, 0, Color::WHITE);

        draw_minimap(&mut d, &maze, &player);
        d.draw_fps(10, SCREEN_HEIGHT - 30);
    }
}

fn render_scene(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    texture_manager: &TextureManager,
    player: &Player,
    time: f32,
) {
    let width = framebuffer.width;
    let height = framebuffer.height;

    // parpadeo sutil de fluorescentes, para el ambiente inquietante
    let flicker = 1.0 + (time * 7.3).sin() * 0.02 + (time * 2.1).sin() * 0.015;

    let ceiling_color = shade(Color::new(74, 68, 40, 255), flicker);
    let floor_color = shade(Color::new(96, 82, 46, 255), flicker);

    for y in 0..height / 2 {
        framebuffer.set_current_color(ceiling_color);
        for x in 0..width {
            framebuffer.set_pixel(x as u32, y as u32);
        }
    }
    for y in (height / 2)..height {
        let t = (y - height / 2) as f32 / (height as f32 / 2.0);
        let c = lerp_color(floor_color, Color::new(20, 18, 10, 255), t * 0.5);
        framebuffer.set_current_color(c);
        for x in 0..width {
            framebuffer.set_pixel(x as u32, y as u32);
        }
    }

    // paredes, columna por columna
    for x in 0..width {
        let camera_x = x as f32 / width as f32 - 0.5;
        let ray_angle = player.a + camera_x * FOV;

        let hit = cast_ray(maze, player.x, player.y, ray_angle);

        // correccion de ojo de pez
        let corrected_dist = (hit.perp_dist * (ray_angle - player.a).cos()).max(0.0001);

        let line_height = (height as f32 / corrected_dist) as i32;
        let draw_start = (-line_height / 2 + height / 2).max(0);
        let draw_end = (line_height / 2 + height / 2).min(height - 1);

        let fog = (corrected_dist / MAX_DEPTH).min(1.0);
        let side_shade = if hit.side == 1 { 0.75 } else { 1.0 };

        let span = (draw_end - draw_start).max(1);
        for y in draw_start..draw_end {
            let ty = (y - draw_start) as f32 / span as f32;
            let mut color = texture_manager.sample(hit.wall_char, hit.wall_x, ty);
            color = shade(color, side_shade * flicker);
            color = lerp_color(color, Color::new(15, 14, 8, 255), fog * 0.85);
            framebuffer.set_current_color(color);
            framebuffer.set_pixel(x as u32, y as u32);
        }
    }
}

fn shade(color: Color, factor: f32) -> Color {
    Color::new(
        ((color.r as f32) * factor).clamp(0.0, 255.0) as u8,
        ((color.g as f32) * factor).clamp(0.0, 255.0) as u8,
        ((color.b as f32) * factor).clamp(0.0, 255.0) as u8,
        color.a,
    )
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        (a.r as f32 + (b.r as f32 - a.r as f32) * t) as u8,
        (a.g as f32 + (b.g as f32 - a.g as f32) * t) as u8,
        (a.b as f32 + (b.b as f32 - a.b as f32) * t) as u8,
        255,
    )
}

fn draw_minimap(d: &mut RaylibDrawHandle, maze: &Maze, player: &Player) {
    let cell_px = 5;
    let margin = 10;
    let map_w = maze.width() as i32 * cell_px;
    let map_h = maze.height() as i32 * cell_px;
    let origin_x = SCREEN_WIDTH - map_w - margin;
    let origin_y = margin;

    d.draw_rectangle(origin_x - 2, origin_y - 2, map_w + 4, map_h + 4, Color::new(0, 0, 0, 180));

    for y in 0..maze.height() {
        for x in 0..maze.width() {
            let c = maze.cell_at(x as i32, y as i32);
            let color = match c {
                '.' => Color::new(40, 38, 25, 255),
                'g' => Color::new(70, 160, 70, 255),
                _ => Color::new(190, 170, 80, 255),
            };
            d.draw_rectangle(
                origin_x + x as i32 * cell_px,
                origin_y + y as i32 * cell_px,
                cell_px,
                cell_px,
                color,
            );
        }
    }

    let px = origin_x + (player.x * cell_px as f32) as i32;
    let py = origin_y + (player.y * cell_px as f32) as i32;
    d.draw_circle(px, py, 3.0, Color::RED);

    let dir_len = 10.0;
    let dx = player.a.cos() * dir_len;
    let dy = player.a.sin() * dir_len;
    d.draw_line(px, py, px + dx as i32, py + dy as i32, Color::RED);
}