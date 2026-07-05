//! The Automator parser: line-oriented recursive descent over the lexed
//! token lines, producing a typed AST plus recoverable parse errors.
//!
//! The grammar is one command per line; `{` opens a block ending on a line
//! holding only `}` (`src/core/automator/parser.js`). Error recovery is
//! per-line: a malformed line yields one error and parsing continues, so a
//! single typo doesn't hide errors further down (matching the original's
//! one-error-per-line presentation).

use super::lexer::{CmpOp, Kw, LexedScript, StudyPathKind, Token, TokenKind};
use super::program::PrestigeLayer;
use super::AutomatorError;

/// One parsed command with its source line.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    pub line: u32,
    pub kind: CommandAst,
}

/// A comparison side, pre-validation.
#[derive(Debug, Clone, PartialEq)]
pub enum CmpValueAst {
    Currency(super::program::AutomatorCurrency, String),
    Number(String),
    Const(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonAst {
    pub left: CmpValueAst,
    pub op: CmpOp,
    pub right: CmpValueAst,
}

/// `auto <prestige> <arg>`.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoArgAst {
    On,
    Off,
    /// number + time unit; `ms` is the scaled value, texts are as typed.
    Duration {
        ms: f64,
        num: String,
        unit: String,
    },
    XHighest(String),
    /// number + currency.
    Amount {
        num: String,
        currency: super::program::AutomatorCurrency,
        currency_image: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PauseArgAst {
    Duration { ms: f64, num: String, unit: String },
    Const(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StudyListEntryAst {
    Id(String),
    Range(String, String),
    Path(StudyPathKind),
    /// The `infinity` prestige token doubles as the Infinity-Dimension path.
    InfinityPath,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StudyListAst {
    pub entries: Vec<StudyListEntryAst>,
    /// `|N` suffix and its `!` flag.
    pub ec: Option<String>,
    pub start_ec: bool,
    /// The list as typed (trimmed), for display.
    pub image: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StudiesArgAst {
    List(StudyListAst),
    Const(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PresetRefAst {
    Id(Option<u8>),
    Name(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StoreGameTimeAction {
    On,
    Off,
    Use,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UntilCondAst {
    Cmp(ComparisonAst),
    Prestige(PrestigeLayer),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WaitCondAst {
    Cmp(ComparisonAst),
    Prestige(PrestigeLayer),
    BlackHole { off: bool, hole: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandAst {
    Auto {
        layer: PrestigeLayer,
        arg: AutoArgAst,
    },
    BlackHole {
        on: bool,
    },
    Blob,
    Comment,
    If {
        cmp: ComparisonAst,
        block: Vec<ParsedCommand>,
        end_line: u32,
    },
    Notify {
        text: String,
    },
    Pause {
        arg: PauseArgAst,
    },
    Prestige {
        layer: PrestigeLayer,
        nowait: bool,
        respec: bool,
    },
    StartDilation,
    StartEc {
        ec: u32,
    },
    StoreGameTime {
        action: StoreGameTimeAction,
    },
    StudiesBuy {
        arg: StudiesArgAst,
        nowait: bool,
    },
    StudiesLoad {
        preset: PresetRefAst,
        nowait: bool,
    },
    StudiesRespec,
    UnlockDilation {
        nowait: bool,
    },
    UnlockEc {
        ec: u32,
        nowait: bool,
    },
    Until {
        cond: UntilCondAst,
        block: Vec<ParsedCommand>,
        end_line: u32,
    },
    Wait {
        cond: WaitCondAst,
    },
    While {
        cmp: ComparisonAst,
        block: Vec<ParsedCommand>,
        end_line: u32,
    },
    Stop,
}

pub struct ParsedScript {
    pub commands: Vec<ParsedCommand>,
    pub errors: Vec<AutomatorError>,
}

/// Parse the lexed script.
pub fn parse(lexed: &LexedScript) -> ParsedScript {
    let mut errors: Vec<AutomatorError> = lexed
        .errors
        .iter()
        .map(|e| AutomatorError {
            line: e.line,
            info: format!("Unexpected characters: {}", e.chars),
            tip: format!("{} cannot be part of a command, remove them", e.chars),
        })
        .collect();

    let mut lines = lexed.lines.iter().enumerate().peekable();
    let (commands, _) = parse_block(&mut lines, &mut errors, None);
    ParsedScript { commands, errors }
}

type LineIter<'a> =
    std::iter::Peekable<std::iter::Enumerate<std::slice::Iter<'a, Vec<Token>>>>;

/// Parse commands until end of input or a lone `}` line, returning the
/// commands and the `}` line number (None when input ran out). `open` is the
/// line of the block-opening command (None at top level), for the missing-`}`
/// error attribution (`checkBlock`).
fn parse_block(
    lines: &mut LineIter,
    errors: &mut Vec<AutomatorError>,
    open: Option<u32>,
) -> (Vec<ParsedCommand>, Option<u32>) {
    let mut commands = Vec::new();
    while let Some((idx, tokens)) = lines.next() {
        let line_no = idx as u32 + 1;
        if tokens.is_empty() {
            continue;
        }
        // A lone `}` closes the current block; at top level it is stray.
        if tokens[0].kind == TokenKind::RCurly {
            if tokens.len() > 1 {
                errors.push(AutomatorError {
                    line: line_no,
                    info: format!("Unexpected input {}", tokens[1].image),
                    tip: format!("Remove {}", tokens[1].image),
                });
            }
            if open.is_some() {
                return (commands, Some(line_no));
            }
            errors.push(AutomatorError {
                line: line_no,
                info: "Unexpected }".to_string(),
                tip: "Remove }".to_string(),
            });
            continue;
        }
        if let Some(cmd) = parse_command(line_no, tokens, lines, errors) {
            commands.push(ParsedCommand {
                line: line_no,
                kind: cmd,
            });
        }
    }
    if let Some(open_line) = open {
        errors.push(AutomatorError {
            line: open_line,
            info: "Missing closing }".to_string(),
            tip: "This loop has mismatched brackets, add a corresponding } on \
                  another line to close the loop"
                .to_string(),
        });
    }
    (commands, None)
}

/// A cursor over one line's tokens with the shared error conventions.
struct Cursor<'a> {
    tokens: &'a [Token],
    pos: usize,
    line: u32,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&'a Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    fn eat_kw(&mut self, kw: Kw) -> bool {
        if matches!(self.peek(), Some(t) if t.kind == TokenKind::Kw(kw)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// The "ran out of tokens" error (chevrotain's EarlyExitException after
    /// message modification).
    fn incomplete(&self) -> AutomatorError {
        AutomatorError {
            line: self.line,
            info: "Unexpected end of command".to_string(),
            tip: "Complete the command by adding the other parameters".to_string(),
        }
    }

    /// The "extra/unexpected token" error (NoViableAltException).
    fn unexpected(&self, token: &Token) -> AutomatorError {
        AutomatorError {
            line: self.line,
            info: format!("Unexpected input {}", token.image),
            tip: format!("Remove {}", token.image),
        }
    }

    /// Require the line to be fully consumed.
    fn expect_end(&mut self, errors: &mut Vec<AutomatorError>) {
        if let Some(t) = self.peek() {
            let err = self.unexpected(t);
            errors.push(err);
        }
    }
}

/// Parse one command line (recursing into following lines for `{` blocks).
/// Returns None when the line is malformed (an error was recorded).
fn parse_command(
    line_no: u32,
    tokens: &[Token],
    lines: &mut LineIter,
    errors: &mut Vec<AutomatorError>,
) -> Option<CommandAst> {
    let mut c = Cursor {
        tokens,
        pos: 0,
        line: line_no,
    };
    let first = c.next().expect("caller checked non-empty");

    let parsed = match &first.kind {
        TokenKind::Comment => Some(CommandAst::Comment),
        TokenKind::Blob => Some(CommandAst::Blob),
        TokenKind::Kw(Kw::Auto) => parse_auto(&mut c, errors),
        TokenKind::Kw(Kw::BlackHole) => {
            let on = if c.eat_kw(Kw::On) {
                true
            } else if c.eat_kw(Kw::Off) {
                false
            } else {
                errors.push(c.incomplete());
                return None;
            };
            Some(CommandAst::BlackHole { on })
        }
        TokenKind::Kw(Kw::Notify) => match c.next() {
            Some(t) if t.kind == TokenKind::Str => Some(CommandAst::Notify {
                text: t.image.clone(),
            }),
            Some(t) => {
                let err = c.unexpected(t);
                errors.push(err);
                return None;
            }
            None => {
                errors.push(c.incomplete());
                return None;
            }
        },
        TokenKind::Kw(Kw::Pause) => parse_pause(&mut c, errors),
        TokenKind::Kw(Kw::If) => {
            let cmp = parse_comparison(&mut c, errors)?;
            let (block, end_line) = parse_block_body(&mut c, lines, errors)?;
            return Some(CommandAst::If {
                cmp,
                block,
                end_line,
            });
        }
        TokenKind::Kw(Kw::While) => {
            let cmp = parse_comparison(&mut c, errors)?;
            let (block, end_line) = parse_block_body(&mut c, lines, errors)?;
            return Some(CommandAst::While {
                cmp,
                block,
                end_line,
            });
        }
        TokenKind::Kw(Kw::Until) => {
            let cond = if let Some(Token {
                kind: TokenKind::Prestige(layer),
                ..
            }) = c.peek()
            {
                // Only when the prestige token is the whole condition (`until
                // eternity {`); `until ep > 5` parses as a comparison.
                if matches!(c.tokens.get(c.pos + 1), Some(t) if t.kind == TokenKind::LCurly)
                {
                    let layer = *layer;
                    c.pos += 1;
                    UntilCondAst::Prestige(layer)
                } else {
                    UntilCondAst::Cmp(parse_comparison(&mut c, errors)?)
                }
            } else {
                UntilCondAst::Cmp(parse_comparison(&mut c, errors)?)
            };
            let (block, end_line) = parse_block_body(&mut c, lines, errors)?;
            return Some(CommandAst::Until {
                cond,
                block,
                end_line,
            });
        }
        TokenKind::Kw(Kw::Wait) => parse_wait(&mut c, errors),
        TokenKind::Kw(Kw::Start) => {
            if c.eat_kw(Kw::Dilation) {
                Some(CommandAst::StartDilation)
            } else {
                let ec = parse_ec_ref(&mut c, errors)?;
                Some(CommandAst::StartEc { ec })
            }
        }
        TokenKind::Kw(Kw::Unlock) => {
            let nowait = c.eat_kw(Kw::Nowait);
            if c.eat_kw(Kw::Dilation) {
                Some(CommandAst::UnlockDilation { nowait })
            } else {
                let ec = parse_ec_ref(&mut c, errors)?;
                Some(CommandAst::UnlockEc { ec, nowait })
            }
        }
        TokenKind::Kw(Kw::Studies) => parse_studies(&mut c, errors),
        TokenKind::Kw(Kw::StoreGameTime) => {
            let action = if c.eat_kw(Kw::On) {
                StoreGameTimeAction::On
            } else if c.eat_kw(Kw::Off) {
                StoreGameTimeAction::Off
            } else if c.eat_kw(Kw::Use) {
                StoreGameTimeAction::Use
            } else {
                errors.push(c.incomplete());
                return None;
            };
            Some(CommandAst::StoreGameTime { action })
        }
        TokenKind::Kw(Kw::Stop) => Some(CommandAst::Stop),
        TokenKind::Prestige(layer) => {
            let layer = *layer;
            let nowait = c.eat_kw(Kw::Nowait);
            let respec = c.eat_kw(Kw::Respec);
            Some(CommandAst::Prestige {
                layer,
                nowait,
                respec,
            })
        }
        // Lines starting with an identifier / number / operator are
        // `badCommand` in the original grammar.
        TokenKind::Identifier | TokenKind::Number | TokenKind::Op(_) => {
            errors.push(AutomatorError {
                line: line_no,
                info: format!("Unrecognized command \"{}\"", first.image),
                tip: "Check to make sure you have typed in the command name correctly"
                    .to_string(),
            });
            return None;
        }
        // Any other token can't start a command (NoViableAlt).
        _ => {
            let err = c.unexpected(first);
            errors.push(err);
            return None;
        }
    };

    let parsed = parsed?;
    c.expect_end(errors);
    Some(parsed)
}

/// `{`, end of line, then the inner lines up to `}`. The cursor must be
/// sitting on the `{`.
fn parse_block_body(
    c: &mut Cursor,
    lines: &mut LineIter,
    errors: &mut Vec<AutomatorError>,
) -> Option<(Vec<ParsedCommand>, u32)> {
    match c.next() {
        Some(t) if t.kind == TokenKind::LCurly => {}
        Some(t) => {
            let err = c.unexpected(t);
            errors.push(err);
            return None;
        }
        None => {
            // A block command without `{` (`checkBlock`'s missing-{ case).
            errors.push(AutomatorError {
                line: c.line,
                info: "Missing opening {".to_string(),
                tip: "This line has an extra } closing a loop which does not exist, \
                      remove the }"
                    .to_string(),
            });
            return None;
        }
    }
    c.expect_end(errors);
    let (block, close_line) = parse_block(lines, errors, Some(c.line));
    // A block running to EOF recorded the missing-} error; the end line is
    // then moot (errors block compilation) — fall back to the opening line.
    Some((block, close_line.unwrap_or(c.line)))
}

/// `auto <prestige> (on|off|duration|x highest|amount currency)`.
fn parse_auto(c: &mut Cursor, errors: &mut Vec<AutomatorError>) -> Option<CommandAst> {
    let layer = match c.next() {
        Some(Token {
            kind: TokenKind::Prestige(layer),
            ..
        }) => *layer,
        Some(t) => {
            let err = c.unexpected(t);
            errors.push(err);
            return None;
        }
        None => {
            errors.push(c.incomplete());
            return None;
        }
    };
    let arg = if c.eat_kw(Kw::On) {
        AutoArgAst::On
    } else if c.eat_kw(Kw::Off) {
        AutoArgAst::Off
    } else {
        match c.next() {
            Some(num) if num.kind == TokenKind::Number => match c.next() {
                Some(Token {
                    kind: TokenKind::TimeUnit(scale),
                    image,
                    ..
                }) => AutoArgAst::Duration {
                    ms: num.image.parse::<f64>().unwrap_or(f64::NAN) * scale,
                    num: num.image.clone(),
                    unit: image.clone(),
                },
                Some(Token {
                    kind: TokenKind::Kw(Kw::XHighest),
                    ..
                }) => AutoArgAst::XHighest(num.image.clone()),
                Some(Token {
                    kind: TokenKind::Currency(currency),
                    image,
                    ..
                }) => AutoArgAst::Amount {
                    num: num.image.clone(),
                    currency: *currency,
                    currency_image: image.clone(),
                },
                Some(t) => {
                    let err = c.unexpected(t);
                    errors.push(err);
                    return None;
                }
                None => {
                    errors.push(c.incomplete());
                    return None;
                }
            },
            Some(t) => {
                let err = c.unexpected(t);
                errors.push(err);
                return None;
            }
            None => {
                errors.push(c.incomplete());
                return None;
            }
        }
    };
    Some(CommandAst::Auto { layer, arg })
}

/// `pause (<number> <unit> | <constant>)`.
fn parse_pause(c: &mut Cursor, errors: &mut Vec<AutomatorError>) -> Option<CommandAst> {
    match c.next() {
        Some(num) if num.kind == TokenKind::Number => match c.next() {
            Some(Token {
                kind: TokenKind::TimeUnit(scale),
                image,
                ..
            }) => Some(CommandAst::Pause {
                arg: PauseArgAst::Duration {
                    ms: num.image.parse::<f64>().unwrap_or(f64::NAN) * scale,
                    num: num.image.clone(),
                    unit: image.clone(),
                },
            }),
            _ => {
                errors.push(AutomatorError {
                    line: c.line,
                    info: "Missing time unit".to_string(),
                    tip: "Provide a unit of time (eg. seconds or minutes)".to_string(),
                });
                None
            }
        },
        Some(t) if t.kind == TokenKind::Identifier => Some(CommandAst::Pause {
            arg: PauseArgAst::Const(t.image.clone()),
        }),
        Some(t) => {
            let err = c.unexpected(t);
            errors.push(err);
            None
        }
        None => {
            errors.push(c.incomplete());
            None
        }
    }
}

/// `wait (<comparison> | <prestige event> | black hole <off|bh1|bh2>)`.
fn parse_wait(c: &mut Cursor, errors: &mut Vec<AutomatorError>) -> Option<CommandAst> {
    match c.peek() {
        Some(Token {
            kind: TokenKind::Kw(Kw::BlackHole),
            ..
        }) => {
            c.pos += 1;
            match c.next() {
                Some(Token {
                    kind: TokenKind::Kw(Kw::Off),
                    ..
                }) => Some(CommandAst::Wait {
                    cond: WaitCondAst::BlackHole { off: true, hole: 0 },
                }),
                Some(Token {
                    kind: TokenKind::BlackHoleLit(hole),
                    ..
                }) => Some(CommandAst::Wait {
                    cond: WaitCondAst::BlackHole {
                        off: false,
                        hole: *hole,
                    },
                }),
                Some(t) => {
                    let err = c.unexpected(t);
                    errors.push(err);
                    None
                }
                None => {
                    errors.push(c.incomplete());
                    None
                }
            }
        }
        Some(Token {
            kind: TokenKind::Prestige(layer),
            ..
        }) if c.tokens.len() == c.pos + 1 => {
            let layer = *layer;
            c.pos += 1;
            Some(CommandAst::Wait {
                cond: WaitCondAst::Prestige(layer),
            })
        }
        _ => {
            let cmp = parse_comparison(c, errors)?;
            Some(CommandAst::Wait {
                cond: WaitCondAst::Cmp(cmp),
            })
        }
    }
}

/// `studies [nowait] (purchase <list|const> | load <id N|name X> | respec)`.
fn parse_studies(
    c: &mut Cursor,
    errors: &mut Vec<AutomatorError>,
) -> Option<CommandAst> {
    let nowait = c.eat_kw(Kw::Nowait);
    if c.eat_kw(Kw::Respec) {
        if nowait {
            // `studies nowait respec` isn't a rule in the original grammar.
            errors.push(AutomatorError {
                line: c.line,
                info: "Unexpected input respec".to_string(),
                tip: "Remove respec".to_string(),
            });
            return None;
        }
        return Some(CommandAst::StudiesRespec);
    }
    if c.eat_kw(Kw::Purchase) {
        // A lone identifier is a study-string constant.
        if matches!(c.peek(), Some(t) if t.kind == TokenKind::Identifier)
            && c.tokens.len() == c.pos + 1
        {
            let name = c.next().unwrap().image.clone();
            return Some(CommandAst::StudiesBuy {
                arg: StudiesArgAst::Const(name),
                nowait,
            });
        }
        let list = parse_study_list(c, errors)?;
        return Some(CommandAst::StudiesBuy {
            arg: StudiesArgAst::List(list),
            nowait,
        });
    }
    if c.eat_kw(Kw::Load) {
        return match c.next() {
            Some(Token {
                kind: TokenKind::PresetId(digit),
                ..
            }) => Some(CommandAst::StudiesLoad {
                preset: PresetRefAst::Id(*digit),
                nowait,
            }),
            Some(Token {
                kind: TokenKind::PresetName(name),
                ..
            }) => Some(CommandAst::StudiesLoad {
                preset: PresetRefAst::Name(name.clone()),
                nowait,
            }),
            Some(t) => {
                let err = c.unexpected(t);
                errors.push(err);
                None
            }
            None => {
                errors.push(c.incomplete());
                None
            }
        };
    }
    match c.peek() {
        Some(t) => {
            let err = c.unexpected(t);
            errors.push(err);
            None
        }
        None => {
            errors.push(c.incomplete());
            None
        }
    }
}

/// The study list: entries (id / range / path) separated by optional commas,
/// with an optional `|N[!]` suffix.
fn parse_study_list(
    c: &mut Cursor,
    errors: &mut Vec<AutomatorError>,
) -> Option<StudyListAst> {
    let mut entries = Vec::new();
    let mut image = String::new();
    let start_pos = c.pos;
    loop {
        match c.peek() {
            Some(t) if t.kind == TokenKind::Number => {
                let first = c.next().unwrap();
                if matches!(c.peek(), Some(d) if d.kind == TokenKind::Dash) {
                    c.pos += 1;
                    match c.next() {
                        Some(last) if last.kind == TokenKind::Number => {
                            entries.push(StudyListEntryAst::Range(
                                first.image.clone(),
                                last.image.clone(),
                            ));
                        }
                        _ => {
                            errors.push(AutomatorError {
                                line: c.line,
                                info: "Missing Time Study number in range".to_string(),
                                tip: "Provide starting and ending IDs for Time Study \
                                      number ranges"
                                    .to_string(),
                            });
                            return None;
                        }
                    }
                } else {
                    entries.push(StudyListEntryAst::Id(first.image.clone()));
                }
            }
            Some(t) if matches!(t.kind, TokenKind::StudyPath(_)) => {
                let TokenKind::StudyPath(p) = t.kind else {
                    unreachable!()
                };
                entries.push(StudyListEntryAst::Path(p));
                c.pos += 1;
            }
            Some(t) if t.kind == TokenKind::Prestige(PrestigeLayer::Infinity) => {
                entries.push(StudyListEntryAst::InfinityPath);
                c.pos += 1;
            }
            _ => break,
        }
        // Optional comma between entries.
        if matches!(c.peek(), Some(t) if t.kind == TokenKind::Comma) {
            c.pos += 1;
        }
    }
    if entries.is_empty() {
        errors.push(c.incomplete());
        return None;
    }

    let mut ec = None;
    let mut start_ec = false;
    if matches!(c.peek(), Some(t) if t.kind == TokenKind::Pipe) {
        c.pos += 1;
        match c.next() {
            Some(t) if t.kind == TokenKind::Number => ec = Some(t.image.clone()),
            _ => {
                errors.push(AutomatorError {
                    line: c.line,
                    info: "Missing Eternity Challenge number".to_string(),
                    tip: "Specify which Eternity Challenge is being referred to"
                        .to_string(),
                });
                return None;
            }
        }
        if matches!(c.peek(), Some(t) if t.kind == TokenKind::Exclamation) {
            c.pos += 1;
            start_ec = true;
        }
    }

    // The list as typed (used for display; rebuilt from token images).
    for t in &c.tokens[start_pos..c.pos] {
        if !image.is_empty()
            && !matches!(
                t.kind,
                TokenKind::Comma
                    | TokenKind::Dash
                    | TokenKind::Pipe
                    | TokenKind::Exclamation
            )
            && !image.ends_with(['-', ',', '|'])
        {
            image.push(' ');
        }
        image.push_str(&t.image);
    }

    Some(StudyListAst {
        entries,
        ec,
        start_ec,
        image,
    })
}

/// `ec <N>` or `ec<N>` (the `eternityChallenge` rule).
fn parse_ec_ref(c: &mut Cursor, errors: &mut Vec<AutomatorError>) -> Option<u32> {
    match c.next() {
        Some(Token {
            kind: TokenKind::EcLiteral(n),
            ..
        }) => Some(*n),
        Some(Token {
            kind: TokenKind::Kw(Kw::Ec),
            ..
        }) => match c.next() {
            Some(t) if t.kind == TokenKind::Number => {
                t.image.parse::<u32>().ok().or_else(|| {
                    errors.push(AutomatorError {
                        line: c.line,
                        info: "Missing Eternity Challenge number".to_string(),
                        tip: "Specify which Eternity Challenge is being referred to"
                            .to_string(),
                    });
                    None
                })
            }
            _ => {
                errors.push(AutomatorError {
                    line: c.line,
                    info: "Missing Eternity Challenge number".to_string(),
                    tip: "Specify which Eternity Challenge is being referred to"
                        .to_string(),
                });
                None
            }
        },
        Some(t) => {
            let err = c.unexpected(t);
            errors.push(err);
            None
        }
        None => {
            errors.push(c.incomplete());
            None
        }
    }
}

/// `comparison`: value op value.
fn parse_comparison(
    c: &mut Cursor,
    errors: &mut Vec<AutomatorError>,
) -> Option<ComparisonAst> {
    let left = parse_compare_value(c, errors)?;
    let op = match c.next() {
        Some(Token {
            kind: TokenKind::Op(op),
            ..
        }) => *op,
        _ => {
            errors.push(AutomatorError {
                line: c.line,
                info: "Missing comparison operator (<, >, <=, >=)".to_string(),
                tip: "Insert the appropriate comparison operator".to_string(),
            });
            return None;
        }
    };
    let right = match parse_compare_value(c, errors) {
        Some(v) => v,
        None => {
            // Overwrite the generic error with the original's message.
            errors.pop();
            errors.push(AutomatorError {
                line: c.line,
                info: "Missing value for comparison".to_string(),
                tip: "Ensure that the comparison has two values".to_string(),
            });
            return None;
        }
    };
    Some(ComparisonAst { left, op, right })
}

fn parse_compare_value(
    c: &mut Cursor,
    errors: &mut Vec<AutomatorError>,
) -> Option<CmpValueAst> {
    match c.next() {
        Some(Token {
            kind: TokenKind::Number,
            image,
            ..
        }) => Some(CmpValueAst::Number(image.clone())),
        Some(Token {
            kind: TokenKind::Currency(cur),
            image,
            ..
        }) => Some(CmpValueAst::Currency(*cur, image.clone())),
        Some(Token {
            kind: TokenKind::Identifier,
            image,
            ..
        }) => Some(CmpValueAst::Const(image.clone())),
        Some(t) => {
            let err = c.unexpected(t);
            errors.push(err);
            None
        }
        None => {
            errors.push(c.incomplete());
            None
        }
    }
}
