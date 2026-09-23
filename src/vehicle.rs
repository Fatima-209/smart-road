use std::time::Instant;

use rand::Rng;

use crate::config::{
    CAR_HEIGHT, CENTER_X, CENTER_Y, CORRIDOR_WIDTH, LANE_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH,
};

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
    pub stats_recorded: bool,
    pub turn_completed: bool,
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
            stats_recorded: false,
            turn_completed: route == Route::Straight,
        }
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
        self.velocity_history.push(velocity);
    }

    /// Moves along the entry lane to the intersection turn point, rotates
    /// the heading, then follows the corresponding exit lane.
    pub fn advance(&mut self, dt: f32) {
        let mut remaining = self.velocity * dt;
        while remaining > 0.0 {
            let (dx, dy) = heading_vector(self.direction);
            let turn_at = if !self.turn_completed {
                Some(turn_point(self.direction, self.route))
            } else {
                None
            };
            if let Some((tx, ty)) = turn_at {
                let to_turn = if dx != 0.0 {
                    (tx - self.x) * dx
                } else {
                    (ty - self.y) * dy
                };
                if to_turn >= 0.0 && to_turn <= remaining {
                    self.x += dx * to_turn;
                    self.y += dy * to_turn;
                    self.distance_traveled += to_turn;
                    self.distance_remaining = (self.distance_remaining - to_turn).max(0.0);
                    remaining -= to_turn;
                    self.direction = turn_direction(self.direction, self.route);
                    self.turn_completed = true;
                    continue;
                }
            }
            self.x += dx * remaining;
            self.y += dy * remaining;
            self.distance_traveled += remaining;
            self.distance_remaining = (self.distance_remaining - remaining).max(0.0);
            remaining = 0.0;
        }
    }

    /// True once the vehicle has fully driven past the far edge of the
    /// window (used to remove it from the world).
    pub fn has_left_window(&self) -> bool {
        match self.direction {
            Direction::North => self.y < -CAR_HEIGHT,
            Direction::South => self.y > WINDOW_HEIGHT as f32 + CAR_HEIGHT,
            Direction::East => self.x > WINDOW_WIDTH as f32 + CAR_HEIGHT,
            Direction::West => self.x < -CAR_HEIGHT,
        }
    }

    /// True while any part of the vehicle's travel puts it within the
    /// shared 300x300 intersection box (see DESIGN.md).
    pub fn is_inside_box(&self) -> bool {
        (CENTER_X - CORRIDOR_WIDTH..=CENTER_X + CORRIDOR_WIDTH).contains(&self.x)
            && (CENTER_Y - CORRIDOR_WIDTH..=CENTER_Y + CORRIDOR_WIDTH).contains(&self.y)
    }

    /// How far the vehicle still has to travel before reaching the near
    /// edge of the intersection box. 0 once it's inside OR already past -
    /// callers that care about the difference must also check
    /// `is_inside_box`/`has_passed_box`, since this alone can't tell "about
    /// to arrive" apart from "long gone".
    pub fn distance_to_box(&self) -> f32 {
        match self.direction {
            Direction::North => (self.y - (CENTER_Y + CORRIDOR_WIDTH)).max(0.0),
            Direction::South => ((CENTER_Y - CORRIDOR_WIDTH) - self.y).max(0.0),
            Direction::East => ((CENTER_X - CORRIDOR_WIDTH) - self.x).max(0.0),
            Direction::West => (self.x - (CENTER_X + CORRIDOR_WIDTH)).max(0.0),
        }
    }

    /// True once the vehicle has driven all the way past the far edge of
    /// the box. Without this, a vehicle that's long gone still reads as
    /// "distance to box = 0" from `distance_to_box`, which looks identical
    /// to "about to arrive" and would make other traffic yield to it
    /// forever.
    pub fn has_passed_box(&self) -> bool {
        match self.direction {
            Direction::North => self.y < CENTER_Y - CORRIDOR_WIDTH,
            Direction::South => self.y > CENTER_Y + CORRIDOR_WIDTH,
            Direction::East => self.x > CENTER_X + CORRIDOR_WIDTH,
            Direction::West => self.x < CENTER_X - CORRIDOR_WIDTH,
        }
    }
}

fn turn_direction(from: Direction, route: Route) -> Direction {
    use Direction::*;
    match (from, route) {
        (North, Route::Left) => West,
        (North, Route::Right) => East,
        (South, Route::Left) => East,
        (South, Route::Right) => West,
        (East, Route::Left) => North,
        (East, Route::Right) => South,
        (West, Route::Left) => South,
        (West, Route::Right) => North,
        (d, Route::Straight) => d,
    }
}

fn turn_point(direction: Direction, route: Route) -> (f32, f32) {
    let out = turn_direction(direction, route);
    let entry_lane = lane_center(direction, route);
    let exit_lane = lane_center(out, route);
    match direction {
        Direction::North | Direction::South => (entry_lane, exit_lane),
        Direction::East | Direction::West => (exit_lane, entry_lane),
    }
}

/// The unit vector a vehicle moves along for its heading (screen space:
/// y grows downward).
pub fn heading_vector(direction: Direction) -> (f32, f32) {
    match direction {
        Direction::North => (0.0, -1.0),
        Direction::South => (0.0, 1.0),
        Direction::East => (1.0, 0.0),
        Direction::West => (-1.0, 0.0),
    }
}

/// Degrees to rotate the sprite (clockwise) so it visually faces its
/// direction of travel. The source art (car_black.png) is drawn facing
/// up/North, so North needs no rotation. Recomputed from the vehicle's
/// current direction every frame by the caller, not stored/hardcoded -
/// once turning is implemented, `direction` itself will change mid-route
/// and this will automatically follow it.
pub fn facing_angle_degrees(direction: Direction) -> f64 {
    match direction {
        Direction::North => 0.0,
        Direction::East => 90.0,
        Direction::South => 180.0,
        Direction::West => 270.0,
    }
}

/// Picks one of the 3 routes with equal probability. Spec requires spawns
/// to pick a random route within the chosen direction, not just a random
/// direction.
pub fn random_route() -> Route {
    match rand::thread_rng().gen_range(0..3) {
        0 => Route::Left,
        1 => Route::Straight,
        _ => Route::Right,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_routes_change_heading_at_the_turn_point() {
        use Direction::*;
        let cases = [
            (North, Route::Left, West),
            (North, Route::Right, East),
            (South, Route::Left, East),
            (South, Route::Right, West),
            (East, Route::Left, North),
            (East, Route::Right, South),
            (West, Route::Left, South),
            (West, Route::Right, North),
        ];
        for (from, route, expected) in cases {
            let mut vehicle = Vehicle::new(0, from, route);
            vehicle.set_velocity(200.0);
            for _ in 0..500 {
                vehicle.advance(0.016);
                if vehicle.turn_completed {
                    break;
                }
            }
            assert!(
                vehicle.turn_completed,
                "{from:?} {route:?} never reached its turn point"
            );
            assert_eq!(
                vehicle.direction, expected,
                "wrong turn for {from:?} {route:?}"
            );
        }
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
