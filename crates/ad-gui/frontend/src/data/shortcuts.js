// Keyboard shortcut list shown in the Hotkey List modal.
// Mirrors the original game's shortcut definitions
// (../antimatter-dimensions/src/core/hotkeys.js): these are every shortcut
// the original shows unconditionally (its `visible: true` entries), in the
// same order. Entries gated behind later mechanics (Eternity, Reality, the
// Automator, …) are hidden in the original until unlocked, and hidden
// easter-egg binds are never listed; both are omitted here too. The bindings
// themselves live in util/shortcuts.js (and ConfirmModal.vue for Enter); all
// are wired up except "Modify visible tabs" (TAB), which is not implemented
// yet. Keep names/keys in sync with the original.
//
// `keys` is an ordered list of key tokens; `format()` in the modal turns
// each into its displayed label (e.g. "mod" -> "CTRL/⌘", "t" -> "T").
export const shortcuts = [
  { name: "Toggle Autobuyers", keys: ["a"] },
  { name: "Buy one Tickspeed", keys: ["shift", "t"] },
  { name: "Buy max Tickspeed", keys: ["t"] },
  { name: "Max all", keys: ["m"] },
  { name: "Dimensional Sacrifice", keys: ["s"] },
  { name: "Dimension Boost", keys: ["d"] },
  { name: "Antimatter Galaxy", keys: ["g"] },
  { name: "Big Crunch", keys: ["c"] },
  { name: "Save game", keys: ["mod", "s"] },
  { name: "Export game", keys: ["mod", "e"] },
  { name: "Open Hotkey List Modal", keys: ["?"] },
  { name: "Open How To Play Modal", keys: ["h"] },
  { name: "Modify visible tabs", keys: ["tab"] },
  { name: "Confirm Modal", keys: ["enter"] },
  { name: "Close Modal or open Options", keys: ["esc"] },
];
