#!/usr/bin/env bash
# In-game check for Bridging Mod (reach-around placement).
#
# The player stands on the south platform at its edge, looks north and down
# over the edge into the void, and right-clicks in the air with dirt. The
# crosshair must MISS (no block within reach), so the click reaches the mod's
# `right_click_item` hook and the port fills the cell beside the platform,
# (0,99,-1), from the platform block at (0,99,0). Upstream is a client-only aim
# assist; here the server's own pick misses, so the server can do the same
# target.
#
# Since the SDK migration (2026-11-18) the mod is a flat `.hmod` the launcher
# compiles into the game, so GAME must name a game the launcher built with it:
#   horizon-launcher native-build ports/bridgingmod --instance <i> --trust-native-code
# and then `GAME=<that horizon-client> tools/e2e.sh`. The default below is the
# retired `tools/horizon.py build` output and no longer exists; the script fails
# with "missing binary" rather than running an unmodded game, which is intended.
#
#   GAME=… tools/e2e.sh                        # bridge + negatives, below
#   GAME=… COUNTERFACTUAL=1 tools/e2e.sh       # same click with a stick: nothing happens
#   GAME=… SELFTEST=1 tools/e2e.sh             # phase 1 asserted inverted: must FAIL
#   GAME=… CONTROL_GAME=<unmodded game> tools/e2e.sh   # extra leg B: no mod, no bridge
#
# Why the crosshair must miss (and is asserted): with a block under the
# crosshair, Minecraft's own use-on-block would place dirt at (0,99,-1) from the
# far platform's south face, and the check would pass without the port (the
# false PASS this scenario was rewritten to stop on 2026-11-08).
#
# Phases in the default run:
#   1. bridge: a miss into the gap places exactly one dirt at (0,99,-1);
#   2. sky: a block item clicked with no target places nothing and spends
#      nothing (the click path itself cannot place);
#   3. normal block click: vanilla places on the clicked face, the bridging
#      gap stays empty, and the bridge did not divert the placement;
#   4. (COUNTERFACTUAL=1) the same gap click with a plain stick: nothing is
#      placed and no dirt is spent.
set -euo pipefail
trap 'echo "FAILED at line $LINENO: $BASH_COMMAND" >&2' ERR

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/bridgingmod
SHARED=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$SHARED/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/bridgingmod-e2e.sock}
COUNTERFACTUAL=${COUNTERFACTUAL:-0}
SELFTEST=${SELFTEST:-0}

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || {
    echo "missing horizon-client: $GAME" >&2
    echo "build one with the mod: horizon-launcher native-build ports/bridgingmod --instance <i> --trust-native-code" >&2
    exit 1
}
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
echo "ctl: $CTL"
echo "game: $GAME"

if [ "$COUNTERFACTUAL" = 1 ]; then
    TOOL=minecraft:stick
    EXPECT=minecraft:air
    EXPECT_DELTA=0
elif [ "$SELFTEST" = 1 ]; then
    # The inverted phase-1 assertion: with the bridge working this must FAIL.
    TOOL=minecraft:dirt
    EXPECT=minecraft:air
    EXPECT_DELTA=0
else
    TOOL=minecraft:dirt
    EXPECT=minecraft:dirt
    EXPECT_DELTA=1
fi

