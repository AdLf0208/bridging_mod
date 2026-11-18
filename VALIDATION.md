# Validation

**This file is the 0.1.0 (`.hnmod`) record, kept as history.** The package is now a flat SDK
`.hmod` (`api = "0.2"`, version 0.2.0) — the current ladder, its results and the evidence files are
in [`notes.md`](notes.md) → "Migration to the SDK format → The ladder". The retired layout and its
tools live in
[`../_shared/evidence/bridgingmod/retired-hnmod-0.1.0/`](../_shared/evidence/bridgingmod/retired-hnmod-0.1.0/).

## 0.1.0 record

Pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` (main, 2026-11-07),
horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6).
Minecraft 26.3 assets/build from `26.3.zip`; prepared resources at
`/home/adlf/Desktop/horizon/.local/horizon-client/26.3`.

Re-pinned from `c9d474d`/`d9c83d7` on 2026-11-08. Every build/test/e2e step ran
through the shared registry (`ports/tools/build-slot.sh`), with
`CARGO_INCREMENTAL=0`, no `--target-dir`, no private `BUILD_SLOTS_DIR`, and
`HORIZON_MIXINS=ports/_shared/pins/horizon-mixins-5a64af9`.

## Standalone crate (in `ports/bridgingmod`)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings   # in a slot
cargo test  --workspace --locked --offline                                # in a slot
```

Results: fmt clean, clippy exit 0 with no diagnostics, **10 passed / 0 failed**
(`game::bridgingmod` model tests run standalone too).

## Pinned ladder

```sh
# once, online, because the pin checkout is shallow and its lock gained crates:
build-slot.sh run --job bridge-1d63e0a --port ports/bridgingmod --step fetch -- \
  bash -c 'cd "$HORIZON_STAGE_ROOT/modded" && cargo fetch --locked'      # exit 0

# then, offline, in a lease each:
python3 tools/horizon.py stage
python3 tools/horizon.py check
python3 tools/horizon.py test
python3 tools/horizon.py build
```

Results (see `../_shared/evidence/bridgingmod/{stage,check,test,build}.log`): stage exit 0,
check `cargo_exit: 0` with 0 warnings, test **10 passed / 0 failed / 1240 filtered**,
build exit 0 with 0 warnings.

## In-game (0.1.0)

`tools/e2e.sh` (headless, scratch `HORIZON_HOME`, single player): **PASS**, `EXIT:0` — the air
click filled (0,99,-1), a sky click placed nothing, a block click was vanilla's.
`COUNTERFACTUAL=1` **PASS**, `EXIT:0`; the same script on the unmodded shared game **FAILED at
phase 1** (exit 1), so the assertion is not vacuous. Transcripts:
`../_shared/evidence/bridgingmod/e2e-{default,counterfactual,unmodded-negative}.log`.

## Package (0.1.0)

`tools/package.py`: 10,094 B, SHA-256
`916b6acd50570ee8b608f748817638ab39f7701e3100ea3dd39c1ef6cff67500`, 5 entries; the
bundle's plan check (`horizon-mixins plan`) reported `status: ok`, 1 injection (`use_item` v1),
2 modules; the game parsed it (`bridgingmod 0.1.0: OFF (Not built)`).
