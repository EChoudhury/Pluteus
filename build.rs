//! Rasterizes the application icons from `assets/feather.svg` at build time.
//!
//! The generated files are written to a `pluteus-icons` directory next to the
//! build's executable, so the release workflow can bundle them:
//!
//! - `pluteus.ico`: Windows executable icon (embedded into the `.exe`)
//! - `Pluteus.icns`: macOS application icon (used by the `.app` bundle)
//! - `pluteus.png`: Linux desktop icon
//!
//! The iced window icon is written separately into `OUT_DIR` as raw RGBA.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// Size of the window icon, in pixels.
const ICON_SIZE: u32 = 256;

/// Size of the Linux desktop icon, in pixels.
const DESKTOP_ICON_SIZE: u32 = 512;

/// Color used for the SVG's `currentColor` strokes.
///
/// The Feather icon set inherits the text color; the window icon is static, so
/// the app's accent blue is used to keep the glyph visible on both light and
/// dark title bars. Change this to recolor the icon.
const ICON_COLOR: &str = "#5E7CE2";

/// ICO sizes embedded into the Windows executable.
const ICO_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];

/// ICNS entries for the macOS application bundle.
const ICNS_SIZES: [(&str, u32); 7] = [
    ("icp4", 16),
    ("icp5", 32),
    ("icp6", 64),
    ("ic07", 128),
    ("ic08", 256),
    ("ic09", 512),
    ("ic10", 1024),
];

fn main() {
    println!("cargo:rerun-if-changed=assets/feather.svg");

    let svg = fs::read_to_string("assets/feather.svg")
        .expect("assets/feather.svg should be readable")
        .replace("currentColor", ICON_COLOR);

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&svg, &options)
        .expect("assets/feather.svg should be valid SVG");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should be set"));

    // Window icon for iced.
    let window_icon = render(&tree, ICON_SIZE);
    fs::write(out_dir.join("feather_icon.rgba"), &window_icon)
        .expect("window icon should be writable");

    // Platform icons for packaging.
    let icons_dir = icons_dir(&out_dir).unwrap_or_else(|| out_dir.clone());
    fs::create_dir_all(&icons_dir).expect("icon directory should be creatable");

    let ico_entries: Vec<(u32, Vec<u8>)> = ICO_SIZES
        .iter()
        .map(|&size| (size, encode_png(&render(&tree, size), size)))
        .collect();
    fs::write(icons_dir.join("pluteus.ico"), build_ico(&ico_entries))
        .expect("Windows icon should be writable");

    let icns_entries: Vec<(&str, Vec<u8>)> = ICNS_SIZES
        .iter()
        .map(|&(kind, size)| (kind, encode_png(&render(&tree, size), size)))
        .collect();
    fs::write(icons_dir.join("Pluteus.icns"), build_icns(&icns_entries))
        .expect("macOS icon should be writable");

    fs::write(
        icons_dir.join("pluteus.png"),
        encode_png(&render(&tree, DESKTOP_ICON_SIZE), DESKTOP_ICON_SIZE),
    )
    .expect("Linux icon should be writable");

    #[cfg(windows)]
    embed_windows_icon(&icons_dir.join("pluteus.ico"));
}

/// Renders the SVG into straight RGBA pixels at the given square size.
fn render(tree: &resvg::usvg::Tree, size: u32) -> Vec<u8> {
    let source = tree.size();
    let scale =
        (size as f32 / source.width()).min(size as f32 / source.height());
    let dx = (size as f32 - source.width() * scale) / 2.0;
    let dy = (size as f32 - source.height() * scale) / 2.0;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)
        .expect("icon pixmap should be allocatable");

    let transform =
        resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, dx, dy);
    resvg::render(tree, transform, &mut pixmap.as_mut());

    let mut rgba = pixmap.take();
    unpremultiply(&mut rgba);
    rgba
}

/// `tiny-skia` stores premultiplied colors; icons expect straight RGBA.
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

/// Encodes straight RGBA pixels as an 8-bit PNG.
fn encode_png(rgba: &[u8], size: u32) -> Vec<u8> {
    let mut png = Vec::new();

    {
        let mut encoder = png::Encoder::new(&mut png, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().expect("PNG header should be writable");
        writer
            .write_image_data(rgba)
            .expect("PNG data should be writable");
    }

    png
}

/// Builds a PNG-based ICO container.
fn build_ico(entries: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut ico = Vec::new();

    ico.extend_from_slice(&0u16.to_le_bytes()); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // icon
    ico.extend_from_slice(&(entries.len() as u16).to_le_bytes());

    let mut offset = 6 + 16 * entries.len();

    for (size, png) in entries {
        let dimension = if *size >= 256 { 0 } else { *size as u8 };

        ico.push(dimension);
        ico.push(dimension);
        ico.push(0); // palette size
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&(offset as u32).to_le_bytes());

        offset += png.len();
    }

    for (_, png) in entries {
        ico.extend_from_slice(png);
    }

    ico
}

/// Builds a PNG-based ICNS container.
fn build_icns(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut body = Vec::new();

    for (kind, png) in entries {
        body.extend_from_slice(kind.as_bytes());
        body.extend_from_slice(&((8 + png.len()) as u32).to_be_bytes());
        body.extend_from_slice(png);
    }

    let mut icns = Vec::with_capacity(8 + body.len());
    icns.extend_from_slice(b"icns");
    icns.extend_from_slice(&((8 + body.len()) as u32).to_be_bytes());
    icns.extend_from_slice(&body);

    icns
}

/// Finds the profile directory that contains the build's executable, so the
/// generated icons end up next to it (`<profile>/pluteus-icons`).
fn icons_dir(out_dir: &Path) -> Option<PathBuf> {
    out_dir
        .ancestors()
        .find(|path| path.file_name() == Some(OsStr::new("build")))
        .and_then(Path::parent)
        .map(|profile| profile.join("pluteus-icons"))
}

/// Embeds the generated icon into the Windows executable.
#[cfg(windows)]
fn embed_windows_icon(icon: &Path) {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winres::WindowsResource::new();
    resource.set_icon(icon.to_str().expect("icon path should be UTF-8"));

    if let Err(error) = resource.compile() {
        panic!("failed to embed the Windows executable icon: {error}");
    }
}
