use raylib::ffi;
use raylib::prelude::*;

/// Crea una Image en blanco de un color solido, usando el binding de bajo
/// nivel porque en raylib-rs 6.0 `Image::gen_image_color` ya no existe
/// como metodo de alto nivel.
pub fn gen_blank_image(width: i32, height: i32, color: Color) -> Image {
    let raw = unsafe {
        ffi::GenImageColor(
            width,
            height,
            ffi::Color {
                r: color.r,
                g: color.g,
                b: color.b,
                a: color.a,
            },
        )
    };
    unsafe { Image::from_raw(raw) }
}