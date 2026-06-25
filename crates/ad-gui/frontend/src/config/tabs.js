import AntimatterDimensionsTab from "../components/tabs/AntimatterDimensionsTab.vue";
import NormalAchievementsTab from "../components/tabs/NormalAchievementsTab.vue";

// Single source of truth for the tab/subtab structure: display name,
// sidebar symbol, and the page component each subtab renders. `component:
// null` means "not implemented yet" (renders a placeholder). Only the
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
