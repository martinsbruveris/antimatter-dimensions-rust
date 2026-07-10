import { defineAsyncComponent } from "vue";
import AntimatterDimensionsTab from "../components/tabs/AntimatterDimensionsTab.vue";
import InfinityUpgradesTab from "../components/tabs/InfinityUpgradesTab.vue";
import BreakInfinityTab from "../components/tabs/BreakInfinityTab.vue";
import InfinityDimensionsTab from "../components/tabs/InfinityDimensionsTab.vue";
import ReplicantiTab from "../components/tabs/ReplicantiTab.vue";
import ChallengesTab from "../components/tabs/ChallengesTab.vue";
import InfinityChallengesTab from "../components/tabs/InfinityChallengesTab.vue";
import EternityChallengesTab from "../components/tabs/EternityChallengesTab.vue";
import EternityUpgradesTab from "../components/tabs/EternityUpgradesTab.vue";
import TimeDilationTab from "../components/tabs/TimeDilationTab.vue";
import EternityMilestonesTab from "../components/tabs/EternityMilestonesTab.vue";
import TimeDimensionsTab from "../components/tabs/TimeDimensionsTab.vue";
import TimeStudiesTab from "../components/tabs/TimeStudiesTab.vue";
import NormalAchievementsTab from "../components/tabs/NormalAchievementsTab.vue";
import StatisticsTab from "../components/tabs/StatisticsTab.vue";
import ChallengeRecordsTab from "../components/tabs/ChallengeRecordsTab.vue";
import PastPrestigeRunsTab from "../components/tabs/PastPrestigeRunsTab.vue";
// Lazy-loaded: the Automator subtab pulls in CodeMirror + vuedraggable, which
// are only reachable post-Reality. Splitting it into its own chunk keeps those
// deps out of the initial bundle. Rendered via <component :is> in App.vue, so
// no other wiring is needed.
const AutomatorTab = defineAsyncComponent(() =>
  import("../components/tabs/AutomatorTab.vue"));
import GlyphsTab from "../components/tabs/reality/GlyphsTab.vue";
import PerksTab from "../components/tabs/reality/PerksTab.vue";
import RealityUpgradesTab from "../components/tabs/reality/RealityUpgradesTab.vue";
import BlackHoleTab from "../components/tabs/reality/BlackHoleTab.vue";
import TeresaTab from "../components/tabs/celestials/TeresaTab.vue";
import EffarigTab from "../components/tabs/celestials/EffarigTab.vue";
import EnslavedTab from "../components/tabs/celestials/EnslavedTab.vue";
import VTab from "../components/tabs/celestials/VTab.vue";
import RaTab from "../components/tabs/celestials/RaTab.vue";
import LaitelaTab from "../components/tabs/celestials/LaitelaTab.vue";
import PelleTab from "../components/tabs/celestials/PelleTab.vue";
import AutobuyersTab from "../components/tabs/AutobuyersTab.vue";
import OptionsSavingTab from "../components/tabs/OptionsSavingTab.vue";
import OptionsVisualTab from "../components/tabs/OptionsVisualTab.vue";
import OptionsGameplayTab from "../components/tabs/OptionsGameplayTab.vue";

