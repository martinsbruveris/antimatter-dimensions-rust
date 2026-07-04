//! Tab notification badges — the pulsing yellow `!` on tab/subtab buttons that
//! points the player at newly relevant content (Challenges after the first
//! Infinity, an affordable autobuyer, a freshly unlocked Infinity Challenge, …).
//!
//! Ported from the original `core/tab-notifications.js` +
//! `core/secret-formula/tab-notifications.js`. Two persisted fields drive it:
//! [`GameState::tab_notifications`] (the set of badged `parentKey + subtabKey`
//! strings, ↔ `player.tabNotifications`) and
//! [`GameState::triggered_tab_notification_bits`]
//! (↔ `player.triggeredTabNotificationBits`, which notifications ever fired so
//! each fires once). Triggers fire inline from the relevant engine sites
//! (`big_crunch_reset`, `break_infinity`, `upgrade_autobuyer_interval`, the
//! per-tick IC-unlock/autobuyer checks); the frontend renders the badges from
//! the snapshot and acknowledges a viewed tab via
//! [`tab_notification_seen`](GameState::tab_notification_seen).
//!
//! Two intentional differences from the original's `tryTrigger`: we never
//! exclude the currently open tab (the engine doesn't know it — the frontend
//! hides the badge on the open subtab instead, which is observationally
//! equivalent), and we skip the force-unhide step (we don't model hidden tabs).
//! See `design-docs/2026-07-04-tab-notifications.md`.

use break_infinity::Decimal;

use crate::autobuyers::AutobuyerTarget;
use crate::infinity_challenges::INFINITY_CHALLENGE_COUNT;
use crate::replicanti::REPLICANTI_UNLOCK_COST;
use crate::state::GameState;

/// The notification definitions we model, by original id (the bit in
/// `triggeredTabNotificationBits`). Ids 5–11 and 13–16 are Eternity-and-later;
/// id 2 (`IDUnlock`) is dead config in the original (no trigger site). Their
/// bits still round-trip through the save untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabNotificationId {
    /// The first-ever Big Crunch: badge the tabs Infinity opens up.
    FirstInfinity = 0,
    /// The Big Crunch autobuyer's interval reached the 100 ms floor, so Break
    /// Infinity is purchasable.
    BreakInfinity = 1,
    /// An Infinity Challenge became available (Break Infinity bought, or peak
    /// antimatter crossed an IC's unlock threshold). Re-armed per IC.
    IcUnlock = 3,
    /// Replicanti are affordable (1e140 IP) after a Big Crunch.
    Replicanti = 4,
    /// An antimatter-dimension/tickspeed autobuyer unlock is affordable.
    /// Re-armed every check, so it re-badges whenever the player leaves the
    /// Automation tab with an unlock still pending.
    NewAutobuyer = 12,
}

impl TabNotificationId {
    /// This notification's bit in `triggeredTabNotificationBits`.
    fn bit(self) -> u32 {
        1 << (self as u32)
    }

    /// The `tabsToHighLight` targets as the original's concatenated
    /// `parent + tab` keys (the save's `tabNotifications` wire format).
    fn target_keys(self) -> &'static [&'static str] {
        match self {
            Self::FirstInfinity => &[
                "infinityupgrades",
                "challengesnormal",
                "statisticsmultipliers",
            ],
            Self::BreakInfinity => &["infinitybreak"],
            Self::IcUnlock => &["challengesinfinity"],
            Self::Replicanti => &["infinityreplicanti"],
            Self::NewAutobuyer => &["automationautobuyers"],
        }
    }
}

impl GameState {
    /// The notification's `condition()`. The originals also require
    /// Eternity/Reality to not be unlocked (and `!Pelle.isDoomed` for
    /// `NewAutobuyer`) — all beyond our frontier, hence always true here.
    fn tab_notification_condition(&self, id: TabNotificationId) -> bool {
        match id {
            TabNotificationId::FirstInfinity => !self.infinity_unlocked,
            TabNotificationId::BreakInfinity => self.break_infinity_unlockable(),
            TabNotificationId::IcUnlock => true,
            TabNotificationId::Replicanti => {
                self.infinity_points >= REPLICANTI_UNLOCK_COST
            }
            TabNotificationId::NewAutobuyer => true,
        }
    }

