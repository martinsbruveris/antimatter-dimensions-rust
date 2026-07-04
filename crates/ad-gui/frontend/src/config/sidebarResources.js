// The Modern-UI sidebar resources we can render — the in-frontier subset of
// the original's GameDatabase.sidebarResources (secret-formula/
// sidebar-resources.js), keeping the **original ids** so the engine-stored
// `sidebarResourceID` option round-trips through real saves (id 1 is the
// secret-theme Blob; ids 5+ are post-Eternity). Id 0 is not listed: it means
// "the latest (highest-id) unlocked resource".
//
// Each entry: display name, unlock gate against the live snapshot, the
// snapshot field it shows, its formatting, and the colour class (vendored /
// replicated in SidebarCurrency.vue's scoped block).
import { formatDecimal } from "../util/format";

export const SIDEBAR_RESOURCES = [
  {
    id: 2,
    optionName: "Antimatter",
    isAvailable: () => true,
    value: (s) => s.antimatter,
    formatValue: (x) => formatDecimal(x, 2, 1),
    formatClass: "o-sidebar-currency--antimatter",
  },
  {
    id: 3,
    optionName: "Infinity Points",
    isAvailable: (s) => Boolean(s?.infinity_unlocked),
    value: (s) => s.infinity_points,
    formatValue: (x) => formatDecimal(x, 2, 0),
    formatClass: "o-sidebar-currency--infinity",
  },
  {
    id: 4,
    optionName: "Replicanti",
    isAvailable: (s) => Boolean(s?.replicanti?.unlocked),
    value: (s) => s.replicanti.amount,
    formatValue: (x) => formatDecimal(x, 2, 0),
    formatClass: "o-sidebar-currency--replicanti",
  },
];
