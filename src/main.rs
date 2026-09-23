mod config;
mod render;
mod statistics;
mod vehicle;
mod world;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

use config::{WINDOW_HEIGHT, WINDOW_WIDTH};
use render::Assets;
use vehicle::{facing_angle_degrees, random_route, Direction};
use world::World;

fn main() {
    let mut world = World::new();

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
    let background = match render::create_static_scene(&mut canvas, &texture_creator, &assets) {
        Ok(background) => Some(background),
        Err(error) => {
            eprintln!("Could not cache the road scene; rendering it each frame: {error}");
            None
        }
    };

    let mut event_pump = sdl_context
        .event_pump()
        .expect("failed to create event pump");
    let mut last_frame = Instant::now();
    let mut show_statistics = false;

    'running: loop {
        let now = Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    show_statistics = true;
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } => {
                    world.try_spawn(Direction::North, random_route(), now);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } => {
                    world.try_spawn(Direction::South, random_route(), now);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    repeat: false,
                    ..
                } => {
                    world.try_spawn(Direction::East, random_route(), now);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    repeat: false,
                    ..
                } => {
                    world.try_spawn(Direction::West, random_route(), now);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } => {
                    world.toggle_random_mode(now);
                }
                _ => {}
            }
        }

        let dt = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;
        world.random_tick(now);
        world.update(dt);

        if let Some(background) = &background {
            canvas
                .copy(background, None, None)
                .expect("draw_static_scene failed");
        } else {
            render::draw_background(&mut canvas, &assets).expect("draw_background failed");
            render::draw_road(&mut canvas, &assets).expect("draw_road failed");
            render::draw_lane_lines(&mut canvas).expect("draw_lane_lines failed");
            render::draw_route_markers(&mut canvas).expect("draw_route_markers failed");
        }
        for vehicle in &world.vehicles {
            let angle = facing_angle_degrees(vehicle.direction);
            render::draw_car(&mut canvas, &assets.car_black, vehicle.x, vehicle.y, angle)
                .expect("draw_car failed");
        }
        canvas.present();

        std::thread::sleep(Duration::from_millis(1000 / 60));
    }

    if show_statistics {
        statistics::show(&video_subsystem, &mut event_pump, &world);
    }
}
