import AntimatterDimensionsTab from "../components/tabs/AntimatterDimensionsTab.vue";
import InfinityUpgradesTab from "../components/tabs/InfinityUpgradesTab.vue";
import BreakInfinityTab from "../components/tabs/BreakInfinityTab.vue";
import InfinityDimensionsTab from "../components/tabs/InfinityDimensionsTab.vue";
import ChallengesTab from "../components/tabs/ChallengesTab.vue";
import InfinityChallengesTab from "../components/tabs/InfinityChallengesTab.vue";
import NormalAchievementsTab from "../components/tabs/NormalAchievementsTab.vue";
import AutobuyersTab from "../components/tabs/AutobuyersTab.vue";
import OptionsSavingTab from "../components/tabs/OptionsSavingTab.vue";
import OptionsVisualTab from "../components/tabs/OptionsVisualTab.vue";
import OptionsGameplayTab from "../components/tabs/OptionsGameplayTab.vue";

// Single source of truth for the tab/subtab structure: display name,
// sidebar symbol, and the page component each subtab renders. `component:
// null` means "not implemented yet" (renders a placeholder). An optional
// `condition(snapshot)` hides a tab until the game unlocks it. Only the
// early-game tabs are listed for now; more get added as systems land.
export const TABS = [
  {
    key: "dimensions",
    name: "Dimensions",
    subtabs: [
      { key: "antimatter", name: "Antimatter Dimensions", symbol: "Ω", component: AntimatterDimensionsTab },
    ],
  },
  {
    key: "infinity",
    name: "Infinity",
    // JS: `condition: () => PlayerProgress.infinityUnlocked()` — appears after the
    // first Big Crunch and stays. `uiClass` gives the tab its infinity coloring
    // (original `UIClass: "o-tab-btn--infinity"`). Only the Infinity Upgrades
    // subtab is built; Break Infinity and Replicanti come later.
    condition: (s) => Boolean(s?.infinity_unlocked),
    uiClass: "o-tab-btn--infinity",
    subtabs: [
      { key: "upgrades", name: "Infinity Upgrades", symbol: "<i class='fas fa-arrow-up'></i>", component: InfinityUpgradesTab },
      // Infinity Dimensions: appear once Infinity is broken (their unlock AM
      // exceeds 1e308) or the 1st is already unlocked.
      {
        key: "dimensions",
        name: "Infinity Dimensions",
        symbol: "<i class='fas fa-times'></i>",
        component: InfinityDimensionsTab,
        condition: (s) => Boolean(s?.infinity_dimensions?.unlocked),
      },
      // Break Infinity: appears once Infinity is broken (`player.break`).
      {
        key: "break",
        name: "Break Infinity",
        symbol: "<i class='fas fa-arrows-alt-h'></i>",
        component: BreakInfinityTab,
        condition: (s) => Boolean(s?.break_infinity?.unlocked),
      },
    ],
  },
  {
    key: "automation",
    name: "Automation",
    // JS: tab unlocks at total antimatter >= 1e40 (around buying the 7th
    // Antimatter Dimension). The Automator subtab is post-Reality, so it is
    // omitted here.
    condition: (s) => Boolean(s?.autobuyers?.tab_unlocked),
    subtabs: [
      { key: "autobuyers", name: "Autobuyers", symbol: "<i class='fas fa-cog'></i>", component: AutobuyersTab },
    ],
  },
  {
    key: "challenges",
    name: "Challenges",
    // JS: `condition: () => PlayerProgress.infinityUnlocked()`. Only the Normal
    // Challenges subtab is built; Infinity/Eternity challenges come later.
    condition: (s) => Boolean(s?.challenges_unlocked),
    subtabs: [
      { key: "normal", name: "Challenges", symbol: "<i class='fas fa-fist-raised'></i>", component: ChallengesTab },
      // Infinity Challenges: appear once any is unlocked (needs Break Infinity).
      {
        key: "infinity",
        name: "Infinity Challenges",
        symbol: "<i class='fas fa-infinity'></i>",
        component: InfinityChallengesTab,
        condition: (s) => Boolean(s?.infinity_challenges_unlocked),
      },
    ],
  },
  {
    key: "achievements",
    name: "Achievements",
    subtabs: [
      { key: "normal", name: "Achievements", symbol: "<i class='fas fa-trophy'></i>", component: NormalAchievementsTab },
    ],
  },
  {
    key: "statistics",
    name: "Statistics",
    subtabs: [
      { key: "statistics", name: "Statistics", symbol: "<i class='fas fa-clipboard-list'></i>", component: null },
    ],
  },
  {
    key: "options",
    name: "Options",
    subtabs: [
      { key: "saving", name: "Saving", symbol: "<i class='fas fa-save'></i>", component: OptionsSavingTab },
      { key: "visual", name: "Visual", symbol: "<i class='fas fa-palette'></i>", component: OptionsVisualTab },
      { key: "gameplay", name: "Gameplay", symbol: "<i class='fas fa-wrench'></i>", component: OptionsGameplayTab },
    ],
  },
];