// Single source of truth for the tab/subtab structure: display name,
// sidebar symbol, and the page component each subtab renders. `component:
// null` means "not implemented yet" (renders a placeholder). An optional
// `condition(snapshot)` hides a tab until the game unlocks it. Only the
// early-game tabs are listed for now; more get added as systems land.
//
// Hidden-tab bookkeeping (the Modify Visible Tabs modal): each tab carries the
// **original game's tab id** as `hideId` and each subtab a `[tabId, subtabId]`
// pair — the bit positions in the save's `hiddenTabBits`/`hiddenSubtabBits`, so
// hidden-tab state round-trips through real saves. Note the Infinity →
// Infinity Dimensions subtab maps to the original's Dimensions-tab "Infinity
// Dimensions" subtab (0,1) — we host that page under the Infinity tab instead.
// `hidable: false` marks the Options tab/subtabs, which can never be hidden.
export const TABS = [
  {
    key: "dimensions",
    name: "Dimensions",
    hideId: 0,
    subtabs: [
      { key: "antimatter", name: "Antimatter Dimensions", symbol: "Ω", component: AntimatterDimensionsTab, hideId: [0, 0] },
    ],
  },
  {
    key: "automation",
    name: "Automation",
    hideId: 4,
    // JS: tab unlocks at total antimatter >= 1e40 (around buying the 7th
    // Antimatter Dimension). The Automator subtab is post-Reality, so it is
    // omitted here.
    condition: (s) => Boolean(s?.autobuyers?.tab_unlocked),
    subtabs: [
      { key: "autobuyers", name: "Autobuyers", symbol: "<i class='fas fa-cog'></i>", component: AutobuyersTab, hideId: [4, 0] },
    ],
  },
  {
    key: "challenges",
    name: "Challenges",
    hideId: 5,
    // JS: `condition: () => PlayerProgress.infinityUnlocked()`. Only the Normal
    // Challenges subtab is built; Infinity/Eternity challenges come later.
    condition: (s) => Boolean(s?.challenges_unlocked),
    subtabs: [
      { key: "normal", name: "Challenges", symbol: "Ω", component: ChallengesTab, hideId: [5, 0] },
      // Infinity Challenges: appear once any is unlocked (needs Break Infinity).
      {
        key: "infinity",
        name: "Infinity Challenges",
        symbol: "<i class='fas fa-infinity'></i>",
        component: InfinityChallengesTab,
        condition: (s) => Boolean(s?.infinity_challenges_unlocked),
        hideId: [5, 1],
      },
      // Eternity Challenges: appear once an EC study is held or any completed.
      {
        key: "eternity",
        name: "Eternity Challenges",
        symbol: "Δ",
        component: EternityChallengesTab,
        condition: (s) => Boolean(s?.eternity_challenges_unlocked),
        hideId: [5, 2],
      },
    ],
  },
  {
    key: "infinity",
    name: "Infinity",
    hideId: 6,
    // JS: `condition: () => PlayerProgress.infinityUnlocked()` — appears after the
    // first Big Crunch and stays. `uiClass` gives the tab its infinity coloring
    // (original `UIClass: "o-tab-btn--infinity"`).
    condition: (s) => Boolean(s?.infinity_unlocked),
    uiClass: "o-tab-btn--infinity",
    subtabs: [
      { key: "upgrades", name: "Infinity Upgrades", symbol: "<i class='fas fa-arrow-up'></i>", component: InfinityUpgradesTab, hideId: [6, 0] },
      // Infinity Dimensions: appear once Infinity is broken (their unlock AM
      // exceeds 1e308) or the 1st is already unlocked. Hide-bit-wise this is
      // the original's Dimensions-tab "Infinity Dimensions" subtab.
      {
        key: "dimensions",
        name: "Infinity Dimensions",
        symbol: "<i class='fas fa-times'></i>",
        component: InfinityDimensionsTab,
        condition: (s) => Boolean(s?.infinity_dimensions?.unlocked),
        hideId: [0, 1],
      },
      // Break Infinity: appears from the first Big Crunch (JS condition is
      // `infinityUnlocked()`), showing the "reduce the Big Crunch interval to
      // 0.1s" hint and a disabled BREAK INFINITY button until it can be broken.
      {
        key: "break",
        name: "Break Infinity",
        symbol: "∝",
        component: BreakInfinityTab,
        condition: (s) => Boolean(s?.infinity_unlocked),
        hideId: [6, 1],
      },
      // Replicanti: visible from the first Infinity (JS `infinityUnlocked()`); the
      // in-tab button unlocks the mechanic once 1e140 IP is affordable.
      {
        key: "replicanti",
        name: "Replicanti",
        symbol: "Ξ",
        component: ReplicantiTab,
        hideId: [6, 2],
      },
    ],
  },
  {
    key: "eternity",
    name: "Eternity",
    hideId: 7,
    // JS: `condition: () => PlayerProgress.eternityUnlocked()` (or Reality,
    // out of frontier). `uiClass` gives the tab its eternity coloring.
    condition: (s) => Boolean(s?.eternity_unlocked),
    uiClass: "o-tab-btn--eternity",
    subtabs: [
      { key: "studies", name: "Time Studies", symbol: "<i class='fas fa-book'></i>", component: TimeStudiesTab, hideId: [7, 0] },
      // Time Dimensions: hide-bit-wise the original's Dimensions-tab "Time
      // Dimensions" subtab (0,2) — we host the page under the Eternity tab,
      // like Infinity Dimensions under Infinity.
      { key: "timedims", name: "Time Dimensions", symbol: "Δ", component: TimeDimensionsTab, hideId: [0, 2] },
      { key: "upgrades", name: "Eternity Upgrades", symbol: "<i class='fas fa-arrow-up'></i>", component: EternityUpgradesTab, hideId: [7, 1] },
      { key: "milestones", name: "Eternity Milestones", symbol: "<i class='fas fa-star'></i>", component: EternityMilestonesTab, hideId: [7, 2] },
      // Time Dilation: appears once unlocked (dilation study 1).
      {
        key: "dilation",
        name: "Time Dilation",
        symbol: "Ψ",
        component: TimeDilationTab,
        condition: (s) => Boolean(s?.dilation?.unlocked),
        hideId: [7, 3],
      },
    ],
  },
  {
    key: "reality",
    name: "Reality",
    hideId: 8,
    // JS: `condition: () => PlayerProgress.realityUnlocked() ||
    // TimeStudy.reality.isBought`.
    condition: (s) =>
      Boolean(s?.reality?.unlocked || s?.reality?.has_reality_study),
    uiClass: "o-tab-btn--reality",
    subtabs: [
      { key: "glyphs", name: "Glyphs", symbol: "<i class='fas fa-clone'></i>", component: GlyphsTab, hideId: [8, 0] },
      { key: "upgrades", name: "Reality Upgrades", symbol: "<i class='fas fa-arrow-up'></i>", component: RealityUpgradesTab, hideId: [8, 1] },
      { key: "automator", name: "Automator", symbol: "<i class='fas fa-cog'></i>", component: AutomatorTab, hideId: [8, 2] },
      { key: "perks", name: "Perks", symbol: "<i class='fas fa-project-diagram'></i>", component: PerksTab, hideId: [8, 3] },
      {
        key: "hole",
        name: "Black Hole",
        symbol: "<i class='fas fa-circle'></i>",
        component: BlackHoleTab,
        condition: (s) => Boolean(s?.reality?.unlocked),
        hideId: [8, 4],
      },
    ],
  },
  {
    // Celestials (Phase 7). JS `condition: () => Teresa.isUnlocked`; we gate on
    // reality being unlocked (design doc §5). The original's celestial-navigation
    // SVG hub subtab is cut in favour of plain per-celestial subtabs. Effarig /
    // Enslaved / V land with their own features.
    key: "celestials",
    name: "Celestials",
    hideId: 9,
    condition: (s) => Boolean(s?.celestials?.unlocked),
    uiClass: "o-tab-btn--celestial",
    subtabs: [
      { key: "teresa", name: "Teresa", symbol: "Ϟ", component: TeresaTab, hideId: [9, 1] },
      {
        key: "effarig",
        name: "Effarig",
        symbol: "Ϙ",
        component: EffarigTab,
        condition: (s) => Boolean(s?.celestials?.effarig?.unlocked),
        hideId: [9, 2],
      },
      {
        key: "enslaved",
        name: "The Nameless Ones",
        symbol: "<i class='fas fa-link'></i>",
        component: EnslavedTab,
        condition: (s) => Boolean(s?.celestials?.enslaved?.unlocked),
        hideId: [9, 3],
      },
      {
        key: "v",
        name: "V",
        symbol: "⌬",
        component: VTab,
        condition: (s) => Boolean(s?.celestials?.v?.unlocked),
        hideId: [9, 4],
      },
      {
        key: "ra",
        name: "Ra",
        symbol: "<i class='fas fa-sun'></i>",
        component: RaTab,
        condition: (s) => Boolean(s?.celestials?.ra?.unlocked),
        hideId: [9, 5],
      },
      {
        key: "laitela",
        name: "Lai'tela",
        symbol: "ᛝ",
        component: LaitelaTab,
        condition: (s) => Boolean(s?.celestials?.laitela?.unlocked),
        hideId: [9, 6],
      },
      {
        key: "pelle",
        name: "Pelle",
        symbol: "♅",
        component: PelleTab,
        condition: (s) => Boolean(s?.celestials?.pelle?.unlocked),
        hideId: [9, 7],
      },
    ],
  },
  {
    key: "achievements",
    name: "Achievements",
    hideId: 3,
    subtabs: [
      { key: "normal", name: "Achievements", symbol: "<i class='fas fa-trophy'></i>", component: NormalAchievementsTab, hideId: [3, 0] },
    ],
  },
  {
    key: "statistics",
    name: "Statistics",
    hideId: 2,
    subtabs: [
      { key: "statistics", name: "Statistics", symbol: "<i class='fas fa-clipboard-list'></i>", component: StatisticsTab, hideId: [2, 0] },
      // Challenge records: appear once a challenge has been completed (or a
      // later prestige layer reached) — original `PlayerProgress
      // .challengeCompleted() || eternityUnlocked() || realityUnlocked()`.
      {
        key: "challenges",
        name: "Challenge records",
        symbol: "<i class='fas fa-stopwatch'></i>",
        component: ChallengeRecordsTab,
        condition: (s) =>
          Boolean(s?.eternity_unlocked) ||
          Boolean(s?.reality?.unlocked) ||
          (s?.challenges ?? []).some((c) => c.is_completed),
        hideId: [2, 1],
      },
      // Past Prestige Runs: appear after the first Infinity.
      {
        key: "prestige runs",
        name: "Past Prestige Runs",
        symbol: "<i class='fas fa-list-ol'></i>",
        component: PastPrestigeRunsTab,
        condition: (s) => Boolean(s?.infinity_unlocked),
        hideId: [2, 2],
      },
    ],
  },
  {
    key: "options",
    name: "Options",
    hideId: 1,
    hidable: false,
    subtabs: [
      { key: "saving", name: "Saving", symbol: "<i class='fas fa-save'></i>", component: OptionsSavingTab, hideId: [1, 0], hidable: false },
      { key: "visual", name: "Visual", symbol: "<i class='fas fa-palette'></i>", component: OptionsVisualTab, hideId: [1, 1], hidable: false },
      { key: "gameplay", name: "Gameplay", symbol: "<i class='fas fa-wrench'></i>", component: OptionsGameplayTab, hideId: [1, 2], hidable: false },
    ],
  },
];
