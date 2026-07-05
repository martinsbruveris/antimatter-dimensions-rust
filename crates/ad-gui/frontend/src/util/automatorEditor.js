// The Automator text editor singleton (the original's `AutomatorTextUI` +
// `AutomatorHighlighter` + `AutomatorScroller`, adapted to the snapshot/IPC
// architecture). A single CodeMirror instance and its container live for the
// whole session; the Vue component adopts the container while mounted, so
// documents, undo history, and scroll position survive tab switches.
//
// CodeMirror's built-in per-document undo/redo (mod+z / mod+y) is used
// instead of the original's custom cross-editor undo buffer — a Stage D
// deviation noted in the design doc (there is no block editor yet).
import { ref } from "vue";

import CodeMirror, { registerAutomatorMode } from "./automatorMode";
import { useGameStore } from "../stores/game";

export const LINE_TYPES = ["active", "event", "error"];

// Latest compile errors of the editor's script — reactive, so the controls
// bar / docs button / error panel track it.
export const automatorErrors = ref([]);

export const AutomatorTextUI = {
  container: null,
  textArea: null,
  editor: null,
  /** CodeMirror.Doc per script id. */
  documents: {},
  /** Highlighted line (1-based) per line type; -1 = none. */
  lines: { active: -1, event: -1, error: -1 },
  /** Debounce handle for save-on-change. */
  saveTimer: null,
  /** The script id the editor currently shows. */
  currentScriptId: 0,

  initialize() {
    if (this.container) return;
    registerAutomatorMode();
    this.container = document.createElement("div");
    this.container.className = "l-automator-editor__codemirror-container";
    this.textArea = document.createElement("textarea");
    this.container.appendChild(this.textArea);
    this.editor = CodeMirror.fromTextArea(this.textArea, {
      mode: "automato",
      lineNumbers: true,
      theme: "liquibyte",
      tabSize: 2,
      extraKeys: {
        Tab: (cm) => cm.execCommand("indentMore"),
        "Shift-Tab": (cm) => cm.execCommand("indentLess"),
      },
      autoCloseBrackets: true,
      lineWrapping: true,
    });

    this.editor.on("keydown", (editor, event) => {
      if (editor.state.completionActive) return;
      if (event.ctrlKey || event.altKey || event.metaKey) return;
      if (!/^[a-zA-Z0-9 \t]$/u.test(event.key)) return;
      CodeMirror.commands.autocomplete(editor, null, { completeSingle: false });
    });

    this.editor.on("change", (editor, event) => {
      if (event.origin === "setValue") return;
      // Any edit may shift lines; drop stale highlights immediately.
      this.clearAllHighlights();
      this.scheduleSave();
    });
  },

  /** Show `id`'s stored `content`, creating/refreshing its document. */
  openScript(id, content) {
    this.initialize();
    this.currentScriptId = id;
    if (!this.documents[id] || this.documents[id].getValue() !== content) {
      this.documents[id] = CodeMirror.Doc(content, "automato");
    }
    if (this.editor.getDoc() !== this.documents[id]) {
      this.editor.swapDoc(this.documents[id]);
    }
    this.refreshErrors();
  },

  dropDocument(id) {
    delete this.documents[id];
  },

  scheduleSave() {
    if (this.saveTimer) clearTimeout(this.saveTimer);
    const id = this.currentScriptId;
    this.saveTimer = setTimeout(async () => {
      this.saveTimer = null;
      const game = useGameStore();
      const content = this.editor.getDoc().getValue();
      const result = await game.saveAutomatorScript(id, content);
      automatorErrors.value = result.errors;
      this.markErrorLines();
    }, 170);
  },

  async refreshErrors() {
    const game = useGameStore();
    automatorErrors.value = await game.getAutomatorErrors(this.currentScriptId);
    this.markErrorLines();
  },

  /** Whole-line error underlines via the lint gutter classes. */
  markErrorLines() {
    if (!this.editor) return;
    // Reuse the "error" line-type slot for the squiggle-equivalent marking;
    // only the first error line gets the persistent highlight (the panel's
    // jump buttons highlight specific ones).
    const doc = this.editor.getDoc();
    doc.getAllMarks().forEach((m) => m.clear());
    for (const error of automatorErrors.value) {
      const line = error.line - 1;
      if (line < 0 || line >= doc.lineCount()) continue;
      doc.markText(
        { line, ch: 0 },
        { line, ch: doc.getLine(line).length },
        { className: "CodeMirror-lint-mark-error", title: error.info },
      );
    }
  },

  /** Move a highlight (active/event/error) to a 1-based line; -1 clears. */
  updateHighlightedLine(line, type) {
    if (!this.editor) return;
    const old = this.lines[type];
    if (old > 0) {
      this.editor.removeLineClass(old - 1, "background", `c-automator-editor__${type}-line`);
      this.editor.removeLineClass(old - 1, "gutter", `c-automator-editor__${type}-line-gutter`);
    }
    this.lines[type] = -1;
    if (line > 0 && line <= this.editor.getDoc().lineCount()) {
      this.editor.addLineClass(line - 1, "background", `c-automator-editor__${type}-line`);
      this.editor.addLineClass(line - 1, "gutter", `c-automator-editor__${type}-line-gutter`);
      this.lines[type] = line;
    }
  },

  clearAllHighlights() {
    if (!this.editor) return;
    for (const type of LINE_TYPES) {
      const doc = this.editor.getDoc();
      for (let line = 0; line < doc.lineCount(); line++) {
        this.editor.removeLineClass(line, "background", `c-automator-editor__${type}-line`);
        this.editor.removeLineClass(line, "gutter", `c-automator-editor__${type}-line-gutter`);
      }
      this.lines[type] = -1;
    }
  },

  /** Scroll the editor so a 1-based line is visible (`AutomatorScroller`). */
  scrollToLine(line) {
    if (!this.editor) return;
    this.editor.scrollIntoView({ line: Math.max(0, line - 1), ch: 0 }, 36);
  },
};
