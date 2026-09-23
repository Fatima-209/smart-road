// All numeric decisions used by simulation and rendering live here.
// Shared world and display measurements, in pixels.
pub const WINDOW_WIDTH: u32 = 1000;
pub const WINDOW_HEIGHT: u32 = 1000;
pub const CENTER_X: f32 = WINDOW_WIDTH as f32 / 2.0;
pub const CENTER_Y: f32 = WINDOW_HEIGHT as f32 / 2.0;
pub const LANE_WIDTH: f32 = 50.0;
// Half-width of the cross-road, from its center line to its edge.
pub const CORRIDOR_WIDTH: f32 = LANE_WIDTH * 3.0;
pub const VELOCITY_STOPPED: f32 = 0.0;
pub const VELOCITY_SLOW: f32 = 60.0;
pub const VELOCITY_MEDIUM: f32 = 120.0;
pub const VELOCITY_FAST: f32 = 200.0;
// Center-to-center following and approach buffer.
pub const SAFETY_DISTANCE: f32 = 60.0;
// Strictly below SAFETY_DISTANCE. Counts tighter-than-safe passes only when
// the vehicle rectangles do not overlap; actual overlaps are collisions.
pub const CLOSE_CALL_THRESHOLD: f32 = SAFETY_DISTANCE - 5.0;
// Manual key debounce, applied per incoming direction.
pub const SPAWN_COOLDOWN_MS: u64 = 650;
// The manager allows only one vehicle in the box at a time; random arrivals
// stay below its approximate crossing throughput.
pub const RANDOM_SPAWN_INTERVAL_MS: u64 = 4_000;
// Range at which vehicles are considered by the intersection manager and
// crossing-time measurement begins.
pub const REACTION_DISTANCE: f32 = 150.0;
pub const CAR_WIDTH: f32 = 30.0;
pub const CAR_HEIGHT: f32 = 50.0;
