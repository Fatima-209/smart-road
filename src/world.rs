use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::config::{
    RANDOM_SPAWN_INTERVAL_MS, REACTION_DISTANCE, SAFETY_DISTANCE, SPAWN_COOLDOWN_MS, VELOCITY_FAST,
    VELOCITY_MEDIUM, VELOCITY_SLOW, VELOCITY_STOPPED,
};
use crate::vehicle::{heading_vector, spawn_position, Direction, Route, Vehicle};

pub struct World {
    pub vehicles: Vec<Vehicle>,
    next_id: u32,
    last_manual_spawn: HashMap<Direction, Instant>,
    last_random_spawn: Instant,
    random_mode: bool,
    completed: Vec<CompletedVehicle>,
    close_calls: HashSet<(u32, u32)>,
}

#[derive(Clone, Debug)]
pub struct CompletedVehicle {
    pub elapsed: Duration,
    pub min_velocity: f32,
    pub max_velocity: f32,
}

impl World {
    pub fn new() -> Self {
        World {
            vehicles: Vec::new(),
            next_id: 0,
            last_manual_spawn: HashMap::new(),
            last_random_spawn: Instant::now(),
            random_mode: false,
            completed: Vec::new(),
            close_calls: HashSet::new(),
        }
    }

    pub fn set_random_mode(&mut self, enabled: bool) {
        self.random_mode = enabled;
    }

    pub fn random_tick(&mut self, now: Instant) {
        if self.random_mode {
            self.try_random_spawn(random_direction(), crate::vehicle::random_route(), now);
        }
    }

