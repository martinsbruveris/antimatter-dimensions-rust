// CodeMirror setup for the Automator's text editor: the "automato" syntax
// highlighting mode (vendored verbatim from the original's
// automator-codemirror.js state machine) and a simplified word-list
// autocomplete (the original walks chevrotain's content assist; we filter a
// static token list by prefix — a noted deviation).
import CodeMirror from "codemirror";
import "codemirror/addon/mode/simple";
import "codemirror/addon/hint/show-hint";
import "codemirror/addon/lint/lint";
import "codemirror/addon/edit/closebrackets";

const commentRule = { regex: /(\/\/|#).*/u, token: "comment", next: "start" };

// Words offered by Tab / typing autocomplete (the tokens' `$autocomplete`).
const AUTOCOMPLETE_WORDS = [
  "auto", "black hole", "if", "notify", "pause", "studies", "start", "stop",
  "unlock", "until", "wait", "while", "infinity", "eternity", "reality",
  "nowait", "respec", "on", "off", "load", "purchase", "dilation", "ec",
  "x highest", "name", "id",
  "am", "ip", "ep", "rm", "dt", "tp", "rg", "rep", "tt", "total tt",
  "spent tt", "infinities", "banked infinities", "eternities", "realities",
  "pending ip", "pending ep", "pending tp", "pending rm",
  "pending glyph level", "total completions", "pending completions",
  "sec", "min", "hours", "ms",
];

let registered = false;

export function registerAutomatorMode() {
  if (registered) return;
  registered = true;

  CodeMirror.registerHelper("hint", "anyword", (editor) => {
    const cursor = editor.getDoc().getCursor();
    let start = cursor.ch;
    const end = cursor.ch;
    const line = editor.getLine(cursor.line);
    while (start && /\w/u.test(line.charAt(start - 1))) --start;
    const currentPrefix = line.slice(start, end).toLowerCase();
    if (!currentPrefix) return undefined;
    const list = AUTOCOMPLETE_WORDS.filter(
      (w) => w.startsWith(currentPrefix) && w !== currentPrefix,
    );
    return {
      list,
      from: CodeMirror.Pos(cursor.line, start),
      to: CodeMirror.Pos(cursor.line, end),
    };
  });

  // The syntax-highlighting state machine, vendored from the original
  // automator-codemirror.js (purely visual; colors come from liquibyte.css).
  CodeMirror.defineSimpleMode("automato", {
    start: [
      commentRule,
      { regex: /studies\s+/ui, token: "keyword", next: "studiesArgs" },
      { regex: /blob\s\s/ui, token: "blob" },
      {
        // eslint-disable-next-line max-len
        regex: /(auto|if|pause|studies|time[ \t]+theorems?|space[ \t]+theorems?|until|wait|while|black[ \t]+hole|stored?[ \t]+game[ \t]+time|notify)\s/ui,
        token: "keyword",
        next: "commandArgs",
      },
      { regex: /stop/ui, token: "keyword", next: "commandDone" },
      { regex: /start\s|unlock\s/ui, token: "keyword", next: "startUnlock" },
      {
        regex: /infinity\S+|eternity\S+|reality\S+|pause\S+|restart\S+/ui,
        token: "error",
        next: "commandDone",
      },
      { regex: /infinity|eternity|reality/ui, token: "keyword", next: "prestige" },
      { regex: /pause|restart/ui, token: "keyword", next: "commandDone" },
      { regex: /\}/ui, dedent: true },
      { regex: /\S+\s/ui, token: "error", next: "commandDone" },
    ],
    studiesArgs: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /load(\s+|$)/ui, token: "variable-2", next: "studiesLoad" },
      { regex: /respec/ui, token: "variable-2", next: "commandDone" },
      { regex: /purchase/ui, token: "variable-2", next: "studiesList" },
      { regex: /nowait(\s+|$)/ui, token: "property" },
    ],
    studiesList: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /(antimatter|infinity|time)(?=[\s,|]|$)/ui, token: "number" },
      { regex: /(active|passive|idle)(?=[\s,|]|$)/ui, token: "number" },
      { regex: /(light|dark)(?=[\s,|]|$)/ui, token: "number" },
      { regex: /([1-9][0-9]+)(?=[\s,!|-]|$)/ui, token: "number" },
      { regex: /[a-zA-Z_][a-zA-Z_0-9]*/u, token: "variable", next: "commandDone" },
      { regex: /!$/ui, token: "variable-2" },
      { regex: /([1-9]|1[0-2])(?=!|$)/ui, token: "number" },
    ],
    studiesLoad: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /id(\s+|$)/ui, token: "variable-2", next: "studiesLoadId" },
      { regex: /name(\s+|$)/ui, token: "variable-2", next: "studiesLoadPreset" },
      { regex: /\S+/ui, token: "error" },
    ],
    studiesLoadId: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /\d/ui, token: "qualifier", next: "commandDone" },
    ],
    studiesLoadPreset: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /(\/(?!\/)|[^\s#/])+/ui, token: "qualifier", next: "commandDone" },
    ],
    prestige: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /nowait(\s|$)/ui, token: "property" },
      { regex: /respec/ui, token: "variable-2" },
    ],
    commandDone: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /\}/ui, dedent: true },
      { regex: /\S+/ui, token: "error" },
    ],
    startUnlock: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /ec\s?(1[0-2]|[1-9])|dilation/ui, token: "variable-2", next: "commandDone" },
      { regex: /nowait(\s|$)/ui, token: "property" },
    ],
    commandArgs: [
      commentRule,
      { sol: true, next: "start" },
      { regex: /<=|>=|<|>/ui, token: "operator" },
      { regex: /nowait(\s|$)/ui, token: "property" },
      { regex: /".*"/ui, token: "string", next: "commandDone" },
      { regex: /'.*'/ui, token: "string", next: "commandDone" },
      { regex: /(on|off|bh1|bh2|dilation|load|respec)(\s|$)/ui, token: "variable-2" },
      { regex: /(eternity|reality|use)(\s|$)/ui, token: "variable-2" },
      { regex: /(antimatter|infinity|time)(\s|$|(?=,))/ui, token: "variable-2" },
      { regex: /(active|passive|idle)(\s|$|(?=,))/ui, token: "variable-2" },
      { regex: /(light|dark)(\s|$|(?=,))/ui, token: "variable-2" },
      { regex: /x[\t ]+highest(\s|$)/ui, token: "variable-2" },
      {
        regex: /pending[\t ]+(completions|ip|ep|tp|rm|glyph[\t ]+level)(\s|$)/ui,
        token: "variable-2",
      },
      { regex: /total[\t ]+(completions|tt|space theorems)(\s|$)/ui, token: "variable-2" },
      { regex: /spent[\t ]+tt(\s|$)/ui, token: "variable-2" },
      { regex: /filter[ \t]+score/ui, token: "variable-2" },
      { regex: /ec(1[0-2]|[1-9])[\t ]+completions(\s|$)/ui, token: "variable-2" },
      { regex: /(am|ip|ep|all)(\s|$)/ui, token: "variable-2" },
      {
        // eslint-disable-next-line max-len
        regex: /(rm|rg|dt|tp|tt|space theorems|(banked )?infinities|eternities|realities|rep(licanti)?)(\s|$)/ui,
        token: "variable-2",
      },
      { regex: / sec(onds ?) ?| min(utes ?) ?| hours ?/ui, token: "variable-2" },
      {
        regex: /([0-9]+:[0-5][0-9]:[0-5][0-9]|[0-5]?[0-9]:[0-5][0-9]|t[1-4])/ui,
        token: "number",
      },
      { regex: /-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?/ui, token: "number" },
      { regex: /[a-zA-Z_][a-zA-Z_0-9]*/u, token: "variable" },
      { regex: /\{/ui, indent: true, next: "commandDone" },
      { regex: /\}/ui, dedent: true },
    ],
    meta: {
      lineComment: "//",
      electricChars: "}",
    },
  });
}

export default CodeMirror;
