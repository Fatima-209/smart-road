# Smart Road — Design Decisions

The subject deliberately leaves several things unspecified. This file is the
single source of truth for the choices made, so nothing is decided ad hoc
inside the code later.

## World coordinate system

- Units: **pixels**, used consistently for both physics and rendering. No
  separate "grid cell" abstraction — `distance` and `velocity` are always in
  px and px/s.
- Window size: **1000 x 1000**.
- Origin `(0,0)` is the top-left corner, y grows downward (standard screen
  space, matches SDL2).
- `CENTER = (500.0, 500.0)`.
- `LANE_WIDTH = 50.0` px.
- Each approach direction gets a corridor of `3 * LANE_WIDTH = 150` px
  (3 lanes). Opposite directions sit on opposite sides of the center point,
  so the full N-S road is 300px wide and the full E-W road is 300px wide.
  The **intersection box** is therefore the 300x300 square centered on
  `CENTER`.

## Lane layout & turn convention

Traffic drives on the **right**, like the ASCII diagram in the subject.
For a vehicle traveling in a given direction, lanes are ordered from the
median (center line) outward to the curb:

| position relative to median | route    |
|---|---|
| closest to median  | left turn  |
| middle             | straight   |
| closest to curb    | right turn |

This is the real-world convention (left-turn lanes sit innermost, next to
oncoming traffic; right-turn lanes sit outermost, next to the sidewalk) and
it's implemented once in `Vehicle::lane_center`, so it's the same rule for
all 12 lanes instead of 12 special cases.

Concretely, a corridor of 3 lanes width 50 gives lane centerlines at
`+25 / +75 / +125` px from the median, on whichever side of `CENTER` that
direction's traffic drives on:

- **North** (spawns south, drives up): corridor is *east* of the vertical
  median (`x > 500`). left=`x525`, straight=`x575`, right=`x625`.
- **South** (spawns north, drives down): corridor is *west* of the vertical
  median (`x < 500`). left=`x475`, straight=`x425`, right=`x375`.
- **East** (spawns west, drives right): corridor is *south* of the
  horizontal median (`y > 500`). left=`y525`, straight=`y575`, right=`y625`.
- **West** (spawns east, drives left): corridor is *north* of the horizontal
  median (`y < 500`). left=`y475`, straight=`y425`, right=`y375`.

Spawn points sit just outside the window edge on the appropriate lane
centerline, and vehicles travel in a straight line toward the intersection.

## Turn path geometry

Chosen: **two straight segments with a rotation applied at the corner**
(not an arc). A turning vehicle drives straight down its entry lane to a
fixed turn point inside the intersection box, then its heading snaps to the
new direction and it drives straight down the exit lane. This is simpler to
implement and reason about than integrating along an arc (no trig needed
for the position update, only for the rendered rotation angle), while still
satisfying the "vehicle visually rotates" requirement. A smooth arc is a
possible later upgrade, not required for the base project.

## Vehicle physics

- At least 3 velocity tiers, **instantaneous** changes (meets the minimum
  spec; eased acceleration is listed as a bonus, not done in the base
  version):
  - `STOPPED = 0.0` px/s (its own tier — a vehicle waiting for the
    intersection to clear)
  - `SLOW = 60.0` px/s
  - `MEDIUM = 120.0` px/s
  - `FAST = 200.0` px/s
- `distance`: remaining px until the vehicle fully clears the intersection
  box, tracked per vehicle.
- `time`: elapsed time from **detection** (an explicit `detected_at`
  timestamp set the first time the intersection algorithm evaluates the
  vehicle — Stage 3) until the vehicle clears the intersection. Not the
  same as spawn time, even though today they happen to coincide until
  Stage 3 wires up real detection.
- `velocity_history`: every velocity the vehicle has held during its
  crossing is recorded, so min/max stats are sampled from the full history,
  not just spawn/completion values.

## Safety distance & close calls

- `SAFETY_DISTANCE = 45.0` px — strictly positive, enforced between any two
  vehicles whose paths can intersect (not just the same-lane vehicle ahead).
- `CLOSE_CALL_THRESHOLD = 25.0` px — smaller than `SAFETY_DISTANCE`, so a
  close call (gap < threshold, but no actual collision) is a real,
  reachable condition rather than an impossible one.

## Spawn cooldowns

- `SPAWN_COOLDOWN_MS = 400` — minimum time between two manual (arrow-key)
  spawns **in the same direction**, so holding/spamming a key can't stack
  vehicles on top of each other. Tracked per-direction, not globally, so
  spamming Up doesn't block Down.
- `RANDOM_SPAWN_INTERVAL_MS = 800` — separate interval for the R-key's
  continuous random generation, on its own timer independent of the manual
  cooldowns above.
- A spawn attempt is **dropped** (not queued) if the cooldown hasn't
  elapsed, or if the spawn point in the chosen lane isn't clear.

## Intersection management strategy

Chosen: **decentralized velocity negotiation**. Each tick, every vehicle
inspects the shared vehicle list (world state) and adjusts its own velocity
tier based on the nearest relevant vehicle ahead of it or on a conflicting
path — no traffic lights, no central reservation table. Reasons:

- Simpler to build and test incrementally one conflicting route-pair at a
  time (per the task list), since there's no separate reservation data
  structure to design up front.
- Matches the subject's own phrasing for this option: "vehicles
  continuously check nearby vehicles each tick and adjust speed to avoid
  entering a shared cell simultaneously."
- Trade-off accepted: likely more conservative/less optimal throughput than
  a full time-space reservation system, but far lower risk of a subtle bug
  causing a collision, which is the one unforgivable failure mode here.

Conflict enumeration across the 12 lanes and the actual velocity-control
logic are implemented in Stage 3, not here — this section just fixes the
strategy so Stage 3 has a target to build.

## Vehicle struct shape

See `src/vehicle.rs`. Fields: `id`, `direction` (heading), `route`,
position (`x`, `y`), `velocity`, `state` (lifecycle), `distance_remaining`,
`distance_traveled`, `detected_at`, `cleared_at`, `velocity_history`.
