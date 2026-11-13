//! Rasterizes the application icon from `assets/feather.svg` at build time.

use std::env;
use std::fs;
use std::path::PathBuf;

/// Square size of the generated icon, in pixels.
const ICON_SIZE: u32 = 256;

/// Color used for the SVG's `currentColor` strokes.
///
/// The Feather icon set inherits the text color; the window icon is static, so
/// the app's accent blue is used to keep the glyph visible on both light and
/// dark title bars. Change this to recolor the icon.
const ICON_COLOR: &str = "#5E7CE2";

fn main() {
    println!("cargo:rerun-if-changed=assets/feather.svg");

    let svg = fs::read_to_string("assets/feather.svg")
        .expect("assets/feather.svg should be readable")
        .replace("currentColor", ICON_COLOR);

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&svg, &options)
        .expect("assets/feather.svg should be valid SVG");

    let source = tree.size();
    let scale = (ICON_SIZE as f32 / source.width())
        .min(ICON_SIZE as f32 / source.height());
    let dx = (ICON_SIZE as f32 - source.width() * scale) / 2.0;
    let dy = (ICON_SIZE as f32 - source.height() * scale) / 2.0;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(ICON_SIZE, ICON_SIZE)
        .expect("icon pixmap should be allocatable");

    let transform = resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, dx, dy);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut rgba = pixmap.take();
    unpremultiply(&mut rgba);

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR should be set");
    let icon_path = PathBuf::from(out_dir).join("feather_icon.rgba");
    fs::write(icon_path, &rgba).expect("rasterized icon should be writable");
}

/// `tiny-skia` stores premultiplied colors; window icons expect straight RGBA.
fn unpremultiply(rgba: &mut [u8]) {
    let (pixels, _) = rgba.as_chunks_mut::<4>();

    for pixel in pixels {
        let alpha = pixel[3] as u32;

        if alpha == 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
        } else if alpha < 255 {
            pixel[0] = ((pixel[0] as u32 * 255) / alpha).min(255) as u8;
            pixel[1] = ((pixel[1] as u32 * 255) / alpha).min(255) as u8;
            pixel[2] = ((pixel[2] as u32 * 255) / alpha).min(255) as u8;
        }
    }
}
