//! The Automator lexer — a hand-written, line-oriented scanner mirroring the
//! original's chevrotain token set (`src/core/automator/lexer.js`).
//!
//! Words are scanned as maximal identifier chunks and then classified
//! (case-insensitively) against the keyword/currency tables; that naturally
//! reproduces chevrotain's `longer_alt: Identifier` behavior ("ecological" is
//! an identifier, not `ec` + junk). Multi-word tokens ("black hole",
//! "pending ip", "x highest", "ec5 completions", …) are matched greedily,
//! longest phrase first, before single-word classification.

use super::program::{AutomatorCurrency, PrestigeLayer};

/// Comparison operators. `Eq` (`==` / `=`) lexes but is rejected in
/// validation ("Comparisons cannot be done with equality").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
}

impl CmpOp {
    /// The operator as typed (for event-log text).
    pub fn symbol(self) -> &'static str {
        match self {
            CmpOp::Gt => ">",
            CmpOp::Lt => "<",
            CmpOp::Gte => ">=",
            CmpOp::Lte => "<=",
            CmpOp::Eq => "==",
        }
    }
}

/// Single-word keywords (the original's `Keyword` category plus the structural
/// words). Multi-word keywords (`black hole`, `store game time`, `x highest`)
/// have their own [`TokenKind`] variants via phrase matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kw {
    Auto,
    Buy,
    Dilation,
    Ec,
    If,
    Load,
    Notify,
    Nowait,
    Off,
    On,
    Pause,
    Purchase,
    Respec,
    Restart,
    Start,
    Stop,
    Studies,
    Unlock,
    Until,
    Use,
    Wait,
    While,
    BlackHole,
    StoreGameTime,
    XHighest,
}

/// Study-path shorthands usable in study lists (`StudyPath` category). The
/// Infinity path is the `infinity` PrestigeEvent token in the original
/// (`extraCategories: [StudyPath]`); the parser accepts
/// `TokenKind::Prestige(Infinity)` in study lists for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyPathKind {
    Idle,
    Passive,
    Active,
    Antimatter,
    Time,
    Light,
    Dark,
}

impl StudyPathKind {
    /// The studies the shorthand expands to (`NormalTimeStudies.paths`).
    pub fn studies(self) -> &'static [u16] {
        match self {
            StudyPathKind::Antimatter => &[71, 81, 91, 101],
            StudyPathKind::Time => &[73, 83, 93, 103],
            StudyPathKind::Active => &[121, 131, 141],
            StudyPathKind::Passive => &[122, 132, 142],
            StudyPathKind::Idle => &[123, 133, 143],
            StudyPathKind::Light => &[221, 223, 225, 227, 231, 233],
            StudyPathKind::Dark => &[222, 224, 226, 228, 232, 234],
        }
    }

    /// The Infinity-Dimension path (the `infinity` token's `$studyPath`).
    pub const INFINITY_PATH: &'static [u16] = &[72, 82, 92, 102];
}

/// One lexed token. `image` keeps the raw text as typed (needed for
/// case-sensitive constant lookups and faithful event-log/error text).
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub image: String,
    /// 1-based source line.
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// A number literal (unsigned — `-` always lexes as `Dash`, matching the
    /// original's token order). The image is reparsed to `Decimal` at
    /// compile time.
    Number,
    /// A quoted string, image including the quotes (the original logs and
    /// displays the raw image).
    Str,
    Comment,
    /// `bh1` / `bh2`.
    BlackHoleLit(u8),
    Currency(AutomatorCurrency),
    Op(CmpOp),
    LCurly,
    RCurly,
    Comma,
    Pipe,
    Dash,
    Exclamation,
    Kw(Kw),
    Prestige(PrestigeLayer),
    StudyPath(StudyPathKind),
    /// A duration unit; the payload is the milliseconds-per-unit scale.
    TimeUnit(f64),
    /// `ec<N>` (any N — range-checked in validation for clearer errors).
    EcLiteral(u32),
    Identifier,
    /// `name <rest of line>` — the preset-by-name form consumes its argument
    /// at lex time like the original's `Name` token. Payload: the argument.
    PresetName(String),
    /// `id [digit]` — the preset-by-slot form.
    PresetId(Option<u8>),
    /// The hidden `blob  ` easter-egg no-op (requires two trailing spaces).
    Blob,
}

