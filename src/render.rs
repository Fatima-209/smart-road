use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::config::{CENTER_X, CENTER_Y, LANE_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};

const CORRIDOR_HALF: f32 = LANE_WIDTH * 3.0; // 150.0, matches DESIGN.md

/// Every texture the renderer needs, loaded once up front. Decoded with the
/// pure-Rust `image` crate (not SDL2_image) so we don't need another native
/// DLL on top of SDL2 itself.
pub struct Assets<'a> {
    pub grass: Texture<'a>,
    pub road: Texture<'a>,
    pub car_black: Texture<'a>,
}

impl<'a> Assets<'a> {
    pub fn load(creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        Ok(Assets {
            grass: load_texture(creator, "assets/road/grass.png")?,
            road: load_texture(creator, "assets/road/road_mid.png")?,
            car_black: load_texture(creator, "assets/car_black.png")?,
        })
    }
}

fn load_texture<'a>(
    creator: &'a TextureCreator<WindowContext>,
    relative_path: &str,
) -> Result<Texture<'a>, String> {
    let full_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), relative_path);
    let image = image::open(&full_path)
        .map_err(|e| format!("failed to load {full_path}: {e}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();

    let mut texture = creator
        .create_texture_static(PixelFormatEnum::RGBA32, width, height)
        .map_err(|e| e.to_string())?;
    texture.set_blend_mode(BlendMode::Blend);
    texture
        .update(None, &image.into_raw(), (width * 4) as usize)
        .map_err(|e| e.to_string())?;
    Ok(texture)
}

/// Tiles the grass texture across the whole window as the base background.
pub fn draw_background(canvas: &mut Canvas<Window>, assets: &Assets) -> Result<(), String> {
    let tile = 128;
    let mut y = 0;
    while y < WINDOW_HEIGHT as i32 {
        let mut x = 0;
        while x < WINDOW_WIDTH as i32 {
            canvas.copy(&assets.grass, None, Some(Rect::new(x, y, tile as u32, tile as u32)))?;
            x += tile;
        }
        y += tile;
    }
    Ok(())
}

/// Draws the cross-shaped road surface (the two 300px-wide corridors from
/// DESIGN.md) by stretching the plain road texture to fill each corridor.
pub fn draw_road(canvas: &mut Canvas<Window>, assets: &Assets) -> Result<(), String> {
    let vertical = Rect::new(
        (CENTER_X - CORRIDOR_HALF) as i32,
        0,
        (CORRIDOR_HALF * 2.0) as u32,
        WINDOW_HEIGHT,
    );
    let horizontal = Rect::new(
        0,
        (CENTER_Y - CORRIDOR_HALF) as i32,
        WINDOW_WIDTH,
        (CORRIDOR_HALF * 2.0) as u32,
    );
    canvas.copy(&assets.road, None, Some(vertical))?;
    canvas.copy(&assets.road, None, Some(horizontal))?;
    Ok(())
}

/// Lane boundary lines for all 12 lanes, drawn only on the approach
/// segments (not through the intersection box itself, matching real
/// roads). The two median lines (x=CENTER_X and y=CENTER_Y, separating
/// opposing traffic) are drawn yellow; the rest are white.
pub fn draw_lane_lines(canvas: &mut Canvas<Window>) -> Result<(), String> {
    let offsets = [-CORRIDOR_HALF, -100.0, -50.0, 0.0, 50.0, 100.0, CORRIDOR_HALF];

    for &offset in &offsets {
        let x = (CENTER_X + offset) as i32;
        let color = if offset == 0.0 { Color::RGB(230, 200, 40) } else { Color::RGB(240, 240, 240) };
        canvas.set_draw_color(color);
        canvas.fill_rect(Rect::new(x, 0, 2, (CENTER_Y - CORRIDOR_HALF) as u32))?;
        canvas.fill_rect(Rect::new(
            x,
            (CENTER_Y + CORRIDOR_HALF) as i32,
            2,
            WINDOW_HEIGHT - (CENTER_Y + CORRIDOR_HALF) as u32,
        ))?;
    }

    for &offset in &offsets {
        let y = (CENTER_Y + offset) as i32;
        let color = if offset == 0.0 { Color::RGB(230, 200, 40) } else { Color::RGB(240, 240, 240) };
        canvas.set_draw_color(color);
        canvas.fill_rect(Rect::new(0, y, (CENTER_X - CORRIDOR_HALF) as u32, 2))?;
        canvas.fill_rect(Rect::new(
            (CENTER_X + CORRIDOR_HALF) as i32,
            y,
            WINDOW_WIDTH - (CENTER_X + CORRIDOR_HALF) as u32,
            2,
        ))?;
    }

    Ok(())
}

/// Draws one car sprite centered on (x, y), at the fixed on-screen size
/// from config.rs regardless of the source image's native resolution.
pub fn draw_car(canvas: &mut Canvas<Window>, texture: &Texture, x: f32, y: f32) -> Result<(), String> {
    use crate::config::{CAR_HEIGHT, CAR_WIDTH};
    let dest = Rect::new(
        (x - CAR_WIDTH / 2.0) as i32,
        (y - CAR_HEIGHT / 2.0) as i32,
        CAR_WIDTH as u32,
        CAR_HEIGHT as u32,
    );
    canvas.copy(texture, None, Some(dest))?;
    Ok(())
}
