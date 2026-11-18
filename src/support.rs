//! Game-side glue for Bridging Mod: recompute upstream's reach-around target from the acting
//! player's eye ray, then place the held block.
//!
//! Upstream is a client-side assist that intercepts `Minecraft.startUseItem` when the crosshair
//! misses. Here the same click reaches `Hooks::right_click_item` (NeoForge's
//! `PlayerInteractEvent.RightClickItem`, immediately before `Item.use`), so the target and the
//! placement happen on the authoritative side.
//!
//! The target model itself is [`crate::runtime`]; this module only reads the game (block facts,
//! the player's box, the eye ray, the held stack) and writes the result back.

use mod_api::{
    BlockFace, BlockKey, BlockMetadata, BlockState, ClipBlock, ClipFluid, Host, HostBlockStates,
    HostTransaction, InteractionResult, ItemClick, ItemPlace, ItemResource, ItemStack, LevelRef,
    ServerHost, SoundSource, StatType, StateConstraint, Tag,
};

use crate::runtime::{
    Aabb, AxisMode, Block, Face, MIN_BRIDGE_FRACTION, REACH_FALLBACK, SNAP_STRENGTH, Settings,
    View, World, assist_target, is_torch,
};

/// The upstream defaults this slice keeps (see `BridgingConfig`).
const SETTINGS: Settings = Settings {
    min_fraction: MIN_BRIDGE_FRACTION,
    adjacency: true,
    snap: SNAP_STRENGTH,
    axis: AxisMode::Both,
    replace_non_solid: true,
};

/// A player's block interaction range (`Player.blockInteractionRange`, the
/// `minecraft:block_interaction_range` attribute).
const REACH_ATTRIBUTE: &str = "minecraft:block_interaction_range";
/// The item component naming the placed state's properties (vanilla applies it on top of the
/// block's own placement state).
const BLOCK_STATE_COMPONENT: &str = "minecraft:block_state";
/// The item component carrying a block entity's own saved data onto the placed block.
const BLOCK_ENTITY_DATA: &str = "minecraft:block_entity_data";

/// The unit view vector of a vanilla look (`Entity.getViewVector`).
fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let (yaw, pitch) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        yaw.cos() * pitch.cos(),
    ]
}

/// The state properties an item's `minecraft:block_state` component names, in the component's
/// own order. Anything but a string value is ignored: the component's codec is a map of property
/// names to serialized values.
fn placement_constraints(tag: Option<&Tag>) -> Vec<(String, String)> {
    let Some(Tag::Compound(entries)) = tag else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|(name, value)| match value {
            Tag::String(text) => Some((name.to_string_lossy(), text.to_string_lossy())),
            _ => None,
        })
        .collect()
}

/// The volume and pitch vanilla plays a place sound at (`BlockItem.place`):
/// `(volume + 1) / 2` and `pitch * 0.8`.
fn place_sound_levels(volume: f32, pitch: f32) -> (f32, f32) {
    ((volume + 1.0) / 2.0, pitch * 0.8)
}

/// A runtime face as the game's `BlockFace`.
const fn block_face(face: Face) -> BlockFace {
    match face {
        Face::Down => BlockFace::Down,
        Face::Up => BlockFace::Up,
        Face::North => BlockFace::North,
        Face::South => BlockFace::South,
        Face::West => BlockFace::West,
        Face::East => BlockFace::East,
    }
}

/// Block reads for the target search: the loaded level's own states.
struct GameWorld<'a> {
    host: &'a dyn Host,
    level: LevelRef,
}

impl World for GameWorld<'_> {
    fn block(&self, pos: [i32; 3]) -> Option<Block> {
        if !self.level.is_loaded(self.host, pos)
            || self.level.in_world_bounds(self.host, pos) != Some(true)
        {
            return None;
        }
        let state = self.level.block_state(self.host, pos)?;
        let states = self.host.block_states();
        let name = states.name(state.block()).ok()?.to_string();
        let replaceable = states.replaceable(state).unwrap_or(false);
        let liquid = self
            .level
            .fluid_state(self.host, pos)
            .is_some_and(|fluid| !fluid.is_empty());
        Some(Block::new(name, replaceable, liquid))
    }
}

/// The state the held item places: its block's default with the item's `minecraft:block_state`
/// component applied. That is the closest shape the SDK supports; vanilla's
/// `getStateForPlacement` from the fake hit has no query here (see the gaps in `notes.md`).
fn placement_state(host: &dyn Host, block: BlockKey, stack: &ItemStack) -> Option<BlockState> {
    let states = host.block_states();
    let default = BlockState::of(&states, block, &[]).ok()?;
    let properties: Vec<StateConstraint> = placement_constraints(
        stack
            .components()
            .and_then(|patch| patch.as_nbt().get(&BLOCK_STATE_COMPONENT.into())),
    )
    .into_iter()
    .map(|(name, value)| StateConstraint::new(name, value))
    .collect();
    if properties.is_empty() {
        return Some(default);
    }
    Some(BlockState::of(&states, block, &properties).unwrap_or(default))
}

/// The `minecraft:block_entity_data` compound of a stack, if it carries one.
fn block_entity_data(stack: &ItemStack) -> Option<&Tag> {
    stack
        .components()
        .and_then(|patch| patch.as_nbt().get(&BLOCK_ENTITY_DATA.into()))
}