    /// Fire `id` if its condition holds and it has not fired before: badge all
    /// its target tabs and record the trigger. Mirrors
    /// `TabNotificationState.tryTrigger` (minus the current-tab exclusion and
    /// tab unhiding; see the module docs).
    pub(crate) fn try_trigger_tab_notification(&mut self, id: TabNotificationId) {
        if !self.tab_notification_condition(id)
            || self.triggered_tab_notification_bits & id.bit() != 0
        {
            return;
        }
        for key in id.target_keys() {
            self.tab_notifications.insert((*key).to_string());
        }
        self.triggered_tab_notification_bits |= id.bit();
    }

    /// Re-arm `id`: clear its triggered bit and un-badge its targets, so a
    /// later `try_trigger` fires again. Mirrors `clearTrigger`.
    pub(crate) fn clear_tab_notification_trigger(&mut self, id: TabNotificationId) {
        self.triggered_tab_notification_bits &= !id.bit();
        for key in id.target_keys() {
            self.tab_notifications.remove(*key);
        }
    }

    /// Acknowledge that the player viewed the tab behind `key` (the
    /// concatenated `parentKey + subtabKey`), removing its badge. Mirrors the
    /// `player.tabNotifications.delete` in `TabState.show`; called by the
    /// frontend on navigation.
    pub fn tab_notification_seen(&mut self, key: &str) {
        self.tab_notifications.remove(key);
    }

    /// Per-tick `NewAutobuyer` check, standing in for the original's
    /// antimatter-setter hook: when the cheapest still-locked AD/tickspeed
    /// autobuyer is affordable, clear-then-trigger so the badge re-arms every
    /// time it is acknowledged while an unlock stays pending. The original
    /// compares current antimatter against `GameCache.cheapestAntimatterAutobuyer`;
    /// our unlock gate is all-time `total_antimatter` (see `autobuyers.rs`), so
    /// the badge appears exactly when the unlock becomes purchasable.
    pub(crate) fn notify_new_autobuyer(&mut self) {
        let Some(cheapest) = self.cheapest_locked_autobuyer_requirement() else {
            return;
        };
        if self.total_antimatter < cheapest {
            return;
        }
        // Already badged: the clear-then-trigger pair would be a net no-op, so
        // skip the per-tick set churn.
        if self.tab_notifications.contains("automationautobuyers") {
            return;
        }
        self.clear_tab_notification_trigger(TabNotificationId::NewAutobuyer);
        self.try_trigger_tab_notification(TabNotificationId::NewAutobuyer);
    }

    /// Per-tick `IcUnlock` check, standing in for the original's
    /// `InfinityChallenges.notifyICUnlock`: when this tick's peak antimatter
    /// (`max_am_this_eternity`, which derives IC unlocks) crossed the unlock
    /// threshold of a not-yet-completed IC, clear-then-trigger so each new IC
    /// re-badges. `prev_peak` is the record's value before this tick's update.
    pub(crate) fn notify_ic_unlock(&mut self, prev_peak: Decimal) {
        for id in 1..=INFINITY_CHALLENGE_COUNT {
            if self.infinity_challenge_completed(id) {
                continue;
            }
            let unlock_am = Self::infinity_challenge_unlock_am(id);
            if prev_peak < unlock_am && self.records.this_eternity.max_am >= unlock_am {
                self.clear_tab_notification_trigger(TabNotificationId::IcUnlock);
                self.try_trigger_tab_notification(TabNotificationId::IcUnlock);
            }
        }
    }

    /// The antimatter requirement of the cheapest AD/tickspeed autobuyer that is
    /// neither bought nor challenge-unlocked (the original's
    /// `GameCache.cheapestAntimatterAutobuyer`); `None` when all are unlocked.
    fn cheapest_locked_autobuyer_requirement(&self) -> Option<Decimal> {
        let mut cheapest: Option<Decimal> = None;
        for tier in 0..8 {
            if !self.autobuyer_is_unlocked(AutobuyerTarget::AdTier(tier)) {
                let req = Self::ad_autobuyer_requirement(tier);
                cheapest = Some(cheapest.map_or(req, |c| c.min(&req)));
            }
        }
        if !self.autobuyer_is_unlocked(AutobuyerTarget::Tickspeed) {
            let req = Self::tickspeed_autobuyer_requirement();
            cheapest = Some(cheapest.map_or(req, |c| c.min(&req)));
        }
        cheapest
    }
}

#[cfg(test)]
mod tests {
    use break_infinity::Decimal;

    use super::*;
    use crate::autobuyers::AutobuyerTarget;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    fn badged(game: &GameState, key: &str) -> bool {
        game.tab_notifications.contains(key)
    }

