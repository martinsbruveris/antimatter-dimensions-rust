use crate::infinity_upgrades::InfinityUpgrade;
use crate::state::GameState;

/// A single legal player action.
///
/// This is an *action vocabulary* — a caller's intent expressed as a value that
/// [`GameState::apply_action`] turns into a state mutation. It exists for
/// consumers that must treat actions as **data**: the simulation layer
/// (`ad-sim`, its only consumer today), the Python bindings, and a potential
/// future replay/recording format.
///
/// It is deliberately **not** a universal mutation seam. The GUI's commands,
/// the autobuyers, and the Automator call the underlying `GameState` methods
/// directly, and should keep doing so: in the original game the "same" action
/// often has different per-caller semantics (e.g. the manual Galaxy purchase
/// checks confirmations and the RU7 lock, while the Galaxy autobuyer's
/// `requestGalaxyReset(bulk, limit)` applies its cap to the purchase but not
/// the timer reset). A unified vocabulary would either need per-caller
/// variants or would flatten those distinctions and break fidelity — so the
/// per-caller engine methods *are* the shared interface, and this enum is a
/// thin adapter over the manual-play subset of them.
///
/// The vocabulary therefore only covers what `ad-sim` plays today (pre-Infinity
/// strategy runs plus a few prestige actions). Grow it demand-driven — one
/// prestige layer at a time as the simulation frontier advances — rather than
/// per GUI command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Action {
    /// Buy a single unit of the dimension tier (0-indexed).
    BuyDimension(usize),
    /// Buy as many of the dimension tier as affordable.
    BuyMaxDimension(usize),
    /// Buy the dimension tier up to the next group of ten.
    BuyUntil10Dimension(usize),
    /// Buy a single tickspeed upgrade.
    BuyTickspeed,
    /// Buy as many tickspeed upgrades as affordable.
    BuyMaxTickspeed,
    /// Buy one antimatter galaxy.
    Galaxy,
    /// Buy one dimension boost.
    DimBoost,
    /// Sacrifice dimensions.
    Sacrifice,
    /// Perform a Big Crunch (first Infinity).
    Crunch,
    /// Perform an Eternity (second prestige).
    Eternity,
    /// Buy an Infinity Upgrade from the grid.
    BuyInfinityUpgrade(InfinityUpgrade),
    /// Start a normal challenge (2..=12).
    StartChallenge(u8),
    /// Exit the current normal challenge.
    ExitChallenge,
    /// Unlock the antimatter-dimension autobuyer for the tier (0-indexed).
    UnlockAdAutobuyer(usize),
    /// Unlock the tickspeed autobuyer.
    UnlockTickspeedAutobuyer,
    /// Set the global autobuyers on/off switch.
    SetAutobuyers(bool),
}

/// The result of applying an [`Action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionOutcome {
    /// Whether the action changed game state (a purchase, reset, or toggle).
    pub applied: bool,
    /// Units affected. For single actions this is 0 or 1; for bulk buy actions it
    /// is the number bought.
    pub count: u64,
}

impl ActionOutcome {
    /// Outcome of an action that either happens once or not at all.
    fn single(applied: bool) -> Self {
        Self {
            applied,
            count: applied as u64,
        }
    }

    /// Outcome of a bulk action that bought `count` units.
    fn bulk(count: u64) -> Self {
        Self {
            applied: count > 0,
            count,
        }
    }
}

