use raylib::prelude::*;
use crate::imgutil::gen_blank_image;

pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    pub image: Image,
    current_color: Color,
}

impl Framebuffer {
    pub fn new(width: i32, height: i32, background_color: Color) -> Self {
        let image = gen_blank_image(width, height, background_color);
        Framebuffer {
            width,
            height,
            image,
            current_color: Color::WHITE,
        }
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn set_pixel(&mut self, x: u32, y: u32) {
        let xi = x as i32;
        let yi = y as i32;
        if xi >= 0 && yi >= 0 && xi < self.width && yi < self.height {
            self.image.draw_pixel(xi, yi, self.current_color);
        }
    }

    fn texture_bytes(&self) -> Vec<u8> {
        let colors = self.image.get_image_data();
        let mut bytes = Vec::with_capacity((self.width * self.height * 4) as usize);
        for color in colors.iter() {
            bytes.push(color.r);
            bytes.push(color.g);
            bytes.push(color.b);
            bytes.push(color.a);
        }
        bytes
    }

    pub fn update_texture(&self, texture: &mut Texture2D) {
        texture
            .update_texture(&self.texture_bytes())
            .expect("failed to update framebuffer texture");
    }
}