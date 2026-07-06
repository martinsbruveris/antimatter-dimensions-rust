use crate::infinity_upgrades::InfinityUpgrade;
use crate::state::GameState;

/// A single legal player action.
///
/// This is an *action vocabulary* — a caller's intent expressed as a value that
/// [`GameState::apply_action`] turns into a state mutation. Today the simulation
/// layer (`ad-sim`) is its only consumer: it produces `Action`s and feeds them to
/// `apply_action`. The GUI and the autobuyers do **not** go through this seam —
/// they call the underlying `GameState` methods directly.
///
/// Because of that, the vocabulary is *not* exhaustive over the engine's action
/// surface. It holds the variants the simulation needs, plus a few prestige
/// actions; several implemented mechanics (Break Infinity, Infinity Challenges,
/// Infinity Dimensions, Break Infinity upgrades) have no `Action` yet. The
/// intended end state is to route every caller — including the GUI — through
/// `apply_action`, so the set becomes exhaustive and a mechanic that forgets to
/// wire up its action is a compile error; that refactor is still pending.
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
    /// This is the single mutation seam used by every action producer. Actions
    /// that are not currently legal (unaffordable purchase, unmet requirement) are
    /// no-ops that return `applied == false` — the per-mechanic `can_*` checks
    /// already guard each path.
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