/// A lexer error: a run of characters no token matches.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub line: u32,
    pub chars: String,
}

/// The lexed script: tokens grouped per source line (the grammar is
/// line-oriented, so this is the natural shape for the parser).
pub struct LexedScript {
    pub lines: Vec<Vec<Token>>,
    pub errors: Vec<LexError>,
}

/// Words that may start a multi-word phrase, with the completions tried
/// longest-first: (rest-of-phrase, resulting kind).
fn phrase_completions(first: &str) -> &'static [(&'static [&'static str], PhraseKind)] {
    use AutomatorCurrency as C;
    use PhraseKind::*;
    match first {
        "pending" => &[
            (&["glyph", "level"], Currency(C::PendingGlyphLevel)),
            (&["completions"], Currency(C::PendingCompletions)),
            (&["ip"], Currency(C::PendingIp)),
            (&["ep"], Currency(C::PendingEp)),
            (&["tp"], Currency(C::PendingTp)),
            (&["rm"], Currency(C::PendingRm)),
        ],
        "total" => &[
            (&["space", "theorems"], Currency(C::TotalSpaceTheorems)),
            (&["completions"], Currency(C::TotalCompletions)),
            (&["tt"], Currency(C::TotalTt)),
        ],
        "banked" => &[(&["infinities"], Currency(C::BankedInfinities))],
        "spent" => &[(&["tt"], Currency(C::SpentTt))],
        "time" => &[
            (&["theorems"], Currency(C::Tt)),
            (&["theorem"], Currency(C::Tt)),
        ],
        "space" => &[(&["theorems"], Currency(C::SpaceTheorems))],
        "filter" => &[(&["score"], Currency(C::FilterScore))],
        "black" => &[(&["hole"], Keyword(Kw::BlackHole))],
        "store" | "stored" => &[(&["game", "time"], Keyword(Kw::StoreGameTime))],
        "x" => &[(&["highest"], Keyword(Kw::XHighest))],
        _ => &[],
    }
}

enum PhraseKind {
    Currency(AutomatorCurrency),
    Keyword(Kw),
}

