// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lazy descent into braced bodies and command substitutions.
//!
//! At its own level a `{…}` body or `[…]` command substitution is a
//! single `Str` / `Cmd` token whose `raw` owns the full span —
//! delimiters included.  *Descending* re-lexes its inner text as a child
//! concrete syntax tree, anchored **one byte past the opener**, so the
//! child owns the delimiter-excluding interior with absolute positions
//! matching where that interior sits in the document — the "a delimited
//! region is a node owning its full span with delimiter-excluding
//! children" shape, realised for nested bodies.
//!
//! Descent is lazy (built only when a caller asks) and flags an
//! unterminated region as *recovered* (`terminated == false`), the
//! lossless representation of malformed input.  Because both this module
//! and the segmenter tokenise the same inner text at the same anchor, the
//! descended token stream is identical to a direct re-lex by construction.
//!
//! **Dialect / offsets.**  Like [`super::build::build_document`], the
//! child green tree is built in local-offset space and the red
//! [`SyntaxTree`] supplies the absolute anchoring; the dialect
//! [`LexerConfig`] carries over (the `_and_config` plumbing).  No
//! `insidequote` handling is threaded — `descend_command` never
//! passes such a flag, and no production caller descends a quoted region.

use tcl_lexer::{LexerConfig, SourceMap, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use super::build::build_document;
use super::green::GreenNode;
use super::red::{SyntaxNode, SyntaxTree, build_line_starts};

/// Whether `token`'s closing delimiter is present in the source `sm` maps.
///
/// The closer sits `content_offset` bytes past the opener, plus the
/// `token_text` bytes of inner content — not a hardcoded one byte:
/// `content_offset` is normally `1` for a real `[…]` / `{…}` token (it
/// skips the opener), but a synthetic token built by error recovery
/// (e.g. `Analyser::recover_stray_close_bracket`'s virtual `Cmd` token,
/// which has no real opener in the source at all) sets it to `0`.
/// Deriving the closer position from the content *length* (not the
/// token's end offset) is what classifies an empty `{}` / `[]`
/// correctly — the lexer reports `end` *on* the closer there, which
/// `end + 1` would overshoot.
fn terminated(sm: &SourceMap<'_>, token: Token) -> bool {
    let closer = match token.kind {
        TokenType::Str => b'}',
        TokenType::Cmd => b']',
        _ => return false,
    };
    let idx =
        token.span.start() as usize + token.content_offset as usize + sm.token_text(token).len();
    sm.source().as_bytes().get(idx) == Some(&closer)
}

/// The child CST of a descended body / command substitution.
#[derive(Debug, Clone)]
pub struct Descended {
    tree: SyntaxTree,
    terminated: bool,
}

impl Descended {
    /// The inner region's concrete syntax tree, anchored at its absolute
    /// position.
    #[must_use]
    pub fn tree(&self) -> &SyntaxTree {
        &self.tree
    }

    /// The red root of the inner tree.
    #[must_use]
    pub fn root(&self) -> SyntaxNode<'_> {
        self.tree.root()
    }

    /// The green document of the inner tree.
    #[must_use]
    pub fn document(&self) -> &GreenNode {
        self.tree.green()
    }

    /// Whether the closing delimiter was present in the parent region.
    /// `false` marks a *recovered* (unterminated) `{…}` / `[…]`.
    #[must_use]
    pub fn is_terminated(&self) -> bool {
        self.terminated
    }
}

/// Descend an absolutely-positioned `Str` / `Cmd` (or recovery) token.
///
/// For an opaque `{…}` / `[…]` token the child is anchored
/// `content_offset` bytes past the opener — normally one byte, but a
/// synthetic recovery token may set `content_offset` to `0` when its
/// span already starts at the content (see `terminated`'s doc); any
/// other token (an `Esc` recovery body) is anchored at its own position
/// and is always *recovered*.  `sm` maps the region containing `token`
/// (used to resolve its position, inner text, and whether the delimiter
/// is terminated); `config` is the dialect the inner text is re-lexed
/// under.
#[must_use]
pub fn descend_token(sm: &SourceMap<'_>, token: Token, config: LexerConfig) -> Descended {
    let (start, _end) = sm.range_positions(token.span);
    let (b_off, b_line, b_col, term) = if matches!(token.kind, TokenType::Str | TokenType::Cmd) {
        (
            start.offset + u32::from(token.content_offset),
            start.line,
            start.character.get() + u32::from(token.content_offset),
            terminated(sm, token),
        )
    } else {
        (start.offset, start.line, start.character.get(), false)
    };
    let inner = sm.token_text(token);
    let (document, _warnings) = build_document(inner, config);
    let tree = SyntaxTree::with_parts(
        document,
        b_off,
        b_line,
        b_col,
        inner.to_string(),
        build_line_starts(inner),
    );
    Descended {
        tree,
        terminated: term,
    }
}

