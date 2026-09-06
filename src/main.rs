use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 900;

fn main() {
    let sdl_context = sdl2::init().expect("failed to init SDL2");
    let video_subsystem = sdl_context.video().expect("failed to init video subsystem");

    let window = video_subsystem
        .window("Smart Road", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .expect("failed to create window");

    let mut canvas = window
        .into_canvas()
        .build()
        .expect("failed to create canvas");

    let mut event_pump = sdl_context.event_pump().expect("failed to create event pump");

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(40, 40, 40));
        canvas.clear();
        canvas.present();

        std::thread::sleep(Duration::from_millis(1000 / 60));
    }
}