/// A right click in the air: fill the gap the player looks into, if any.
///
/// `Some(SuccessServer)` claims the use (the server swings the hand); `None` lets vanilla's own
/// item use run, so a click this mod cannot bridge behaves exactly as it would without it.
pub fn on_right_click(
    host: &mut dyn ServerHost,
    click: &ItemClick<'_>,
) -> Option<InteractionResult> {
    let stack = click.stack;
    let player = host.player(click.player)?;
    if player.is_spectator(host) == Some(true) {
        return None;
    }
    let level = player.level(host)?;
    let entity = player.entity();
    let eye = entity.eye_position(host)?;
    let [yaw, pitch] = entity.rotation(host)?;
    let look = view_vector(yaw, pitch);
    let reach = host
        .living(entity)
        .and_then(|living| living.attribute_value(host, REACH_ATTRIBUTE))
        .unwrap_or(REACH_FALLBACK);
    // Upstream bridges only when the crosshair misses: a click on a block is vanilla placement.
    // `Entity.pick` is the same eye ray out to the block interaction range, so a miss here is the
    // check.
    let far = [
        eye[0] + look[0] * reach,
        eye[1] + look[1] * reach,
        eye[2] + look[2] * reach,
    ];
    if !level
        .clip(host, eye, far, ClipBlock::Outline, ClipFluid::None)?
        .miss
    {
        return None;
    }
    let block = host.item_block(stack.item())?;
    if host.block_states().name(block).is_ok_and(is_torch) {
        return None;
    }
    let [min, max] = entity.bounding_box(host)?;
    let view = View { eye, look, reach };
    let assist = {
        let world = GameWorld {
            host: &*host,
            level,
        };
        assist_target(&world, &view, Aabb::new(min, max), &SETTINGS)
    }?;
    let pos = assist.pos;
    let state = placement_state(host, block, stack)?;
    // `BlockItem.place` asks `Player.mayUseItemAt` before it writes; the query exists here, so the
    // 0.1.0 gap ("no native query") closes.
    if player.may_use_item_at(host, pos, block_face(assist.face), stack) == Some(false) {
        return None;
    }
    // The write, with `setPlacedBy` run in the player's turn (a bed's head, a door's top half).
    player.place_block(host, level, pos, state, true).ok()?;
    if let Some(sound) = host.block_states().sound(state) {
        let (volume, pitch) = place_sound_levels(sound.volume, sound.pitch);
        level.play_sound(
            host,
            sound.place.as_ref(),
            [
                f64::from(pos[0]) + 0.5,
                f64::from(pos[1]) + 0.5,
                f64::from(pos[2]) + 0.5,
            ],
            SoundSource::Blocks,
            volume,
            pitch,
        );
    }
    // `BlockItem.place`'s tail: the block entity's own data from the stack, then the item's
    // `used` statistic and one consumed item (none in creative).
    if host.block_states().has_block_entity(state) == Some(true)
        && let Some(Tag::Compound(data)) = block_entity_data(stack)
        && let Some(entity) = host.block_entity(level, pos)
    {
        let _ = entity.merge_data(host, data.clone());
    }
    let item_id = stack.item().id(host).to_string();
    player.award_stat(host, StatType::Used, &item_id, 1);
    let resource = ItemResource::of(stack);
    let hand = ItemPlace::PlayerHand {
        player: click.player,
        hand: click.hand,
    };
    let _ = host.transaction(|host, tx| {
        // A creative player's hand gives without losing any, as vanilla's creative use does.
        hand.extract(host, tx, &resource, 1);
        Ok::<(), ()>(())
    });
    Some(InteractionResult::SuccessServer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(got: [f64; 3], want: [f64; 3]) -> bool {
        (0..3).all(|i| (got[i] - want[i]).abs() < 1e-9)
    }

    #[test]
    fn view_vector_faces_south_at_zero_angles() {
        assert!(near(view_vector(0.0, 0.0), [0.0, 0.0, 1.0]));
    }

    #[test]
    fn view_vector_turns_with_yaw_and_pitch() {
        // Yaw 90 looks west; pitch 90 looks straight down (`Entity.getViewVector`).
        assert!(near(view_vector(90.0, 0.0), [-1.0, 0.0, 0.0]));
        assert!(near(view_vector(0.0, 90.0), [0.0, -1.0, 0.0]));
    }

    #[test]
    fn placement_constraints_read_the_string_properties() {
        let tag = Tag::Compound(
            [("facing".into(), Tag::String("east".into()))]
                .into_iter()
                .collect(),
        );
        assert_eq!(
            placement_constraints(Some(&tag)),
            vec![("facing".to_string(), "east".to_string())]
        );
    }

    #[test]
    fn placement_constraints_ignore_a_missing_or_non_compound_component() {
        assert!(placement_constraints(None).is_empty());
        assert!(placement_constraints(Some(&Tag::Int(3))).is_empty());
    }

    #[test]
    fn place_sound_levels_are_vanillas_volume_and_pitch() {
        assert_eq!(place_sound_levels(1.0, 1.0), (1.0, 0.8));
        assert_eq!(place_sound_levels(0.0, 0.5), (0.5, 0.4));
    }
}
