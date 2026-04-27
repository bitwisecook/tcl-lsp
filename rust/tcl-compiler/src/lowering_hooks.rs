//! Per-command lowering specialisations.
//!
//! Each function takes a [`LoweringCommand`] (the parsed command context)
//! and returns `Some(Statement)` if the command is handled, or `None` to
//! fall through to the default `IRCall` path.
//!
//! Ports `core/compiler/lowering_hooks/_control.py` and `_var.py`.

use std::collections::HashSet;

use tcl_lexer::Span;

use crate::alias::{expr_alias_names, CommandAliasMap};
use crate::expr_parser::parse_expr;
use crate::ir::{CommandTokens, Statement};
use crate::naming::normalise_var_name;

/// Parsed command context passed to lowering hooks.
///
/// This replaces the Python `_Command` dataclass.
pub struct LoweringCommand<'a> {
    /// Source span of the full command.
    pub span: Span,
    /// Command name (first word).
    pub name: &'a str,
    /// Arguments (words after the command name).
    pub args: &'a [String],
    /// Whether each word is a single token.
    pub single_token_word: &'a [bool],
    /// `{*}` expansion markers, if any word uses expansion.
    pub expand_word: Option<&'a [bool]>,
    /// Snapshot of parsed tokens for downstream passes.
    pub tokens: Option<CommandTokens>,
    /// Per-arg token kinds (mapped from Python `arg_tokens[i].type`).
    /// Uses a simplified enum since we only check STR/ESC/CMD.
    pub arg_kinds: &'a [ArgTokenKind],
}

/// Simplified token kind for hook arg inspection.
///
/// We only need to distinguish braced strings, plain words, and
/// command substitutions in lowering hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgTokenKind {
    /// Braced string `{...}`.
    Str,
    /// Plain word (bare text, possibly with backslash escapes).
    Esc,
    /// Command substitution `[...]`.
    Cmd,
    /// Variable reference `$...`.
    Var,
    /// Any other token type.
    Other,
}

/// Try to lower a command via a registered hook.
///
/// Returns `Some(statement)` if the command was handled, `None` to
/// fall through to the default `IRCall` path.
#[must_use]
pub fn try_lower_hook(cmd: &LoweringCommand<'_>, aliases: &CommandAliasMap) -> Option<Statement> {
    match cmd.name {
        "expr" => lower_expr(cmd),
        "return" => Some(lower_return(cmd, aliases)),
        "set" => Some(lower_set(cmd, aliases)),
        "incr" => Some(crate::lowering::hooks::incr::try_lower_incr(cmd)),
        "append" | "lappend" => lower_append_lappend(cmd),
        "unset" => Some(lower_unset(cmd)),
        "global" => lower_global(cmd),
        "variable" => Some(lower_variable(cmd)),
        "upvar" => lower_upvar(cmd),
        _ => None,
    }
}

/// Whether this command has `{*}` expansion on any argument.
fn has_expansion(cmd: &LoweringCommand<'_>) -> bool {
    cmd.expand_word.is_some_and(|ew| ew.iter().any(|&e| e))
}

// ── expr ──────────────────────────────────────────────────────────

fn lower_expr(cmd: &LoweringCommand<'_>) -> Option<Statement> {
    if has_expansion(cmd) {
        return None;
    }
    if cmd.args.len() != 1 {
        return None;
    }
    // Only specialise when the arg is a single token.
    if cmd.single_token_word.len() < 2 || !cmd.single_token_word[1] {
        return None;
    }
    let expr = parse_expr(&cmd.args[0], None);
    Some(Statement::ExprEval {
        span: cmd.span,
        expr,
    })
}

// ── return ────────────────────────────────────────────────────────

