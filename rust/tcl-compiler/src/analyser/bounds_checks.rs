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

//! Loop-termination index-bounds checks (W230-W242).
//!
//! Covers the **loop-termination** family (W240 / W241 / W242 over
//! `while` / `for`, including the `for`-step provably-infinite heuristic)
//! and the **index-bounds** family (W230 over `lindex` / `lrange` /
//! `lreplace`, W231 over `lset`, W232 over `string index` / `range` /
//! `replace`).  W242 (unprovable termination) is always emitted here; its
//! default-off opt-in is a consuming-layer concern.
//!
//! The analysis is intentionally shallow — it inspects the literal text
//! of the condition and body.  A dynamic condition (anything not a
//! constant true/false literal) yields no diagnostic, avoiding false
//! positives.

use tcl_core_types::DiagCode;
use tcl_lexer::{ExprToken, ExprTokenType, Token, TokenType, tokenise_expr_checked_with_grammar};

use crate::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};

use super::types::{Diagnostic, Severity};

/// The comparison operators a simple `for`-condition may use.
const SIMPLE_CMP_OPS: &[&str] = &["<=", ">=", "<", ">", "==", "!=", "eq", "ne"];

/// Whether `name` in command position ends the current loop's straight-line
/// flow — the set consulted by the W241 "provably infinite" check.
///
/// `break` exits the loop and `tailcall` replaces the current procedure's
/// frame (Tcl 8.6+) and never returns to it; neither is a block-terminator, so
/// they are named explicitly here, mirroring the CFG builder's own
/// `is_tailcall_command`. Everything that unwinds the enclosing block/proc —
/// `return` / `error` / `exit` / `throw` — is read from the registry's
/// [`tcl_registry::Traits::TERMINATES_BLOCK`] trait, so a newly-added
/// block-terminating command is recognised automatically instead of needing a
/// second hardcoded list (this is what closes the `throw`/`tailcall` W241
/// false positive: both leave the loop but neither was in the old literal set).
fn is_loop_exit_command(name: &str, registry: Option<&tcl_registry::CommandRegistry>) -> bool {
    let bare = name.trim_start_matches(':');
    if bare == "break" || bare == "tailcall" {
        return true;
    }
    registry
        .and_then(|r| r.get(bare))
        .is_some_and(|spec| spec.traits.contains(tcl_registry::Traits::TERMINATES_BLOCK))
}

/// W240 (constant-false condition → dead body) / W241 (constant-true
/// condition whose body never leaves the loop → provably infinite) for
/// `while` / `for`.  The loop-exit set is registry-driven — see
/// [`is_loop_exit_command`].  `args` / `arg_tokens` exclude the command name.
pub(crate) fn loop_termination_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
    registry: Option<&tcl_registry::CommandRegistry>,
    lexer_config: tcl_lexer::LexerConfig,
    grammar: &tcl_dialect::LexerGrammar,
) -> Vec<Diagnostic> {
    // (init, cond, step, body, cond_tok) — init/step empty for `while`.
    let (init_text, cond_text, step_text, body_text, cond_tok) = match cmd_name {
        "while" if args.len() >= 2 && arg_tokens.len() >= 2 => {
            ("", args[0].as_str(), "", args[1].as_str(), &arg_tokens[0])
        }
        "for" if args.len() >= 4 && arg_tokens.len() >= 4 => (
            args[0].as_str(),
            args[1].as_str(),
            args[2].as_str(),
            args[3].as_str(),
            &arg_tokens[1],
        ),
        _ => return Vec::new(),
    };

    match condition_constant(cond_text) {
        Some(false) => {
            return vec![crate::analyser::types::Diagnostic::new(
                DiagCode::W240,
                cond_tok.span,
                format!("{cmd_name} condition is constant false; body never executes."),
                Severity::Warning,
            )];
        }
        Some(true) if !body_may_exit(body_text, registry, lexer_config) => {
            return vec![crate::analyser::types::Diagnostic::new(
                DiagCode::W241,
                cond_tok.span,
                format!(
                    "{cmd_name} is provably infinite: condition is constant true and the body \
                     never leaves the loop (no break/return/error/exit/throw/tailcall)."
                ),
                Severity::Warning,
            )];
        }
        Some(true) => return Vec::new(),
        None => {}
    }

    // `for {init} {cond} {step} body` provably-infinite counter shape.
    if cmd_name == "for"
        && let Some(reason) = for_is_provably_infinite(
            init_text,
            cond_text,
            step_text,
            body_text,
            registry,
            lexer_config,
            grammar,
        )
    {
        return vec![crate::analyser::types::Diagnostic::new(
            DiagCode::W241,
            cond_tok.span,
            format!("for loop is provably infinite: {reason}"),
            Severity::Warning,
        )];
    }

    // W242 (default-off): a counter variable appears in the condition but
    // neither the step nor the body provably modifies it.  Reported on
    // the condition token, like W240/W241.  The analyser always emits
    // W242; the default-off opt-in is applied by the consuming LSP/config
    // layer.
    if let Some(var) = extract_counter_name(cond_text, grammar)
        && !loop_modifies_var(&var, step_text, body_text, registry, lexer_config)
    {
        return vec![crate::analyser::types::Diagnostic::new(
            DiagCode::W242,
            cond_tok.span,
            format!(
                "{cmd_name} termination cannot be proven: variable '{var}' in the \
                     condition is never modified by the step or body."
            ),
            Severity::Hint,
        )];
    }
    Vec::new()
}

/// Return the scalar name of the first variable referenced by a
/// condition expression, or `None`.
///
/// The condition is tokenised with the expression lexer (CST level) and
/// the first `Variable` token is taken; its leading scalar name is then
/// read off the token text.
///
/// Abstains (`None`) when the condition contains a `[cmd ...]` command
/// substitution: a `Variable` token found *outside* it is not necessarily
/// the loop's actual progress variable, since the substitution's own
/// argument words can reference (and the body can update) a variable this
/// shallow scan never sees — `while {[string length $u] > $rest}` picked
/// `rest`, a threshold the loop never touches, over `u`, which the body
/// visibly shrinks each iteration (issue #1316; the module doc's own
/// "intentionally shallow ... avoiding false positives" philosophy,
/// extended to the one case that slipped through it — corpus example:
/// `tcltest.tcl`'s option-usage word-wrapper).
fn extract_counter_name(cond: &str, grammar: &tcl_dialect::LexerGrammar) -> Option<String> {
    let tokens = tokenise_expr_checked_with_grammar(strip_braces(cond), grammar).0;
    if tokens.iter().any(|t| t.kind == ExprTokenType::Command) {
        return None;
    }
    let var = tokens.iter().find(|t| t.kind == ExprTokenType::Variable)?;
    var_scalar_name(&var.text)
}

/// The leading scalar name of a `Variable` token's text — the `name`
/// portion of `$name`, `${name}`, `$name(idx)`, `$ns::name` (taking the
/// first word run, matching the `\$\{?(\w+)\}?` capture shape).
fn var_scalar_name(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'$') {
        return None;
    }
    let mut i = 1;
    if bytes.get(i) == Some(&b'{') {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && is_word_byte(bytes[i]) {
        i += 1;
    }
    (i > start).then(|| text[start..i].to_string())
}

/// A `\w` byte: ASCII alphanumeric or underscore.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// True when the step expression or the body provably updates `var`.
fn loop_modifies_var(
    var: &str,
    step: &str,
    body: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    lexer_config: tcl_lexer::LexerConfig,
) -> bool {
    if !step.is_empty() {
        if let Some((step_var, _)) = parse_step_incr(step, lexer_config)
            && step_var == var
        {
            return true;
        }
        if body_writes_var(strip_braces(step), var, registry, lexer_config) {
            return true;
        }
    }
    body_writes_var(body, var, registry, lexer_config)
}