/// A registry-resolved `Body` argument of a command, descended.
#[derive(Debug, Clone)]
pub struct CommandBody {
    /// The real argument index (into `args` / `arg_tokens`).
    pub index: usize,
    /// The body word's inner text.
    pub text: String,
    /// The owning `Str` token.
    pub token: Token,
    /// The body's child CST.
    pub descended: Descended,
}

/// Descend each [`ArgRole::Body`] argument of a command as a child CST.
///
/// The single registry-aware descent entry point — the analogue of
/// `green_tree.descend_command`, resolving body arguments through the
/// registry's [`CommandRegistry::arg_indices_for_role`] (the Rust
/// `iter_body_arguments`) so the set of descended bodies matches exactly.
/// A body that is not a non-empty `Str` word is skipped (it is data, not
/// a script the tree should re-lex).  [`ArgRole::Expr`] arguments are
/// deliberately not routed here — they are handled by the expression
/// lexer.
///
/// `args` and `arg_tokens` are the command's arguments (excluding the
/// command name), parallel and 0-indexed; `sm` maps the region the tokens
/// belong to.
#[must_use]
pub fn descend_command(
    registry: &CommandRegistry,
    sm: &SourceMap<'_>,
    cmd_name: &str,
    args: &[&str],
    arg_tokens: &[Token],
    config: LexerConfig,
) -> Vec<CommandBody> {
    let mut bodies = Vec::new();
    for index in registry.arg_indices_for_role(cmd_name, args, ArgRole::Body) {
        let (Some(&token), Some(&text)) = (arg_tokens.get(index), args.get(index)) else {
            continue;
        };
        if token.kind != TokenType::Str || text.trim().is_empty() {
            continue;
        }
        bodies.push(CommandBody {
            index,
            text: text.to_string(),
            token,
            descended: descend_token(sm, token, config),
        });
    }
    bodies
}

#[cfg(test)]
mod tests {
    use super::super::green::SyntaxKind;
    use super::super::red::SyntaxElement;
    use super::*;
    use tcl_lexer::Lexer;

    /// Lex `src` and return the first token of `kind`.
    fn first_of(src: &str, kind: TokenType) -> Token {
        let toks = Lexer::new(src).tokenise_all().unwrap();
        *toks
            .iter()
            .find(|t| t.kind == kind)
            .expect("a token of kind")
    }

    #[test]
    fn descend_braced_body_is_terminated_and_lossless() {
        // `proc f {} {set x 1}` — descend the final body `{set x 1}`.
        let src = "proc f {} {set x 1}";
        let sm = SourceMap::new(src);
        // The body STR is the last STR token.
        let toks = Lexer::new(src).tokenise_all().unwrap();
        let body = *toks.iter().rfind(|t| t.kind == TokenType::Str).unwrap();
        let d = descend_token(&sm, body, LexerConfig::default());
        assert!(d.is_terminated());
        // The child tree's text is the inner body, anchored absolutely.
        assert_eq!(d.tree().text(), "set x 1");
        // Its sole command's first token sits at the inner start (one byte
        // past the opening `{` at offset 10 → 11).
        let cmd = match d.root().children().next().unwrap() {
            SyntaxElement::Node(n) => n,
            SyntaxElement::Token(_) => panic!("command node"),
        };
        assert_eq!(cmd.kind(), SyntaxKind::Command);
        assert_eq!(cmd.tokens()[0].start_position().offset, 11);
    }

    #[test]
    fn descend_command_substitution() {
        // `[expr 1]` — a CMD token; descend its inner `expr 1`.
        let src = "set x [expr 1]";
        let sm = SourceMap::new(src);
        let cmd_tok = first_of(src, TokenType::Cmd);
        let d = descend_token(&sm, cmd_tok, LexerConfig::default());
        assert!(d.is_terminated());
        assert_eq!(d.tree().text(), "expr 1");
        // Inner `expr` starts one past the `[` (at offset 6 → 7).
        let inner_cmd = match d.root().children().next().unwrap() {
            SyntaxElement::Node(n) => n,
            SyntaxElement::Token(_) => panic!("command node"),
        };
        assert_eq!(inner_cmd.tokens()[0].start_position().offset, 7);
    }

