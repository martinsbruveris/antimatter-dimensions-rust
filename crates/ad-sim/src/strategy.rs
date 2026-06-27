/// Complete configuration for a buying strategy.
#[derive(Debug, Clone)]
pub struct StrategyConfig {
    pub sacrifice: SacrificeConfig,
    pub purchase: PurchaseConfig,
    pub prestige: PrestigeMode,
}

/// When to sacrifice dimensions.
#[derive(Debug, Clone)]
pub struct SacrificeConfig {
    pub enabled: bool,
    /// Sacrifice when new_multiplier / old_multiplier > threshold.
    /// E.g., threshold=2.0 means only sacrifice for a 2x gain.
    pub min_gain_ratio: f64,
}

/// How to spend antimatter between prestige events.
#[derive(Debug, Clone)]
pub struct PurchaseConfig {
    /// How to decide between tickspeed and dimensions.
    pub priority: BuyPriority,
    /// Which dimension to buy when buying dimensions.
    pub dimension_order: DimensionOrder,
}

/// Tickspeed vs dimension priority.
#[derive(Debug, Clone, Copy)]
pub enum BuyPriority {
    /// Compare effective costs:
    ///   effective_tickspeed_cost = tickspeed_cost / tickspeed_weight
    ///   effective_dim_cost = dimension_cost_until_10
    /// Buy whichever is cheaper. Loop until nothing affordable.
    ///
    /// weight > 1 → prefer tickspeed (it appears cheaper)
    /// weight < 1 → prefer dimensions
    /// weight = 1 → pure cost comparison
    Weighted { tickspeed_weight: f64 },
}

/// Order in which to evaluate dimension purchases.
#[derive(Debug, Clone, Copy)]
pub enum DimensionOrder {
    /// Buy highest unlocked tier first.
    HighestFirst,
    /// Buy lowest tier first.
    LowestFirst,
    /// Buy whichever costs least.
    CheapestFirst,
}

/// How to handle prestige events (dim boosts and galaxies).
#[derive(Debug, Clone)]
pub enum PrestigeMode {
    /// Buy dim boosts and galaxies whenever affordable.
    /// Galaxy is prioritised over dim boost (since it gives
    /// a permanent tickspeed improvement that helps more than
    /// an additional boost multiplier).
    Auto,
    /// Follow a prescribed sequence of prestige events.
    /// After all steps are exhausted, continue buying
    /// dims/tickspeed until Big Crunch.
    Plan(Vec<PrestigeStep>),
}

/// A step in the prestige plan.
#[derive(Debug, Clone, Copy)]
pub enum PrestigeStep {
    /// Buy N dimension boosts (accumulating between each).
    DimBoost(u32),
    /// Buy one antimatter galaxy.
    Galaxy,
}

impl StrategyConfig {
    /// Baseline strategy: auto-buy all prestige, sacrifice at
    /// 10x gain, equal tickspeed/dimension weighting, highest
    /// dimension first.
    pub fn baseline() -> Self {
        Self {
            sacrifice: SacrificeConfig {
                enabled: true,
                min_gain_ratio: 10.0,
            },
            purchase: PurchaseConfig {
                priority: BuyPriority::Weighted {
                    tickspeed_weight: 1.0,
                },
                dimension_order: DimensionOrder::HighestFirst,
            },
            prestige: PrestigeMode::Auto,
        }
    }
}
