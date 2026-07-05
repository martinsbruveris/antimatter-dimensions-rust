// Block-editor session state (the original's `BlockAutomator`, adapted to
// the snapshot/IPC architecture): the reactive block tree for the script
// being edited, text↔block conversion, and the shared mode-switch modal
// request.
import { ref } from "vue";

import { hydrateBlock, parseLines } from "../data/automatorBlocks";
import { AutomatorTextUI, automatorErrors } from "./automatorEditor";

/** The hydrated block rows of the editor script (block mode only). */
export const blockLines = ref([]);

/**
 * Set to `{ lostBlocks, errorCount }` when the mode switch needs
 * confirmation (the original's `Modal.switchAutomatorEditorMode`).
 */
export const pendingModeSwitch = ref(null);

/** A plain message to show (the original's `Modal.message`), or null. */
export const blockSwitchMessage = ref(null);

/**
 * Custom template blocks created this session (`AutomatorData.
 * blockTemplates`, transient in the original too): `{ name, blocks }` with
 * raw engine blockify output.
 */
export const blockTemplates = ref([]);

/**
 * `AutomatorBackend.changeModes`: flush the current editor's content, then
 * flip the editor type (the engine side also stops a running script).
 */
export async function performModeSwitch(game) {
  const auto = game.snapshot.automator;
  const id = auto.editor_script;
  if (auto.editor_type === "text") {
    const content = AutomatorTextUI.editor?.getDoc().getValue();
    if (content !== undefined) {
      await game.saveAutomatorScript(id, content);
    }
    await game.automatorSetEditorType(true);
  } else {
    await saveBlocksToScript(game, id);
    await game.automatorSetEditorType(false);
  }
}

/** Load a script's stored text into blocks. Returns the lost-line count. */
export async function loadBlocksFromScript(game, id) {
  const result = await game.automatorBlockify(id);
  blockLines.value = result.blocks.map(hydrateBlock);
  return result.lost_lines;
}

/** Regenerate script text from the blocks and persist it. */
export async function saveBlocksToScript(game, id) {
  const content = parseLines(blockLines.value).join("\n");
  const result = await game.saveAutomatorScript(id, content);
  automatorErrors.value = result.errors;
  return result;
}

/** 1-based text line of the block with `id` (nested headers add a `}`). */
export function lineNumberOfBlock(id) {
  let line = 0;
  const walk = (blocks) => {
    for (const block of blocks) {
      line += 1;
      if (block.id === id) return true;
      if (block.nested) {
        if (walk(block.nest ?? [])) return true;
        line += 1; // the closing `}` line
      }
    }
    return false;
  };
  return walk(blockLines.value) ? line : -1;
}

/** The block id at a 1-based text line (for the active-line highlight). */
export function blockIdAtLine(target) {
  let line = 0;
  let found = null;
  const walk = (blocks) => {
    for (const block of blocks) {
      line += 1;
      if (line === target) {
        found = block.id;
        return true;
      }
      if (block.nested) {
        if (walk(block.nest ?? [])) return true;
        line += 1;
      }
    }
    return false;
  };
  walk(blockLines.value);
  return found;
}