    #[test]
    fn descend_empty_brace_is_terminated() {
        // `{}` — the length-based `_terminated` classifies the empty body
        // correctly (the closer is at start+1, not overshot).
        let src = "proc f {} body";
        let sm = SourceMap::new(src);
        let empty = first_of(src, TokenType::Str);
        let d = descend_token(&sm, empty, LexerConfig::default());
        assert!(d.is_terminated());
        assert_eq!(d.tree().text(), "");
    }

    #[test]
    fn descend_unterminated_brace_is_recovered() {
        // `{abc` runs to EOF with no closer — a recovered child that still
        // carries its inner tokens.
        let src = "{abc";
        let sm = SourceMap::new(src);
        let tok = first_of(src, TokenType::Str);
        let d = descend_token(&sm, tok, LexerConfig::default());
        assert!(!d.is_terminated());
        assert_eq!(d.tree().text(), "abc");
    }

    #[test]
    fn multi_level_descent_keeps_absolute_positions() {
        // A command substitution inside a descended body: descend the
        // body, then a `[…]` token within it; the inner positions stay
        // absolute relative to the original document.
        let src = "proc f {} {set x [incr y]}";
        let sm = SourceMap::new(src);
        let body = *sm_strs(&sm, src).last().unwrap();
        let body_d = descend_token(&sm, body, LexerConfig::default());
        assert_eq!(body_d.tree().text(), "set x [incr y]");
        // Find the CMD token in the descended body and descend it.
        let inner_tokens: Vec<Token> = body_d
            .root()
            .tokens()
            .iter()
            .map(super::super::red::SyntaxToken::to_token)
            .collect();
        let cmd_tok = *inner_tokens
            .iter()
            .find(|t| t.kind == TokenType::Cmd)
            .expect("inner command substitution");
        let sub = descend_token(&sm, cmd_tok, LexerConfig::default());
        assert_eq!(sub.tree().text(), "incr y");
        // `incr` sits one past the `[` (the `[` is at offset 17 → inner 18).
        let inner_cmd = match sub.root().children().next().unwrap() {
            SyntaxElement::Node(n) => n,
            SyntaxElement::Token(_) => panic!("command node"),
        };
        assert_eq!(inner_cmd.tokens()[0].start_position().offset, 18);
        assert_eq!(&src[18..22], "incr");
    }

    /// All STR tokens of `src`, in order.
    fn sm_strs(_sm: &SourceMap<'_>, src: &str) -> Vec<Token> {
        Lexer::new(src)
            .tokenise_all()
            .unwrap()
            .into_iter()
            .filter(|t| t.kind == TokenType::Str)
            .collect()
    }

    #[test]
    fn descend_command_resolves_body_args_only() {
        // `for {set i 0} {$i < 10} {incr i} {puts $i}` — only the loop
        // body (arg index 3) is a Body role; the init/next bodies are also
        // Body for `for`, while the condition is Expr.  Verify the EXPR
        // condition is NOT descended and the braced bodies are.
        let registry = CommandRegistry::build_default();
        let src = "for {set i 0} {$i < 10} {incr i} {puts $i}";
        let sm = SourceMap::new(src);
        // Segment to get cmd_name / args / arg_tokens parallel arrays.
        let segs = crate::segmenter::segment_commands(src);
        let cmd = &segs[0];
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
        let bodies = descend_command(
            &registry,
            &sm,
            cmd.name(),
            &args,
            cmd.arg_tokens(),
            LexerConfig::default(),
        );
        // `for` has Body args at init (0), next (2), body (3); the
        // condition (1) is Expr and must not appear.
        let indices: Vec<usize> = bodies.iter().map(|b| b.index).collect();
        assert!(indices.contains(&3), "loop body must be descended");
        assert!(
            !indices.contains(&1),
            "the Expr condition must NOT be descended"
        );
        // The descended body re-lexes to its inner script.
        let loop_body = bodies.iter().find(|b| b.index == 3).unwrap();
        assert!(loop_body.descended.is_terminated());
        assert_eq!(loop_body.descended.tree().text(), "puts $i");
    }
}
