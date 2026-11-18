// Bridging Mod for Horizon — the reach-around placement target model (pure).
//
// Ported from Bridging Mod 2.7.0 (squeeglii/BridgingMod, MIT) for Minecraft
// Java 26.3. Upstream is a client-side aim assist: it computes a target from
// the camera ray in `Minecraft.startUseItem` and sends a placement packet.
// This rewrite recomputes the same target on the server from the acting
// player's eye ray, so the behaviour works without a client mod.
//
// The model here has no game access: the glue (`support.rs`) feeds it block
// facts, the player's box and the view ray, then places the returned target
// through vanilla's placement rules.

/// Vanilla block interaction range when the player has no attribute override.
// `cargo test` uses it; the injected glue reads the live attribute instead.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the model's documented fallback, used by its tests"
    )
)]
pub const REACH_FALLBACK: f64 = 4.5;

/// Upstream's default `minBridgeDistance` (20% of the reach ray).
pub const MIN_BRIDGE_FRACTION: f64 = 0.20;

/// Upstream's default `bridgingSnapStrength` (1.0 = full block half-extent).
pub const SNAP_STRENGTH: f64 = 1.0;

/// Upstream's `DIRECTION_SIMILARITY_THRESHOLD`: a support side counts when the
/// view points into it at least this much.
pub const DIRECTION_SIMILARITY_THRESHOLD: f64 = 0.1;

/// Floating-point slack for the Bresenham termination test.
const NEAR_ZERO: f64 = 0.01;

/// A block face, in vanilla's `Direction.values()` order (the order decides
/// which support side wins when several qualify).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    /// Every face, in `Direction.values()` order.
    pub const ALL: [Face; 6] = [
        Face::Down,
        Face::Up,
        Face::North,
        Face::South,
        Face::West,
        Face::East,
    ];

    /// The unit offset of this face.
    #[must_use]
    pub const fn offset(self) -> [i32; 3] {
        match self {
            Face::Down => [0, -1, 0],
            Face::Up => [0, 1, 0],
            Face::North => [0, 0, -1],
            Face::South => [0, 0, 1],
            Face::West => [-1, 0, 0],
            Face::East => [1, 0, 0],
        }
    }

    /// The unit vector of this face, for dot products with the view.
    #[must_use]
    pub const fn normal(self) -> [f64; 3] {
        match self {
            Face::Down => [0.0, -1.0, 0.0],
            Face::Up => [0.0, 1.0, 0.0],
            Face::North => [0.0, 0.0, -1.0],
            Face::South => [0.0, 0.0, 1.0],
            Face::West => [-1.0, 0.0, 0.0],
            Face::East => [1.0, 0.0, 0.0],
        }
    }

    /// The face pointing the other way.
    #[must_use]
    pub const fn opposite(self) -> Face {
        match self {
            Face::Down => Face::Up,
            Face::Up => Face::Down,
            Face::North => Face::South,
            Face::South => Face::North,
            Face::West => Face::East,
            Face::East => Face::West,
        }
    }

    /// Whether this face is one of the four horizontal ones.
    #[must_use]
    pub const fn horizontal(self) -> bool {
        matches!(self, Face::North | Face::South | Face::West | Face::East)
    }
}

/// Upstream's `PlacementAxisMode`: which support sides may be bridged off.
// This slice hard-codes `Both`; the other modes are the config's, kept for the
// tests and a later config slice.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "upstream's other axis modes; slice 1 hard-codes Both"
    )
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisMode {
    Horizontal,
    Vertical,
    Both,
}

impl AxisMode {
    /// Whether a support side on `face` is allowed.
    #[must_use]
    pub const fn allows(self, face: Face) -> bool {
        match self {
            AxisMode::Both => true,
            AxisMode::Horizontal => face.horizontal(),
            AxisMode::Vertical => !face.horizontal(),
        }
    }
}

/// What the model needs to know about one block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub name: String,
    /// `BlockState.canBeReplaced` (air, grass, water, snow layers, ...).
    pub replaceable: bool,
    /// A water or lava block: never a support.
    pub liquid: bool,
}

