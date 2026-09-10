mod config;
mod render;
mod vehicle;
mod world;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

use config::{WINDOW_HEIGHT, WINDOW_WIDTH};
use render::Assets;
use vehicle::{lane_center, Direction, Route};
use world::World;

fn main() {
    // Stage 0 smoke test: confirm World/Vehicle compile and the spawn
    // cooldown + spawn-tile-clear logic from DESIGN.md actually behaves.
    let mut world = World::new();
    let now = Instant::now();
    let first = world.try_spawn(Direction::North, Route::Straight, now);
    let immediate_repeat = world.try_spawn(Direction::North, Route::Straight, now);
    println!(
        "spawned {} vehicle(s); first spawn ok={first}, immediate repeat blocked (should be false)={immediate_repeat}",
        world.vehicles.len()
    );

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

    let texture_creator = canvas.texture_creator();
    let assets = Assets::load(&texture_creator).expect("failed to load assets");

    // Fixed test position (Stage 1): the North/Straight lane, partway up
    // from its spawn edge, just to confirm the asset pipeline works.
    let test_car_x = lane_center(Direction::North, Route::Straight);
    let test_car_y = WINDOW_HEIGHT as f32 - 150.0;

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

        render::draw_background(&mut canvas, &assets).expect("draw_background failed");
        render::draw_road(&mut canvas, &assets).expect("draw_road failed");
        render::draw_lane_lines(&mut canvas).expect("draw_lane_lines failed");
        render::draw_car(&mut canvas, &assets.car_black, test_car_x, test_car_y)
            .expect("draw_car failed");
        canvas.present();

        std::thread::sleep(Duration::from_millis(1000 / 60));
    }
}