/// Prove that a `for {set v INT} {$v OP INT} {incr v INT} body` loop
/// never terminates (no write to `v` elsewhere); returns the reason.
fn for_is_provably_infinite(
    init: &str,
    cond: &str,
    step: &str,
    body: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    lexer_config: tcl_lexer::LexerConfig,
    grammar: &tcl_dialect::LexerGrammar,
) -> Option<String> {
    let (var_c, op, bound) = parse_simple_for_cond(cond, grammar)?;
    let (var_i, start) = parse_init_var_value(init, lexer_config)?;
    let (var_s, delta) = parse_step_incr(step, lexer_config)?;
    if var_c != var_i || var_c != var_s {
        return None;
    }
    if body_writes_var(body, &var_c, registry, lexer_config)
        || body_may_exit(body, registry, lexer_config)
    {
        return None;
    }
    let counter = format!("${var_c}");
    // Step of zero with the condition initially true → infinite.
    if delta == 0 && cond_true_at(&op, start, bound) {
        return Some("step is zero ('incr' with 0) and condition holds on entry".to_string());
    }
    // Wrong-direction step: moving away from the bound.
    if matches!(op.as_str(), "<" | "<=") && cond_true_at(&op, start, bound) && delta < 0 {
        return Some(format!(
            "counter {counter} starts at {start}, moves by {delta} per step, and compares {op} \
             {bound} (never reached)"
        ));
    }
    if matches!(op.as_str(), ">" | ">=") && cond_true_at(&op, start, bound) && delta > 0 {
        return Some(format!(
            "counter {counter} starts at {start}, moves by {delta} per step, and compares {op} \
             {bound} (never reached)"
        ));
    }
    if matches!(op.as_str(), "!=" | "ne") {
        if delta == 0 && start != bound {
            return Some(format!("counter {counter} never changes and !={bound}"));
        }
        if delta != 0 && start != bound {
            let diff = bound - start;
            if diff * delta < 0 {
                return Some(format!(
                    "counter {counter} starts at {start}, moves by {delta} per step, never \
                     reaches {bound}"
                ));
            }
            if diff % delta != 0 {
                return Some(format!(
                    "counter {counter} starts at {start}, moves by {delta} per step, never \
                     exactly equals {bound}"
                ));
            }
        }
    }
    None
}

/// Evaluate a simple comparison at a concrete value.
fn cond_true_at(op: &str, value: i64, bound: i64) -> bool {
    match op {
        "<" => value < bound,
        "<=" => value <= bound,
        ">" => value > bound,
        ">=" => value >= bound,
        "==" | "eq" => value == bound,
        "!=" | "ne" => value != bound,
        _ => false,
    }
}

/// `(var, op, bound)` when `cond` is exactly `$v OP literal` or
/// `literal OP $v` (no compound `&&` / `||` / `?` / `!`).
///
/// The condition is tokenised with the expression lexer.  Exactly one
/// comparison operator must split it into a single-variable side and a
/// signed-integer side; any logical / ternary operator, or a second
/// comparison, disqualifies it as compound.
fn parse_simple_for_cond(
    cond: &str,
    grammar: &tcl_dialect::LexerGrammar,
) -> Option<(String, String, i64)> {
    let all = tokenise_expr_checked_with_grammar(strip_braces(cond), grammar).0;
    let tokens: Vec<&ExprToken> = all
        .iter()
        .filter(|t| !t.kind.is_skipped() && t.kind != ExprTokenType::Eof)
        .collect();
    // Locate the single comparison operator; reject compound conditions.
    let mut split = None;
    for (i, t) in tokens.iter().enumerate() {
        if matches!(t.kind, ExprTokenType::TernaryQ | ExprTokenType::TernaryC) {
            return None;
        }
        if t.kind == ExprTokenType::Operator {
            if matches!(t.text.as_str(), "&&" | "||" | "!") {
                return None;
            }
            if SIMPLE_CMP_OPS.contains(&t.text.as_str()) {
                if split.is_some() {
                    return None; // a second comparison → compound
                }
                split = Some(i);
            }
        }
    }
    let at = split?;
    let (lhs, rhs) = (&tokens[..at], &tokens[at + 1..]);
    let op = tokens[at].text.as_str();
    // `$v OP int`
    if let (Some(v), Some(bound)) = (tokens_as_scalar_var(lhs), tokens_as_int(rhs)) {
        return Some((v, op.to_string(), bound));
    }
    // `int OP $v` — flip the operator so the variable is on the left.
    if let (Some(bound), Some(v)) = (tokens_as_int(lhs), tokens_as_scalar_var(rhs)) {
        return Some((v, flip_comparison(op).to_string(), bound));
    }
    None
}

/// A single `Variable` token slice → its scalar name, but only for a
/// plain `$name` / `${name}` token.  Array (`$a(i)`) and namespace-
/// qualified (`$ns::v`) forms are rejected, accepting only a full match
/// of `\$\{?(\w+)\}?` on the comparison's variable side.
fn tokens_as_scalar_var(tokens: &[&ExprToken]) -> Option<String> {
    match tokens {
        [v] if v.kind == ExprTokenType::Variable => strict_scalar_name(&v.text),
        _ => None,
    }
}

/// The scalar name when `text` is exactly `$name`, `${name}`, `${name`
/// or `$name}` (the `\$\{?(\w+)\}?` fullmatch shape — leading `{` and
/// trailing `}` each independently optional), with nothing trailing.
fn strict_scalar_name(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'$') {
        return None;
    }
    let mut i = 1;
    if bytes.get(i) == Some(&b'{') {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && is_word_byte(bytes[i]) {
        i += 1;
    }
    if i == start {
        return None; // no word
    }
    let name = text[start..i].to_string();
    if bytes.get(i) == Some(&b'}') {
        i += 1;
    }
    (i == bytes.len()).then_some(name) // reject any trailing chars
}

/// A signed-integer token slice — a `Number`, optionally preceded by a
/// unary `-` operator (the expression lexer tokenises `-5` as two
/// tokens).  A leading `+` is *not* accepted, matching the `-?\d+`
/// bound pattern.  Returns the value when the slice is exactly that shape.
fn tokens_as_int(tokens: &[&ExprToken]) -> Option<i64> {
    match tokens {
        [num] if num.kind == ExprTokenType::Number => parse_decimal(&num.text),
        [sign, num]
            if sign.kind == ExprTokenType::Operator
                && sign.text == "-"
                && num.kind == ExprTokenType::Number =>
        {
            parse_decimal(&num.text).map(|value| -value)
        }
        _ => None,
    }
}