    #[test]
    fn first_crunch_badges_infinity_challenges_statistics_once() {
        let mut game = GameState::new();
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        assert!(badged(&game, "infinityupgrades"));
        assert!(badged(&game, "challengesnormal"));
        assert!(badged(&game, "statisticsmultipliers"));

        // Acknowledge and crunch again: the badges do not come back (the
        // triggered bit is set and the condition — first Infinity — is gone).
        game.tab_notification_seen("infinityupgrades");
        game.tab_notification_seen("challengesnormal");
        game.tab_notification_seen("statisticsmultipliers");
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(!badged(&game, "infinityupgrades"));
        assert!(!badged(&game, "challengesnormal"));
    }

    #[test]
    fn replicanti_badge_requires_1e140_ip_and_a_crunch() {
        let mut game = GameState::new();
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(!badged(&game, "infinityreplicanti"));

        game.infinity_points = Decimal::new(1.0, 140);
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(badged(&game, "infinityreplicanti"));
    }

    #[test]
    fn maxing_big_crunch_interval_badges_break_infinity() {
        let mut game = GameState::new();
        game.complete_challenge(12);
        game.infinity_points = Decimal::from_float(1e9);
        while !game.autobuyer_has_maxed_interval(AutobuyerTarget::BigCrunch) {
            assert!(game.upgrade_autobuyer_interval(AutobuyerTarget::BigCrunch));
        }
        assert!(badged(&game, "infinitybreak"));
    }

    #[test]
    fn non_final_interval_upgrade_does_not_badge() {
        let mut game = GameState::new();
        game.complete_challenge(12);
        game.infinity_points = Decimal::from_float(1e9);
        assert!(game.upgrade_autobuyer_interval(AutobuyerTarget::BigCrunch));
        assert!(!badged(&game, "infinitybreak"));
    }

    #[test]
    fn break_infinity_badges_ic_tab() {
        let mut game = GameState::new();
        game.complete_challenge(12);
        game.infinity_points = Decimal::from_float(1e9);
        while !game.autobuyer_has_maxed_interval(AutobuyerTarget::BigCrunch) {
            game.upgrade_autobuyer_interval(AutobuyerTarget::BigCrunch);
        }
        assert!(game.break_infinity());
        assert!(badged(&game, "challengesinfinity"));
    }

    #[test]
    fn ic_unlock_crossing_badges_and_rearms_after_completion() {
        let mut game = GameState::new();

        // Cross IC1's 1e2000 unlock threshold.
        let prev = game.records.this_eternity.max_am;
        game.records.this_eternity.max_am = Decimal::new(1.0, 2000);
        game.notify_ic_unlock(prev);
        assert!(badged(&game, "challengesinfinity"));

        // Seen, then no new crossing: stays clear (the bit is still set).
        game.tab_notification_seen("challengesinfinity");
        let prev = game.records.this_eternity.max_am;
        game.notify_ic_unlock(prev);
        assert!(!badged(&game, "challengesinfinity"));

        // Completing an IC clears the trigger, so the next crossing (IC2 at
        // 1e11000) re-badges.
        game.clear_tab_notification_trigger(TabNotificationId::IcUnlock);
        let prev = game.records.this_eternity.max_am;
        game.records.this_eternity.max_am = Decimal::new(1.0, 11_000);
        game.notify_ic_unlock(prev);
        assert!(badged(&game, "challengesinfinity"));
    }

    #[test]
    fn affordable_autobuyer_badges_and_rearms_after_seen() {
        let mut game = GameState::new();
        assert!(!badged(&game, "automationautobuyers"));

        // The 1st AD autobuyer's requirement (1e40 total antimatter) is met.
        game.total_antimatter = Decimal::new(1.0, 40);
        game.notify_new_autobuyer();
        assert!(badged(&game, "automationautobuyers"));

        // Acknowledged, but the unlock is still pending: it re-badges (the
        // original's clear-then-try on every antimatter change).
        game.tab_notification_seen("automationautobuyers");
        game.notify_new_autobuyer();
        assert!(badged(&game, "automationautobuyers"));

        // Unlocking clears it, and with nothing affordable it stays clear.
        assert!(game.unlock_ad_autobuyer(0));
        assert!(!badged(&game, "automationautobuyers"));
        game.notify_new_autobuyer();
        assert!(!badged(&game, "automationautobuyers"));
    }

    #[test]
    fn tick_drives_the_autobuyer_badge() {
        let mut game = GameState::new();
        game.total_antimatter = Decimal::new(1.0, 40);
        game.tick(100.0);
        assert!(badged(&game, "automationautobuyers"));
    }
}
