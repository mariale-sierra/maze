mod framebuffer;
mod imgutil;
mod maze;
mod target;
mod texture;

use framebuffer::Framebuffer;
use maze::{cast_ray, Maze};
use raylib::prelude::*;
use std::f32::consts::PI;
use target::{draw_targets, spawn_targets, try_shoot, Target};
use texture::TextureManager;

const SCREEN_WIDTH: i32 = 1024;
const SCREEN_HEIGHT: i32 = 600;

const FOV: f32 = PI / 3.0;
const MOVE_SPEED: f32 = 3.0;

// --- sensibilidad de mouse corregida ---
// antes estaba demasiado alta y sin limite, lo que hacia que un solo
// movimiento pequeno del mouse rotara la camara descontroladamente.
const ROT_SPEED_MOUSE: f32 = 0.0015;
const MAX_MOUSE_DELTA_PER_FRAME: f32 = 40.0; // clamp para evitar "saltos"

const PLAYER_RADIUS: f32 = 0.2;
const MAX_DEPTH: f32 = 20.0;

#[derive(Clone, Copy, PartialEq)]
enum GameState {
    Welcome,
    Playing,
    Success,
}

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

    let texture_manager = TextureManager::new(&mut rl, &thread);

    // --- placeholder de musica de fondo ---
    // Coloca tu archivo en assets/music.mp3 (o .ogg / .wav) y esto la
    // cargara y reproducira en loop automaticamente. Si el archivo no
    // existe, el juego sigue corriendo normal, solo sin musica.
    let audio = RaylibAudio::init_audio_device().expect("failed to init audio device");
    let test_sound = audio.new_sound("assets/music.mp3").ok();
    if let Some(s) = &test_sound {
        s.play();
        println!("sound de prueba reproduciendo, is_playing: {}", s.is_playing());
    } else {
        println!("no se pudo cargar como Sound");
    }

    let mut framebuffer = Framebuffer::new(SCREEN_WIDTH, SCREEN_HEIGHT, Color::new(30, 28, 15, 255));
    let mut display_texture = rl
        .load_texture_from_image(&thread, &framebuffer.image)
        .expect("failed to create framebuffer texture");

    let mut state = GameState::Welcome;
    let mut selected_level: usize = 1;

    let mut maze = Maze::new_level(selected_level);
    let mut player = Player { x: 1.5, y: 1.5, a: 0.0 };
    let mut targets: Vec<Target> = Vec::new();

    let mut time: f32 = 0.0;
    let mut cursor_locked = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        time += dt;


        match state {
            GameState::Welcome => {
                if cursor_locked {
                    rl.enable_cursor();
                    cursor_locked = false;
                }

                if rl.is_key_pressed(KeyboardKey::KEY_ONE)
                    || rl.is_key_pressed(KeyboardKey::KEY_LEFT)
                {
                    selected_level = 1;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_TWO)
                    || rl.is_key_pressed(KeyboardKey::KEY_RIGHT)
                {
                    selected_level = 2;
                }

                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    maze = Maze::new_level(selected_level);
                    player = Player { x: 1.5, y: 1.5, a: 0.0 };
                    let target_count = if selected_level >= 2 { 2 } else { 1 };
                    targets = spawn_targets(&maze, target_count, player.x, player.y);
                    state = GameState::Playing;
                    rl.disable_cursor();
                    cursor_locked = true;
                }

                let mut d = rl.begin_drawing(&thread);
                d.clear_background(Color::new(18, 17, 10, 255));
                draw_welcome_screen(&mut d, selected_level);
            }

            GameState::Playing => {
                if !cursor_locked {
                    rl.disable_cursor();
                    cursor_locked = true;
                }

                // --- rotacion con mouse, con clamp para que no se sienta "loca" ---
                let mut raw_dx = rl.get_mouse_delta().x;
                raw_dx = raw_dx.clamp(-MAX_MOUSE_DELTA_PER_FRAME, MAX_MOUSE_DELTA_PER_FRAME);
                player.a += raw_dx * ROT_SPEED_MOUSE;

                // mantener el angulo acotado
                while player.a > PI {
                    player.a -= 2.0 * PI;
                }
                while player.a < -PI {
                    player.a += 2.0 * PI;
                }

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

                let try_x = player.x + move_x;
                if !maze.collides(try_x, player.y, PLAYER_RADIUS) {
                    player.x = try_x;
                }
                let try_y = player.y + move_y;
                if !maze.collides(player.x, try_y, PLAYER_RADIUS) {
                    player.y = try_y;
                }

                if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                    try_shoot(&maze, player.x, player.y, player.a, &mut targets);
                }

                if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                    state = GameState::Welcome;
                }

                let all_dead = !targets.is_empty() && targets.iter().all(|t| !t.alive);
                if all_dead {
                    state = GameState::Success;
                }

                render_scene(&mut framebuffer, &maze, &texture_manager, &player, time);
                draw_targets(&mut framebuffer, player.x, player.y, player.a, FOV, &maze, &targets, time);
                framebuffer.update_texture(&mut display_texture);

                let mut d = rl.begin_drawing(&thread);
                d.clear_background(Color::BLACK);
                d.draw_texture(&display_texture, 0, 0, Color::WHITE);

                draw_minimap(&mut d, &maze, &player);
                draw_crosshair(&mut d);
                draw_hud(&mut d, &targets);
                d.draw_fps(10, SCREEN_HEIGHT - 30);
            }

            GameState::Success => {
                if cursor_locked {
                    rl.enable_cursor();
                    cursor_locked = false;
                }

                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    state = GameState::Welcome;
                }

                let mut d = rl.begin_drawing(&thread);
                d.clear_background(Color::new(10, 9, 6, 255));
                draw_success_screen(&mut d, time);
            }
        }
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

    for x in 0..width {
        let camera_x = x as f32 / width as f32 - 0.5;
        let ray_angle = player.a + camera_x * FOV;

        let hit = cast_ray(maze, player.x, player.y, ray_angle);
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

fn draw_crosshair(d: &mut RaylibDrawHandle) {
    let cx = SCREEN_WIDTH / 2;
    let cy = SCREEN_HEIGHT / 2;
    d.draw_line(cx - 8, cy, cx + 8, cy, Color::new(255, 255, 255, 200));
    d.draw_line(cx, cy - 8, cx, cy + 8, Color::new(255, 255, 255, 200));
}

fn draw_hud(d: &mut RaylibDrawHandle, targets: &[Target]) {
    let alive = targets.iter().filter(|t| t.alive).count();
    let text = format!("Objetivos restantes: {}", alive);
    d.draw_text(&text, 10, 10, 20, Color::new(255, 230, 150, 255));
    d.draw_text("Click izquierdo para disparar", 10, 35, 16, Color::new(200, 200, 200, 200));
}

fn draw_welcome_screen(d: &mut RaylibDrawHandle, selected_level: usize) {
    let title = "BACKROOMS";
    let title_size = 90;
    let title_w = d.measure_text(title, title_size);
    d.draw_text(
        title,
        SCREEN_WIDTH / 2 - title_w / 2,
        120,
        title_size,
        Color::new(196, 178, 74, 255),
    );

    let subtitle = "Selecciona un nivel";
    let sub_w = d.measure_text(subtitle, 24);
    d.draw_text(
        subtitle,
        SCREEN_WIDTH / 2 - sub_w / 2,
        250,
        24,
        Color::new(200, 200, 200, 255),
    );

    let box_w = 140;
    let box_h = 90;
    let gap = 40;
    let total_w = box_w * 2 + gap;
    let start_x = SCREEN_WIDTH / 2 - total_w / 2;
    let box_y = 300;

    for (i, label) in [("1", 1usize), ("2", 2usize)].iter().map(|(l, n)| (*l, *n)).enumerate() {
        let (label, level_n) = label;
        let x = start_x + i as i32 * (box_w + gap);
        let selected = level_n == selected_level;
        let bg = if selected {
            Color::new(196, 178, 74, 255)
        } else {
            Color::new(50, 46, 28, 255)
        };
        let fg = if selected { Color::BLACK } else { Color::new(180, 180, 180, 255) };

        d.draw_rectangle(x, box_y, box_w, box_h, bg);
        let text = format!("Nivel {}", label);
        let tw = d.measure_text(&text, 22);
        d.draw_text(&text, x + box_w / 2 - tw / 2, box_y + box_h / 2 - 11, 22, fg);
    }

    let hint = "1 / 2 o flechas para elegir - ENTER para jugar";
    let hint_w = d.measure_text(hint, 18);
    d.draw_text(
        hint,
        SCREEN_WIDTH / 2 - hint_w / 2,
        box_y + box_h + 40,
        18,
        Color::new(160, 160, 160, 255),
    );

    let controls = "WASD moverse - Mouse rotar - Click disparar";
    let controls_w = d.measure_text(controls, 16);
    d.draw_text(
        controls,
        SCREEN_WIDTH / 2 - controls_w / 2,
        SCREEN_HEIGHT - 40,
        16,
        Color::new(120, 120, 120, 255),
    );
}

fn draw_success_screen(d: &mut RaylibDrawHandle, time: f32) {
    let cx = SCREEN_WIDTH / 2;
    let cy = SCREEN_HEIGHT / 2;

    // luz tenue detras de la puerta
    let glow = (0.5 + (time * 1.5).sin() * 0.15).clamp(0.2, 0.7);
    d.draw_circle(cx, cy - 20, 260.0, Color::new(120, 110, 40, (glow * 60.0) as u8));

    // marco de la puerta
    let door_w = 220;
    let door_h = 380;
    let frame_x = cx - door_w / 2;
    let frame_y = cy - door_h / 2 + 20;

    d.draw_rectangle(
        frame_x - 15,
        frame_y - 15,
        door_w + 30,
        door_h + 20,
        Color::new(40, 32, 20, 255),
    );

    // puerta
    d.draw_rectangle(frame_x, frame_y, door_w, door_h, Color::new(70, 45, 25, 255));
    d.draw_rectangle(
        frame_x + 15,
        frame_y + 25,
        door_w - 30,
        (door_h as f32 * 0.4) as i32,
        Color::new(58, 38, 20, 255),
    );
    d.draw_rectangle(
        frame_x + 15,
        frame_y + 25 + (door_h as f32 * 0.45) as i32,
        door_w - 30,
        (door_h as f32 * 0.4) as i32,
        Color::new(58, 38, 20, 255),
    );

    // perilla
    d.draw_circle(frame_x + door_w - 25, frame_y + door_h / 2, 6.0, Color::new(200, 180, 90, 255));

    // luz brillante saliendo del marco (como si la puerta se abriera a algo)
    let crack_alpha = (0.4 + (time * 2.0).sin() * 0.2).clamp(0.15, 0.6);
    d.draw_rectangle(
        frame_x + door_w - 6,
        frame_y,
        6,
        door_h,
        Color::new(255, 240, 200, (crack_alpha * 255.0) as u8),
    );

    let title = "ESCAPASTE....¿SEGURO?";
    let title_size = 42;
    let title_w = d.measure_text(title, title_size);
    d.draw_text(
        title,
        cx - title_w / 2,
        frame_y - 90,
        title_size,
        Color::new(230, 220, 190, 255),
    );

    let hint = "Presiona ENTER para volver al menu";
    let hint_w = d.measure_text(hint, 18);
    d.draw_text(
        hint,
        cx - hint_w / 2,
        SCREEN_HEIGHT - 50,
        18,
        Color::new(160, 160, 160, 255),
    );
}