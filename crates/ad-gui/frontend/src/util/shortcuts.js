// Keyboard shortcut handling. Mirrors the bindings the original game exposes
// (../../../antimatter-dimensions/src/core/hotkeys.js), limited to the
// mechanics implemented so far. Called from App.vue's window keydown
// listener with the live game/ui Pinia stores.
//
// Letter and digit keys are matched via `e.code` ("KeyT", "Digit1", …) so a
// binding works regardless of Shift turning the character upper-case or, for
// digits, into a symbol ("!"); Numpad digits map the same way. "?" is matched
// by character since its physical key is layout-dependent.

// Buy a dimension: 1-8 / Numpad 1-8 buy "until 10", Shift+digit buys a single.
const DIM_KEY = /^(?:Digit|Numpad)([1-8])$/;

export function handleShortcut(e, game, ui) {
  // Never hijack typing in a text field (import/export inputs land later).
  const tag = e.target?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return;

  // Ctrl/Cmd+S saves the game (original "Save game" bind = mod+s). Handle it
  // before the general Ctrl/Cmd guard below, and stop the browser's own Save.
  if (e.code === "KeyS" && (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey) {
    game.saveGame().then(() => ui.notify("Game saved"));
    e.preventDefault();
    return;
  }

  // Ctrl/Cmd+E exports the save to the clipboard (original "Export save"
  // bind = mod+e).
  if (e.code === "KeyE" && (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey) {
    game
      .exportSave()
      .then((saveStr) => navigator.clipboard.writeText(saveStr))
      .then(() => ui.notify("Save exported to clipboard"));
    e.preventDefault();
    return;
  }

  // Ignore other Ctrl/Cmd/Alt combos: in the original these map to binds we
  // don't implement yet (autobuyer toggles), and swallowing them would
  // break browser/OS shortcuts.
  if (e.ctrlKey || e.metaKey || e.altKey) return;

  // Popups.
  if (e.key === "?") {
    ui.toggleModal("hotkeys");
    return;
  }
  if (e.code === "KeyH") {
    ui.toggleModal("help");
    return;
  }
  if (e.key === "Escape") {
    if (ui.openModal) ui.closeModal();
    return;
  }

  // Arrow keys cycle pages: Up/Down through tabs, Left/Right through the
  // current tab's subtabs. Like the original these are bound with `bind` (not
  // `bindHotkey`), so they stay active even when hotkeys are disabled.
  switch (e.code) {
    case "ArrowUp":
      ui.moveTab(-1);
      e.preventDefault();
      return;
    case "ArrowDown":
      ui.moveTab(1);
      e.preventDefault();
      return;
    case "ArrowLeft":
      ui.moveSubtab(-1);
      e.preventDefault();
      return;
    case "ArrowRight":
      ui.moveSubtab(1);
      e.preventDefault();
      return;
    default:
      break;
  }

  // Everything below relates to game functionality and obeys the "Hotkeys"
  // option, mirroring the original's `bindHotkey` (gated by
  // `player.options.hotkeys`) versus `bind` (always active) split.
  if (game.snapshot && game.snapshot.options && !game.snapshot.options.hotkeys) {
    return;
  }

  // Buy dimensions. Tiers are 0-indexed, so digit N -> tier N-1.
  const dim = e.code.match(DIM_KEY);
  if (dim) {
    const tier = Number(dim[1]) - 1;
    if (e.shiftKey) game.buyDimSingle(tier);
    else game.buyDimMany(tier);
    return;
  }

  // Tickspeed: T buys max, Shift+T buys one.
  if (e.code === "KeyT") {
    if (e.shiftKey) game.buyTickspeed();
    else game.buyMaxTickspeed();
    return;
  }

  switch (e.code) {
    case "KeyA":
      // Toggle (pause/resume) all autobuyers — only once the Automation tab is
      // unlocked, matching the original's `Tab.automation.isUnlocked` guard.
      // The original also shows a blue "info" toast; the toggle just flips the
      // global flag, so the new state is the inverse of the current snapshot.
      if (game.snapshot?.autobuyers?.tab_unlocked) {
        const resumed = !game.snapshot.autobuyers.enabled;
        game.toggleAutobuyers();
        ui.notify(`Autobuyers ${resumed ? "resumed" : "paused"}`);
      }
      break;
    case "KeyM":
      game.maxAll();
      break;
    case "KeyS":
      // Route through the confirm gate, like the original's hotkeys
      // (sacrificeBtnClick / manualRequest*): shows the modal if enabled.
      game.requestSacrifice();
      break;
    case "KeyD":
      game.requestDimBoost();
      break;
    case "KeyG":
      game.requestGalaxy();
      break;
    case "KeyC":
      // Big Crunch; the engine no-ops unless antimatter is at the threshold.
      game.requestBigCrunch();
      break;
    default:
      break;
  }
}