/// Classify a single word (already known not to start a matched phrase).
fn classify_word(word: &str) -> TokenKind {
    use AutomatorCurrency as C;
    let lower = word.to_ascii_lowercase();
    // `ec<digits>` (also `bh1`/`bh2`).
    if let Some(digits) = lower.strip_prefix("ec") {
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(n) = digits.parse::<u32>() {
                return TokenKind::EcLiteral(n);
            }
        }
    }
    match lower.as_str() {
        "bh1" => return TokenKind::BlackHoleLit(1),
        "bh2" => return TokenKind::BlackHoleLit(2),
        _ => {}
    }
    let kind = match lower.as_str() {
        // Currencies.
        "am" => TokenKind::Currency(C::Am),
        "ip" => TokenKind::Currency(C::Ip),
        "ep" => TokenKind::Currency(C::Ep),
        "rm" => TokenKind::Currency(C::Rm),
        "dt" => TokenKind::Currency(C::Dt),
        "tp" => TokenKind::Currency(C::Tp),
        "rg" => TokenKind::Currency(C::Rg),
        "rep" | "replicanti" => TokenKind::Currency(C::Rep),
        "tt" => TokenKind::Currency(C::Tt),
        "infinities" => TokenKind::Currency(C::Infinities),
        "eternities" => TokenKind::Currency(C::Eternities),
        "realities" => TokenKind::Currency(C::Realities),
        // Prestige events.
        "infinity" => TokenKind::Prestige(PrestigeLayer::Infinity),
        "eternity" => TokenKind::Prestige(PrestigeLayer::Eternity),
        "reality" => TokenKind::Prestige(PrestigeLayer::Reality),
        // Study paths.
        "idle" => TokenKind::StudyPath(StudyPathKind::Idle),
        "passive" => TokenKind::StudyPath(StudyPathKind::Passive),
        "active" => TokenKind::StudyPath(StudyPathKind::Active),
        "antimatter" => TokenKind::StudyPath(StudyPathKind::Antimatter),
        "time" => TokenKind::StudyPath(StudyPathKind::Time),
        "light" => TokenKind::StudyPath(StudyPathKind::Light),
        "dark" => TokenKind::StudyPath(StudyPathKind::Dark),
        // Time units (`ms`, `s|sec|second(s)`, `m|min|minute(s)`, `h|hour(s)`).
        "ms" => TokenKind::TimeUnit(1.0),
        "s" | "sec" | "second" | "seconds" => TokenKind::TimeUnit(1000.0),
        "m" | "min" | "minute" | "minutes" => TokenKind::TimeUnit(60.0 * 1000.0),
        "h" | "hour" | "hours" => TokenKind::TimeUnit(3600.0 * 1000.0),
        // Keywords.
        "auto" => TokenKind::Kw(Kw::Auto),
        "buy" => TokenKind::Kw(Kw::Buy),
        "dilation" => TokenKind::Kw(Kw::Dilation),
        "ec" => TokenKind::Kw(Kw::Ec),
        "if" => TokenKind::Kw(Kw::If),
        "load" => TokenKind::Kw(Kw::Load),
        "notify" => TokenKind::Kw(Kw::Notify),
        "nowait" => TokenKind::Kw(Kw::Nowait),
        "off" => TokenKind::Kw(Kw::Off),
        "on" => TokenKind::Kw(Kw::On),
        "pause" => TokenKind::Kw(Kw::Pause),
        "purchase" => TokenKind::Kw(Kw::Purchase),
        "respec" => TokenKind::Kw(Kw::Respec),
        "restart" => TokenKind::Kw(Kw::Restart),
        "start" => TokenKind::Kw(Kw::Start),
        "stop" => TokenKind::Kw(Kw::Stop),
        "studies" => TokenKind::Kw(Kw::Studies),
        "unlock" => TokenKind::Kw(Kw::Unlock),
        "until" => TokenKind::Kw(Kw::Until),
        "use" => TokenKind::Kw(Kw::Use),
        "wait" => TokenKind::Kw(Kw::Wait),
        "while" => TokenKind::Kw(Kw::While),
        _ => TokenKind::Identifier,
    };
    kind
}

