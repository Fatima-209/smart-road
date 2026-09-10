use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::{
    RANDOM_SPAWN_INTERVAL_MS, REACTION_DISTANCE, SAFETY_DISTANCE, SPAWN_COOLDOWN_MS,
    VELOCITY_MEDIUM, VELOCITY_SLOW, VELOCITY_STOPPED,
};
use crate::vehicle::{heading_vector, is_perpendicular, spawn_position, Direction, Route, Vehicle};

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

    /// Decides each vehicle's speed for this tick, applies it, then moves
    /// everyone and drops any that have fully driven off-screen. Stage 4
    /// will need to capture stats (crossing time, etc.) from a vehicle
    /// right before it's removed here.
    pub fn update(&mut self, dt: f32) {
        let velocities: Vec<f32> = (0..self.vehicles.len())
            .map(|i| self.desired_velocity(i))
            .collect();

        for (vehicle, velocity) in self.vehicles.iter_mut().zip(velocities) {
            vehicle.set_velocity(velocity);
            vehicle.advance(dt);
        }

        self.vehicles.retain(|v| !v.has_left_window());
    }

    /// How fast `vehicles[index]` should go this tick, based on every other
    /// vehicle currently in the world. Two independent rules apply, and the
    /// vehicle obeys whichever is more restrictive:
    /// - don't run into the vehicle ahead of it in the same lane;
    /// - don't enter the intersection box while a perpendicular vehicle
    ///   that got there first (or is already inside) is still using it.
    ///
    /// Straight-line-only limitation: only perpendicular directions are
    /// treated as conflicting, since every route currently drives straight
    /// through its spawn lane. Once turning is implemented, a turning
    /// vehicle's real path can cross lanes its spawn direction alone
    /// wouldn't suggest, and this will need to account for that.
    fn desired_velocity(&self, index: usize) -> f32 {
        let vehicle = &self.vehicles[index];
        let mut target = VELOCITY_MEDIUM;

        for (other_index, other) in self.vehicles.iter().enumerate() {
            if other_index == index {
                continue;
            }

            if other.direction == vehicle.direction && other.route == vehicle.route {
                if let Some(gap) = following_gap(vehicle, other) {
                    target = target.min(speed_for_gap(gap));
                }
            } else if is_perpendicular(vehicle.direction, other.direction)
                && !vehicle.is_inside_box()
                && !vehicle.has_passed_box()
                && vehicle.distance_to_box() < REACTION_DISTANCE
                && !other.has_passed_box()
                && (other.is_inside_box() || other.distance_to_box() < REACTION_DISTANCE)
                && other_has_priority(vehicle, other)
            {
                target = target.min(speed_for_gap(vehicle.distance_to_box()));
            }
        }

        target
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

/// How far `other` is ahead of `vehicle` along `vehicle`'s direction of
/// travel. `None` if `other` isn't ahead (same lane, but behind or beside).
fn following_gap(vehicle: &Vehicle, other: &Vehicle) -> Option<f32> {
    let (dx, dy) = heading_vector(vehicle.direction);
    let ahead = (other.x - vehicle.x) * dx + (other.y - vehicle.y) * dy;
    (ahead > 0.0).then_some(ahead)
}

/// True if `other` should go first at a perpendicular conflict: already
/// inside the box, or spawned earlier (lower id). Deliberately NOT based on
/// live distance-to-box - that seems more "fair" (whoever's closer goes
/// first) but breaks the moment one vehicle stops: its distance freezes
/// while the other's keeps shrinking, so the two vehicles' priority checks
/// can disagree with each other mid-negotiation (each thinks the other
/// should go), and they swap who's yielding instead of one committing.
/// Id is fixed at spawn time, so it can never flip-flop like that - exactly
/// one of any two conflicting vehicles yields, consistently, for as long as
/// the conflict lasts.
fn other_has_priority(vehicle: &Vehicle, other: &Vehicle) -> bool {
    other.is_inside_box() || other.id < vehicle.id
}

/// Maps a gap (to a vehicle ahead, or to the intersection box) down to one
/// of the 3 velocity tiers: full speed while there's room, slow down inside
/// REACTION_DISTANCE, stop before closing to less than SAFETY_DISTANCE.
fn speed_for_gap(gap: f32) -> f32 {
    if gap < SAFETY_DISTANCE {
        VELOCITY_STOPPED
    } else if gap < REACTION_DISTANCE {
        VELOCITY_SLOW
    } else {
        VELOCITY_MEDIUM
    }
}
