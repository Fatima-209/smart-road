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

Each turning point is the intersection of the incoming lane centerline and
the outgoing lane centerline. The route-to-exit mapping is fixed:

| Incoming heading | Left | Straight | Right |
|---|---|---|---|
| North | West | North | East |
| South | East | South | West |
| East | North | East | South |
| West | South | West | North |

The 12 approach lanes carry small directional arrows matching that table.
The static road, grass, and lane-marking scene is composed once into a target
texture; each frame reuses it and redraws moving car sprites.

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
- `time`: elapsed wall-clock time from `detected_at` (when the vehicle enters
  `REACTION_DISTANCE`) until its center clears the intersection box.
- `velocity_history`: every speed selected after detection until the vehicle
  clears the box is recorded, including any stops while it waits.

## Safety distance & close calls

- `SAFETY_DISTANCE = 60.0` px — strictly positive, enforced between any two
  vehicles whose paths can intersect (not just the same-lane vehicle ahead).
  Deliberately kept **bigger than the car sprite's longest side**
  (`CAR_HEIGHT = 50`) — a smaller safety distance would let two vehicles
  pass the "safe" check (center-to-center) while their sprites still
  visually overlap on screen.
- `CLOSE_CALL_THRESHOLD = 55.0` px, below the 60px safety distance. A close
  call is counted when potentially conflicting vehicle centers come within
  55px without their rendered rectangles overlapping. Overlaps are collisions.

## Spawn cooldowns

- `SPAWN_COOLDOWN_MS = 650` — minimum time between two manual (arrow-key)
  spawns **in the same direction**, so holding/spamming a key can't stack
  vehicles on top of each other. Tracked per-direction, not globally, so
  spamming Up doesn't block Down. The spawn-tile check remains the final
  guard if traffic has stopped the previous vehicle near the spawn point.
- `RANDOM_SPAWN_INTERVAL_MS = 4000` — separate interval for the R-key's
  continuous random generation, set below the one-at-a-time intersection's
  approximate crossing throughput to limit queue growth.
- A spawn attempt is **dropped** (not queued) if the cooldown hasn't
  elapsed, or if the spawn point in the chosen lane isn't clear.

## Intersection management strategy

**Current code status (updated 2026-09-23):** Vehicles now turn at fixed
waypoints and rotate their heading at the turn point. The manager uses a
conservative single-vehicle intersection rule: approaching vehicles yield to
an older vehicle that is approaching or already inside. This serializes
crossings rather than scheduling individual conflict cells. `R` toggles
random vehicle generation; Escape opens an SDL statistics window.
Crossing time starts within `REACTION_DISTANCE` and ends when the vehicle
clears the intersection box. Close calls are counted once per pair of vehicle
centers on potentially conflicting paths that pass within
`CLOSE_CALL_THRESHOLD` without overlapping. Collisions are tracked separately.
Only vehicles already cleared when Escape is pressed count in the passed total;
vehicles still on the road are excluded.

The manager is centralized in `World`. Its effective conflict matrix is
deliberately conservative: let the 12 lanes be `{N-L, N-S, N-R, S-L, S-S,
S-R, E-L, E-S, E-R, W-L, W-S, W-R}`. For each lane, its conflict set is all
other 11 lanes, so all 66 distinct lane pairs are serialized. The strategy
allows zero non-conflicting pairs to occupy the box together, even where the
physical paths would not cross. Same-lane followers also use the following
distance check. This complete matrix is fixed by policy and not inferred from
current positions or chosen routes at runtime.

The implementation notes below describe the original straight-line version
and are retained as design history; they do not describe the current code.

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

### Implementation, first pass (straight-line traffic only)

`World::desired_velocity` (in `src/world.rs`) recomputes every vehicle's
target speed each tick from two independent rules — the vehicle obeys
whichever is more restrictive:

1. **Same-lane following.** If another vehicle in the same (direction,
   route) lane is ahead, match speed to the gap: full speed beyond
   `REACTION_DISTANCE`, `VELOCITY_SLOW` inside it, `VELOCITY_STOPPED` inside
   `SAFETY_DISTANCE`. This is what stops a vehicle from rear-ending one
   that's stopped ahead of it at the intersection.
2. **Perpendicular right-of-way.** A vehicle within `REACTION_DISTANCE` of
   the intersection box, and not yet inside it, yields to any perpendicular
   vehicle that's already inside the box, or that has a lower id (spawned
   earlier). Priority is **id-based, not live-distance-based** — an earlier
   version compared current distance-to-box instead, which seemed more
   "fair" (whoever's closer goes first) but broke the moment one vehicle
   stopped: its distance freezes while the other's keeps shrinking, so the
   two vehicles' checks could disagree with each other mid-negotiation and
   swap who was yielding instead of one committing. Id is fixed at spawn
   time, so it can't flip-flop like that. A vehicle already inside the box
   is never stopped, so nothing ever halts mid-crossing.

**Conflict enumeration across the 12 lanes, today:** only *perpendicular*
direction pairs (N/S vs E/W) are treated as conflicting — that's every pair
except (N,S) and (E,W) together. This is a deliberate, temporary
simplification: since no route curves yet (see the movement note in
`vehicle.rs`), every vehicle's real path *is* just its spawn direction, so
two vehicles only physically cross paths when their directions are
perpendicular. Parallel directions (opposing N/S, or opposing E/W) never
cross while driving straight, regardless of route, so they're correctly
never flagged.

**This is not the final 12-lane conflict table** the spec asks for — once
turning is implemented, a Left/Right route's *actual* path will cross lanes
its spawn direction alone doesn't predict (e.g. a North-Left car ends up
heading West, and can conflict with East-bound traffic it currently
doesn't). The conflict check will need to move from "compare spawn
directions" to "compare actual paths" at that point. Documented here so
that revision is a known, planned step and not a surprise.

## Vehicle struct shape

See `src/vehicle.rs`. Fields include `id`, current `direction` (heading),
fixed `route`, position (`x`, `y`), `velocity`, lifecycle `state`, turn
completion state, crossing timestamps, and velocity history.