/// Lex a whole script. Never fails — unexpected characters become
/// [`LexError`]s and scanning continues, mirroring chevrotain's recovery.
pub fn lex(text: &str) -> LexedScript {
    let mut lines = Vec::new();
    let mut errors = Vec::new();

    for (line_idx, line) in text.split('\n').enumerate() {
        let line_no = line_idx as u32 + 1;
        let mut tokens = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0usize;
        let mut bad_run = String::new();
        let flush_bad = |bad: &mut String, errors: &mut Vec<LexError>| {
            if !bad.is_empty() {
                errors.push(LexError {
                    line: line_no,
                    chars: std::mem::take(bad),
                });
            }
        };

        while i < chars.len() {
            let c = chars[i];
            if c == ' ' || c == '\t' || c == '\r' {
                flush_bad(&mut bad_run, &mut errors);
                i += 1;
                continue;
            }

            // Comments run to the end of the line.
            if c == '#' || (c == '/' && chars.get(i + 1) == Some(&'/')) {
                flush_bad(&mut bad_run, &mut errors);
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    image: chars[i..].iter().collect(),
                    line: line_no,
                });
                break;
            }

            // Strings (must close on the same line, like the original's /".*"/).
            if c == '"' || c == '\'' {
                if let Some(close) = chars[i + 1..].iter().position(|&x| x == c) {
                    flush_bad(&mut bad_run, &mut errors);
                    let end = i + 1 + close + 1;
                    tokens.push(Token {
                        kind: TokenKind::Str,
                        image: chars[i..end].iter().collect(),
                        line: line_no,
                    });
                    i = end;
                } else {
                    bad_run.push(c);
                    i += 1;
                }
                continue;
            }

            // Punctuation and operators.
            let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            let punct = match two.as_str() {
                ">=" => Some((TokenKind::Op(CmpOp::Gte), 2)),
                "<=" => Some((TokenKind::Op(CmpOp::Lte), 2)),
                "==" => Some((TokenKind::Op(CmpOp::Eq), 2)),
                _ => match c {
                    '>' => Some((TokenKind::Op(CmpOp::Gt), 1)),
                    '<' => Some((TokenKind::Op(CmpOp::Lt), 1)),
                    '=' => Some((TokenKind::Op(CmpOp::Eq), 1)),
                    '{' => Some((TokenKind::LCurly, 1)),
                    '}' => Some((TokenKind::RCurly, 1)),
                    ',' => Some((TokenKind::Comma, 1)),
                    '|' => Some((TokenKind::Pipe, 1)),
                    '-' => Some((TokenKind::Dash, 1)),
                    '!' => Some((TokenKind::Exclamation, 1)),
                    _ => None,
                },
            };
            if let Some((kind, len)) = punct {
                flush_bad(&mut bad_run, &mut errors);
                tokens.push(Token {
                    kind,
                    image: chars[i..i + len].iter().collect(),
                    line: line_no,
                });
                i += len;
                continue;
            }

            // Numbers: `(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?` — like the
            // original, a leading 0 is not followed by more digits.
            if c.is_ascii_digit() {
                flush_bad(&mut bad_run, &mut errors);
                let start = i;
                if c == '0' {
                    i += 1;
                } else {
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if chars.get(i) == Some(&'.')
                    && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())
                {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if matches!(chars.get(i), Some('e') | Some('E')) {
                    let mut j = i + 1;
                    if matches!(chars.get(j), Some('+') | Some('-')) {
                        j += 1;
                    }
                    if chars.get(j).is_some_and(|d| d.is_ascii_digit()) {
                        i = j;
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Number,
                    image: chars[start..i].iter().collect(),
                    line: line_no,
                });
                continue;
            }

            // Word chunks.
            if c.is_ascii_alphabetic() || c == '_' {
                flush_bad(&mut bad_run, &mut errors);
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let lower = word.to_ascii_lowercase();

                // `name <anything to EOL, stopping at a comment>` — the
                // original's Name token swallows its argument.
                if lower == "name" {
                    let (arg, consumed) = scan_name_arg(&chars, i);
                    let image: String = chars[start..consumed].iter().collect();
                    tokens.push(Token {
                        kind: TokenKind::PresetName(arg),
                        image,
                        line: line_no,
                    });
                    i = consumed;
                    continue;
                }

                // `id [digit]`.
                if lower == "id" {
                    let mut j = i;
                    while matches!(chars.get(j), Some(' ') | Some('\t')) {
                        j += 1;
                    }
                    let digit = chars
                        .get(j)
                        .filter(|d| d.is_ascii_digit())
                        .map(|d| *d as u8 - b'0');
                    let consumed = if digit.is_some() { j + 1 } else { i };
                    tokens.push(Token {
                        kind: TokenKind::PresetId(digit),
                        image: chars[start..consumed].iter().collect(),
                        line: line_no,
                    });
                    i = consumed;
                    continue;
                }

                // The hidden `blob  ` no-op needs two trailing whitespace
                // characters; a bare "blob" stays an identifier.
                if lower == "blob"
                    && matches!(chars.get(i), Some(' ') | Some('\t'))
                    && matches!(chars.get(i + 1), Some(' ') | Some('\t'))
                {
                    tokens.push(Token {
                        kind: TokenKind::Blob,
                        image: word,
                        line: line_no,
                    });
                    i += 2;
                    continue;
                }

                // Multi-word phrases, longest completion first. `ec<N>
                // completions` is a phrase whose head is an EcLiteral.
                let mut matched_phrase = false;
                if let Some(digits) = lower.strip_prefix("ec") {
                    if let Ok(n) = digits.parse::<u32>() {
                        if (1..=12).contains(&n) {
                            if let Some(end) = match_words(&chars, i, &["completions"]) {
                                tokens.push(Token {
                                    kind: TokenKind::Currency(
                                        AutomatorCurrency::EcCompletions(n as u8),
                                    ),
                                    image: chars[start..end].iter().collect(),
                                    line: line_no,
                                });
                                i = end;
                                matched_phrase = true;
                            }
                        }
                    }
                }
                if !matched_phrase {
                    for (rest, kind) in phrase_completions(&lower) {
                        if let Some(end) = match_words(&chars, i, rest) {
                            let image: String = chars[start..end].iter().collect();
                            let kind = match kind {
                                PhraseKind::Currency(c) => TokenKind::Currency(*c),
                                PhraseKind::Keyword(k) => TokenKind::Kw(*k),
                            };
                            tokens.push(Token {
                                kind,
                                image,
                                line: line_no,
                            });
                            i = end;
                            matched_phrase = true;
                            break;
                        }
                    }
                }
                if matched_phrase {
                    continue;
                }

                tokens.push(Token {
                    kind: classify_word(&word),
                    image: word,
                    line: line_no,
                });
                continue;
            }

            // Anything else is a lexer error; group consecutive bad chars.
            bad_run.push(c);
            i += 1;
        }
        flush_bad(&mut bad_run, &mut errors);
        lines.push(tokens);
    }

    LexedScript { lines, errors }
}