SCRATCH=$(mktemp -d /tmp/opencode/bridgingmod-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
# The game's game folder defaults to the working directory: run from scratch so
# saves/options/caches never land in the port tree.
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
count_item() { ctl inventory | jq -r --arg i "$1" '[.counts[$i] // 0] | add'; }
# One cell's block name (air when the reply has no block).
cell() { ctl block "$1" "$2" "$3" | jq -r '.block // "minecraft:air"'; }
# The crosshair's block, or "none" when the pick misses.
crosshair() { ctl state | jq -r '.looking_at.kind'; }
# The crosshair must be a miss: a block click would be vanilla's placement.
expect_miss() {
    local kind
    kind=$(crosshair)
    echo "crosshair: $kind ($1)"
    [ "$kind" = "none" ] || fail "the crosshair is on a $kind ($1): the click would be vanilla"
}

STATUS=0
cleanup() {
    STATUS=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" || true
    echo "EXIT:$STATUS"
}
trap cleanup EXIT

echo "== launch (survival world BridgingE2E) =="
"$CTL" --socket "$SOCK" launch \
    --game "$GAME" --world BridgingE2E --seed 20261105 --headless --wait \
    --game-dir "$SCRATCH/game" --resources "$RES"
ctl wait-ready 120

echo "== a near platform with an open edge; the gap cell is beside it =="
ctl command "time set noon"
ctl command "difficulty peaceful"
ctl command "clear @s"
# Open air around the rig so the look ray misses everything below the platform.
ctl fill -6 95 -8 6 110 8 minecraft:air
# The near platform: z=0..2 at y=99. The gap cell (0,99,-1) sits off its north
# edge and is built off the platform block at (0,99,0).
ctl fill -2 99 0 2 99 2 minecraft:stone
# A platform out of reach (>= 6 blocks away) stays as the far scenery control;
# it must not move in any phase.
ctl fill -2 99 -6 2 99 -5 minecraft:stone
ctl tp 0 100 1
ctl wait 2

# Yaw 180 is north (-Z); pitch 45 looks down 45 degrees over the edge.
ctl look 180 45
ctl wait 2

ctl give minecraft:dirt 16 >/dev/null || fail "cannot give dirt"
ctl give minecraft:stick 1 >/dev/null || fail "cannot give the control stick"
ctl wait 2
ctl select "$TOOL" >/dev/null || fail "cannot select $TOOL"
ctl wait 2
held=$(ctl inventory | jq -r '.held.item // "none"')
echo "held: $held"
[ "$held" = "$TOOL" ] || fail "the held item is $held, expected $TOOL"

echo "== phase 1: right click in the air with $TOOL =="
expect_miss "the bridge click"
before=$(count_item minecraft:dirt)
gap_before=$(cell 0 99 -1)
echo "dirt before: $before"
echo "gap before: $gap_before"
[ "$gap_before" = "minecraft:air" ] || fail "the gap is not empty before the click"
[ "$(cell 0 99 -2)" = "minecraft:air" ] || fail "the cell beyond the gap is not air"

ctl use-item
ctl wait 10

after=$(count_item minecraft:dirt)
gap_after=$(cell 0 99 -1)
echo "dirt after: $after"
echo "gap after: $gap_after"

[ "$gap_after" = "$EXPECT" ] || fail "the gap holds $gap_after, expected $EXPECT"
[ $((before - after)) -eq "$EXPECT_DELTA" ] \
    || fail "the click consumed $((before - after)) dirt, expected $EXPECT_DELTA"
# The placement is the gap cell alone: the cell beyond it and the far platform
# (out of reach) must be untouched, so vanilla's use-on-block is not the source.
[ "$(cell 0 99 -2)" = "minecraft:air" ] || fail "the click placed at (0,99,-2), beyond the gap"
[ "$(cell 0 99 -6)" = "minecraft:stone" ] || fail "the click changed the far platform"

if [ "$SELFTEST" = 1 ]; then
    # Reached only if the inverted assertion held, i.e. the bridge did not fire:
    # that means phase 1 cannot tell the mod from the unmodded game.
    echo "SELFTEST: the bridge did not fire, so phase 1's assertion is vacuous"
    exit 0
fi

if [ "$COUNTERFACTUAL" = 1 ]; then
    # Nothing else may have moved: the far platform and the cell beyond the
    # gap stay as they were.
    [ "$(cell 0 99 -2)" = "minecraft:air" ] || fail "the counterfactual changed (0,99,-2)"
    [ "$(cell 0 99 -6)" = "minecraft:stone" ] || fail "the counterfactual changed the far platform"
    echo "PASS (counterfactual, plain minecraft:stick): the gap stayed empty and no dirt was used"
    exit 0
fi

echo "== phase 2: a block item clicked with no target places nothing =="
# Straight up: no replaceable cell along the ray has a solid support.
ctl look 180 -90
ctl wait 2
expect_miss "the sky click"
sky_before=$(count_item minecraft:dirt)
ctl use-item
ctl wait 10
sky_after=$(count_item minecraft:dirt)
above=$(ctl blocks 0 102 1 0 106 1 | jq -r '.blocks | length')
echo "dirt before/after: $sky_before/$sky_after, blocks above: $above"
[ "$sky_after" -eq "$sky_before" ] || fail "the sky click spent $((sky_before - sky_after)) dirt"
[ "$above" -eq 0 ] || fail "the sky click placed $above block(s)"
[ "$(cell 0 99 -1)" = "minecraft:dirt" ] || fail "the sky click changed the bridged gap"

echo "== phase 3: a normal block click is vanilla's, not the bridge's =="
# An empty gap again, then a click with a real block under the crosshair: the
# dirt goes on the clicked face, and the gap must stay empty.
ctl setblock 0 99 -1 minecraft:air
ctl wait 2
[ "$(cell 0 99 -1)" = "minecraft:air" ] || fail "cannot clear the gap"
block_before=$(count_item minecraft:dirt)
ctl interact 2 99 0 up
ctl wait 10
block_after=$(count_item minecraft:dirt)
placed=$(cell 2 100 0)
gap_after_block=$(cell 0 99 -1)
echo "dirt before/after: $block_before/$block_after"
echo "(2,100,0): $placed   gap (0,99,-1): $gap_after_block"
[ "$placed" = "minecraft:dirt" ] || fail "the normal block click placed $placed at (2,100,0)"
[ "$gap_after_block" = "minecraft:air" ] \
    || fail "the bridge fired on a block click: the gap holds $gap_after_block"
[ $((block_before - block_after)) -eq 1 ] \
    || fail "the normal block click spent $((block_before - block_after)) dirt, expected 1 (vanilla)"

echo "PASS: the air click filled the gap cell beside the platform and consumed one dirt; a sky click and a block click placed nothing in the gap"

if [ -n "$CONTROL_GAME" ] && [ -x "$CONTROL_GAME" ]; then
    # Leg B: the same click on a game built without this package. It must NOT
    # bridge, so the pass above is the mod's and not the engine's.
    echo "== leg B: the unmodified game ($CONTROL_GAME) does not bridge =="
    ctl quit >/dev/null 2>&1 || true
    sleep 2
    "$CTL" --socket "$SOCK" launch \
        --game "$CONTROL_GAME" --world BridgingControl --seed 20261105 --headless --wait \
        --game-dir "$SCRATCH/control" --resources "$RES"
    ctl wait-ready 120
    ctl command "difficulty peaceful"
    ctl fill -6 95 -8 6 110 8 minecraft:air
    ctl fill -2 99 0 2 99 2 minecraft:stone
    ctl fill -2 99 -6 2 99 -5 minecraft:stone
    ctl tp 0 100 1
    ctl look 180 45
    ctl wait 2
    ctl give minecraft:dirt 16 >/dev/null
    ctl select minecraft:dirt >/dev/null
    ctl wait 2
    ctl use-item
    ctl wait 10
    control_gap=$(cell 0 99 -1)
    echo "control gap: $control_gap"
    [ "$control_gap" = "minecraft:air" ] \
        || fail "the unmodified game filled the gap, so phase 1 does not prove the mod"
fi