fn lower_return(cmd: &LoweringCommand<'_>, aliases: &CommandAliasMap) -> Statement {
    if has_expansion(cmd) {
        return Statement::Barrier {
            span: cmd.span,
            reason: "return with expansion".into(),
            command: cmd.name.into(),
            args: cmd.args.to_vec(),
            tokens: cmd.tokens.clone(),
        };
    }
    if !cmd.args.is_empty() && cmd.args[0].starts_with('-') {
        return Statement::Barrier {
            span: cmd.span,
            reason: "return with options".into(),
            command: cmd.name.into(),
            args: cmd.args.to_vec(),
            tokens: cmd.tokens.clone(),
        };
    }

    let value = cmd.args.first().cloned();
    let mut expr = None;
    let mut braced = false;

    if value.is_some()
        && !cmd.arg_kinds.is_empty()
        && cmd.single_token_word.len() >= 2
        && cmd.single_token_word[1]
    {
        match cmd.arg_kinds[0] {
            ArgTokenKind::Str => braced = true,
            ArgTokenKind::Cmd => {
                let inner = cmd.args[0]
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .unwrap_or(&cmd.args[0]);
                let alias_names = expr_alias_names(aliases);
                if let Some(expr_arg) = extract_single_expr_arg(inner, &alias_names) {
                    expr = Some(parse_expr(&expr_arg, None));
                }
            }
            _ => {}
        }
    }

    Statement::Return {
        span: cmd.span,
        value,
        expr,
        braced,
    }
}

// ── set ───────────────────────────────────────────────────────────

fn lower_set(cmd: &LoweringCommand<'_>, aliases: &CommandAliasMap) -> Statement {
    if has_expansion(cmd) || cmd.args.len() != 2 {
        return make_call(cmd);
    }

    let name = &cmd.args[0];
    let value = &cmd.args[1];

    // Check if value arg is a single token.
    if cmd.single_token_word.len() >= 3 && cmd.single_token_word[2] && cmd.arg_kinds.len() >= 2 {
        match cmd.arg_kinds[1] {
            ArgTokenKind::Str => {
                return Statement::AssignConst {
                    span: cmd.span,
                    name: name.clone(),
                    value: value.clone(),
                };
            }
            ArgTokenKind::Esc => {
                if let Some(int_val) = parse_decimal_int(value) {
                    return Statement::AssignConst {
                        span: cmd.span,
                        name: name.clone(),
                        value: int_val,
                    };
                }
                let needs_backsubst = value.contains('\\');
                return Statement::AssignValue {
                    span: cmd.span,
                    name: name.clone(),
                    value: value.clone(),
                    value_needs_backsubst: needs_backsubst,
                    tokens: cmd.tokens.clone(),
                };
            }
            ArgTokenKind::Cmd => {
                // The segmenter wraps CMD tokens as [text]; strip the
                // brackets so extract_single_expr_arg sees "expr {arg}".
                let inner = value
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .unwrap_or(value);
                let alias_names = expr_alias_names(aliases);
                if let Some(expr_arg) = extract_single_expr_arg(inner, &alias_names) {
                    let expr = parse_expr(&expr_arg, None);
                    return Statement::AssignExpr {
                        span: cmd.span,
                        name: name.clone(),
                        expr,
                    };
                }
            }
            _ => {}
        }
    }

    Statement::AssignValue {
        span: cmd.span,
        name: name.clone(),
        value: value.clone(),
        value_needs_backsubst: false,
        tokens: cmd.tokens.clone(),
    }
}

// ── incr ──────────────────────────────────────────────────────────
//
// Moved to `crate::lowering::hooks::incr::try_lower_incr` (chunk
// **C43**). The dispatcher above delegates the `"incr"` case to
// the per-hook module.

// ── append / lappend ──────────────────────────────────────────────

fn lower_append_lappend(cmd: &LoweringCommand<'_>) -> Option<Statement> {
    if cmd.args.is_empty() {
        return None;
    }
    let name = normalise_var_name(&cmd.args[0]).to_owned();
    Some(Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        args: cmd.args.to_vec(),
        defs: vec![name],
        reads: vec![],
        reads_own_defs: true,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
    })
}

