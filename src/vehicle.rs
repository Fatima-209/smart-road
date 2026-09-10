use std::time::Instant;

use crate::config::{CENTER_X, CENTER_Y, LANE_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};

/// The direction a vehicle travels *toward* (its heading), matching the
/// keyboard command that spawned it (Arrow Up -> heading North, etc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

/// A vehicle's route fully determines its lane (see DESIGN.md) and never
/// changes after spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Left,
    Straight,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleState {
    Approaching,
    InIntersection,
    Cleared,
}

pub struct Vehicle {
    pub id: u32,
    pub direction: Direction,
    pub route: Route,
    pub x: f32,
    pub y: f32,
    pub velocity: f32,
    pub state: VehicleState,
    pub distance_traveled: f32,
    pub distance_remaining: f32,
    /// Set once, the first time the intersection algorithm evaluates this
    /// vehicle (Stage 3). Crossing time is measured from here, not spawn.
    pub detected_at: Option<Instant>,
    pub cleared_at: Option<Instant>,
    /// Every velocity this vehicle has held during its crossing, so stats
    /// can be sampled from the full history instead of a single snapshot.
    pub velocity_history: Vec<f32>,
}

impl Vehicle {
    pub fn new(id: u32, direction: Direction, route: Route) -> Self {
        let (x, y) = spawn_position(direction, route);
        Vehicle {
            id,
            direction,
            route,
            x,
            y,
            velocity: 0.0,
            state: VehicleState::Approaching,
            distance_traveled: 0.0,
            distance_remaining: total_travel_distance(direction),
            detected_at: None,
            cleared_at: None,
            velocity_history: Vec::new(),
        }
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
        self.velocity_history.push(velocity);
    }
}

/// The fixed centerline (the coordinate perpendicular to travel) for a
/// given (direction, route) lane. See DESIGN.md for the derivation:
/// right-hand traffic, left-turn lanes innermost (median-adjacent),
/// right-turn lanes outermost (curb-adjacent).
pub fn lane_center(direction: Direction, route: Route) -> f32 {
    let offset = match route {
        Route::Left => LANE_WIDTH * 0.5,
        Route::Straight => LANE_WIDTH * 1.5,
        Route::Right => LANE_WIDTH * 2.5,
    };
    match direction {
        Direction::North => CENTER_X + offset,
        Direction::South => CENTER_X - offset,
        Direction::East => CENTER_Y + offset,
        Direction::West => CENTER_Y - offset,
    }
}

/// Spawn point just outside the window edge, on the lane's centerline.
pub fn spawn_position(direction: Direction, route: Route) -> (f32, f32) {
    let lane = lane_center(direction, route);
    match direction {
        Direction::North => (lane, WINDOW_HEIGHT as f32),
        Direction::South => (lane, 0.0),
        Direction::East => (0.0, lane),
        Direction::West => (WINDOW_WIDTH as f32, lane),
    }
}

/// Straight-line distance from a fresh spawn point to the far edge of the
/// window along the direction of travel. Used as the initial
/// `distance_remaining`; Stage 2 will refine this once turn paths exist.
fn total_travel_distance(direction: Direction) -> f32 {
    match direction {
        Direction::North | Direction::South => WINDOW_HEIGHT as f32,
        Direction::East | Direction::West => WINDOW_WIDTH as f32,
    }
}
