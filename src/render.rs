use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::config::{
    CENTER_X, CENTER_Y, CORRIDOR_WIDTH as CORRIDOR_HALF, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::vehicle::{lane_center, Direction, Route};

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

/// Builds the unchanging road scene once. The main loop copies this texture
/// each frame and redraws only the moving vehicles.
pub fn create_static_scene<'a>(
    canvas: &mut Canvas<Window>,
    creator: &'a TextureCreator<WindowContext>,
    assets: &Assets,
) -> Result<Texture<'a>, String> {
    let mut scene = creator
        .create_texture_target(PixelFormatEnum::RGBA8888, WINDOW_WIDTH, WINDOW_HEIGHT)
        .map_err(|error| error.to_string())?;
    let mut drawing_result: Result<(), String> = Ok(());
    canvas
        .with_texture_canvas(&mut scene, |target| {
            drawing_result = (|| {
                draw_background(target, assets)?;
                draw_road(target, assets)?;
                draw_lane_lines(target)?;
                draw_route_markers(target)?;
                Ok(())
            })();
        })
        .map_err(|error| error.to_string())?;
    drawing_result?;
    Ok(scene)
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
            canvas.copy(
                &assets.grass,
                None,
                Some(Rect::new(x, y, tile as u32, tile as u32)),
            )?;
            x += tile;
        }
        y += tile;
    }
    Ok(())
}

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

pub fn draw_lane_lines(canvas: &mut Canvas<Window>) -> Result<(), String> {
    let offsets = [
        -CORRIDOR_HALF,
        -100.0,
        -50.0,
        0.0,
        50.0,
        100.0,
        CORRIDOR_HALF,
    ];

    for &offset in &offsets {
        let x = (CENTER_X + offset) as i32;
        let color = if offset == 0.0 {
            Color::RGB(230, 200, 40)
        } else {
            Color::RGB(240, 240, 240)
        };
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
        let color = if offset == 0.0 {
            Color::RGB(230, 200, 40)
        } else {
            Color::RGB(240, 240, 240)
        };
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

/// Paints the intended maneuver in each of the 12 approach lanes so drivers
/// can see the fixed lane-to-route mapping before entering the box.
pub fn draw_route_markers(canvas: &mut Canvas<Window>) -> Result<(), String> {
    for direction in [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ] {
        for route in [Route::Left, Route::Straight, Route::Right] {
            let lane = lane_center(direction, route);
            let (x, y) = match direction {
                Direction::North => (lane, CENTER_Y + CORRIDOR_HALF + 50.0),
                Direction::South => (lane, CENTER_Y - CORRIDOR_HALF - 50.0),
                Direction::East => (CENTER_X - CORRIDOR_HALF - 50.0, lane),
                Direction::West => (CENTER_X + CORRIDOR_HALF + 50.0, lane),
            };
            draw_route_arrow(canvas, x, y, outgoing_vector(direction, route))?;
        }
    }
    Ok(())
}

fn outgoing_vector(direction: Direction, route: Route) -> (f32, f32) {
    use Direction::*;
    match (direction, route) {
        (North, Route::Left) | (South, Route::Right) => (-1.0, 0.0),
        (North, Route::Right) | (South, Route::Left) => (1.0, 0.0),
        (East, Route::Left) | (West, Route::Right) => (0.0, -1.0),
        (East, Route::Right) | (West, Route::Left) => (0.0, 1.0),
        (North, Route::Straight) => (0.0, -1.0),
        (South, Route::Straight) => (0.0, 1.0),
        (East, Route::Straight) => (1.0, 0.0),
        (West, Route::Straight) => (-1.0, 0.0),
    }
}

fn draw_route_arrow(
    canvas: &mut Canvas<Window>,
    x: f32,
    y: f32,
    direction: (f32, f32),
) -> Result<(), String> {
    let (dx, dy) = direction;
    let tip = (x + dx * 13.0, y + dy * 13.0);
    let tail = (x - dx * 11.0, y - dy * 11.0);
    let normal = (-dy, dx);
    let wing_a = (
        tip.0 - dx * 7.0 + normal.0 * 6.0,
        tip.1 - dy * 7.0 + normal.1 * 6.0,
    );
    let wing_b = (
        tip.0 - dx * 7.0 - normal.0 * 6.0,
        tip.1 - dy * 7.0 - normal.1 * 6.0,
    );
    let color = Color::RGB(255, 211, 105);
    canvas.set_draw_color(color);
    for offset in -1..=1 {
        let ox = normal.0 * offset as f32;
        let oy = normal.1 * offset as f32;
        canvas.draw_line(
            ((tail.0 + ox) as i32, (tail.1 + oy) as i32),
            ((tip.0 + ox) as i32, (tip.1 + oy) as i32),
        )?;
    }
    canvas.draw_line(
        (tip.0 as i32, tip.1 as i32),
        (wing_a.0 as i32, wing_a.1 as i32),
    )?;
    canvas.draw_line(
        (tip.0 as i32, tip.1 as i32),
        (wing_b.0 as i32, wing_b.1 as i32),
    )?;
    Ok(())
}

pub fn draw_car(
    canvas: &mut Canvas<Window>,
    texture: &Texture,
    x: f32,
    y: f32,
    angle_degrees: f64,
) -> Result<(), String> {
    use crate::config::{CAR_HEIGHT, CAR_WIDTH};
    let dest = Rect::new(
        (x - CAR_WIDTH / 2.0) as i32,
        (y - CAR_HEIGHT / 2.0) as i32,
        CAR_WIDTH as u32,
        CAR_HEIGHT as u32,
    );
    canvas.copy_ex(texture, None, Some(dest), angle_degrees, None, false, false)?;
    Ok(())
}
