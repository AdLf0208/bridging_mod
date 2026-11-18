# Bridging Mod — port notes

Ported from **Bridging Mod 2.7.0** (`squeeglii/BridgingMod`, MIT) for Minecraft
Java 26.3. Local-only port (ARCHITECTURE.md §8).

Version **0.2.0** — a first publication as one flat SDK `.hmod` (`api = "0.2"`),
migrated 2026-11-18 from the retired `.hnmod` container (Horizon's mod-API merge #16941,
launcher ≥ 0.2.70). SDK nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78. The previous 0.1.0
bundle was built at Horizon `1d63e0a` / mixins `5a64af9`; its tree is kept as evidence in
[`../_shared/evidence/bridgingmod/retired-hnmod-0.1.0/`](../_shared/evidence/bridgingmod/retired-hnmod-0.1.0/).

Status: **ported** — slice 1 (reach-around placement into the gap the player
looks into). Client-only VFX/config are out of scope by design (see Gaps). The
0.1.0 in-game evidence for this behaviour is recorded below; the 0.2.0 package has
**no in-game run yet** (no launcher here — see "Not re-run in this migration").

## Probe result: client-only, but server-expressible

The upstream implementation is **client-only**: it is a `Minecraft` mixin that
intercepts `startUseItem` on the client, computes a target from the client
camera, and sends the placement with `MultiPlayerGameMode.useItemOn`
(`MinecraftClientMixin.java:37,69,85,135`; `BridgingStateTracker.java:27-37`).
There is no server class, packet or block entity in the mod.

At pin `c9d474d` the behaviour is nevertheless expressible on the server:

- the air click arrives at the `use_item` hook when the server's own block
  pick misses (`game.rs:5700,5918-5926`; `use_item_as_player` at `:5572`);
- the acting player's eye ray is available (`eye_position`/`aim`/`reach`,
  `game.rs:3009-3034`);
- block facts (name, replaceable, fluid) come from `game.facts()`
  (`game.rs:1785`) and the loaded level — at the campaign pin this is
  `game.res.facts()` (`game.rs:2151`);
- vanilla placement is callable from a `crate::game` support module
  (`game.rs:6094-6162`, `horizon-world/src/level/place.rs:1569-1677`).

So this is a server-side rewrite of the same target model, not a fake. The
detailed evidence table is in
[`../_shared/upstream/bridgingmod/FINDINGS.md`](../_shared/upstream/bridgingmod/FINDINGS.md).

## Behaviour implemented (slice 1)