    pub fn completed(&self) -> &[CompletedVehicle] {
        &self.completed
    }
    pub fn close_call_count(&self) -> usize {
        self.close_calls.len()
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
        if now.duration_since(self.last_random_spawn)
            < Duration::from_millis(RANDOM_SPAWN_INTERVAL_MS)
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
    /// everyone, records vehicles as they clear the box, and removes cars
    /// only after they leave the visible road.
    pub fn update(&mut self, dt: f32) {
        for vehicle in &mut self.vehicles {
            if vehicle.detected_at.is_none()
                && !vehicle.has_passed_box()
                && vehicle.distance_to_box() <= REACTION_DISTANCE
            {
                vehicle.detected_at = Some(Instant::now());
            }
        }
        let velocities: Vec<f32> = (0..self.vehicles.len())
            .map(|i| self.desired_velocity(i))
            .collect();

        for (vehicle, velocity) in self.vehicles.iter_mut().zip(velocities) {
            vehicle.set_velocity(velocity);
            vehicle.advance(dt);
            if vehicle.is_inside_box() {
                vehicle.state = crate::vehicle::VehicleState::InIntersection;
            }
            if vehicle.has_passed_box() && vehicle.state != crate::vehicle::VehicleState::Cleared {
                vehicle.state = crate::vehicle::VehicleState::Cleared;
                vehicle.cleared_at = Some(Instant::now());
            }
        }

        for vehicle in &mut self.vehicles {
            if vehicle.state == crate::vehicle::VehicleState::Cleared && !vehicle.stats_recorded {
                let elapsed = vehicle
                    .cleared_at
                    .zip(vehicle.detected_at)
                    .map(|(end, start)| end.duration_since(start))
                    .unwrap_or_default();
                let min_velocity = vehicle
                    .velocity_history
                    .iter()
                    .copied()
                    .reduce(f32::min)
                    .unwrap_or(0.0);
                let max_velocity = vehicle
                    .velocity_history
                    .iter()
                    .copied()
                    .reduce(f32::max)
                    .unwrap_or(0.0);
                self.completed.push(CompletedVehicle {
                    elapsed,
                    min_velocity,
                    max_velocity,
                });
                vehicle.stats_recorded = true;
            }
        }

        for i in 0..self.vehicles.len() {
            for j in i + 1..self.vehicles.len() {
                let a = &self.vehicles[i];
                let b = &self.vehicles[j];
                if distance(a.x, a.y, b.x, b.y) < crate::config::CLOSE_CALL_THRESHOLD {
                    self.close_calls.insert((a.id.min(b.id), a.id.max(b.id)));
                }
            }
        }

        self.vehicles.retain(|v| !v.has_left_window());
    }

    /// How fast `vehicles[index]` should go this tick, based on every other
    /// vehicle currently in the world. Two independent rules apply, and the
    /// vehicle obeys whichever is more restrictive:
    /// - don't run into the vehicle ahead of it in the same lane;
    /// - only one vehicle at a time may approach/use the intersection box;
    ///   fixed spawn-id priority avoids negotiation deadlocks.
    fn desired_velocity(&self, index: usize) -> f32 {
        let vehicle = &self.vehicles[index];
        let mut target = VELOCITY_FAST;

        for (other_index, other) in self.vehicles.iter().enumerate() {
            if other_index == index {
                continue;
            }

            if other.direction == vehicle.direction && other.route == vehicle.route {
                if let Some(gap) = following_gap(vehicle, other) {
                    target = target.min(speed_for_gap(gap));
                }
            }
            if !vehicle.is_inside_box()
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

fn random_direction() -> Direction {
    use rand::Rng;
    match rand::thread_rng().gen_range(0..4) {
        0 => Direction::North,
        1 => Direction::South,
        2 => Direction::East,
        _ => Direction::West,
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

/// True if `other` should go first in the single-vehicle intersection queue:
/// already inside the box, or spawned earlier (lower id). Deliberately NOT based on
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
    } else if gap < REACTION_DISTANCE * 2.0 {
        VELOCITY_MEDIUM
    } else {
        VELOCITY_FAST
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CENTER_X, CENTER_Y, CORRIDOR_WIDTH};

    #[test]
    fn conflicting_approaches_use_the_box_one_at_a_time() {
        let mut world = World::new();
        world.spawn(Direction::North, Route::Straight);
        world.spawn(Direction::East, Route::Straight);
        world.vehicles[0].y = CENTER_Y + CORRIDOR_WIDTH + 130.0;
        world.vehicles[1].x = CENTER_X - CORRIDOR_WIDTH - 130.0;

        for _ in 0..2_000 {
            world.update(0.016);
            let in_box = world.vehicles.iter().filter(|v| v.is_inside_box()).count();
            assert!(in_box <= 1, "multiple vehicles entered the box together");
        }
        assert_eq!(
            world.completed().len(),
            2,
            "both vehicles should clear the box"
        );
        assert_eq!(world.close_call_count(), 0);
    }

    #[test]
    fn all_twelve_lanes_clear_without_overlap_or_deadlock() {
        let mut world = World::new();
        for direction in [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ] {
            for route in [Route::Left, Route::Straight, Route::Right] {
                world.spawn(direction, route);
                let vehicle = world.vehicles.last_mut().unwrap();
                match direction {
                    Direction::North => vehicle.y = CENTER_Y + CORRIDOR_WIDTH + 130.0,
                    Direction::South => vehicle.y = CENTER_Y - CORRIDOR_WIDTH - 130.0,
                    Direction::East => vehicle.x = CENTER_X - CORRIDOR_WIDTH - 130.0,
                    Direction::West => vehicle.x = CENTER_X + CORRIDOR_WIDTH + 130.0,
                }
            }
        }

        for _ in 0..5_000 {
            world.update(0.016);
            let in_box = world.vehicles.iter().filter(|v| v.is_inside_box()).count();
            assert!(in_box <= 1, "multiple vehicles entered the box together");
            if world.completed().len() == 12 {
                break;
            }
        }
        assert_eq!(
            world.completed().len(),
            12,
            "some lane or route became stuck"
        );
        assert_eq!(world.close_call_count(), 0);
    }
}