/// Parse an unsigned decimal integer literal (`\d+`), rejecting floats
/// and signs (the sign is handled by the caller).
fn parse_decimal(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Flip a comparison operator left-to-right (`<` ↔ `>`, `<=` ↔ `>=`);
/// symmetric operators are unchanged.
fn flip_comparison(op: &str) -> &str {
    match op {
        "<" => ">",
        ">" => "<",
        "<=" => ">=",
        ">=" => "<=",
        other => other,
    }
}

/// `(var, value)` from an init clause `set v INT`.  Parsed via the
/// segmenter: a lone `set` command with a scalar-name word and a
/// signed-integer literal.
fn parse_init_var_value(init: &str, lexer_config: tcl_lexer::LexerConfig) -> Option<(String, i64)> {
    let cmd = sole_command(init, lexer_config)?;
    if cmd.name() != "set" {
        return None;
    }
    let args = cmd.args();
    if args.len() != 2 {
        return None;
    }
    let var = scalar_word(&args[0])?;
    let value = parse_signed_decimal(&args[1])?;
    Some((var, value))
}

/// `(var, delta)` from a step clause `incr v ?INT?`.  Parsed via the
/// segmenter; a missing delta defaults to `1`.
fn parse_step_incr(step: &str, lexer_config: tcl_lexer::LexerConfig) -> Option<(String, i64)> {
    let cmd = sole_command(step, lexer_config)?;
    if cmd.name() != "incr" {
        return None;
    }
    match cmd.args() {
        [v] => Some((scalar_word(v)?, 1)),
        [v, delta] => Some((scalar_word(v)?, parse_signed_decimal(delta)?)),
        _ => None,
    }
}

/// The single command in `fragment` (after stripping an enclosing brace
/// pair), or `None` when it segments to zero or more than one command.
fn sole_command(fragment: &str, lexer_config: tcl_lexer::LexerConfig) -> Option<SegmentedCommand> {
    let mut cmds =
        segment_commands_with_offset_and_config(strip_braces(fragment).trim(), 0, lexer_config);
    match cmds.len() {
        1 => cmds.pop(),
        _ => None,
    }
}

/// A scalar variable name word — non-empty and all `\w` bytes (no array
/// index, namespace qualifier, or substitution).
fn scalar_word(word: &str) -> Option<String> {
    if !word.is_empty() && word.bytes().all(is_word_byte) {
        Some(word.to_string())
    } else {
        None
    }
}

/// Parse a signed decimal integer literal (`-?\d+`); `+`-signs and
/// non-decimal forms are rejected.
fn parse_signed_decimal(word: &str) -> Option<i64> {
    let digits = word.strip_prefix('-').unwrap_or(word);
    parse_decimal(digits).map(|v| if word.starts_with('-') { -v } else { v })
}

/// Does `body` write `var` via `set` / `incr` / `lset` / `append` /
/// `lappend`?  Resolves commands with the segmenter (recursing into
/// braced/quoted word bodies) rather than a flat-text scan, so writes
/// inside string arguments don't count and nested-body writes still do.
///
/// A write counts only in command position (a `\bset\s+var\b` flat regex
/// would also match `set var(i)` array writes and matches inside
/// strings), which keeps W241/W242 counts accurate — full-fidelity
/// parsing rather than text matching.
fn body_writes_var(
    body: &str,
    var: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    lexer_config: tcl_lexer::LexerConfig,
) -> bool {
    any_command_recursive(body, lexer_config, &mut |cmd| {
        writes_first_arg(cmd.name(), registry)
            && cmd.args().first().map(String::as_str) == Some(var)
    })
}

/// Whether `name` writes/modifies the variable named by its first argument
/// (`set` / `incr` / `append` / `lappend` / `lset`) — the registry's
/// `writes_first_arg_variable` query, with the cached default registry as
/// the registry-less fallback (mirroring [`is_loop_exit_command`]'s shape).
fn writes_first_arg(name: &str, registry: Option<&tcl_registry::CommandRegistry>) -> bool {
    registry
        .unwrap_or_else(|| tcl_registry::model::ingress::static_context_for("tcl8.6").commands())
        .writes_first_arg_variable(name.trim_start_matches(':'))
}

/// Walk every command in `script`, recursing into braced / quoted word
/// arguments (which may be nested scripts), and return `true` as soon as
/// `pred` matches.  A shallow-but-structural body scan.
fn any_command_recursive(
    script: &str,
    lexer_config: tcl_lexer::LexerConfig,
    pred: &mut impl FnMut(&SegmentedCommand) -> bool,
) -> bool {
    for cmd in segment_commands_with_offset_and_config(script, 0, lexer_config) {
        if pred(&cmd) {
            return true;
        }
        let args = cmd.args();
        for (i, tok) in cmd.arg_tokens().iter().enumerate() {
            if tok.kind == TokenType::Str
                && let Some(inner) = args.get(i)
                && any_command_recursive(inner, lexer_config, pred)
            {
                return true;
            }
        }
    }
    false
}

/// W230: a constant list literal with a constant out-of-range index
/// (`lindex`) or a provably-empty slice (`lrange` / `lreplace`)
/// silently returns empty / clamps.  `args` / `arg_tokens` exclude the
/// command name.  (W231 `lset` needs const-var tracking and is handled
/// separately.)
pub(crate) fn list_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
    numbers: tcl_dialect::NumberSyntax,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Vec<Diagnostic> {
    if !matches!(cmd_name, "lindex" | "lrange" | "lreplace")
        || args.len() < 2
        || arg_tokens.len() < 2
    {
        return Vec::new();
    }
    let list_tok = &arg_tokens[0];
    if !is_braced_or_esc(list_tok) || has_subst(&args[0], list_tok) {
        return Vec::new();
    }
    // `args[0]` already has its outer `{…}` delimiter stripped by the
    // segmenter, so split the list content directly — a second
    // `strip_braces` here would wrongly peel a single-element list like
    // `{{a b c}}` (segmented to `{a b c}`) down to its three inner words.
    let length = i64::try_from(crate::tcl_expr_eval::split_tcl_list(&args[0], rules).len())
        .unwrap_or(i64::MAX);

    if cmd_name == "lindex" {
        return lindex_diagnostics(args, arg_tokens, length, numbers);
    }

    // lrange / lreplace: a (first, last) pair that resolves to an empty
    // slice.
    if args.len() < 3 || arg_tokens.len() < 3 || (cmd_name == "lrange" && args.len() != 3) {
        return Vec::new();
    }
    let (lo_text, hi_text) = (&args[1], &args[2]);
    let (lo_token, hi_token) = (&arg_tokens[1], &arg_tokens[2]);
    if has_subst(lo_text, lo_token)
        || !is_literal_index(lo_text, numbers)
        || has_subst(hi_text, hi_token)
        || !is_literal_index(hi_text, numbers)
    {
        return Vec::new();
    }
    let (Some(lo_index), Some(hi_index)) = (
        resolve_index(lo_text, length, numbers),
        resolve_index(hi_text, length, numbers),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(lo_index, hi_index, length) {
        return Vec::new();
    }
    let verb = if cmd_name == "lrange" {
        "lrange slice is empty".to_string()
    } else if lo_index < 0 && hi_index < 0 {
        "lreplace prepends instead of replacing (both indices resolve before the list)".to_string()
    } else if lo_index >= length && hi_index >= length {
        "lreplace appends instead of replacing (both indices resolve past the list)".to_string()
    } else {
        "lreplace touches no element (first > last after clamping)".to_string()
    };
    vec![crate::analyser::types::Diagnostic::new(
        DiagCode::W230,
        tcl_lexer::Span::new(lo_token.span.start(), hi_token.span.end()),
        format!(
            "{verb}: first='{lo_text}' resolves to {lo_index}, last='{hi_text}' resolves \
             to {hi_index} (list has {length} element{}).",
            if length == 1 { "" } else { "s" }
        ),
        Severity::Warning,
    )]
}

/// The per-index `lindex` arm of W230.
fn lindex_diagnostics(
    args: &[String],
    arg_tokens: &[Token],
    length: i64,
    numbers: tcl_dialect::NumberSyntax,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (pos, idx_text) in args.iter().enumerate().skip(1) {
        let Some(idx_tok) = arg_tokens.get(pos) else {
            continue;
        };
        if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text, numbers) {
            continue;
        }
        let Some(resolved) = resolve_index(idx_text, length, numbers) else {
            continue;
        };
        if (0..length).contains(&resolved) {
            continue;
        }
        out.push(crate::analyser::types::Diagnostic::new(
            DiagCode::W230,
            idx_tok.span,
            format!(
                "Index '{}' {}; lindex silently returns empty string.",
                idx_text.trim(),
                describe_index(resolved, length)
            ),
            Severity::Warning,
        ));
    }
    out
}

/// W231: an `lset` with a constant index known to be out of range.
/// Unlike `lindex` (which silently returns empty), `lset` raises a
/// runtime error; a plain negative literal always errors, and — when the
/// list length is recoverable from a recent literal `set` — an index past
/// the append slot (`> length`) or below zero errors too.
pub(crate) fn lset_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
    source: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    lexer_config: tcl_lexer::LexerConfig,
    numbers: tcl_dialect::NumberSyntax,
) -> Vec<Diagnostic> {
    if cmd_name != "lset" || args.len() < 3 || arg_tokens.len() < 3 {
        return Vec::new();
    }
    // `lset varName ?index ...? value` — index positions are 1..len-1.
    let index_positions: Vec<usize> = (1..args.len() - 1).collect();
    let nested = index_positions.len() > 1;
    // Multi-index (nested-sublist) forms only fire on plain-negative
    // literals; length-based checks need the single-index top-level form.
    let list_len = if nested {
        None
    } else {
        infer_list_length_from_recent_set(
            source,
            &args[0],
            arg_tokens[0].span.start(),
            registry,
            lexer_config,
        )
    };

    let mut out = Vec::new();
    for pos in index_positions {
        let idx_text = &args[pos];
        let Some(idx_tok) = arg_tokens.get(pos) else {
            continue;
        };
        if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text, numbers) {
            continue;
        }
        let stripped = idx_text.trim();
        // Plain negative integer: always errors.
        if let Some(n) = absolute_index(stripped, numbers)
            && n < 0
        {
            out.push(crate::analyser::types::Diagnostic::new(
                DiagCode::W231,
                idx_tok.span,
                format!(
                    "lset index '{stripped}' is negative; \
                         raises 'index out of range' at runtime."
                ),
                Severity::Warning,
            ));
            continue;
        }
        // `lset` accepts the append slot (`index == length`); only
        // indices past it or below zero are runtime errors.
        if let Some(length) = list_len {
            let Some(resolved) = resolve_index(stripped, length, numbers) else {
                continue;
            };
            if resolved < 0 || resolved > length {
                out.push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W231,
                    idx_tok.span,
                    format!(
                        "lset index '{stripped}' {}; \
                         raises 'index out of range' at runtime.",
                        describe_index(resolved, length)
                    ),
                    Severity::Warning,
                ));
            }
        }
    }
    out
}