/// Try to match `words` (case-insensitive, whitespace-separated) starting at
/// `from`; returns the index just past the last word.
fn match_words(chars: &[char], from: usize, words: &[&str]) -> Option<usize> {
    let mut i = from;
    for word in words {
        // At least one space/tab between words.
        let mut j = i;
        while matches!(chars.get(j), Some(' ') | Some('\t')) {
            j += 1;
        }
        if j == i {
            return None;
        }
        let end = j + word.len();
        if end > chars.len() {
            return None;
        }
        let candidate: String = chars[j..end].iter().collect();
        if !candidate.eq_ignore_ascii_case(word) {
            return None;
        }
        // The match must end at a word boundary.
        if chars
            .get(end)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
        {
            return None;
        }
        i = end;
    }
    Some(i)
}

/// The `name` token's argument: `[ \t]+` then characters matching the
/// original's `(\/(?!\/)|[^\n#/])*` (anything except a comment start). The
/// argument is trimmed like `presetSplitter` effectively sees it.
fn scan_name_arg(chars: &[char], from: usize) -> (String, usize) {
    let mut i = from;
    let mut saw_space = false;
    while matches!(chars.get(i), Some(' ') | Some('\t')) {
        saw_space = true;
        i += 1;
    }
    if !saw_space {
        return (String::new(), from);
    }
    let start = i;
    while i < chars.len() {
        let c = chars[i];
        if c == '#' {
            break;
        }
        if c == '/' {
            if chars.get(i + 1) == Some(&'/') {
                break;
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    let arg: String = chars[start..i].iter().collect();
    (arg.trim().to_string(), i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<TokenKind> {
        let lexed = lex(line);
        assert!(lexed.errors.is_empty(), "unexpected lex errors");
        lexed.lines[0].iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn words_classify_case_insensitively() {
        assert_eq!(
            kinds("Studies NOWAIT purchase 11,21-33"),
            vec![
                TokenKind::Kw(Kw::Studies),
                TokenKind::Kw(Kw::Nowait),
                TokenKind::Kw(Kw::Purchase),
                TokenKind::Number,
                TokenKind::Comma,
                TokenKind::Number,
                TokenKind::Dash,
                TokenKind::Number,
            ]
        );
    }

    #[test]
    fn multi_word_tokens_match_greedily() {
        assert_eq!(
            kinds("wait pending glyph level >= 5000"),
            vec![
                TokenKind::Kw(Kw::Wait),
                TokenKind::Currency(AutomatorCurrency::PendingGlyphLevel),
                TokenKind::Op(CmpOp::Gte),
                TokenKind::Number,
            ]
        );
        assert_eq!(
            kinds("auto infinity 5 x highest"),
            vec![
                TokenKind::Kw(Kw::Auto),
                TokenKind::Prestige(PrestigeLayer::Infinity),
                TokenKind::Number,
                TokenKind::Kw(Kw::XHighest),
            ]
        );
        assert_eq!(
            kinds("wait black hole bh1"),
            vec![
                TokenKind::Kw(Kw::Wait),
                TokenKind::Kw(Kw::BlackHole),
                TokenKind::BlackHoleLit(1),
            ]
        );
        assert_eq!(
            kinds("if total tt < 14000"),
            vec![
                TokenKind::Kw(Kw::If),
                TokenKind::Currency(AutomatorCurrency::TotalTt),
                TokenKind::Op(CmpOp::Lt),
                TokenKind::Number,
            ]
        );
    }

    #[test]
    fn ec_literals_and_ec_completions() {
        assert_eq!(
            kinds("if ec10 completions < 5"),
            vec![
                TokenKind::Kw(Kw::If),
                TokenKind::Currency(AutomatorCurrency::EcCompletions(10)),
                TokenKind::Op(CmpOp::Lt),
                TokenKind::Number,
            ]
        );
        assert_eq!(
            kinds("unlock ec10"),
            vec![TokenKind::Kw(Kw::Unlock), TokenKind::EcLiteral(10)]
        );
        // `ec13 completions` is not a currency (only 1–12 exist).
        assert_eq!(
            kinds("wait ec13 completions"),
            vec![
                TokenKind::Kw(Kw::Wait),
                TokenKind::EcLiteral(13),
                TokenKind::Identifier,
            ]
        );
    }

    #[test]
    fn longer_words_stay_identifiers() {
        // chevrotain `longer_alt` behavior.
        assert_eq!(kinds("ecological"), vec![TokenKind::Identifier]);
        assert_eq!(kinds("stopping"), vec![TokenKind::Identifier]);
        assert_eq!(kinds("repl"), vec![TokenKind::Identifier]);
    }

    #[test]
    fn name_token_swallows_argument() {
        let lexed = lex("studies load name my preset!");
        let tokens = &lexed.lines[0];
        assert_eq!(tokens.len(), 3);
        assert_eq!(
            tokens[2].kind,
            TokenKind::PresetName("my preset!".to_string())
        );
    }

    #[test]
    fn id_token_takes_one_digit() {
        let lexed = lex("studies load id 2");
        assert_eq!(lexed.lines[0][2].kind, TokenKind::PresetId(Some(2)));
        let lexed = lex("studies load id");
        assert_eq!(lexed.lines[0][2].kind, TokenKind::PresetId(None));
    }

    #[test]
    fn strings_comments_numbers() {
        assert_eq!(
            kinds("notify \"hello world\""),
            vec![TokenKind::Kw(Kw::Notify), TokenKind::Str]
        );
        assert_eq!(kinds("# a comment"), vec![TokenKind::Comment]);
        assert_eq!(kinds("// a comment"), vec![TokenKind::Comment]);
        let lexed = lex("pause 1.5e3 s");
        assert_eq!(lexed.lines[0][1].image, "1.5e3");
        assert_eq!(lexed.lines[0][2].kind, TokenKind::TimeUnit(1000.0));
    }

    #[test]
    fn unexpected_characters_are_collected() {
        let lexed = lex("pause @@ 10s");
        assert_eq!(lexed.errors.len(), 1);
        assert_eq!(lexed.errors[0].chars, "@@");
        // Scanning continued past the error.
        assert_eq!(lexed.lines[0].len(), 3);
    }

    #[test]
    fn blob_needs_two_trailing_spaces() {
        let lexed = lex("blob  ");
        assert_eq!(lexed.lines[0][0].kind, TokenKind::Blob);
        let lexed = lex("blob");
        assert_eq!(lexed.lines[0][0].kind, TokenKind::Identifier);
    }
}
