use ad_core::action::Action;
use ad_core::observed::ObservedState;
use ad_core::state::GameState;
use break_infinity::Decimal;

use crate::strategy::{
    BuyPriority, DimensionOrder, PrestigeMode, PrestigeStep, SacrificeConfig,
    StrategyConfig,
};

/// An external agent that decides which legal actions to take.
///
/// A controller is *not* a game mechanic: it lives outside the
/// engine and may only influence the game by emitting [`Action`]s,
/// which the engine validates. This keeps "deciding" cleanly
/// separated from "the rules" — a controller bug can at most pick
/// a legal action at the wrong moment, never an illegal state
/// transition.
///
/// The per-tick `act` shape matches both the in-game autobuyer
/// timer model and (later) the Automator's interval-throttled VM,
/// so every action producer — manual play, a fixed strategy, the
/// real Automator — is one implementation of this trait.
pub trait Controller {
    /// Run once before the simulation loop, with mutable access to
    /// the fresh game state. Use it to set engine flags the
    /// controller relies on. The default does nothing.
    fn on_start(&mut self, _game: &mut GameState) {}

    /// Decide what to do given the current observation.
    ///
    /// Returns the actions to apply now. The driver calls `act`
    /// repeatedly within a tick (re-observing between calls) until
    /// it returns no further applicable action, so a controller may
    /// return one action at a time and rely on the next call seeing
    /// the updated state.
    fn act(&mut self, obs: &ObservedState, game_time_ms: f64) -> Vec<Action>;
}

/// Tracks progress through a fixed prestige plan.
#[derive(Debug)]
struct PlanCursor {
    /// Index into the prestige plan.
    step_index: usize,
    /// For DimBoost(N) steps, how many boosts have been bought in
    /// this step so far.
    boosts_done: u32,
}

impl PlanCursor {
    fn new() -> Self {
        Self {
            step_index: 0,
            boosts_done: 0,
        }
    }

    /// Returns the current step, or None if the plan is exhausted.
    fn current_step(&self, plan: &[PrestigeStep]) -> Option<PrestigeStep> {
        plan.get(self.step_index).copied()
    }

    /// Advance to the next step in the plan.
    fn advance(&mut self) {
        self.step_index += 1;
        self.boosts_done = 0;
    }
}

/// A controller that plays a fixed [`StrategyConfig`]: an
/// "infinitely fast" player that, every tick, buys everything its
/// strategy permits.
///
/// This is the re-expression of the original `execute_strategy`
/// loop as a [`Controller`]. The driver ([`run_simulation`]) calls
/// [`act`] repeatedly within a tick; each call returns the single
/// next primitive action, mirroring one step of the original
/// loop's control flow. The `in_buy_step` flag reproduces the
/// original ordering exactly: prestige and sacrifice are only
/// re-checked at buy-loop boundaries, never between individual
/// buy-max operations.
///
/// [`act`]: Controller::act
/// [`run_simulation`]: crate::simulator::run_simulation
#[derive(Debug)]
pub struct StrategyController {
    config: StrategyConfig,
    cursor: PlanCursor,
    /// Whether we are mid buy-loop. While set, only buy actions are
    /// emitted; prestige/sacrifice are re-checked once the buy loop
    /// drains.
    in_buy_step: bool,
}

impl StrategyController {
    pub fn new(config: StrategyConfig) -> Self {
        Self {
            config,
            cursor: PlanCursor::new(),
            in_buy_step: false,
        }
    }

    /// Decide the next prestige action (and advance the plan cursor
    /// as needed). Returns `None` if no prestige is due right now.
    fn prestige_action(&mut self, obs: &ObservedState) -> Option<Action> {
        match &self.config.prestige {
            PrestigeMode::Auto => {
                // Galaxy takes priority — a permanent tickspeed
                // improvement that helps more than another boost.
                if obs.can_buy_galaxy {
                    Some(Action::Galaxy)
                } else if obs.can_dim_boost {
                    Some(Action::DimBoost)
                } else {
                    None
                }
            }
            PrestigeMode::Plan(plan) => loop {
                let step = self.cursor.current_step(plan)?;
                match step {
                    PrestigeStep::DimBoost(total) => {
                        if self.cursor.boosts_done >= total {
                            self.cursor.advance();
                            continue;
                        }
                        if obs.can_dim_boost {
                            self.cursor.boosts_done += 1;
                            if self.cursor.boosts_done >= total {
                                self.cursor.advance();
                            }
                            return Some(Action::DimBoost);
                        }
                        return None;
                    }
                    PrestigeStep::Galaxy => {
                        if obs.can_buy_galaxy {
                            self.cursor.advance();
                            return Some(Action::Galaxy);
                        }
                        return None;
                    }
                }
            },
        }
    }