/// The literal length of the list `var_name` holds when the `lset` at
/// `before_offset` runs, or `None` when it is not statically recoverable.
///
/// Structural, not textual (issue #1391).  The walk segments the document,
/// descends the braced word that *contains* the `lset` one level at a time,
/// and takes the length from the last literal assignment to `var_name` in the
/// innermost script the `lset` shares.  Descending resets the accumulator,
/// which is the structural spelling of the brace-depth-zero rule the byte
/// scan enforced: a `set` in an enclosing (or sibling) script is never
/// trusted for a nested `lset`.
///
/// What that buys over the byte scan it replaces:
///
/// * a `set` written inside a comment or a string literal is not a command,
///   so it no longer supplies a length (the false positive);
/// * an intervening `proc` / `apply` / `try` / `namespace eval` *sibling* no
///   longer blocks recovery — only actually descending into a body does — so
///   the `set x {a b c}` … `proc p {} {…}` … `lset x 9 v` shape reports again
///   (the false negative from the four-name marker list);
/// * a `set` not written at a line start, or one whose literal contains a
///   nested `{…}` sublist, now counts, because the segmenter reports words
///   rather than a line-anchored `\{[^{}]*\}` regex.
fn infer_list_length_from_recent_set(
    source: &str,
    var_name: &str,
    before_offset: u32,
    registry: Option<&tcl_registry::CommandRegistry>,
    lexer_config: tcl_lexer::LexerConfig,
) -> Option<i64> {
    if before_offset == 0 || before_offset as usize > source.len() || var_name.is_empty() {
        return None;
    }
    let registry = registry
        .unwrap_or_else(|| tcl_registry::model::ingress::static_context_for("tcl8.6").commands());
    let mut script: &str = source;
    let mut base: u32 = 0;
    let mut best: Option<i64> = None;
    for _ in 0..MAX_SCOPE_DESCENT.0 {
        best = None;
        let mut inner: Option<(&str, u32)> = None;
        for cmd in segment_commands_with_offset_and_config(script, base, lexer_config) {
            if cmd.span.start() >= before_offset {
                break;
            }
            if cmd.span.end() >= before_offset {
                // The `lset` lives inside this command; continue in the
                // braced word that holds it, starting a fresh scope.
                inner = cmd
                    .argv
                    .iter()
                    .find(|tok| {
                        tok.kind == TokenType::Str
                            && tok.span.start() < before_offset
                            && before_offset < tok.span.end()
                    })
                    .and_then(|tok| super::scope::inner_of(source, *tok));
                break;
            }
            if let Some(length) = literal_list_assignment(registry, &cmd, var_name) {
                best = Some(length);
            }
        }
        match inner {
            Some((text, text_base)) => {
                script = text;
                base = text_base;
            }
            None => return best,
        }
    }
    best
}

/// Native-stack safety net for [`infer_list_length_from_recent_set`]'s
/// descent through nested braced bodies.
const MAX_SCOPE_DESCENT: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// The literal list length `cmd` assigns to `var_name`, or `None`.
///
/// Registry-driven throughout: which argument is written comes from
/// [`tcl_registry::ArgRole::VarWrite`] (so `set`, `variable`, and any
/// spec-declared setter answer alike, and `::set` answers like `set`), and
/// two trait guards keep the answer honest —
/// [`tcl_registry::Traits::READS_BEFORE_WRITE`] excludes `append` / `incr` /
/// `lappend`, whose trailing word is *not* the resulting value, and
/// [`tcl_registry::Traits::WHOLE_ARRAY_ARG`] excludes `array set`, whose
/// target is an array rather than a list-valued scalar.
///
/// The value must be a single braced word, which is what makes it a literal:
/// the segmenter has already stripped the delimiter, so the text splits
/// directly as a Tcl list.
fn literal_list_assignment(
    registry: &tcl_registry::CommandRegistry,
    cmd: &SegmentedCommand,
    var_name: &str,
) -> Option<i64> {
    // The registry carries the environment's profile, so the assigned list
    // divides under the document's own list grammar.
    let rules = tcl_syntax::word_rules::WordValueRules::of_profile(registry.profile());
    let head = cmd.name().strip_prefix("::").unwrap_or(cmd.name());
    let spec = registry.get(head)?;
    if spec.traits.intersects(
        tcl_registry::Traits::READS_BEFORE_WRITE.union(tcl_registry::Traits::WHOLE_ARRAY_ARG),
    ) {
        return None;
    }
    let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
    let writes = registry.arg_indices_for_role(head, &args, tcl_registry::ArgRole::VarWrite);
    let [name_index] = writes.as_slice() else {
        return None;
    };
    if args.get(*name_index) != Some(&var_name) || args.len() != name_index + 2 {
        return None;
    }
    let value_index = name_index + 1;
    if cmd.arg_tokens().get(value_index)?.kind != TokenType::Str
        || cmd.arg_single_token().get(value_index) != Some(&true)
    {
        return None;
    }
    let value = cmd.args().get(value_index)?;
    Some(
        i64::try_from(crate::tcl_expr_eval::split_tcl_list(value, rules).len()).unwrap_or(i64::MAX),
    )
}

/// True when a `(first, last)` index pair resolves to a provably-empty
/// slice over `length`: both below, both above, clamped first > clamped
/// last, or an empty container.
fn pair_slice_empty(first: i64, last: i64, length: i64) -> bool {
    if length == 0 {
        return true;
    }
    let both_below = first < 0 && last < 0;
    let both_above = first >= length && last >= length;
    let clamped_first = first.clamp(0, length - 1);
    let clamped_last = last.clamp(0, length - 1);
    both_below || both_above || clamped_first > clamped_last
}

/// W232: a constant `string index` / `range` / `replace` / `insert`
/// into a literal string with a constant out-of-range (or negative)
/// index returns empty / is a no-op.
pub(crate) fn string_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
    numbers: tcl_dialect::NumberSyntax,
) -> Vec<Diagnostic> {
    if cmd_name != "string" || args.len() < 2 {
        return Vec::new();
    }
    let sub = args[0].as_str();
    let min_args = match sub {
        "index" => 3,
        "range" | "replace" | "insert" => 4,
        _ => return Vec::new(),
    };
    if args.len() < min_args || arg_tokens.len() < min_args {
        return Vec::new();
    }
    let str_tok = &arg_tokens[1];
    let str_text = &args[1];
    // Safe runtime length: braced strings do no backslash processing;
    // a backslash-free ESC word is byte-identical to its runtime form.
    // Back off (None) otherwise.
    let str_len: Option<i64> = if has_subst(str_text, str_tok) || !is_braced_or_esc(str_tok) {
        None
    } else if str_tok.kind == tcl_lexer::TokenType::Str {
        // `str_text` already has its outer `{…}` delimiter stripped by
        // the segmenter; a braced string does no backslash processing,
        // so its char count is its runtime length.  A second
        // `strip_braces` here would wrongly shorten a literal braced
        // string such as `{{hello}}` (segmented to `{hello}`).
        i64::try_from(str_text.chars().count()).ok()
    } else if str_text.contains('\\') {
        None
    } else {
        i64::try_from(str_text.chars().count()).ok()
    };

    if sub == "index" || sub == "insert" {
        return string_single_index(sub, args, arg_tokens, str_len, numbers);
    }
    string_pair_index(sub, args, arg_tokens, str_len, numbers)
}

