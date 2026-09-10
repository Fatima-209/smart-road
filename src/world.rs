use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::{RANDOM_SPAWN_INTERVAL_MS, SAFETY_DISTANCE, SPAWN_COOLDOWN_MS};
use crate::vehicle::{spawn_position, Direction, Route, Vehicle};

pub struct World {
    pub vehicles: Vec<Vehicle>,
    next_id: u32,
    last_manual_spawn: HashMap<Direction, Instant>,
    last_random_spawn: Instant,
}

impl World {
    pub fn new() -> Self {
        World {
            vehicles: Vec::new(),
            next_id: 0,
            last_manual_spawn: HashMap::new(),
            last_random_spawn: Instant::now(),
        }
    }

    /// Manual (arrow-key) spawn attempt. Returns true if a vehicle was
    /// actually created. Never queues a rejected attempt - it's just
    /// dropped, per DESIGN.md.
    pub fn try_spawn(&mut self, direction: Direction, route: Route, now: Instant) -> bool {
        if let Some(&last) = self.last_manual_spawn.get(&direction) {
            if now.duration_since(last) < Duration::from_millis(SPAWN_COOLDOWN_MS) {
                return false;
            }
        }
        if !self.spawn_tile_clear(direction, route) {
            return false;
        }
        self.spawn(direction, route);
        self.last_manual_spawn.insert(direction, now);
        true
    }

    /// R-key continuous random spawn attempt, gated by its own interval
    /// rather than the manual per-direction cooldowns.
    pub fn try_random_spawn(&mut self, direction: Direction, route: Route, now: Instant) -> bool {
        if now.duration_since(self.last_random_spawn) < Duration::from_millis(RANDOM_SPAWN_INTERVAL_MS)
        {
            return false;
        }
        if !self.spawn_tile_clear(direction, route) {
            return false;
        }
        self.spawn(direction, route);
        self.last_random_spawn = now;
        true
    }

    fn spawn(&mut self, direction: Direction, route: Route) {
        let vehicle = Vehicle::new(self.next_id, direction, route);
        self.next_id += 1;
        self.vehicles.push(vehicle);
    }

    /// The spawn point is only clear if no existing vehicle in the same
    /// lane is still sitting within a safety distance of it.
    fn spawn_tile_clear(&self, direction: Direction, route: Route) -> bool {
        let (spawn_x, spawn_y) = spawn_position(direction, route);
        !self.vehicles.iter().any(|v| {
            v.direction == direction
                && v.route == route
                && distance(v.x, v.y, spawn_x, spawn_y) < SAFETY_DISTANCE
        })
    }
}

fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
}
