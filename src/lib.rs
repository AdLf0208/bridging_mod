//! Bridging Mod for Horizon — the reach-around placement target model (pure) and its glue.
//!
//! A right click in the air with a placeable block item fills the gap the player looks into.
//! Upstream is a client-side aim assist; this is a clean-room server-side rewrite of its target
//! model, so the behaviour works without a client mod.
//!
//! [`runtime`] is the pre-migration target model, byte for byte, and is unit-tested on its own;
//! [`support`] is the glue that reads the world, the player and the held stack and places the
//! block. This file is the one `impl Hooks`: the click is `right_click_item`
//! (`PlayerInteractEvent.RightClickItem`, immediately before `Item.use`), which is the global
//! moment every item's use passes through — `use_item` is an item Part and would have to be
//! attached to every block item one by one.
use mod_api::{self, Flow, Hooks, HostSide, RightClickItemEvent};

pub mod runtime;
pub mod support;

/// The mod has no state of its own: every number is in [`runtime`] and everything else is read
/// from the click.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// A player right-clicked with `event.click.stack`: in the air, or on a block whose own use
    /// let the click through. Bridging fills the gap the player looks into and claims the use
    /// with `SuccessServer` (the server swings the hand); anything it cannot bridge lets the
    /// click continue, so vanilla's own use runs unchanged.
    fn right_click_item(&self, host: HostSide, event: &mut RightClickItemEvent<'_>) -> Flow {
        // The hook runs on both logical sides; only the server owns the world and the inventory,
        // and a client answer would predict a placement the server decides.
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        let Some(result) = support::on_right_click(host, event.click) else {
            return Flow::Continue;
        };
        *event.cancellation_result = result;
        Flow::Cancel
    }
}