/// The single-index `string index` / `string insert` arm of W232.
fn string_single_index(
    sub: &str,
    args: &[String],
    arg_tokens: &[Token],
    str_len: Option<i64>,
    numbers: tcl_dialect::NumberSyntax,
) -> Vec<Diagnostic> {
    let (idx_text, idx_tok) = (&args[2], &arg_tokens[2]);
    if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text, numbers) {
        return Vec::new();
    }
    let stripped = idx_text.trim();
    // A plain negative literal is always invalid.
    if let Some(n) = absolute_index(stripped, numbers)
        && n < 0
    {
        return vec![crate::analyser::types::Diagnostic::new(
            DiagCode::W232,
            idx_tok.span,
            format!("string {sub}: index '{stripped}' is negative; result is empty or a no-op."),
            Severity::Warning,
        )];
    }
    // `string insert` clamps other overshoots; only `string index`
    // flags an in-bounds miss.
    if sub == "index"
        && let Some(len) = str_len
        && let Some(resolved) = resolve_index(stripped, len, numbers)
        && !(0..len).contains(&resolved)
    {
        return vec![crate::analyser::types::Diagnostic::new(
            DiagCode::W232,
            idx_tok.span,
            format!(
                "string index: '{stripped}' {}; returns empty string.",
                describe_index_string(resolved, len)
            ),
            Severity::Warning,
        )];
    }
    Vec::new()
}

