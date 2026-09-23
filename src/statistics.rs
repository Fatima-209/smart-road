use std::time::Duration;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::EventPump;

use crate::world::World;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 570;
const BG: Color = Color::RGB(11, 18, 30);
const PANEL: Color = Color::RGB(21, 34, 51);
const PANEL_EDGE: Color = Color::RGB(38, 57, 78);
const TEXT: Color = Color::RGB(230, 239, 247);
const MUTED: Color = Color::RGB(133, 157, 178);
const ACCENT: Color = Color::RGB(43, 211, 174);
const ACCENT_DARK: Color = Color::RGB(17, 96, 87);

pub fn show(video: &sdl2::VideoSubsystem, event_pump: &mut EventPump, world: &World) {
    let window = match video
        .window("Smart Road | Simulation Summary", WIDTH, HEIGHT)
        .position_centered()
        .build()
    {
        Ok(window) => window,
        Err(error) => {
            eprintln!("Could not open statistics window: {error}");
            return;
        }
    };
    let mut canvas = match window.into_canvas().build() {
        Ok(canvas) => canvas,
        Err(error) => {
            eprintln!("Could not draw statistics window: {error}");
            return;
        }
    };

    draw_summary(&mut canvas, world);
    canvas.present();

    'window: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape | Keycode::Return | Keycode::KpEnter),
                    ..
                }
                | Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => break 'window,
                Event::MouseButtonDown { x, y, .. }
                    if (450..=608).contains(&x) && (510..=554).contains(&y) =>
                {
                    break 'window;
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}

fn draw_summary(canvas: &mut Canvas<Window>, world: &World) {
    canvas.set_draw_color(BG);
    canvas.clear();
    fill(canvas, Rect::new(0, 0, WIDTH, 6), ACCENT);

    draw_text(canvas, "SIMULATION SUMMARY", 32, 26, 2, ACCENT);
    draw_text(canvas, "SMART ROAD", 32, 52, 4, TEXT);
    draw_text(canvas, "CROSSING PERFORMANCE", 34, 91, 1, MUTED);

    card(canvas, Rect::new(32, 116, 576, 92), ACCENT);
    draw_text(canvas, "VEHICLES PASSED", 54, 145, 2, MUTED);
    let count = world.completed().len().to_string();
    let count_width = text_width(&count, 7);
    draw_text(canvas, &count, 582 - count_width, 127, 7, ACCENT);

    let records = world.completed();
    let max_speed = records
        .iter()
        .map(|record| record.max_velocity)
        .reduce(f32::max)
        .map(|value| format!("{value:.0} PX/S"))
        .unwrap_or_else(|| "N/A".to_owned());
    let min_speed = records
        .iter()
        .map(|record| record.min_velocity)
        .reduce(f32::min)
        .map(|value| format!("{value:.0} PX/S"))
        .unwrap_or_else(|| "N/A".to_owned());
    let max_time = records
        .iter()
        .map(|record| record.elapsed)
        .max()
        .map(format_duration)
        .unwrap_or_else(|| "N/A".to_owned());
    let min_time = records
        .iter()
        .map(|record| record.elapsed)
        .min()
        .map(format_duration)
        .unwrap_or_else(|| "N/A".to_owned());

    metric_card(canvas, 32, 226, "MAX VELOCITY", &max_speed, ACCENT);
    metric_card(
        canvas,
        332,
        226,
        "MIN VELOCITY",
        &min_speed,
        Color::RGB(113, 180, 255),
    );
    metric_card(
        canvas,
        32,
        330,
        "MAX CROSSING TIME",
        &max_time,
        Color::RGB(255, 190, 91),
    );
    metric_card(
        canvas,
        332,
        330,
        "MIN CROSSING TIME",
        &min_time,
        Color::RGB(255, 190, 91),
    );

    card(
        canvas,
        Rect::new(32, 438, 576, 54),
        Color::RGB(255, 120, 110),
    );
    draw_text(canvas, "CLOSE CALLS", 54, 458, 2, MUTED);
    let calls = world.close_call_count().to_string();
    draw_text(canvas, &calls, 566 - text_width(&calls, 3), 451, 3, TEXT);

    fill(canvas, Rect::new(450, 510, 158, 44), ACCENT_DARK);
    draw_text(canvas, "CLOSE  [ESC]", 465, 524, 1, TEXT);
    draw_text(canvas, "SMART ROAD SIMULATION", 32, 528, 1, MUTED);
}

fn metric_card(
    canvas: &mut Canvas<Window>,
    x: i32,
    y: i32,
    label: &str,
    value: &str,
    accent: Color,
) {
    card(canvas, Rect::new(x, y, 276, 92), accent);
    draw_text(canvas, label, x + 18, y + 15, 1, MUTED);
    draw_text(canvas, value, x + 18, y + 43, 2, TEXT);
}

fn card(canvas: &mut Canvas<Window>, rect: Rect, accent: Color) {
    fill(
        canvas,
        Rect::new(rect.x(), rect.y() + 4, rect.width(), rect.height()),
        Color::RGB(6, 11, 20),
    );
    fill(canvas, rect, PANEL_EDGE);
    fill(
        canvas,
        Rect::new(
            rect.x() + 1,
            rect.y() + 1,
            rect.width() - 2,
            rect.height() - 2,
        ),
        PANEL,
    );
    fill(
        canvas,
        Rect::new(rect.x(), rect.y(), 4, rect.height()),
        accent,
    );
}

fn fill(canvas: &mut Canvas<Window>, rect: Rect, color: Color) {
    canvas.set_draw_color(color);
    let _ = canvas.fill_rect(rect);
}

fn format_duration(duration: Duration) -> String {
    format!("{:.2} S", duration.as_secs_f32())
}

fn text_width(text: &str, scale: i32) -> i32 {
    text.chars().count() as i32 * 6 * scale
}

fn draw_text(canvas: &mut Canvas<Window>, text: &str, x: i32, y: i32, scale: i32, color: Color) {
    let mut cursor_x = x;
    for character in text.chars() {
        let glyph = glyph(character);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    fill(
                        canvas,
                        Rect::new(
                            cursor_x + column * scale,
                            y + row as i32 * scale,
                            scale as u32,
                            scale as u32,
                        ),
                        color,
                    );
                }
            }
        }
        cursor_x += 6 * scale;
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        _ => [0; 7],
    }
}