- On a right click **in the air** with a placeable block item (main hand, then
  the off hand through vanilla's hand order), walk the player's eye ray from
  20 % of the block interaction range out to the range.
- Return the first cell along the ray that is replaceable (air, or any
  `canBeReplaced` block because `enableNonSolidReplace` is on), outside the
  player's bounding box, and next to a solid non-liquid non-replaceable block
  on a side the view points into (dot > 0.1).
- Place the held block at that cell, consuming one item (none in creative),
  with the place sound, the `used` statistic and the block entity's own data
  when the stack carries it.
- Torches are skipped (`skipTorchBridging`), as upstream.
- A cell already inside the player's body is skipped, and the search continues
  to the next reachable cell (upstream does the same).
- If the mod cannot place (no reachable gap, no permission, the write refused),
  the click falls through to vanilla's own use, so the next hand may still act.

## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge retired `.hnmod`: the registry closed every `.hnmod` submission on
2026-11-18 with one reason, and an `.hnmod` no longer loads. **0.2.0 is the same behaviour
republished as one flat SDK `.hmod`, as a first publication** (there is no closed PR to
supersede, and there is no earlier submission of this mod). Pins: SDK nightly
`0b14f8acfd97fa2c750e42795249dcc981178ca4`, mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78. The reference recipe was
`ports/imfast/notes.md` → "Migration to the SDK format", with `ports/enderstorage/notes.md`
and `ports/naturesaura/notes.md` for the Parts and host-handle shapes.

**Done in this order:** the retired paths were **moved** (never copied) into
`../_shared/evidence/bridgingmod/retired-hnmod-0.1.0/` **before** the first
`cargo horizon-mod build` (the tool refuses a root holding non-package paths), then
`src/runtime.rs` was copied with `cp` and proved with `cmp` + `sha256sum`, `cargo test` ran on
the new crate **before any glue existed** (10 runtime tests green), only then was the glue
written — first with `todo!()` bodies, which produced a real red run (10 passed, 5 failed),
then implemented.

### File map (old → new)

| 0.1.0 path | 0.2.0 | Evidence of the retired copy |
|---|---|---|
| `mixins/bridgingmod/runtime.rs` | `src/runtime.rs` — **byte-identical** (`fa82efe217bf9655…4d1be2`) | `retired-hnmod-0.1.0/mixins/bridgingmod/runtime.rs` |
| `mixins/bridgingmod/support.rs` (module `bridgingmod_state`) | `src/support.rs` (glue) + `src/lib.rs` (the one `impl Hooks`) | `retired-hnmod-0.1.0/mixins/bridgingmod/support.rs` |
| `mixins/bridgingmod/use_item.rs` (the one helper) | `Hooks::right_click_item` → `support::on_right_click`; no per-hook file | `retired-hnmod-0.1.0/mixins/bridgingmod/use_item.rs` |
| `mixins/bridgingmod/horizon-mixin.toml` (2 `[[module]]`, 1 `[[injection]]`) | no plan: one `Hooks` method; `src/runtime.rs` is an ordinary Rust module | `retired-hnmod-0.1.0/mixins/` |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in `horizon-mod.toml`; the launcher's toolchain builds | `retired-hnmod-0.1.0/` |
| `Cargo.toml` (`[lib] path = mixins/…/runtime.rs`, `[workspace] members = ["tools/audit"]`) | `Cargo.toml` (`[lib] path = "src/lib.rs"`, one dependency: `horizon-mod-api` from `.horizon/sdk/`) | `retired-hnmod-0.1.0/root/Cargo.toml` |
| `tools/horizon.py` (stage/check/test/build), `tools/package.py` (.hnmod), `tools/audit/` | gone; the SDK ladder is `cargo test` + `cargo horizon-mod build/check/fits` | `retired-hnmod-0.1.0/` (`horizon.py`, `package.py`, `audit/`) |
| `Cargo.lock` | not kept: it is packed and changes the artifact (trap 5) | not kept |
| `AGENTS.md` (instructed `compatibility.json` + `tools/horizon.py`) | rewritten for the SDK format | `retired-hnmod-0.1.0/root/AGENTS.md` |
| — | `LICENSE-NOTICE.md`, `README.md` (rewritten), `bridgingmod.hmod` | — |

**Kept and updated:** `tools/e2e.sh` (the scenario and its assertions are format-independent;
`GAME=` now names a launcher-built game, `CONTROL_GAME=` an unmodded one, `SELFTEST=1` inverts
phase 1), `notes.md`, `README.md`, `VALIDATION.md`, `LICENSE`.

### Manifest mapping

- identity: `id = "bridgingmod"` (also the resource namespace), `name = "Bridging Mod"`,
  `version = "0.2.0"`, `authors = ["squeeglii", "AdLf0208"]`.
- `api = "0.2"` (exact), `side = "both"` (an integrated server belongs to the client
  installation, so a LAN host and single player need it), `capabilities = ["registries"]`
  (the schema's one value, required even though this mod registers nothing).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and the bundle's mixins revision (`5a64af93…` → `e59da90…`).
- the old `[mod] priority = 100` is gone; nothing in this slice needs ordering.

### Helper → hook table

Every old `[[injection]]` and `[[module]]`, mapped to its 0.2.0 home. Nothing is dropped
silently: the three clauses the SDK cannot express are in **Gaps** below.

| 0.1.0 injection / module | 0.2.0 | Mapping notes |
|---|---|---|
| `[[module]] runtime` (`runtime.rs`, injected as `crate::game::bridgingmod`) | `src/runtime.rs`, `pub mod runtime;` | byte-identical; its tests now run in the package crate itself |
| `[[module]] state` (`support.rs`, injected as `crate::game::bridgingmod_state`) | `src/support.rs` | the same glue against host handles instead of `Game` |
| `[[injection]] use-item` (`hook = "use_item"` v1, `kind = "HEAD"`, helper `use_item.rs`, bindings `game`/`stack`/`claimed`) | `Hooks::right_click_item` (v1, **cancel**, both sides) | `use_item` is an **item Part** in 0.2 (`part:item`), so it can only be attached to named items; bridging must fire for every block item. `right_click_item` is `PlayerInteractEvent.RightClickItem` — "immediately before `Item.use`" — which is the global moment the HEAD injection occupied. It carries `ItemClick { stack, hand, player, player_entity_id }`, so `game`/`stack`/`claimed` become `host`, `click.stack`, and the returned `InteractionResult` (`SuccessServer` = the server swings the hand, which is what claiming did) |
| `game.pick()` (the crosshair-miss check) | `LevelRef::clip(host, eye, eye+look*reach, ClipBlock::Outline, ClipFluid::None)` → `ClipHit.miss` | the same eye ray out to the block interaction range |
| `game.aim()` / `game.reach()` / `game.keys.held(Sneak)` | `EntityRef::eye_position` + `::rotation` (yaw/pitch → `view_vector`), `LivingRef::attribute_value("minecraft:block_interaction_range")`, no sneak read needed | the sneak bit was only passed into the old `PlaceContext` |
| `game.res.facts()` / `game.world.block(..)` (block facts) | `LevelRef::block_state` + `Host::block_states()` (`name`, `replaceable`) + `LevelRef::fluid_state` | fluids: `FluidState::is_empty` |
| `game.items().get(..).block` + `game.pack.assets.registry.state(..)` | `Host::item_block(stack.item())` + `BlockState::of(&host.block_states(), block, props)` | the item's `minecraft:block_state` component is applied as constraints |
| `game.place_block(level, ticks, &ctx)` (`Behaviours::place_block`) | `PlayerRef::may_use_item_at` + `PlayerRef::place_block(host, level, pos, state, true)` | the write plus `setPlacedBy`; the rest of the vanilla tail is composed by the port (see Gaps) |
| `game.new_block_entity` + `apply_placed_item` (the `block_entity_data` component) | `BlockEntityRef::merge_data` with the stack's `minecraft:block_entity_data` | only when `block_states().has_block_entity(state)` |
| `game.process_level_events()` | `LevelRef::play_sound` with `block_states().sound(state).place` at vanilla's volume/pitch | the level-event queue has no mod-side driving |
| `game.award_stat("used", …)` | `PlayerRef::award_stat(host, StatType::Used, item_id, 1)` | same statistic |
| `game.consume_held()` | `ItemPlace::PlayerHand { player, hand }.extract(host, tx, &resource, 1)` in `host.transaction(..)` | a creative hand gives without losing any |
| `game.trigger_block("minecraft:placed_block", …)` | **none** — recorded gap | `trigger_criterion` refuses `minecraft:` triggers |
| `game.native_place_block(stack, target, state)` (notify the `place_block` hook) | **none** — recorded gap | 0.2 has no `place_block` hook; vanilla's own `entity_place` fires only inside `BlockItem.place`, which `place_block` does not run |
| `game.can_use_game_master_blocks()` | not needed | it fed the old `PlaceContext` only |

`uses.json` in the package records what the build actually linked: 242 entries — 3 hooks
(`right_click_item` and its two fields, plus the `blocks`/`block_entity`/`play_sound` dispatches
the handle methods reach), 15 methods, 122 types, 68 handles, 16 traits, 5 Parts, the
`registries` capability and the `horizon_mod` macro (`evidence/bridgingmod/sdk-uses.json`,
summarised in `sdk-uses-summary.txt`).

### The ladder (measured 2026-11-18, all inside `build-slot.sh` leases with `BUILD_LOCAL=1`)

`build-slot.sh` still speaks the retired `tools/horizon.py`, so it is used as the slot guard, not
as the driver: no staging, no staged tree, `CARGO_INCREMENTAL=0`, no `--target-dir`.

| Step | Command | Result |
|---|---|---|
| runtime before glue | `cargo test` with only `pub mod runtime;` | **10 passed**, 0 failed — `evidence/bridgingmod/sdk-test-runtime-first.log` |
| glue red | the same, with `todo!()` bodies in `support.rs` | **10 passed, 5 failed** — `sdk-test-glue-red.log` |
| tests | `cargo test` | **15 passed**, 0 failed (10 `runtime::`, 5 `support::`) — `sdk-test.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings — `sdk-clippy.log` |
| format | `cargo fmt --check` (**not** `--all`) | exit 0, empty diff — `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `SDK 0b14f8a (build 0b14f8ac…) in ./.horizon/sdk` / `Built ./target/horizon-mod/bridgingmod.hmod` — `sdk-build.log` |
| second build | the same again | **byte-identical**: 22,649 B, sha256 `29fc5b7443bd2e28a0182c3d4300ac8f6f3195442133da6c0952d7227bbb60f6` — `sdk-build-second.log`, `sdk-sha256.txt` |
| registry check | `cargo horizon-mod check bridgingmod.hmod` | `bridgingmod 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `22649 bytes, SHA-256 29fc5b74…bb60f6, unsigned`, **EXIT 0** — `sdk-check.log` |
| fits the pinned build | `cargo horizon-mod fits bridgingmod.hmod --build nightly` | `Bridging Mod:` / `0b14f8a  made for`, **EXIT 0** — `sdk-fits.log` |
| `runtime.rs` untouched | `cmp` with the retired file, and `unzip -p bridgingmod.hmod src/runtime.rs \| sha256sum` | `fa82efe217bf96553184e3358e8d3f75ac92c5a1129af94d91a1f7dd5f4d1be2` both ways |
| entries | `unzip -l bridgingmod.hmod` | **9 entries**: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `src/{lib,runtime,support}.rs`, `uses.json`; **no `Cargo.lock`**, no `assets/`/`data/` (this mod carries no content) — `sdk-entries.txt` |

### Traps new to this port

1. **`use_item` is an item Part, not a `Hooks` method.** `impl Hooks { fn use_item(..) }` is
   `E0407: method use_item is not a member of trait Hooks`; it lives on `ItemPart`, which the
   registrar attaches to named items — bridging must fire for *every* block item, so the global
   moment is `Hooks::right_click_item` (`PlayerInteractEvent.RightClickItem`, "immediately before
   `Item.use`"). Read `surface.json`'s `scope`/`part:item` before writing the hook.
2. **A `cancel` hook needs both halves.** `right_click_item` returns `Flow`: set
   `*event.cancellation_result = InteractionResult::SuccessServer` **and** return
   `Flow::Cancel`; `Flow::Continue` leaves vanilla's use untouched (the "cannot bridge" path).
3. **Block facts are on the handle, not the host.** `block_state_replaceable`, `sound`,
   `has_block_entity` and `name` are on `host.block_states()` (the `BlockStates`/`BlockMetadata`
   pair); the same-named methods on `BlockStatesBackend` are the game's private surface and are
   unreachable (`E0599`).
4. **`BlockStateRecord` ≠ `BlockState`.** `BlockStates::of(..)` (the `BlockMetadata` impl) returns
   `BlockStateRecord`; take `BlockState::of(&host.block_states(), block, &props)`.
5. **Host area traits are `Host` supertraits.** `host.player/…`, `host.living/…`,
   `host.item_block/…` resolve with only `Host` in scope; importing
   `mod_api::prelude::{PlayersHost, …}` is then an **unused import** and clippy `-D warnings`
   fails.
6. **`JavaString::to_string()` returns a `Result`** (it shadows `Display`), and a `Compound`'s
   keys are `&JavaString`: property names go through `to_string_lossy()`.
7. **`EntityRef` has no `living()`.** The host does: `host.living(entity)`. And
   `entity.bounding_box(host)` gives `[[f64; 3]; 2]` (the box's corners) — the model's `Aabb` wants
   exactly that, not `EntityRef::dimensions`.
8. **The remote runner does not know this path.** `build-slot.sh run` forwards `test` to the far
   box, which synced copy has no `ports/bridgingmod` directory: `cd: ports/bridgingmod: No such
   file or directory`. Use `BUILD_LOCAL=1` (the SDK crate compiles in place in a minute).
9. **Copy the pinned SDK in.** `cp -a ports/framedblocks/.horizon/sdk ports/bridgingmod/.horizon/`
   keeps every step offline; without it the first `cargo horizon-mod` command fetches from the
   network. (`.horizon/` is gitignored and never packed.)
10. **`cargo test` writes a `Cargo.lock`, and the build would pack it.** `rm -f Cargo.lock` right
    before `cargo horizon-mod build`, and `/Cargo.lock` in `.gitignore` (measured: both builds
    byte-identical only without it).
11. **A code-only port still declares `capabilities = ["registries"]`** — it is the schema's one
    value, not a content claim, and the build refuses an empty list.
12. **`right_click_item` fires on both logical sides.** Returning a result from the client half
    would predict a placement the server decides; the impl matches `HostSide::Server` first and
    answers `Flow::Continue` otherwise.

## Hooks

| 0.2.0 hook | 0.1.0 counterpart | What it does |
|---|---|---|
| `right_click_item` (v1, cancel, both) → `support::on_right_click` | `use_item` v1, HEAD, helper `use_item.rs` | On an air click with a placeable block, compute the reach-around target and place it; claim the use with `SuccessServer`. |

## Deviations from upstream (recorded)

- **Server-side target**, not a client mixin: the view is the acting player's
  head ray (upstream's first-person / `ALWAYS_EYELINE` case). Free-look and
  third-person cameras are not followed.
- **No client VFX**: crosshair arrows, block outline, F3 trace
  (`CrosshairRenderingMixin`, `OutlineRendererMixin`) are not ported; there is
  no client hook for a mod to draw them at this pin.
- **No keybind/toggle, no config file**: the pin has no mod keybind; the slice
  hard-codes the upstream defaults (`BridgingConfig.java:45-97`) and bridging
  is always enabled. A config file via `native_config_dir` is a later slice.
- **No `delayPostBridging`**: the client owns `rightClickDelay`; the server
  sees the client's normal use cadence (4 ticks), the same value.
- **Slab assist is not ported**: upstream aligns horizontal slabs with the
  client's sub-block y fraction (`Bridge.java:41-86`), which the server does
  not keep. Slabs place in the state this slice computes (see the gaps).
- **Special-item compat** (`DankStorage`, `BankStorage`, `PlaceableItem`,
  Sable environments) is not ported; only vanilla `BlockItem`s.
- **`setPlacedBy` editor paths** are not reproduced: a sign's block entity and
  text are applied, but the sign editor does not open; command, structure,
  jigsaw and test blocks are created but not initialised. A bridged
  pumpkin/skull does not run the wither/golem checks either (0.1.0's tail
  called `check_wither_spawn`/`check_golem_spawn`; this package has no route to
  them).

## Gaps

Recorded, not silently dropped. Each names the closest supported shape.

1. **The placed state is the block's default state plus the item's
   `minecraft:block_state` component** — not vanilla's `getStateForPlacement` from the fake hit.
   So a bridged stair, log or observer keeps the default orientation instead of the facing/slab
   half the player's yaw and the hit would give it. The SDK has no query for it: `state_for_placement`
   is a **Part** hook on the mod's own blocks, and `PlacementContext` is a wire struct that is not
   constructible. The closest supported shape is `BlockState::of(block, constraints)`.
   **This is the one visible fidelity loss of the migration**; bridging with dirt, stone, planks
   and other non-directional blocks is unaffected. Worth an upstream issue: *"a helper that returns
   a block's placement state from a fake hit (yaw, pitch, hit point, face, inside)"*.
2. **The write runs no `isUnobstructed`/`canSurvive` check.** Vanilla's `BlockItem.place` refuses a
   cell an entity fills or the state cannot survive in; `PlayerRef::place_block` writes the state
   as given. `LevelRef::has_collisions` is a possible proxy but is unverified in game here, so it is
   not used. 0.1.0 got both checks from `Behaviours::place_block`.
3. **`minecraft:placed_block` never fires.** `PlayerRef::trigger_criterion` refuses `minecraft:`
   triggers, and `trigger_block_use` fires only `default_block_use`. 0.1.0 fired the advancement
   trigger.
4. **`entity_place` is not notified.** Vanilla fires `EntityPlaceEvent` inside `BlockItem.place`;
   `PlayerRef::place_block` does not, and a mod cannot fire a hook. Other mods that cancel
   placements therefore do not see a bridged one.
5. **The crosshair check is the server's own ray.** 0.1.0's `game.pick()` returned a guest's
   client-reported hit (`forced_hit`) when there was one; the SDK exposes no such read, so a LAN
   guest's bridged click is checked against the server's eye ray. For the single-player host the two
   are the same ray.
6. **No diagnostics.** A native mod may not add the `log` crate and the SDK exports no logger; the
   mod has no command readout either, so a refused placement is silent (it falls through to vanilla,
   which is the intended UX).
7. **No in-game run of 0.2.0** — `horizon-launcher` is not installed here, so no launcher-built game
   carrying this package exists (see below).

## Upstream source

Repository `https://github.com/squeeglii/BridgingMod` (MIT), version **2.7.0**,
commit **`fdd6cf6738767795876536014c0a9519317fb766`** (2026-10-30). Areas read:
`common/src/main/java/me/cg360/mod/bridging/{mixin/MinecraftClientMixin,
raytrace/{BridgingStateTracker,PathTraversalHandler,Perspective,PlacementAlignment},
building/Bridge, util/{Path,GameSupport}, config/BridgingConfig,
config/selector/*}`. The clone was deleted after the findings were written.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

## Verification

Pin: SDK nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4`, mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78 (0.1.0 was verified at Horizon
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` / mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`).
Every 0.2.0 step ran through `ports/tools/build-slot.sh` with `BUILD_LOCAL=1`,
`CARGO_INCREMENTAL=0` and no `--target-dir`; the full table is in
"Migration to the SDK format → The ladder" above.

### 0.1.0 in-game evidence (historical — the retired format's run)

The 0.1.0 bytecode-injected bundle was verified in game on 2026-11-08 (single player, headless,
The Overworld, scratch `HORIZON_HOME`):

1. **bridge**: look 180/45 over the platform edge, `looking_at` = none, click
   dirt → (0,99,-1) `minecraft:dirt`, dirt 16 → 15, (0,99,-2) still air and
   the far platform untouched;
2. **sky**: look straight up, click dirt → 0 blocks placed, dirt 15 → 15
   (the click path alone places nothing);
3. **normal block click**: clear the gap, `interact 2 99 0 up` with dirt →
   vanilla places at (2,100,0), gap stays air, dirt 15 → 14 (the bridge does
   not divert a block click);
4. **counterfactual** (`COUNTERFACTUAL=1`): the same gap click with
   `minecraft:stick` → gap stays air, dirt 16 → 16.

Fail-capability was also shown by running the *same* script on the unmodded
shared game (`ports/_shared/bin/horizon-1d63e0a5/horizon-client`, no
`bridgingmod` module): it fails at phase 1 (`the gap holds minecraft:air,
expected minecraft:dirt`, exit 1). That run also caught a **false PASS** in the
first rig version (the crosshair was on a far platform, so vanilla placed the
dirt): the `looking_at == none` assertion was added because of it, and the far
platform moved out of reach.

Transcripts: `../_shared/evidence/bridgingmod/e2e-{default,counterfactual,unmodded-negative}.log`.
**They are the 0.1.0 bundle's**, not this package's: what changed is the *transport* of the same
placement (a typed host method instead of a `Game` call), the rules are byte-identical and tested.

### Not re-run in this migration

- **The in-game e2e**: it needs a *launcher-built* game
  (`horizon-launcher native-build ports/bridgingmod --instance <i> --trust-native-code`), and
  `horizon-launcher` is not installed in this environment. `tools/e2e.sh` is updated for it
  (`GAME=`, optional `CONTROL_GAME=`, `SELFTEST=1` inverting phase 1) and must be run against a
  launcher-built game before this package is called in-game verified.
- **The staged ladder** (`tools/horizon.py stage/check/test/build`) — those tools are deleted with
  the retired format.
- The 0.1.0 logs and package evidence in `../_shared/evidence/bridgingmod/` (`stage.log`,
  `fetch.log`, `check.log`, `cargo-*.log`, `test.log`, `build.log`, `clippy.log`, `crate-test.log`,
  `e2e-*.log`, `package.log`, `packages.txt`, `plan-*.json|txt`, `load-check-*.log`,
  `mods-page-*.json`) are kept as the retired bundle's history.