/// The `string range` / `string replace` `(first, last)` arm of W232.
fn string_pair_index(
    sub: &str,
    args: &[String],
    arg_tokens: &[Token],
    str_len: Option<i64>,
    numbers: tcl_dialect::NumberSyntax,
) -> Vec<Diagnostic> {
    let (first_text, last_text) = (&args[2], &args[3]);
    let (first_tok, last_tok) = (&arg_tokens[2], &arg_tokens[3]);
    if has_subst(first_text, first_tok)
        || !is_literal_index(first_text, numbers)
        || has_subst(last_text, last_tok)
        || !is_literal_index(last_text, numbers)
    {
        return Vec::new();
    }
    let verb = if sub == "range" {
        "slice is empty"
    } else {
        "replace is a no-op"
    };
    let span = tcl_lexer::Span::new(first_tok.span.start(), last_tok.span.end());

    // Both plain negative literals → always empty, even for a dynamic
    // string.
    if let (Some(f), Some(l)) = (
        absolute_index(first_text.trim(), numbers),
        absolute_index(last_text.trim(), numbers),
    ) && f < 0
        && l < 0
    {
        return vec![crate::analyser::types::Diagnostic::new(
            DiagCode::W232,
            span,
            format!(
                "string {sub}: both indices are negative ('{first_text}', '{last_text}'); \
                     {verb}."
            ),
            Severity::Warning,
        )];
    }
    let Some(len) = str_len else {
        return Vec::new();
    };
    let (Some(first_val), Some(last_val)) = (
        resolve_index(first_text, len, numbers),
        resolve_index(last_text, len, numbers),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(first_val, last_val, len) {
        return Vec::new();
    }
    vec![crate::analyser::types::Diagnostic::new(
        DiagCode::W232,
        span,
        format!(
            "string {sub}: {verb}: first='{first_text}' resolves to {first_val}, \
             last='{last_text}' resolves to {last_val} (string has {len} character{}).",
            if len == 1 { "" } else { "s" }
        ),
        Severity::Warning,
    )]
}

/// Human-readable description of a resolved out-of-range string index.
fn describe_index_string(resolved: i64, length: i64) -> String {
    if resolved < 0 {
        format!("resolves to {resolved} (before start of string)")
    } else {
        format!(
            "resolves to {resolved} (string has {length} character{})",
            if length == 1 { "" } else { "s" }
        )
    }
}

/// Token is a literal word (braced string or plain word).
fn is_braced_or_esc(tok: &Token) -> bool {
    matches!(
        tok.kind,
        tcl_lexer::TokenType::Str | tcl_lexer::TokenType::Esc
    )
}

/// Word contains a variable / command substitution.
fn has_subst(text: &str, tok: &Token) -> bool {
    matches!(
        tok.kind,
        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
    ) || text.contains('$')
        || text.contains('[')
}

/// `s` is a constant index expression we can evaluate (`end`, an
/// integer, or `end±N`).
fn is_literal_index(s: &str, numbers: tcl_dialect::NumberSyntax) -> bool {
    tcl_cmd_core::index::resolve_opt_with(s.trim(), 0, numbers).is_some()
}

/// Resolve a constant index to an absolute offset given `length`, or
/// `None` when `s` is not a literal index.
fn resolve_index(s: &str, length: i64, numbers: tcl_dialect::NumberSyntax) -> Option<i64> {
    tcl_cmd_core::index::resolve_opt_with(s.trim(), usize::try_from(length).ok()?, numbers)
}

/// Resolve a length-independent (non-`end`) Tcl index expression.
fn absolute_index(s: &str, numbers: tcl_dialect::NumberSyntax) -> Option<i64> {
    let s = s.trim();
    if s.starts_with("end") {
        return None;
    }
    tcl_cmd_core::index::resolve_opt_with(s, 0, numbers)
}

/// Human-readable description of a resolved out-of-range index.
fn describe_index(resolved: i64, length: i64) -> String {
    if resolved < 0 {
        format!("resolves to {resolved} (before start of list)")
    } else {
        format!(
            "resolves to {resolved} (list has {length} element{})",
            if length == 1 { "" } else { "s" }
        )
    }
}

const TRUE_LITERALS: &[&str] = &["1", "true", "yes", "on", "!0"];
const FALSE_LITERALS: &[&str] = &["0", "false", "no", "off", "!1"];

/// Strip a single enclosing `{ … }` and surrounding whitespace.
fn strip_braces(s: &str) -> &str {
    let t = s.trim();
    if t.starts_with('{') && t.ends_with('}') && t.len() >= 2 {
        t[1..t.len() - 1].trim()
    } else {
        t
    }
}

/// `Some(true)` / `Some(false)` when `cond` is a constant-true /
/// constant-false expression; `None` when dynamic.
fn condition_constant(cond: &str) -> Option<bool> {
    let c = strip_braces(cond).to_ascii_lowercase();
    if TRUE_LITERALS.contains(&c.as_str()) {
        return Some(true);
    }
    if FALSE_LITERALS.contains(&c.as_str()) {
        return Some(false);
    }
    c.trim().parse::<f64>().ok().map(|v| v != 0.0)
}

/// True when `body` contains, in command position, any command that leaves
/// the loop (`break` / `tailcall` / a `TERMINATES_BLOCK` command such as
/// `return` / `error` / `exit` / `throw`) — see [`is_loop_exit_command`].
///
/// Resolved via the segmenter (recursing into nested bodies) so only a
/// command in *command position* counts — a `break` appearing as a bare
/// argument no longer triggers a false exit.
fn body_may_exit(
    body: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    lexer_config: tcl_lexer::LexerConfig,
) -> bool {
    any_command_recursive(body, lexer_config, &mut |cmd| {
        is_loop_exit_command(cmd.name(), registry)
    })
}

#[cfg(test)]
mod tests {
    use super::{any_command_recursive, infer_list_length_from_recent_set, sole_command};
    use crate::analyser::Analyser;
    use tcl_core_types::DiagCode;

    fn config() -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::default()
    }

    fn codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W240" | "W241"))
            .map(|d| d.code.to_string())
            .collect()
    }

    #[test]
    fn w240_constant_false_condition() {
        assert_eq!(codes("while 0 {puts hi}\n"), vec!["W240"]);
        assert_eq!(codes("for {set i 0} 0 {incr i} {}\n"), vec!["W240"]);
    }

    #[test]
    fn configured_bounds_scanners_cover_each_segmenter_site() {
        // Mutation proof for the three bounds callers: iRules treats `}{` as
        // a ghost separator, whereas the default Tcl lexer keeps the second
        // braced word attached to the first command.
        let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
        let one = sole_command("cmd {a}{b}", config).expect("one command");
        assert_eq!(one.args().len(), 2, "iRules words: {:?}", one.texts);
        assert!(any_command_recursive("cmd {a}{b}", config, &mut |cmd| {
            cmd.args().len() == 2
        }));

        let source = "set xs {a b}\nlset xs 9 v\n";
        let before = u32::try_from(source.find("lset").expect("lset")).expect("offset");
        assert_eq!(
            infer_list_length_from_recent_set(source, "xs", before, None, config,),
            Some(2)
        );
    }

    #[test]
    fn w241_constant_true_no_exit() {
        assert_eq!(codes("while 1 {puts hi}\n"), vec!["W241"]);
        // A `break` in the body suppresses W241.
        assert!(codes("while 1 {break}\n").is_empty());
        assert!(codes("while 1 {return}\n").is_empty());
    }

    #[test]
    fn w241_throw_and_tailcall_leave_the_loop() {
        // FP fix: `throw` and `tailcall` both terminate the loop after one
        // iteration (verified against tclsh 9.0.4), so a `while 1` body
        // containing either is NOT provably infinite. `throw` resolves via the
        // registry's TERMINATES_BLOCK trait; `tailcall` is named explicitly.
        assert!(
            codes("while 1 {throw MYERR boom}\n").is_empty(),
            "throw must suppress W241",
        );
        assert!(
            codes("while 1 {tailcall foo}\n").is_empty(),
            "tailcall must suppress W241",
        );
        // TP controls: the other block-terminators still suppress, and a body
        // with no exit at all still fires.
        assert!(codes("while 1 {error boom}\n").is_empty());
        assert!(codes("while 1 {exit 1}\n").is_empty());
        assert_eq!(codes("while 1 {incr n}\n"), vec!["W241"]);
        // `continue` is NOT an exit — it keeps looping — so W241 still fires.
        assert_eq!(codes("while 1 {continue}\n"), vec!["W241"]);
    }

    #[test]
    fn w241_for_loop_throw_tailcall_suppress() {
        // Same coverage on the `for` provably-infinite counter shape: a body
        // that throws / tailcalls is not an infinite loop.
        assert!(codes("for {set i 0} {$i < 10} {incr i 0} {throw E x}\n").is_empty());
        assert!(codes("for {set i 5} {$i > 0} {incr i} {tailcall done}\n").is_empty());
    }

    #[test]
    fn dynamic_condition_is_silent() {
        assert!(codes("while {$x < 10} {incr x}\n").is_empty());
    }

    #[test]
    fn w241_for_step_provably_infinite() {
        // Step 0, wrong-direction step, increment-away, never-equal-skip.
        assert_eq!(
            codes("for {set i 0} {$i < 10} {incr i 0} {}\n"),
            vec!["W241"]
        );
        assert_eq!(
            codes("for {set i 0} {$i < 10} {incr i -1} {}\n"),
            vec!["W241"]
        );
        assert_eq!(codes("for {set i 5} {$i > 0} {incr i} {}\n"), vec!["W241"]);
        assert_eq!(
            codes("for {set i 0} {$i != 10} {incr i 3} {}\n"),
            vec!["W241"]
        );
        // A correct counting loop is silent.
        assert!(codes("for {set i 0} {$i < 10} {incr i} {}\n").is_empty());
    }

    #[test]
    fn for_cond_parsers() {
        assert_eq!(
            super::parse_simple_for_cond("$i < 10", &tcl_dialect::LexerGrammar::default()),
            Some(("i".into(), "<".into(), 10)),
        );
        assert_eq!(
            super::parse_simple_for_cond("10 > $i", &tcl_dialect::LexerGrammar::default()),
            Some(("i".into(), "<".into(), 10)), // flipped
        );
        assert_eq!(
            super::parse_simple_for_cond("$i < 10 && 0", &tcl_dialect::LexerGrammar::default()),
            None
        );
        // Negative bound: the expression lexer splits `-5` into a unary
        // `-` plus `5`, which the signed-integer folding reassembles.
        assert_eq!(
            super::parse_simple_for_cond("$i > -5", &tcl_dialect::LexerGrammar::default()),
            Some(("i".into(), ">".into(), -5)),
        );
        assert_eq!(
            super::parse_simple_for_cond("-5 < $i", &tcl_dialect::LexerGrammar::default()),
            Some(("i".into(), ">".into(), -5)), // flipped
        );
        // String comparison operators are accepted; braces are stripped.
        assert_eq!(
            super::parse_simple_for_cond("{$i ne 3}", &tcl_dialect::LexerGrammar::default()),
            Some(("i".into(), "ne".into(), 3)),
        );
        // `${i}` braced variable form and `==`.
        assert_eq!(
            super::parse_simple_for_cond("${i} == 7", &tcl_dialect::LexerGrammar::default()),
            Some(("i".into(), "==".into(), 7)),
        );
        // Compound / logical / ternary / non-integer bounds reject.
        assert_eq!(
            super::parse_simple_for_cond(
                "$i < 10 || $j > 0",
                &tcl_dialect::LexerGrammar::default()
            ),
            None
        );
        assert_eq!(
            super::parse_simple_for_cond("!$done", &tcl_dialect::LexerGrammar::default()),
            None
        );
        assert_eq!(
            super::parse_simple_for_cond("$i ? 1 : 0", &tcl_dialect::LexerGrammar::default()),
            None
        );
        assert_eq!(
            super::parse_simple_for_cond("$i < 10.5", &tcl_dialect::LexerGrammar::default()),
            None
        );
        // The `-?\d+` bound and `\$\{?(\w+)\}?` var shapes mean a leading
        // `+`, an array index, or a namespace qualifier reject.
        assert_eq!(
            super::parse_simple_for_cond("$i < +5", &tcl_dialect::LexerGrammar::default()),
            None
        );
        assert_eq!(
            super::parse_simple_for_cond("$arr(i) < 5", &tcl_dialect::LexerGrammar::default()),
            None
        );
        assert_eq!(
            super::parse_simple_for_cond("$ns::v < 5", &tcl_dialect::LexerGrammar::default()),
            None
        );
        assert_eq!(
            super::parse_init_var_value("set i 5", config()),
            Some(("i".into(), 5))
        );
        // A non-`set` command, extra words, or a non-integer value reject.
        assert_eq!(super::parse_init_var_value("incr i 5", config()), None);
        assert_eq!(super::parse_init_var_value("set i 5 6", config()), None);
        assert_eq!(super::parse_init_var_value("set i foo", config()), None);
        assert_eq!(
            super::parse_step_incr("incr i", config()),
            Some(("i".into(), 1))
        );
        assert_eq!(
            super::parse_step_incr("incr i -2", config()),
            Some(("i".into(), -2))
        );
        assert_eq!(super::parse_step_incr("set i 3", config()), None);
    }

    #[test]
    fn body_scans_are_command_structural() {
        // A write counts only in command position, not inside a string.
        assert!(super::body_writes_var("incr i", "i", None, config()));
        assert!(super::body_writes_var(
            "if {$c} {set i 9}",
            "i",
            None,
            config()
        )); // nested body
        assert!(!super::body_writes_var(
            "puts \"set i now\"",
            "i",
            None,
            config()
        )); // inside a string
        assert!(!super::body_writes_var("incr index", "i", None, config())); // word boundary
        // `break` / `return` / `throw` / `tailcall` likewise count only as
        // commands. `return`/`throw` resolve via the registry's
        // TERMINATES_BLOCK trait; `break`/`tailcall` are recognised without it.
        let reg = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        assert!(super::body_may_exit("break", Some(reg), config()));
        assert!(super::body_may_exit(
            "if {$c} {return}",
            Some(reg),
            config()
        )); // nested
        assert!(super::body_may_exit(
            "throw MYERR boom",
            Some(reg),
            config()
        )); // now covered
        assert!(super::body_may_exit("tailcall foo", Some(reg), config()));
        assert!(!super::body_may_exit("puts breakfast", Some(reg), config())); // not a command
        // `break`/`tailcall` are recognised even without a registry handle.
        assert!(super::body_may_exit("break", None, config()));
        assert!(super::body_may_exit("tailcall foo", None, config()));
    }

    fn idx_codes_for(src: &str, dialect: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W230" | "W232"))
            .map(|d| d.code.to_string())
            .collect()
    }

    fn idx_codes(src: &str) -> Vec<String> {
        idx_codes_for(src, "tcl8.6")
    }

    #[test]
    fn w230_list_index_out_of_range() {
        assert_eq!(idx_codes("lindex {a b c} 5\n"), vec!["W230"]);
        assert_eq!(idx_codes("lindex {a b c} -1\n"), vec!["W230"]);
        assert_eq!(idx_codes("lindex {a b c} end-5\n"), vec!["W230"]);
        // In range and dynamic list → none.
        assert!(idx_codes("lindex {a b c} 1\n").is_empty());
        assert!(idx_codes("lindex {a b c} end\n").is_empty());
        assert!(idx_codes("lindex $x 5\n").is_empty());
    }

    #[test]
    fn w230_single_element_nested_list_length() {
        // `{{a b c}}` is a one-element list (the inner `{a b c}` is a
        // single braced element).  The segmenter already strips the outer
        // braces, so its length is 1 — index 2 is out of range.  A
        // double brace-strip would wrongly count three words and miss it.
        assert_eq!(idx_codes("lindex {{a b c}} 2\n"), vec!["W230"]);
        // Index 0 is the lone element → in range.
        assert!(idx_codes("lindex {{a b c}} 0\n").is_empty());
    }

    #[test]
    fn w232_string_index_out_of_range() {
        assert_eq!(idx_codes("string index abc 10\n"), vec!["W232"]);
        assert_eq!(idx_codes("string index abc -1\n"), vec!["W232"]);
        assert!(idx_codes("string index abc 1\n").is_empty());
        assert!(idx_codes("string index abc end\n").is_empty());
    }

    #[test]
    fn w232_braced_literal_string_length() {
        // `{{hello}}` is a braced word whose runtime value is the literal
        // 7-char string `{hello}` (the segmenter strips only the outer
        // braces).  Index 6 is the last char — in range.  A double
        // brace-strip would count five chars and flag it spuriously.
        assert!(idx_codes("string index {{hello}} 6\n").is_empty());
        // One past the end is still out of range.
        assert_eq!(idx_codes("string index {{hello}} 7\n"), vec!["W232"]);
    }

    #[test]
    fn w230_lrange_lreplace_empty_slice() {
        assert_eq!(idx_codes("lrange {a b c} 5 7\n"), vec!["W230"]);
        assert_eq!(idx_codes("lrange {a b c} -3 -1\n"), vec!["W230"]);
        assert_eq!(idx_codes("lrange {a b c} 2 0\n"), vec!["W230"]); // clamped first>last
        assert_eq!(idx_codes("lreplace {a b c} 5 7 X\n"), vec!["W230"]);
        assert!(idx_codes("lrange {a b c} 0 1\n").is_empty());
    }

    #[test]
    fn w232_string_range_replace_empty_slice() {
        assert_eq!(idx_codes("string range abc 5 7\n"), vec!["W232"]);
        assert_eq!(idx_codes("string range abc -3 -1\n"), vec!["W232"]);
        assert_eq!(idx_codes("string replace abc 5 7 X\n"), vec!["W232"]);
        assert!(idx_codes("string range abc 0 1\n").is_empty());
    }

    #[test]
    fn pair_slice_empty_logic() {
        assert!(super::pair_slice_empty(5, 7, 3)); // both above
        assert!(super::pair_slice_empty(-3, -1, 3)); // both below
        assert!(super::pair_slice_empty(2, 0, 3)); // clamped first>last
        assert!(super::pair_slice_empty(0, 0, 0)); // empty container
        assert!(!super::pair_slice_empty(0, 1, 3)); // valid
    }

    #[test]
    fn index_helpers() {
        let n = tcl_dialect::NumberSyntax::Tcl85;
        assert_eq!(super::resolve_index("end", 3, n), Some(2));
        assert_eq!(super::resolve_index("end-5", 3, n), Some(-3));
        assert_eq!(super::resolve_index("end+1", 3, n), Some(3));
        assert_eq!(super::resolve_index("2", 3, n), Some(2));
        assert_eq!(super::resolve_index("1+1", 3, n), Some(2));
        assert_eq!(super::resolve_index("0x2", 3, n), Some(2));
        assert_eq!(super::resolve_index("+5", 3, n), Some(5));
        assert_eq!(super::resolve_index("$x", 3, n), None);
        assert!(super::is_literal_index("end-2", n));
        assert!(!super::is_literal_index("end - 2", n));
    }

    #[test]
    fn index_diagnostics_use_the_document_number_syntax() {
        let list = "lindex {a b c d e f g h i} 010\n";
        let string = "string index abcdefghi 010\n";
        // Tcl 8.x reads 010 as octal 8 (in range); Tcl 9.x reads it as
        // decimal 10 (out of range). Mutation: replacing the threaded
        // profile syntax with the ambient parser makes these pairs agree.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            assert!(idx_codes_for(list, dialect).is_empty(), "{dialect}");
            assert!(idx_codes_for(string, dialect).is_empty(), "{dialect}");
        }
        for dialect in ["tcl9.0", "tcl9.1"] {
            assert_eq!(idx_codes_for(list, dialect), vec!["W230"], "{dialect}");
            assert_eq!(idx_codes_for(string, dialect), vec!["W232"], "{dialect}");
        }
    }

    #[test]
    fn condition_constant_classifies() {
        assert_eq!(super::condition_constant("0"), Some(false));
        assert_eq!(super::condition_constant("1"), Some(true));
        assert_eq!(super::condition_constant("{true}"), Some(true));
        assert_eq!(super::condition_constant("$x < 10"), None);
    }

    // -- W231 (lset out of range) & W242 (unprovable termination) -----

    fn code_msgs_for(src: &str, dialect: &str, code: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == code)
            .map(|d| d.message.clone())
            .collect()
    }

    fn code_msgs(src: &str, code: &str) -> Vec<String> {
        code_msgs_for(src, "tcl8.6", code)
    }

    #[test]
    fn w231_lset_negative_index_always_errors() {
        let m = code_msgs("lset L -1 x\n", "W231");
        assert_eq!(m.len(), 1);
        assert!(m[0].contains("lset index '-1' is negative"), "{m:?}");
        // Negative literal fires even for nested (multi-index) forms.
        assert_eq!(code_msgs("lset L i j -1 v\n", "W231").len(), 1);
    }

    #[test]
    fn w231_lset_index_past_append_slot() {
        // List length recovered from the recent literal `set`.
        let m = code_msgs("set L {a b c}\nlset L 5 x\n", "W231");
        assert_eq!(m.len(), 1);
        assert!(
            m[0].contains("resolves to 5 (list has 3 elements)"),
            "{m:?}"
        );
        // The append slot (index == length) and valid indices are fine.
        assert!(code_msgs("set L {a b c}\nlset L 3 x\n", "W231").is_empty());
        assert!(code_msgs("set L {a b c}\nlset L end+1 x\n", "W231").is_empty());
        assert!(code_msgs("set L {a b c}\nlset L 2 x\n", "W231").is_empty());
    }

    #[test]
    fn w231_uses_the_document_number_syntax() {
        let src = "set L {a b c d e f g h i}\nlset L 010 x\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            assert!(
                code_msgs_for(src, dialect, "W231").is_empty(),
                "{dialect} reads 010 as in-range octal 8"
            );
        }
        for dialect in ["tcl9.0", "tcl9.1"] {
            assert_eq!(
                code_msgs_for(src, dialect, "W231").len(),
                1,
                "{dialect} reads 010 as out-of-range decimal 10"
            );
        }
    }

    #[test]
    fn w231_silent_without_recoverable_length() {
        // No prior literal `set` → only negative literals would fire.
        assert!(code_msgs("lset L 5 x\n", "W231").is_empty());
        // A `set` in a deeper scope must not be trusted.
        assert!(
            code_msgs("proc p {} { set L {a b c} }\nlset L 5 x\n", "W231").is_empty(),
            "deeper-scope set should not leak its length"
        );
    }

    #[test]
    fn w242_counter_not_modified() {
        let m = code_msgs("while {$x < 10} {puts hi}\n", "W242");
        assert_eq!(m.len(), 1);
        assert!(
            m[0].contains("variable 'x' in the condition is never modified"),
            "{m:?}"
        );
        // `for` with an empty step and a body that ignores the counter.
        assert_eq!(
            code_msgs("for {set i 0} {$i < 10} {} {puts hi}\n", "W242").len(),
            1
        );
    }

    #[test]
    fn w242_silent_when_counter_modified() {
        // Body writes the counter via incr / set / lappend → no W242.
        assert!(code_msgs("while {$x < 10} {incr x}\n", "W242").is_empty());
        assert!(code_msgs("while {$x < 10} {set x 5}\n", "W242").is_empty());
        // A normal `for` whose step advances the counter is silent.
        assert!(code_msgs("for {set i 0} {$i < 10} {incr i} {puts hi}\n", "W242").is_empty());
    }

    // FP-STY-… (issue #1316 sweep, corpus: `tcltest.tcl`'s option-usage
    // word-wrapper): a `[cmd $var]` command substitution in the condition
    // hides the loop's real progress variable from the shallow scalar scan,
    // which then blames whichever *other* bare variable it finds instead.

    #[test]
    fn w242_silent_when_the_progress_variable_is_inside_a_command_substitution() {
        // FP: `u` shrinks every iteration via `string range`/`string trim` in
        // the body — real, provable progress — but the condition's only
        // *bare* variable is `rest` (a fixed threshold the loop never
        // touches), which the old first-`Variable`-token scan picked
        // instead. Exact corpus shape.
        assert!(
            code_msgs(
                "while {[string length $u] > $rest} {\
                     set u [string trim [string range $u 1 end]]\
                 }\n",
                "W242"
            )
            .is_empty()
        );
    }

    #[test]
    fn w242_silent_for_a_command_substitution_condition_even_when_nothing_is_modified() {
        // FP guard, the genuinely-unprovable case: abstaining is still
        // correct here — the analyser cannot see into `[llength $l]`'s
        // argument to know whether `l` (or anything else) changes, so it
        // must not guess at a bare variable elsewhere in the condition
        // (there is none here) or fabricate a counter name.
        assert!(code_msgs("while {[llength $l] > 0} {puts hi}\n", "W242").is_empty());
    }

    #[test]
    fn w242_severity_is_hint() {
        let mut a = Analyser::new();
        let r = a.analyse("while {$x < 10} {puts hi}\n", "tcl8.6");
        let w242 = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W242)
            .unwrap();
        assert_eq!(w242.severity, super::Severity::Hint);
    }

    fn w231(src: &str) -> usize {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W231)
            .count()
    }

    #[test]
    fn w231_list_length_recovered_inside_proc_body() {
        // The `set l {a b c}` shares the proc body's flat scope with the
        // `lset`, so its length (3) is recovered and `99` proves OOR —
        // the case top-level-only segmentation used to miss.
        assert_eq!(
            w231("proc f {} {\n    set l {a b c}\n    lset l 99 X\n}\n"),
            1
        );
        // `end` on an empty list is out of range.
        assert_eq!(w231("proc f {} {\n    set l {}\n    lset l end X\n}\n"), 1);
        // Still fires at top level.
        assert_eq!(w231("set l {a b c}\nlset l 99 X\n"), 1);
        // The append slot (index == length) is accepted.
        assert_eq!(
            w231("proc f {} {\n    set l {a b c}\n    lset l 3 X\n}\n"),
            0
        );
    }

    fn has_code(src: &str, code: &str) -> bool {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == code)
    }

    #[test]
    fn bounds_checks_recurse_into_command_substitutions() {
        // The main walk never descends a `[…]` substitution, so the
        // bounds family runs on its inner commands via the nested-bounds
        // recursion.
        assert!(has_code("set x [lindex {a b c} 9]\n", "W230"));
        assert!(has_code(
            "proc f {} { return [string index abc 99] }\n",
            "W232"
        ));
        // Nested two deep: `[foo [string index abc 99]]`.
        assert!(has_code(
            "proc f {} { return [join [string index abc 99]] }\n",
            "W232"
        ));
        // A bare (non-substituted) command is still checked exactly once —
        // the recursion only enters `[…]`, so no double-fire.
        let mut a = Analyser::new();
        let n = a
            .analyse("set x [lindex {a b c} 9]\n", "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W230)
            .count();
        assert_eq!(n, 1);
    }

    #[test]
    fn security_checks_recurse_into_command_substitutions() {
        // The full per-command dispatch — not just bounds — runs on a
        // substitution command, so `open "|$cmd"` nested in `[…]` still
        // fires the W103 pipeline-injection check.
        assert!(has_code(
            "proc f {cmd} { set fh [open \"|$cmd\" r] }\n",
            "W103"
        ));
    }

    #[test]
    fn w231_length_not_recovered_across_scope_boundary() {
        // A `set` in a *different* proc body must not leak its length to a
        // top-level `lset` — it belongs to a script the `lset` never enters.
        assert_eq!(w231("proc other {} { set l {a b c} }\nlset l 99 X\n"), 0);
        // Descending into a body starts a fresh scope, so an outer `set` is
        // not trusted for an `lset` written inside one.
        assert_eq!(
            w231("set l {a b c}\nnamespace eval ns {\n    lset l 99 X\n}\n"),
            0
        );
        // Nor across an `oo::define` member body, which the four-name
        // scope-marker scan never knew about (issue #1391).
        assert_eq!(
            w231("set l {a b c}\noo::define C {\n    method m {} { lset l 99 X }\n}\n"),
            0
        );
    }

    #[test]
    fn w231_ignores_a_set_that_is_not_a_command() {
        // The `set` inside the quoted word is text, not an assignment, so it
        // supplies no length and cannot shorten the real one.  The
        // line-anchored byte scan matched it and reported index 4 as out of
        // range for a two-element list (issue #1391).
        let src = "set l {a b c d e f}\nputs \"\nset l {a b}\n\"\nlset l 4 X\n";
        assert_eq!(w231(src), 0);
    }

    #[test]
    fn w231_reports_across_a_sibling_definition() {
        // `proc` here is a *sibling* command, not a scope the `lset` is
        // inside, so the top-level `set`'s length still reaches it.  The
        // `\\b(?:proc|namespace\\s+eval|apply|try)\\b` marker scan saw the
        // word `proc` between the two and went silent (issue #1391).
        assert_eq!(
            w231("set l {a b c}\nproc helper {} { return 1 }\nlset l 9 X\n"),
            1
        );
    }

    #[test]
    fn w231_recovers_a_length_the_line_anchored_scan_missed() {
        // Not at a line start, and a literal carrying a nested sublist:
        // both are ordinary words to the segmenter and neither matched
        // `(?:^|\\n)\\s*set\\s+(\\w+)\\s+(\\{[^{}]*\\})` (issue #1391).
        assert_eq!(w231("puts hi; set l {a {b c} d}\nlset l 9 X\n"), 1);
    }
}
