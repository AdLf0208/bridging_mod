# Bridging Mod for Horizon

A port of **Bridging Mod 2.7.0** ([squeeglii/BridgingMod](https://github.com/squeeglii/BridgingMod),
MIT) to Horizon's mod API (`api = "0.2"`, game 26.3).

Right-click in the air with a placeable block item while looking into a gap over an edge and the
block lands in the gap cell — no need to first place a block to aim at. Upstream does this on the
client (it intercepts `Minecraft.startUseItem` and sends a placement packet); this port computes
the same target from the acting player's eye ray **on the server**, so it works in single player
and on a server alike, with no client mod.

```text
bridgingmod.hmod              # the delivered package (one flat .hmod)
  horizon-mod.toml            # id, version, api, side, capabilities, [native] pins
  Cargo.toml                  # horizon-mod-api from .horizon/sdk/ (fetched, never packed)
  src/lib.rs                  # the one #[mod_api::horizon_mod] impl Hooks: right_click_item
  src/runtime.rs              # the pure target model + its 10 unit tests
  src/support.rs              # the game glue: eye ray, box, block facts, the placement tail
  LICENSE, LICENSE-NOTICE.md  # inside the package
  uses.json                   # written by the build: what the package uses
```

## Build and check

```sh
cp -a ../framedblocks/.horizon/sdk .horizon/sdk     # the pinned SDK, offline
CARGO_INCREMENTAL=0 cargo test                      # 15 tests: 10 runtime, 5 support
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                                   # NOT --all: that checks the fetched SDK
rm -f Cargo.lock && cargo horizon-mod build          # -> target/horizon-mod/bridgingmod.hmod
cp target/horizon-mod/bridgingmod.hmod ./bridgingmod.hmod
cargo horizon-mod check bridgingmod.hmod
cargo horizon-mod fits bridgingmod.hmod --build nightly
```

`Cargo.lock` is packed and changes the artifact: remove it immediately before the build and keep
it gitignored.

## Behaviour

- A right click with a placeable block item whose crosshair **misses** every block: walk the eye
  ray from 20 % of the reach out to the reach, and take the first cell that is replaceable, outside
  the player's box, and next to a solid non-liquid block on a side the view points into.
- Place the held block there, consuming one item (none in creative), with the place sound, the
  `used` statistic and the block's own data when the stack carries it.
- Torches are skipped, as upstream.
- Anything else — a click on a block, an item that places nothing, no reachable gap — falls
  through to vanilla, so the click behaves exactly as it would without the mod.

## Not ported (see `notes.md`, "Gaps")

The client VFX (crosshair arrows, block outline, F3 trace), the keybind and config screen, the
slab/perspective alignment helpers, and the special-item compatibility layer (DankStorage,
BankStorage, PlaceableItem, Sable). The placed state is the block's default state plus the item's
`minecraft:block_state` component — vanilla's `getStateForPlacement` has no SDK query yet.

## Licence

MIT. Local-only until the maintainer approves publishing (ARCHITECTURE.md §8). `LICENSE` and
`LICENSE-NOTICE.md` travel inside the package.
