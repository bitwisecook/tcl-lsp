//! Command segmentation for Tcl token streams.
//!
//! Splits a flat token stream into per-command structures at EOL
//! boundaries. Both the analyser and lowerer consume these structures
//! instead of running their own token-iteration loops.
//!
//! Ports `core/parsing/command_segmenter.py`.

use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};

/// A single Tcl command parsed from the token stream.
#[derive(Debug, Clone)]
pub struct SegmentedCommand {
    /// Byte span covering the whole command.
    pub span: Span,
    /// Per-word representative tokens (one per argv entry).
    pub argv: Vec<Token>,
    /// Per-word reconstructed text.
    pub texts: Vec<String>,
    /// Whether each word is a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Token>,
    /// Whether the command is incomplete (unclosed delimiter).
    pub is_partial: bool,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
}

impl SegmentedCommand {
    /// Command name (first word).
    #[must_use]
    pub fn name(&self) -> &str {
        self.texts.first().map_or("", String::as_str)
    }

    /// Arguments (words after the command name).
    #[must_use]
    pub fn args(&self) -> &[String] {
        if self.texts.len() > 1 {
            &self.texts[1..]
        } else {
            &[]
        }
    }

    /// Per-arg representative tokens.
    #[must_use]
    pub fn arg_tokens(&self) -> &[Token] {
        if self.argv.len() > 1 {
            &self.argv[1..]
        } else {
            &[]
        }
    }

    /// Per-arg single-token flags.
    #[must_use]
    pub fn arg_single_token(&self) -> &[bool] {
        if self.single_token_word.len() > 1 {
            &self.single_token_word[1..]
        } else {
            &[]
        }
    }

    /// Return a copy of `self` with every span shifted by
    /// `base_offset`. Used by
    /// [`segment_commands_with_offset`] to relocate a body
    /// script's spans into the outer source buffer's offset
    /// space.
    #[must_use]
    pub fn shifted_by(mut self, base_offset: u32) -> Self {
        self.span = shift_span(self.span, base_offset);
        for tok in &mut self.argv {
            tok.span = shift_span(tok.span, base_offset);
        }
        for tok in &mut self.all_tokens {
            tok.span = shift_span(tok.span, base_offset);
        }
        self
    }
}

fn shift_span(span: Span, by: u32) -> Span {
    Span::new(span.start() + by, span.end() + by)
}

/// Return the source-level text fragment for a single token.
///
/// Variables are prefixed with `$` and command substitutions are
/// wrapped in `[...]` so that the result mirrors what the user wrote.
#[must_use]
pub fn word_piece(sm: &SourceMap<'_>, tok: Token) -> String {
    let text = sm.token_text(tok);
    match tok.kind {
        TokenType::Var => {
            if text.contains('}') {
                format!("${text}")
            } else {
                format!("${{{text}}}")
            }
        }
        TokenType::Cmd => format!("[{text}]"),
        _ => text.to_owned(),
    }
}

/// Compute the span covering all tokens.
fn span_from_tokens(tokens: &[Token]) -> Span {
    if tokens.is_empty() {
        return Span::new(0, 0);
    }
    let start = tokens.first().unwrap().span.start();
    let end = tokens.last().unwrap().span.end();
    Span::new(start, end)
}

/// Segment a token stream into per-command structures at EOL boundaries.
///
/// The core segmentation loop. Error recovery (scanning for known
/// commands after unclosed delimiters) is not ported here — it can
/// be added in a follow-up chunk when the Python fallback is removed.
#[must_use]
pub fn segment_commands(source: &str) -> Vec<SegmentedCommand> {
    segment_commands_with_offset(source, 0)
}

/// Segment with a base byte offset (for body scripts inside braces).
///
/// The lexer tokenises `source` starting at local offset `0`.
/// Segmentation runs in local-offset space so the `SourceMap`
/// can slice text via [`Token::span`]; immediately before
/// returning, every `SegmentedCommand` has its spans relocated
/// by `base_offset` so downstream IR / optimiser / def-use
/// consumers see absolute offsets into the outer source buffer.
#[must_use]
pub fn segment_commands_with_offset(source: &str, base_offset: u32) -> Vec<SegmentedCommand> {
    let commands = segment_commands_local(source);
    if base_offset == 0 {
        return commands;
    }
    commands
        .into_iter()
        .map(|c| c.shifted_by(base_offset))
        .collect()
}