// ── unset ─────────────────────────────────────────────────────────

fn lower_unset(cmd: &LoweringCommand<'_>) -> Statement {
    let mut i = 0;
    let mut nocomplain = false;
    while i < cmd.args.len() && cmd.args[i].starts_with('-') {
        if cmd.args[i] == "-nocomplain" {
            nocomplain = true;
        }
        if cmd.args[i] == "--" {
            i += 1;
            break;
        }
        i += 1;
    }
    let var_names: Vec<String> = cmd.args[i..]
        .iter()
        .map(|a| normalise_var_name(a).to_owned())
        .collect();
    Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: !nocomplain,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
    }
}

// ── global ────────────────────────────────────────────────────────

fn lower_global(cmd: &LoweringCommand<'_>) -> Option<Statement> {
    if cmd.args.is_empty() {
        return None;
    }
    let var_names: Vec<String> = cmd
        .args
        .iter()
        .map(|a| normalise_var_name(a).to_owned())
        .collect();
    Some(Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
    })
}

// ── variable ──────────────────────────────────────────────────────

fn lower_variable(cmd: &LoweringCommand<'_>) -> Statement {
    let var_names: Vec<String> = cmd
        .args
        .iter()
        .step_by(2)
        .map(|a| normalise_var_name(a).to_owned())
        .collect();
    Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
    }
}

// ── upvar ─────────────────────────────────────────────────────────

fn lower_upvar(cmd: &LoweringCommand<'_>) -> Option<Statement> {
    if cmd.args.len() < 2 {
        return None;
    }
    let has_level = cmd.args[0]
        .trim_start_matches('-')
        .chars()
        .all(|c| c.is_ascii_digit())
        || cmd.args[0].starts_with('#');
    let start = usize::from(has_level);
    let my_vars: Vec<String> = cmd.args[start..]
        .iter()
        .skip(1)
        .step_by(2)
        .map(|a| normalise_var_name(a).to_owned())
        .collect();
    Some(Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        args: cmd.args.to_vec(),
        defs: my_vars,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
    })
}

// ── Helpers ───────────────────────────────────────────────────────

/// Build a generic `Statement::Call` from a lowering command.
fn make_call(cmd: &LoweringCommand<'_>) -> Statement {
    Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        args: cmd.args.to_vec(),
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
    }
}

/// Extract the single expression argument from a `[expr ...]` command.
///
/// Returns `Some(expr_text)` if the text is `expr <one-word>` (or an
/// expr alias), `None` otherwise.
fn extract_single_expr_arg(text: &str, expr_aliases: &HashSet<String>) -> Option<String> {
    use tcl_lexer::{Lexer, SourceMap, TokenType};

    let sm = SourceMap::new(text);
    let lexer = Lexer::new(text);
    let Ok(tokens) = lexer.tokenise_all() else {
        return None;
    };

    let mut words = Vec::new();
    let mut single = Vec::new();
    let mut prev_is_sep = true;
    for tok in &tokens {
        match tok.kind {
            TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment => {
                prev_is_sep = true;
            }
            _ => {
                let t = sm.token_text(*tok);
                if prev_is_sep {
                    words.push(t.to_owned());
                    single.push(true);
                } else if let Some(last) = words.last_mut() {
                    last.push_str(t);
                    if let Some(s) = single.last_mut() {
                        *s = false;
                    }
                } else {
                    words.push(t.to_owned());
                    single.push(true);
                }
                prev_is_sep = false;
            }
        }
    }

    if words.len() != 2 {
        return None;
    }
    let cmd_word = &words[0];
    if cmd_word != "expr" && !expr_aliases.contains(cmd_word.as_str()) {
        return None;
    }
    if !single[1] {
        return None;
    }
    Some(words[1].clone())
}