    /// Whether to sacrifice now. Matches the JS autobuyer rule:
    /// sacrifice when the next boost meets the configured gain
    /// ratio.
    fn should_sacrifice(&self, obs: &ObservedState) -> bool {
        let SacrificeConfig {
            enabled,
            min_gain_ratio,
        } = self.config.sacrifice;
        enabled
            && obs.can_sacrifice
            && obs.next_sacrifice_boost.to_f64() >= min_gain_ratio
    }

    /// One iteration of the buy loop: compare the (weighted)
    /// tickspeed cost against the best eligible dimension and return
    /// the buy-max action for the cheaper, or `None` if nothing is
    /// affordable.
    fn best_buy(&self, obs: &ObservedState) -> Option<Action> {
        let tickspeed_cost = obs.tickspeed.cost;
        let can_afford_tickspeed = obs.antimatter >= tickspeed_cost;
        let dim_option = best_dimension(obs, self.config.purchase.dimension_order);

        let BuyPriority::Weighted { tickspeed_weight } = self.config.purchase.priority;

        match (can_afford_tickspeed, dim_option) {
            (true, Some((tier, dim_cost))) => {
                let effective_ts =
                    tickspeed_cost / Decimal::from_float(tickspeed_weight);
                if effective_ts <= dim_cost {
                    Some(Action::BuyMaxTickspeed)
                } else {
                    Some(Action::BuyMaxDimension(tier))
                }
            }
            (true, None) => Some(Action::BuyMaxTickspeed),
            (false, Some((tier, _))) => Some(Action::BuyMaxDimension(tier)),
            (false, None) => None,
        }
    }
}

impl Controller for StrategyController {
    fn on_start(&mut self, game: &mut GameState) {
        // The strategy does all the buying itself, so the in-game
        // autobuyers must stay out of the way. Route the toggle
        // through the action seam like every other mutation.
        game.apply_action(Action::SetAutobuyers(false));
    }

    fn act(&mut self, obs: &ObservedState, _game_time_ms: f64) -> Vec<Action> {
        // Continue draining an in-progress buy loop first; prestige
        // and sacrifice are not re-checked until it is empty.
        if self.in_buy_step {
            if let Some(buy) = self.best_buy(obs) {
                return vec![buy];
            }
            self.in_buy_step = false;
        }

        // Prestige is greedy and takes priority (it resets progress,
        // so there is no point buying just before one).
        if let Some(prestige) = self.prestige_action(obs) {
            return vec![prestige];
        }

        // No prestige: begin a buy loop pass with a single sacrifice
        // check, then buys.
        self.in_buy_step = true;
        if self.should_sacrifice(obs) {
            return vec![Action::Sacrifice];
        }
        if let Some(buy) = self.best_buy(obs) {
            return vec![buy];
        }

        // Nothing left to do this tick.
        self.in_buy_step = false;
        Vec::new()
    }
}

/// Find the best dimension to buy according to the given order.
/// Returns `(tier, unit_cost)` or `None` if no unlocked dimension
/// is affordable (single unit).
fn best_dimension(
    obs: &ObservedState,
    order: DimensionOrder,
) -> Option<(usize, Decimal)> {
    let unlocked = obs.unlocked_dimensions;

    match order {
        DimensionOrder::HighestFirst => (0..unlocked)
            .rev()
            .find_map(|tier| affordable(obs, tier).map(|cost| (tier, cost))),
        DimensionOrder::LowestFirst => {
            (0..unlocked).find_map(|tier| affordable(obs, tier).map(|cost| (tier, cost)))
        }
        DimensionOrder::CheapestFirst => {
            let mut best: Option<(usize, Decimal)> = None;
            for tier in 0..unlocked {
                if let Some(cost) = affordable(obs, tier) {
                    match best {
                        None => best = Some((tier, cost)),
                        Some((_, best_cost)) if cost < best_cost => {
                            best = Some((tier, cost))
                        }
                        Some(_) => {}
                    }
                }
            }
            best
        }
    }
}

/// Returns the unit cost of `tier` if it is affordable, else `None`.
fn affordable(obs: &ObservedState, tier: usize) -> Option<Decimal> {
    let cost = obs.dimensions[tier].cost;
    (obs.antimatter >= cost).then_some(cost)
}