impl Block {
    /// A block with its name and replaceability.
    #[must_use]
    pub fn new(name: impl Into<String>, replaceable: bool, liquid: bool) -> Self {
        Self {
            name: name.into(),
            replaceable,
            liquid,
        }
    }
}

/// Read access to the loaded world; `None` means the voxel is not loaded.
pub trait World {
    fn block(&self, pos: [i32; 3]) -> Option<Block>;
}

/// The acting player's bounding box, for the cells the body already fills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    /// A box from its corners.
    #[must_use]
    pub fn new(min: [f64; 3], max: [f64; 3]) -> Self {
        Self { min, max }
    }

    /// Whether this box overlaps the unit cell at `pos` (touching is not
    /// overlapping, as vanilla's `AABB.intersects`).
    #[must_use]
    pub fn intersects_cell(self, pos: [i32; 3]) -> bool {
        (0..3).all(|i| {
            let cell_min = f64::from(pos[i]);
            let cell_max = cell_min + 1.0;
            self.min[i] < cell_max && self.max[i] > cell_min
        })
    }
}

/// The eye ray the target is computed along.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub eye: [f64; 3],
    /// A unit view vector (`view_vector`).
    pub look: [f64; 3],
    /// Block interaction range.
    pub reach: f64,
}

/// The upstream defaults this slice keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// How far along the ray the search starts, as a fraction of the reach.
    pub min_fraction: f64,
    /// Upstream's `BridgingAdjacency` (anything but `Strict`).
    pub adjacency: bool,
    /// Upstream's `bridgingSnapStrength`.
    pub snap: f64,
    /// Upstream's `supportedBridgeAxes`.
    pub axis: AxisMode,
    /// Upstream's `enableNonSolidReplace`.
    pub replace_non_solid: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min_fraction: MIN_BRIDGE_FRACTION,
            adjacency: true,
            snap: SNAP_STRENGTH,
            axis: AxisMode::Both,
            replace_non_solid: true,
        }
    }
}

/// The cell to fill and the face its placement ray reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Assist {
    pub pos: [i32; 3],
    /// The `BlockHitResult` direction: from the target towards its support.
    pub face: Face,
}

