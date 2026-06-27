import AntimatterDimensionsTab from "../components/tabs/AntimatterDimensionsTab.vue";
import NormalAchievementsTab from "../components/tabs/NormalAchievementsTab.vue";
import AutobuyersTab from "../components/tabs/AutobuyersTab.vue";

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
    key: "achievements",
    name: "Achievements",
    subtabs: [
      { key: "normal", name: "Achievements", symbol: "<i class='fas fa-trophy'></i>", component: NormalAchievementsTab },
      { key: "secret", name: "Secret Achievements", symbol: "<i class='fas fa-question'></i>", component: null },
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
      { key: "saving", name: "Saving", symbol: "<i class='fas fa-save'></i>", component: null },
      { key: "visual", name: "Visual", symbol: "<i class='fas fa-palette'></i>", component: null },
      { key: "gameplay", name: "Gameplay", symbol: "<i class='fas fa-wrench'></i>", component: null },
    ],
  },
  {
    key: "shop",
    name: "Shop",
    subtabs: [
      { key: "shop", name: "Shop", symbol: "$", component: null },
    ],
  },
];
