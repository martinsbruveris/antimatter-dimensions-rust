//! Simulation driver for the Antimatter Dimensions engine.
//!
//! `ad-sim` decides *what a player does*; `ad-core` owns *the
//! rules*. The dependency is one-way (`ad-sim -> ad-core`), so the
//! compiler guarantees a simulation bug can never change game
//! logic. Everything that mutates the game does so by emitting an
//! [`ad_core::Action`] through [`ad_core::GameState::apply_action`].
//!
//! - [`Controller`] — the abstraction for "who produces actions".
//! - [`StrategyController`] — a fixed-strategy player (the
//!   re-expression of the original in-engine simulator).
//! - [`run_simulation`] — the generic driver loop.
//! - [`simulate`] — strategy-driven convenience entry point.

pub mod controller;
pub mod simulator;
pub mod strategy;

pub use controller::{Controller, StrategyController};
pub use simulator::{
    run_simulation, simulate, SimulationConfig, SimulationResult, Snapshot, StateTrace,
    StopCondition, StopReason,
};
pub use strategy::{
    BuyPriority, DimensionOrder, PrestigeMode, PrestigeStep, PurchaseConfig,
    SacrificeConfig, StrategyConfig,
};