impl GameState {
    /// Apply an [`Action`], routing it to the corresponding game logic.
    ///
    /// This is the mutation seam for action-as-data consumers (`ad-sim`, the
    /// Python bindings); other callers use the `GameState` methods directly —
    /// see the [`Action`] docs. Actions that are not currently legal
    /// (unaffordable purchase, unmet requirement) are no-ops that return
    /// `applied == false` — the per-mechanic `can_*` checks already guard each
    /// path.
    pub fn apply_action(&mut self, action: Action) -> ActionOutcome {
        match action {
            Action::BuyDimension(tier) => {
                ActionOutcome::single(self.buy_dimension(tier))
            }
            Action::BuyMaxDimension(tier) => {
                ActionOutcome::bulk(self.buy_max_dimension(tier))
            }
            Action::BuyUntil10Dimension(tier) => {
                ActionOutcome::bulk(self.buy_until_10_dimension(tier))
            }
            Action::BuyTickspeed => ActionOutcome::single(self.buy_tickspeed()),
            Action::BuyMaxTickspeed => ActionOutcome::bulk(self.buy_max_tickspeed()),
            Action::Galaxy => ActionOutcome::single(self.buy_galaxy()),
            Action::DimBoost => ActionOutcome::single(self.buy_dim_boost()),
            Action::Sacrifice => ActionOutcome::single(self.sacrifice()),
            Action::Crunch => ActionOutcome::single(self.big_crunch()),
            Action::Eternity => ActionOutcome::single(self.eternity()),
            Action::BuyInfinityUpgrade(upgrade) => {
                ActionOutcome::single(self.buy_infinity_upgrade(upgrade))
            }
            Action::StartChallenge(id) => {
                ActionOutcome::single(self.start_challenge(id))
            }
            Action::ExitChallenge => ActionOutcome::single(self.exit_challenge()),
            Action::UnlockAdAutobuyer(tier) => {
                ActionOutcome::single(self.unlock_ad_autobuyer(tier))
            }
            Action::UnlockTickspeedAutobuyer => {
                ActionOutcome::single(self.unlock_tickspeed_autobuyer())
            }
            Action::SetAutobuyers(on) => {
                let changed = self.autobuyers.enabled != on;
                self.autobuyers.enabled = on;
                ActionOutcome::single(changed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::{AD_AUTOBUYER_REQUIREMENTS, BIG_CRUNCH_THRESHOLD};
    use break_infinity::Decimal;

    #[test]
    fn buy_dimension_routes_and_spends_antimatter() {
        let mut game = GameState::new();
        let before = game.antimatter;
        let cost = game.dimension_cost(0);

        let out = game.apply_action(Action::BuyDimension(0));

        assert!(out.applied);
        assert_eq!(out.count, 1);
        assert_eq!(game.dimensions[0].bought, 1);
        assert_eq!(game.antimatter, before - cost);
    }

    #[test]
    fn unaffordable_action_is_a_no_op() {
        let mut game = GameState::new();
        game.antimatter = Decimal::ZERO;

        let out = game.apply_action(Action::BuyDimension(0));

        assert!(!out.applied);
        assert_eq!(out.count, 0);
        assert_eq!(game.dimensions[0].bought, 0);
    }

    #[test]
    fn buy_max_dimension_reports_count() {
        let mut game = GameState::new();
        game.antimatter = Decimal::from_float(1e6);

        let out = game.apply_action(Action::BuyMaxDimension(0));

        assert!(out.applied);
        assert!(out.count > 1);
        assert_eq!(game.dimensions[0].bought, out.count);
    }

    #[test]
    fn crunch_routes_to_big_crunch() {
        let mut game = GameState::new();
        game.antimatter = BIG_CRUNCH_THRESHOLD;

        let out = game.apply_action(Action::Crunch);

        assert!(out.applied);
        assert!(game.infinity_unlocked);
        assert!(!game.can_big_crunch());
    }

    #[test]
    fn unlock_ad_autobuyer_respects_requirement() {
        let mut game = GameState::new();

        // Below requirement: no-op.
        game.total_antimatter = Decimal::ZERO;
        assert!(!game.apply_action(Action::UnlockAdAutobuyer(0)).applied);
        assert!(!game.autobuyers.dimensions[0].is_bought);

        // At requirement: unlocks.
        game.total_antimatter = AD_AUTOBUYER_REQUIREMENTS[0];
        assert!(game.apply_action(Action::UnlockAdAutobuyer(0)).applied);
        assert!(game.autobuyers.dimensions[0].is_bought);
    }

    #[test]
    fn set_autobuyers_toggles_global_flag() {
        let mut game = GameState::new();
        assert!(game.autobuyers.enabled);

        let out = game.apply_action(Action::SetAutobuyers(false));
        assert!(out.applied);
        assert!(!game.autobuyers.enabled);

        // Setting to the same value is a no-op.
        assert!(!game.apply_action(Action::SetAutobuyers(false)).applied);
    }
}