/// Validate text as a decimal integer, returning the source text if valid.
///
/// Rejects non-digit content, hex/octal prefixes, and leading zeros
/// (except for `0` itself). Returns the trimmed source text unchanged
/// to preserve the original representation for `AssignConst`.
fn parse_decimal_int(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let digits = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Reject leading zeros (007 is not a plain decimal integer in Tcl).
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    Some(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_decimal_int_simple() {
        assert_eq!(parse_decimal_int("42"), Some("42".into()));
        assert_eq!(parse_decimal_int("-7"), Some("-7".into()));
        assert_eq!(parse_decimal_int("+3"), Some("+3".into())); // preserves source
        assert_eq!(parse_decimal_int("0"), Some("0".into()));
    }

    #[test]
    fn parse_decimal_int_invalid() {
        assert_eq!(parse_decimal_int("abc"), None);
        assert_eq!(parse_decimal_int("3.14"), None);
        assert_eq!(parse_decimal_int(""), None);
        assert_eq!(parse_decimal_int("0x1f"), None);
        assert_eq!(parse_decimal_int("007"), None); // leading zeros rejected
    }

    #[test]
    fn extract_expr_arg_basic() {
        let aliases = HashSet::new();
        // token_text strips braces from Str tokens.
        assert_eq!(
            extract_single_expr_arg("expr {$a + $b}", &aliases),
            Some("$a + $b".into())
        );
    }

    #[test]
    fn extract_expr_arg_too_many_words() {
        let aliases = HashSet::new();
        assert_eq!(extract_single_expr_arg("expr $a + $b", &aliases), None);
    }

    #[test]
    fn extract_expr_arg_alias() {
        let mut aliases = HashSet::new();
        aliases.insert("=".into());
        assert_eq!(
            extract_single_expr_arg("= {1+2}", &aliases),
            Some("1+2".into())
        );
    }

    #[test]
    fn lower_expr_single_arg() {
        let args = vec!["{1 + 2}".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Str];
        let cmd = LoweringCommand {
            span: Span::new(0, 15),
            name: "expr",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
        };
        let result = lower_expr(&cmd);
        assert!(
            matches!(result, Some(Statement::ExprEval { .. })),
            "expected ExprEval; got {result:?}"
        );
    }

    #[test]
    fn lower_set_const() {
        let args = vec!["x".to_string(), "hello".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Str];
        let aliases = CommandAliasMap::new();
        let cmd = LoweringCommand {
            span: Span::new(0, 13),
            name: "set",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
        };
        let result = lower_set(&cmd, &aliases);
        assert!(
            matches!(result, Statement::AssignConst { .. }),
            "expected AssignConst; got {result:?}"
        );
    }

    #[test]
    fn lower_set_int() {
        let args = vec!["x".to_string(), "42".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Esc];
        let aliases = CommandAliasMap::new();
        let cmd = LoweringCommand {
            span: Span::new(0, 8),
            name: "set",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
        };
        let result = lower_set(&cmd, &aliases);
        assert!(
            matches!(result, Statement::AssignConst { .. }),
            "expected AssignConst for integer; got {result:?}"
        );
    }

    #[test]
    fn lower_upvar_with_level() {
        let args = vec!["1".to_string(), "other".to_string(), "local".to_string()];
        let single = vec![true, true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Esc, ArgTokenKind::Esc];
        let cmd = LoweringCommand {
            span: Span::new(0, 20),
            name: "upvar",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
        };
        let result = lower_upvar(&cmd);
        assert!(result.is_some());
        if let Some(Statement::Call { defs, .. }) = result {
            assert_eq!(defs, vec!["local"]);
        }
    }

    #[test]
    fn lower_return_simple() {
        let args = vec!["$result".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Var];
        let aliases = CommandAliasMap::new();
        let cmd = LoweringCommand {
            span: Span::new(0, 15),
            name: "return",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
        };
        let result = lower_return(&cmd, &aliases);
        assert!(matches!(result, Statement::Return { value: Some(_), .. }));
    }
}