fn segment_commands_local(source: &str) -> Vec<SegmentedCommand> {
    // word_piece only needs token_text (source indexing), no base offset.
    let sm = SourceMap::new(source);
    let config = LexerConfig::default();
    let lexer_sm = SourceMap::new(source);
    let lexer = Lexer::with_source_map(lexer_sm, config);
    let Ok(tokens) = lexer.tokenise_all() else {
        return Vec::new();
    };

    let mut commands = Vec::new();
    let mut argv: Vec<Token> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let mut single: Vec<bool> = Vec::new();
    let mut expand: Vec<bool> = Vec::new();
    let mut all_tokens: Vec<Token> = Vec::new();
    let mut prev_type = TokenType::Eol;
    let mut next_expand = false;
    let mut has_expand = false;

    for &tok in &tokens {
        match tok.kind {
            TokenType::Comment => continue,

            TokenType::Sep => {
                prev_type = tok.kind;
                continue;
            }

            // Backslash-newline continuation between words is whitespace.
            TokenType::Esc if sm.token_text(tok) == "\\\n" => {
                prev_type = TokenType::Sep;
                continue;
            }

            TokenType::Expand => {
                all_tokens.push(tok);
                next_expand = true;
                has_expand = true;
                prev_type = TokenType::Sep;
                continue;
            }

            TokenType::Eol | TokenType::Eof => {
                if !argv.is_empty() {
                    commands.push(SegmentedCommand {
                        span: span_from_tokens(&all_tokens),
                        argv: std::mem::take(&mut argv),
                        texts: std::mem::take(&mut texts),
                        single_token_word: std::mem::take(&mut single),
                        all_tokens: std::mem::take(&mut all_tokens),
                        is_partial: false,
                        expand_word: if has_expand {
                            Some(std::mem::take(&mut expand))
                        } else {
                            expand.clear();
                            None
                        },
                    });
                }
                has_expand = false;
                next_expand = false;
                prev_type = tok.kind;
                continue;
            }

            _ => {}
        }

        all_tokens.push(tok);
        let piece = word_piece(&sm, tok);

        if prev_type == TokenType::Sep || prev_type == TokenType::Eol {
            argv.push(tok);
            texts.push(piece);
            single.push(true);
            expand.push(next_expand);
            next_expand = false;
        } else if let Some(last_text) = texts.last_mut() {
            last_text.push_str(&piece);
            if let Some(s) = single.last_mut() {
                *s = false;
            }
        } else {
            argv.push(tok);
            texts.push(piece);
            single.push(true);
            expand.push(next_expand);
            next_expand = false;
        }

        prev_type = tok.kind;
    }

    // Trailing command without final EOL.
    if !argv.is_empty() {
        commands.push(SegmentedCommand {
            span: span_from_tokens(&all_tokens),
            argv,
            texts,
            single_token_word: single,
            all_tokens,
            is_partial: false,
            expand_word: if has_expand { Some(expand) } else { None },
        });
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source() {
        assert!(segment_commands("").is_empty());
    }

    #[test]
    fn single_command() {
        let cmds = segment_commands("puts hello");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "puts");
        assert_eq!(cmds[0].args(), &["hello"]);
    }

    #[test]
    fn two_commands() {
        let cmds = segment_commands("set x 1\nputs $x");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name(), "set");
        assert_eq!(cmds[0].texts.len(), 3); // set, x, 1
        assert_eq!(cmds[1].name(), "puts");
    }

    #[test]
    fn semicolon_separator() {
        let cmds = segment_commands("set x 1; set y 2");
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn variable_word() {
        let cmds = segment_commands("puts $name");
        assert_eq!(cmds.len(), 1);
        // Variable should be wrapped as ${name}.
        assert_eq!(cmds[0].texts[1], "${name}");
    }

    #[test]
    fn command_substitution() {
        let cmds = segment_commands("puts [expr 1+2]");
        assert_eq!(cmds.len(), 1);
        // Command substitution wrapped in brackets.
        assert!(cmds[0].texts[1].starts_with('['));
        assert!(cmds[0].texts[1].ends_with(']'));
    }

    #[test]
    fn braced_string() {
        let cmds = segment_commands("if {$x > 0} {puts yes}");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "if");
        assert_eq!(cmds[0].texts.len(), 3);
    }

    #[test]
    fn single_token_tracking() {
        let cmds = segment_commands("set x {hello world}");
        assert_eq!(cmds.len(), 1);
        // "set" is single, "x" is single, "{hello world}" is single.
        assert!(cmds[0].single_token_word.iter().all(|&s| s));
    }

    #[test]
    fn multi_token_word() {
        let cmds = segment_commands("puts $a$b");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].texts.len(), 2); // "puts", "${a}${b}"
        assert!(!cmds[0].single_token_word[1]); // multi-token word
    }

    #[test]
    fn comment_ignored() {
        let cmds = segment_commands("# this is a comment\nputs hello");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "puts");
    }

    #[test]
    fn arg_tokens_and_arg_single() {
        let cmds = segment_commands("set x 1");
        assert_eq!(cmds[0].arg_tokens().len(), 2); // x, 1
        assert_eq!(cmds[0].arg_single_token().len(), 2);
    }

    #[test]
    fn blank_lines_between_commands() {
        let cmds = segment_commands("set x 1\n\nset y 2");
        assert_eq!(cmds.len(), 2);
    }
}

#[cfg(test)]
mod span_absolute_tests {
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    #[test]
    fn proc_body_statement_spans_are_absolute() {
        let src = "proc ::f {} { set x 1; return $x }";
        let r = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &r, false);
        let proc = cu.ir_module.procedures.get("::f").expect("proc");
        let first = proc.body.statements.first().expect("body stmt");
        let span = first.span();
        let text = &src[span.as_range()];
        assert!(
            text.starts_with("set"),
            "expected absolute span pointing at `set`, got {text:?}",
        );
    }
}
