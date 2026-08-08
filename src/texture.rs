use raylib::prelude::*;
use std::collections::HashMap;
use crate::imgutil::gen_blank_image;

pub const TEX_SIZE: i32 = 64;

pub struct TextureManager {
    textures: HashMap<char, Texture2D>,
    pixels: HashMap<char, Vec<Color>>,
}

impl TextureManager {
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let mut textures = HashMap::new();
        let mut pixels = HashMap::new();

        let wall_defs: Vec<(char, Color, Color)> = vec![
            ('1', Color::new(196, 178, 74, 255), Color::new(150, 132, 46, 255)),
            ('2', Color::new(180, 160, 70, 255), Color::new(130, 112, 40, 255)),
            ('3', Color::new(160, 150, 90, 255), Color::new(120, 108, 55, 255)),
            ('4', Color::new(210, 190, 90, 255), Color::new(160, 140, 55, 255)),
            ('g', Color::new(90, 140, 70, 255), Color::new(50, 90, 40, 255)),
        ];

        for (ch, base, dark) in wall_defs {
            let (px, image) = generate_wallpaper_image(base, dark);
            let texture = rl
                .load_texture_from_image(thread, &image)
                .expect("failed to create wall texture");
            textures.insert(ch, texture);
            pixels.insert(ch, px);
        }

        TextureManager { textures, pixels }
    }

    #[allow(dead_code)]
    pub fn get_texture(&self, ch: char) -> Option<&Texture2D> {
        self.textures.get(&ch)
    }

    pub fn sample(&self, ch: char, tx: f32, ty: f32) -> Color {
        let pixels = match self.pixels.get(&ch) {
            Some(p) => p,
            None => return Color::MAGENTA,
        };
        let x = ((tx * TEX_SIZE as f32) as i32).clamp(0, TEX_SIZE - 1);
        let y = ((ty * TEX_SIZE as f32) as i32).clamp(0, TEX_SIZE - 1);
        pixels[(y * TEX_SIZE + x) as usize]
    }
}

fn generate_wallpaper_image(base: Color, dark: Color) -> (Vec<Color>, Image) {
    let mut pixels = Vec::with_capacity((TEX_SIZE * TEX_SIZE) as usize);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let cx = x % 16;
            let cy = y % 16;
            let diamond = (cx - 8).abs() + (cy - 8).abs() < 6;
            let is_baseboard = y > TEX_SIZE - 8;

            let color = if is_baseboard {
                Color::new(60, 50, 25, 255)
            } else if diamond {
                dark
            } else {
                base
            };
            pixels.push(color);
        }
    }

    let mut image = gen_blank_image(TEX_SIZE, TEX_SIZE, Color::WHITE);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            image.draw_pixel(x, y, pixels[(y * TEX_SIZE + x) as usize]);
        }
    }

    (pixels, image)
}