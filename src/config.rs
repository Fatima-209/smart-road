// All numeric decisions from DESIGN.md live here, in one place, so nothing
// is hardcoded ad hoc elsewhere.

pub const WINDOW_WIDTH: u32 = 1000;
pub const WINDOW_HEIGHT: u32 = 1000;

pub const CENTER_X: f32 = WINDOW_WIDTH as f32 / 2.0;
pub const CENTER_Y: f32 = WINDOW_HEIGHT as f32 / 2.0;

pub const LANE_WIDTH: f32 = 50.0;
// Width of one direction's 3-lane group; also, since both directions get
// equal space, the distance from CENTER to the intersection box's edge.
pub const CORRIDOR_WIDTH: f32 = LANE_WIDTH * 3.0;

pub const VELOCITY_STOPPED: f32 = 0.0;
pub const VELOCITY_SLOW: f32 = 60.0;
pub const VELOCITY_MEDIUM: f32 = 120.0;
pub const VELOCITY_FAST: f32 = 200.0;

// Must stay bigger than the car sprite's longest side (CAR_HEIGHT, 50) or
// two vehicles can be judged "safe" (center-to-center) while their sprites
// still visually overlap.
pub const SAFETY_DISTANCE: f32 = 60.0;
pub const CLOSE_CALL_THRESHOLD: f32 = 25.0;

// At VELOCITY_MEDIUM (120 px/s), a vehicle covers SAFETY_DISTANCE in 500ms;
// the cooldown is kept above that so a same-lane respawn is never closer
// than SAFETY_DISTANCE by the time the previous car has cleared it.
pub const SPAWN_COOLDOWN_MS: u64 = 650;
pub const RANDOM_SPAWN_INTERVAL_MS: u64 = 800;

// How far ahead a vehicle "looks" - for slowing behind a same-lane vehicle,
// and for checking cross-traffic before entering the intersection box.
pub const REACTION_DISTANCE: f32 = 150.0;

pub const CAR_WIDTH: f32 = 30.0;
pub const CAR_HEIGHT: f32 = 50.0;
