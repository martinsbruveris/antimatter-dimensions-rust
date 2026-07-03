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

  // Ignore other Ctrl/Cmd combos: in the original these map to binds we don't
  // implement yet, and swallowing them would break browser/OS shortcuts.
  if (e.ctrlKey || e.metaKey) return;

  if (!e.altKey) {
    // Popups. Enter ("Confirm Modal") is not handled here: ConfirmModal.vue
    // binds it itself while a confirmation modal is open.
    if (e.key === "?") {
      ui.toggleModal("hotkeys");
      return;
    }
    if (e.code === "KeyH") {
      ui.toggleModal("help");
      return;
    }
    if (e.key === "Escape") {
      // Close the open modal, or open the Options tab when none is (the
      // original's keyboardPressEscape).
      if (ui.openModal) ui.closeModal();
      else ui.setTab("options");
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
  }

  // Everything below relates to game functionality and obeys the "Hotkeys"
  // option, mirroring the original's `bindHotkey` (gated by
  // `player.options.hotkeys`) versus `bind` (always active) split.
  if (game.snapshot && game.snapshot.options && !game.snapshot.options.hotkeys) {
    return;
  }

  // Alt is the autobuyer modifier: Alt+key toggles the key's corresponding
  // autobuyer, Shift+Alt+key its singles/max mode.
  if (e.altKey) {
    handleAutobuyerShortcut(e, game, ui);
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

// Alt+key toggles the matching autobuyer on/off; Shift+Alt+key toggles its
// buys-singles/buys-max mode instead. Mirrors the original's toggleAutobuyer /
// toggleBuySingles (hotkeys.js), including their info toasts. The original's
// Alt+S (Sacrifice) autobuyer doesn't exist in this reimplementation yet.
function handleAutobuyerShortcut(e, game, ui) {
  const autobuyers = game.snapshot?.autobuyers;
  if (!autobuyers) return;

  const dim = e.code.match(DIM_KEY);
  if (dim) {
    const tier = Number(dim[1]) - 1;
    const entry = autobuyers.dimensions[tier];
    if (e.shiftKey) {
      toggleAutobuyerMode(entry, () => game.toggleAdAutobuyerMode(tier), ui);
    } else {
      toggleAutobuyer(entry, () => game.toggleAdAutobuyer(tier), ui);
    }
    return;
  }

  switch (e.code) {
    case "KeyT":
      // Shift+Alt+T (tickspeed singles/max mode) is a no-op for now: the mode
      // is locked pre-Infinity (`can_change_mode` is always false) and the
      // engine has no command to flip it yet.
      if (!e.shiftKey) {
        toggleAutobuyer(
          autobuyers.tickspeed,
          () => game.toggleTickspeedAutobuyer(),
          ui,
        );
      }
      break;
    case "KeyD":
      toggleAutobuyer(
        autobuyers.dim_boost,
        () => game.toggleAutobuyer("dimBoost"),
        ui,
      );
      break;
    case "KeyG":
      toggleAutobuyer(
        autobuyers.galaxy,
        () => game.toggleAutobuyer("galaxy"),
        ui,
      );
      break;
    case "KeyC":
      toggleAutobuyer(
        autobuyers.big_crunch,
        () => game.toggleAutobuyer("bigCrunch"),
        ui,
      );
      break;
    default:
      break;
  }
}

// Toggle one autobuyer on/off if it's unlocked. The toast reports the new
// state, inferred from the pre-toggle snapshot (the command resolves before
// the next snapshot arrives); entry names already end in "Autobuyer".
function toggleAutobuyer(entry, doToggle, ui) {
  if (!entry.is_unlocked) return;
  doToggle();
  ui.notify(`${entry.name} toggled ${entry.is_active ? "off" : "on"}`);
}

// Toggle an autobuyer between buying singles and buying max, where the mode
// exists and is changeable (currently only the AD autobuyers).
function toggleAutobuyerMode(entry, doToggle, ui) {
  if (!entry.is_unlocked || !entry.can_change_mode) return;
  doToggle();
  ui.notify(
    `${entry.name} set to buy ${entry.mode === "single" ? "max" : "singles"}`,
  );
}
