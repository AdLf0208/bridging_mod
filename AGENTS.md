# Working on Bridging Mod for Horizon (native Horizon mod, SDK format)

> **Build slots (v2):** every build/test/lint step goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the kernel** until a slot is
> free — a freed slot is taken in ~0.07 s — so **never poll**: no `while ! slot; sleep` loop, no
> background build, never `--target-dir`. Read `../_shared/build-slots-usage.md` before running
> anything; observe with `status` / `watch` / `events` / `stats`. The runner does **not** cd into
> `--port`, so pass `-- bash -c 'cd ports/bridgingmod && …'`, and use `BUILD_LOCAL=1` (the far box
> has no copy of this port). The runner's step names speak the retired `.hnmod` tool; for this
> package they only set the thread count.

This repository is the port, not Horizon and not horizon-mixins. Never modify the original
checkout or player resources; do not commit `target/`, `.horizon/`, exported sources or Minecraft
assets.

**There is no staging step and no mixin plan.** One flat `.hmod` holds `horizon-mod.toml` (the only
identity, `api = "0.2"`, `[native]` pins), `Cargo.toml` (its only dependency is `horizon-mod-api`
from the fetched `.horizon/sdk/`), `src/lib.rs` (the one `#[mod_api::horizon_mod] impl Hooks`) and
`src/support.rs` (the game glue). The retired `.hnmod` layout is history:
`../_shared/evidence/bridgingmod/retired-hnmod-0.1.0/`.

Keep the target model and every number in `src/runtime.rs` with its test; the glue only reads the
game, calls `runtime`, and writes the result back. **`src/runtime.rs` is byte-identical to the
retired port's file — do not "tidy" it** (its sha256 is `fa82efe2…`). A test for a glue helper
belongs next to it in `src/support.rs`.

The one hook is `Hooks::right_click_item` (`PlayerInteractEvent.RightClickItem`, the moment every
item's use passes through). `use_item` is an **item Part**: it would have to be attached to every
block item one by one, so it is not the route here.

Validate, in this order, each inside a lease:

```sh
CARGO_INCREMENTAL=0 cargo test                        # 15 tests: 10 runtime, 5 support
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                                     # NOT --all: that checks the fetched SDK
rm -f Cargo.lock && cargo horizon-mod build           # target/horizon-mod/bridgingmod.hmod
cp target/horizon-mod/bridgingmod.hmod ./bridgingmod.hmod
cargo horizon-mod check bridgingmod.hmod              # the registry's own check
cargo horizon-mod fits bridgingmod.hmod --build nightly
sha256sum bridgingmod.hmod                            # quote it in notes.md
```

A `Cargo.lock` is packed and changes the artifact: remove it immediately before the build and keep
it gitignored. The in-game leg needs a game the launcher built with this package
(`horizon-launcher native-build`), which is not installed here — do not claim an in-game result
without one.
