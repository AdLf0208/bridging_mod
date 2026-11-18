# Bridging Mod — a Horizon port

**Bridging Mod** is a Minecraft mod. A mod for Minecraft (1.17-1.21+) which adds bedrock-style reacharound-placement mechanics for Fabric, Quilt & Forge. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [squeeglii/BridgingMod](https://github.com/squeeglii/BridgingMod) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Bridging Mod ([source](https://github.com/squeeglii/BridgingMod)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `bridgingmod.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- On a right click **in the air** with a placeable block item (main hand, then
- Return the first cell along the ray that is replaceable (air, or any
- Place the held block at that cell, consuming one item (none in creative),
- Torches are skipped (`skipTorchBridging`), as upstream.
- A cell already inside the player's body is skipped, and the search continues
- If the mod cannot place (no reachable gap, no permission, the write refused),

## Not yet ported

- **The placed state is the block's default state plus the item's
- **The write runs no `isUnobstructed`/`canSurvive` check.** Vanilla's `BlockItem.place` refuses a
- **`minecraft:placed_block` never fires.** `PlayerRef::trigger_criterion` refuses `minecraft:`
- **`entity_place` is not notified.** Vanilla fires `EntityPlaceEvent` inside `BlockItem.place`;
- **The crosshair check is the server's own ray.** 0.1.0's `game.pick()` returned a guest's
- **No diagnostics.** A native mod may not add the `log` crate and the SDK exports no logger; the
- **No in-game run of 0.2.0** — `horizon-launcher` is not installed here, so no launcher-built game

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `bridgingmod.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Bridging Mod was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/squeeglii/BridgingMod

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run bridgingmod` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/bridgingmod/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