/// Whether `block` is one of vanilla's air blocks (`BlockState.isAir`).
#[must_use]
pub fn is_air(block: &str) -> bool {
    matches!(
        block,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

/// Whether `block` is one of the torch blocks upstream skips by default.
///
/// Upstream skips `BaseTorchBlock`, which is the plain, soul and copper
/// torches (the redstone torch is a `RedstoneTorchBlock` and stays placeable).
#[must_use]
pub fn is_torch(block: &str) -> bool {
    matches!(
        block,
        "minecraft:torch"
            | "minecraft:wall_torch"
            | "minecraft:soul_torch"
            | "minecraft:soul_wall_torch"
            | "minecraft:copper_torch"
            | "minecraft:copper_wall_torch"
    )
}

/// The bridging target for `view`, or `None` when no gap can be reached.
///
/// Walks the view ray's cells in order and returns the first one that is
/// replaceable, outside the player's box, and next to a solid block on a side
/// the view points into. That is upstream's `getClosestAssistTarget`.
#[must_use]
pub fn assist_target(
    world: &impl World,
    view: &View,
    player: Aabb,
    settings: &Settings,
) -> Option<Assist> {
    for pos in view_path(view, settings) {
        let Some(target) = world.block(pos) else {
            continue;
        };
        let allowed = if settings.replace_non_solid {
            target.replaceable
        } else {
            is_air(&target.name)
        };
        if !allowed || player.intersects_cell(pos) {
            continue;
        }
        for direction in Face::ALL {
            let normal = direction.normal();
            let similarity = view.look[0].mul_add(
                normal[0],
                view.look[1].mul_add(normal[1], view.look[2] * normal[2]),
            );
            if similarity < DIRECTION_SIMILARITY_THRESHOLD {
                continue;
            }
            // The support sits on the far side of the target from the view:
            // `canSideBeBuiltOffOf(pos, direction.getOpposite())`.
            let support_side = direction.opposite();
            if !settings.axis.allows(support_side) {
                continue;
            }
            let support_pos = offset(pos, support_side.offset());
            let Some(support) = world.block(support_pos) else {
                continue;
            };
            if is_air(&support.name) || support.liquid || support.replaceable {
                continue;
            }
            return Some(Assist {
                pos,
                face: direction,
            });
        }
    }
    None
}

/// The cells upstream's `Path.calculateBresenhamVoxels` walks, in order.
#[must_use]
pub fn view_path(view: &View, settings: &Settings) -> Vec<[i32; 3]> {
    let far = [
        view.eye[0] + view.look[0] * view.reach,
        view.eye[1] + view.look[1] * view.reach,
        view.eye[2] + view.look[2] * view.reach,
    ];
    let distance = {
        let delta = [
            far[0] - view.eye[0],
            far[1] - view.eye[1],
            far[2] - view.eye[2],
        ];
        (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt()
    };
    let near = distance * settings.min_fraction;
    let start = containing([
        view.eye[0] + view.look[0] * near,
        view.eye[1] + view.look[1] * near,
        view.eye[2] + view.look[2] * near,
    ]);
    let end = containing(far);
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let abs = delta.map(i32::abs);
    let step = delta.map(i32::signum);
    let mut points = vec![start];
    let mut working = start.map(f64::from);
    let target = end.map(f64::from);
    // Upstream branches on the dominant axis; the other two are `first` and
    // `second` in the order their error terms are tested.
    let (dominant, first, second) = if abs[0] >= abs[1] && abs[0] >= abs[2] {
        (0, 1, 2)
    } else if abs[1] >= abs[0] && abs[1] >= abs[2] {
        (1, 0, 2)
    } else {
        (2, 1, 0)
    };
    let mut error_first = 2 * abs[first] - abs[dominant];
    let mut error_second = 2 * abs[second] - abs[dominant];
    while (working[dominant] - target[dominant]).abs() > NEAR_ZERO {
        working[dominant] += f64::from(step[dominant]);
        if error_first >= 0 {
            working[first] += f64::from(step[first]);
            error_first -= 2 * abs[dominant];
        }
        if error_second >= 0 {
            working[second] += f64::from(step[second]);
            error_second -= 2 * abs[dominant];
        }
        error_first += 2 * abs[first];
        error_second += 2 * abs[second];
        let next = containing(working);
        if settings.adjacency {
            points.extend(missed_points(&points, next, start, end, settings.snap));
        }
        points.push(next);
    }
    points
}

/// `Mth.floor` of each component, as `BlockPos.containing` does.
fn containing(pos: [f64; 3]) -> [i32; 3] {
    pos.map(|value| value.floor() as i32)
}

/// A block position plus a face offset.
fn offset(pos: [i32; 3], offset: [i32; 3]) -> [i32; 3] {
    [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
}

/// Upstream's `Path.calculateMissedPoints`: the cells a Bresenham step of two
/// or three axes cuts through, kept only when the line passes through the
/// cell's (snapped) box.
fn missed_points(
    points: &[[i32; 3]],
    next: [i32; 3],
    start: [i32; 3],
    end: [i32; 3],
    snap: f64,
) -> Vec<[i32; 3]> {
    let Some(&last) = points.last() else {
        return Vec::new();
    };
    let delta = [next[0] - last[0], next[1] - last[1], next[2] - last[2]];
    let distance: i32 = delta.iter().map(|value| value.abs()).sum();
    if distance <= 1 {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    // A shared edge (two axes) or a shared corner (three axes): the cells
    // between the two points, in upstream's order (single axes, then pairs).
    for axis in 0..3 {
        if delta[axis] != 0 {
            let mut candidate = last;
            candidate[axis] += delta[axis];
            candidates.push(candidate);
        }
    }
    if distance == 3 {
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            let mut candidate = last;
            candidate[a] += delta[a];
            candidate[b] += delta[b];
            candidates.push(candidate);
        }
    }
    let extent = [0.5 * snap; 3];
    candidates
        .into_iter()
        .filter(|candidate| line_meets_cell(start, end, *candidate, extent))
        .collect()
}

/// The separating-axis test from 3dkingdoms' line/box intersection, with the
/// cell at `cell` centred on its lower corner plus 0.5.
fn line_meets_cell(start: [i32; 3], end: [i32; 3], cell: [i32; 3], extent: [f64; 3]) -> bool {
    let line_start = [
        f64::from(start[0] - cell[0]),
        f64::from(start[1] - cell[1]),
        f64::from(start[2] - cell[2]),
    ];
    let line_end = [
        f64::from(end[0] - cell[0]),
        f64::from(end[1] - cell[1]),
        f64::from(end[2] - cell[2]),
    ];
    let mid = [
        (line_start[0] + line_end[0]) * 0.5,
        (line_start[1] + line_end[1]) * 0.5,
        (line_start[2] + line_end[2]) * 0.5,
    ];
    let line = [
        line_start[0] - mid[0],
        line_start[1] - mid[1],
        line_start[2] - mid[2],
    ];
    let line_extent = [line[0].abs(), line[1].abs(), line[2].abs()];
    for axis in 0..3 {
        if mid[axis].abs() > extent[axis] + line_extent[axis] {
            return false;
        }
    }
    let crosses = |a: usize, b: usize| {
        (mid[b] * line[a] - mid[a] * line[b]).abs()
            > extent[b] * line_extent[a] + extent[a] * line_extent[b]
    };
    !crosses(2, 1) && !crosses(2, 0) && !crosses(1, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A world made of named blocks; everything else is air.
    struct TestWorld {
        blocks: Vec<([i32; 3], Block)>,
    }

    impl TestWorld {
        fn new() -> Self {
            Self { blocks: Vec::new() }
        }

        fn put(mut self, pos: [i32; 3], block: Block) -> Self {
            self.blocks.retain(|(at, _)| *at != pos);
            self.blocks.push((pos, block));
            self
        }

        fn platform(
            mut self,
            x: std::ops::RangeInclusive<i32>,
            z: std::ops::RangeInclusive<i32>,
        ) -> Self {
            for x in x {
                for z in z.clone() {
                    self = self.put([x, 99, z], Block::new("minecraft:stone", false, false));
                }
            }
            self
        }
    }

    impl World for TestWorld {
        fn block(&self, pos: [i32; 3]) -> Option<Block> {
            if let Some((_, block)) = self.blocks.iter().find(|(at, _)| *at == pos) {
                return Some(block.clone());
            }
            Some(Block::new("minecraft:air", true, false))
        }
    }

    fn player_at(x: f64, y: f64, z: f64) -> Aabb {
        Aabb::new([x - 0.3, y, z - 0.3], [x + 0.3, y + 1.8, z + 0.3])
    }

    /// The e2e rig: a near platform south of a one-block gap, the far platform
    /// beyond it, the player at the near edge looking north and down.
    fn gap_world() -> TestWorld {
        TestWorld::new()
            .platform(-2..=2, 0..=2)
            .platform(-2..=2, -4..=-2)
    }

    fn gap_view() -> View {
        let d = std::f64::consts::FRAC_1_SQRT_2;
        View {
            eye: [0.0, 101.62, 1.0],
            look: [0.0, -d, -d],
            reach: REACH_FALLBACK,
        }
    }

    #[test]
    fn view_path_marches_the_dominant_axis() {
        let view = View {
            eye: [0.5, 10.5, 0.5],
            look: [1.0, 0.0, 0.0],
            reach: 4.5,
        };
        let settings = Settings {
            min_fraction: 0.0,
            adjacency: false,
            ..Settings::default()
        };
        assert_eq!(
            view_path(&view, &settings),
            vec![
                [0, 10, 0],
                [1, 10, 0],
                [2, 10, 0],
                [3, 10, 0],
                [4, 10, 0],
                [5, 10, 0]
            ]
        );
    }

    #[test]
    fn view_path_starts_at_the_minimum_distance() {
        let view = View {
            eye: [0.5, 10.5, 0.5],
            look: [1.0, 0.0, 0.0],
            reach: 4.5,
        };
        let settings = Settings {
            min_fraction: 0.2,
            adjacency: false,
            ..Settings::default()
        };
        assert_eq!(
            view_path(&view, &settings),
            vec![[1, 10, 0], [2, 10, 0], [3, 10, 0], [4, 10, 0], [5, 10, 0]]
        );
    }

    #[test]
    fn adjacency_adds_the_cells_a_diagonal_step_cuts_through() {
        let view = View {
            eye: [0.5, 0.5, 0.5],
            look: [
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            reach: std::f64::consts::SQRT_2,
        };
        let settings = Settings {
            min_fraction: 0.0,
            adjacency: true,
            ..Settings::default()
        };
        assert_eq!(
            view_path(&view, &settings),
            vec![[0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0]]
        );
    }

    #[test]
    fn assist_fills_the_gap_the_player_looks_into() {
        let assist = assist_target(
            &gap_world(),
            &gap_view(),
            player_at(0.0, 100.0, 1.0),
            &Settings::default(),
        );
        assert_eq!(
            assist,
            Some(Assist {
                pos: [0, 99, -1],
                face: Face::North,
            })
        );
    }

    #[test]
    fn assist_ignores_cells_the_player_already_fills() {
        let player = Aabb::new([-0.3, 99.0, -1.3], [0.3, 100.8, -0.7]);
        // The gap cell is inside the body, so the search moves on to the cell
        // under the far platform (supported from above).
        assert_eq!(
            assist_target(&gap_world(), &gap_view(), player, &Settings::default()),
            Some(Assist {
                pos: [0, 98, -2],
                face: Face::Down,
            })
        );
    }

    #[test]
    fn assist_replaces_a_non_solid_target_only_when_asked() {
        let world = gap_world().put(
            [0, 99, -1],
            Block::new("minecraft:short_grass", true, false),
        );
        let player = player_at(0.0, 100.0, 1.0);
        let replace = Settings::default();
        assert_eq!(
            assist_target(&world, &gap_view(), player, &replace),
            Some(Assist {
                pos: [0, 99, -1],
                face: Face::North,
            })
        );
        // Air only: the grass cell is skipped and the next reachable cell is
        // the one under the far platform.
        let air_only = Settings {
            replace_non_solid: false,
            ..Settings::default()
        };
        assert_eq!(
            assist_target(&world, &gap_view(), player, &air_only),
            Some(Assist {
                pos: [0, 98, -2],
                face: Face::Down,
            })
        );
    }

    #[test]
    fn assist_needs_a_solid_non_liquid_support() {
        let water = gap_world().put([0, 99, 0], Block::new("minecraft:water", true, true));
        assert_eq!(
            assist_target(
                &water,
                &gap_view(),
                player_at(0.0, 100.0, 1.0),
                &Settings::default()
            ),
            Some(Assist {
                pos: [0, 98, -2],
                face: Face::Down,
            })
        );
    }

    #[test]
    fn axis_modes_limit_the_support_side() {
        let player = player_at(0.0, 100.0, 1.0);
        let horizontal = Settings {
            axis: AxisMode::Horizontal,
            ..Settings::default()
        };
        assert_eq!(
            assist_target(&gap_world(), &gap_view(), player, &horizontal),
            Some(Assist {
                pos: [0, 99, -1],
                face: Face::North,
            })
        );
        // The gap cell is built off its south (horizontal) neighbour, so a
        // vertical-only mode skips it for the cell under the far platform.
        let vertical = Settings {
            axis: AxisMode::Vertical,
            ..Settings::default()
        };
        assert_eq!(
            assist_target(&gap_world(), &gap_view(), player, &vertical),
            Some(Assist {
                pos: [0, 98, -2],
                face: Face::Down,
            })
        );
    }

    #[test]
    fn all_air_variants_count_as_air() {
        for name in ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"] {
            assert!(is_air(name), "{name} is air");
        }
        for name in [
            "minecraft:short_grass",
            "minecraft:water",
            "minecraft:stone",
        ] {
            assert!(!is_air(name), "{name} is not air");
        }
    }

    #[test]
    fn torches_are_the_skipped_block_family() {
        for torch in [
            "minecraft:torch",
            "minecraft:wall_torch",
            "minecraft:soul_torch",
            "minecraft:soul_wall_torch",
            "minecraft:copper_torch",
            "minecraft:copper_wall_torch",
        ] {
            assert!(is_torch(torch), "{torch} is a BaseTorchBlock");
        }
        for other in [
            "minecraft:redstone_torch",
            "minecraft:torchflower",
            "minecraft:stone",
            "minecraft:air",
        ] {
            assert!(!is_torch(other), "{other} is not a BaseTorchBlock");
        }
    }
}
