use std::time::Instant;

use rand::Rng;

use crate::config::{
    CAR_HEIGHT, CENTER_X, CENTER_Y, CORRIDOR_WIDTH, LANE_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH,
};

/// Current heading of a vehicle; arrow commands choose its spawn heading.
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
    /// Total path length traveled from its spawn point.
    pub distance_traveled: f32,
    /// Remaining route length until the vehicle center clears the box.
    pub distance_remaining: f32,
    /// Set on first entry into REACTION_DISTANCE; starts crossing-time stats.
    pub detected_at: Option<Instant>,
    /// Set on clearing the far edge of the intersection box.
    pub cleared_at: Option<Instant>,
    /// Speeds sampled from detection through clearance for crossing stats.
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
            distance_remaining: total_travel_distance(direction, route),
            detected_at: None,
            cleared_at: None,
            velocity_history: Vec::new(),
            stats_recorded: false,
            turn_completed: route == Route::Straight,
        }
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
        if self.detected_at.is_some() {
            self.velocity_history.push(velocity);
        }
    }

    /// Advances distance by velocity * dt, switching heading at a turn point.
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

    pub fn has_left_window(&self) -> bool {
        match self.direction {
            Direction::North => self.y < -CAR_HEIGHT,
            Direction::South => self.y > WINDOW_HEIGHT as f32 + CAR_HEIGHT,
            Direction::East => self.x > WINDOW_WIDTH as f32 + CAR_HEIGHT,
            Direction::West => self.x < -CAR_HEIGHT,
        }
    }

    pub fn is_inside_box(&self) -> bool {
        (CENTER_X - CORRIDOR_WIDTH..=CENTER_X + CORRIDOR_WIDTH).contains(&self.x)
            && (CENTER_Y - CORRIDOR_WIDTH..=CENTER_Y + CORRIDOR_WIDTH).contains(&self.y)
    }

    pub fn distance_to_box(&self) -> f32 {
        match self.direction {
            Direction::North => (self.y - (CENTER_Y + CORRIDOR_WIDTH)).max(0.0),
            Direction::South => ((CENTER_Y - CORRIDOR_WIDTH) - self.y).max(0.0),
            Direction::East => ((CENTER_X - CORRIDOR_WIDTH) - self.x).max(0.0),
            Direction::West => (self.x - (CENTER_X + CORRIDOR_WIDTH)).max(0.0),
        }
    }

    pub fn has_passed_box(&self) -> bool {
        match self.direction {
            Direction::North => self.y < CENTER_Y - CORRIDOR_WIDTH,
            Direction::South => self.y > CENTER_Y + CORRIDOR_WIDTH,
            Direction::East => self.x > CENTER_X + CORRIDOR_WIDTH,
            Direction::West => self.x < CENTER_X - CORRIDOR_WIDTH,
        }
    }
}

pub fn turn_direction(from: Direction, route: Route) -> Direction {
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

pub fn turn_point(direction: Direction, route: Route) -> (f32, f32) {
    let out = turn_direction(direction, route);
    let entry_lane = lane_center(direction, route);
    let exit_lane = lane_center(out, route);
    match direction {
        Direction::North | Direction::South => (entry_lane, exit_lane),
        Direction::East | Direction::West => (exit_lane, entry_lane),
    }
}

pub fn heading_vector(direction: Direction) -> (f32, f32) {
    match direction {
        Direction::North => (0.0, -1.0),
        Direction::South => (0.0, 1.0),
        Direction::East => (1.0, 0.0),
        Direction::West => (-1.0, 0.0),
    }
}

pub fn facing_angle_degrees(direction: Direction) -> f64 {
    match direction {
        Direction::North => 0.0,
        Direction::East => 90.0,
        Direction::South => 180.0,
        Direction::West => 270.0,
    }
}

pub fn random_route() -> Route {
    match rand::thread_rng().gen_range(0..3) {
        0 => Route::Left,
        1 => Route::Straight,
        _ => Route::Right,
    }
}

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

    #[test]
    fn remaining_distance_reaches_zero_when_each_route_clears_the_box() {
        use Direction::*;
        for direction in [North, South, East, West] {
            for route in [Route::Left, Route::Straight, Route::Right] {
                let mut vehicle = Vehicle::new(0, direction, route);
                vehicle.set_velocity(200.0);
                for _ in 0..1_000 {
                    if vehicle.has_passed_box() {
                        break;
                    }
                    vehicle.advance(0.016);
                }
                assert!(
                    vehicle.has_passed_box(),
                    "{direction:?} {route:?} did not clear"
                );
                assert_eq!(
                    vehicle.distance_remaining, 0.0,
                    "bad remaining distance for {direction:?} {route:?}"
                );
            }
        }
    }
}

fn total_travel_distance(direction: Direction, route: Route) -> f32 {
    let (spawn_x, spawn_y) = spawn_position(direction, route);
    if route == Route::Straight {
        return match direction {
            Direction::North => spawn_y - (CENTER_Y - CORRIDOR_WIDTH),
            Direction::South => CENTER_Y + CORRIDOR_WIDTH - spawn_y,
            Direction::East => CENTER_X + CORRIDOR_WIDTH - spawn_x,
            Direction::West => spawn_x - (CENTER_X - CORRIDOR_WIDTH),
        };
    }

    let (turn_x, turn_y) = turn_point(direction, route);
    let to_turn = match direction {
        Direction::North => spawn_y - turn_y,
        Direction::South => turn_y - spawn_y,
        Direction::East => turn_x - spawn_x,
        Direction::West => spawn_x - turn_x,
    };
    let outgoing = turn_direction(direction, route);
    let from_turn_to_clear = match outgoing {
        Direction::North => turn_y - (CENTER_Y - CORRIDOR_WIDTH),
        Direction::South => CENTER_Y + CORRIDOR_WIDTH - turn_y,
        Direction::East => CENTER_X + CORRIDOR_WIDTH - turn_x,
        Direction::West => turn_x - (CENTER_X - CORRIDOR_WIDTH),
    };
    to_turn + from_turn_to_clear
}
