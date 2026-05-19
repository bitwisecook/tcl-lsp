//! Per-command lowering specialisations.
//!
//! Each function takes a [`LoweringCommand`] (the parsed command context)
//! and returns `Some(Statement)` if the command is handled, or `None` to
//! fall through to the default `IRCall` path.
//!
//! Ports `core/compiler/lowering_hooks/_control.py` and `_var.py`.
//! As of chunk **C43** the per-command logic is being split out into
//! [`crate::lowering::hooks`] (one file per command); this module
//! retains the dispatcher, the [`LoweringCommand`] context type, and
//! the shared helpers (`make_call`, `extract_single_expr_arg`,
//! `parse_decimal_int`).

use std::collections::HashSet;

use tcl_lexer::Span;
use tcl_registry::dialects::DialectSet;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::CommandRegistry;

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

/// Try to lower a command via a registry-described hook.
///
/// Resolves `cmd` against the registry, looks up the lowering hook
/// identifier on the matched [`tcl_registry::CommandSpec`] /
/// [`tcl_registry::SubCommand`], and dispatches to the per-hook
/// algorithm. Returns `Some(statement)` if a hook handled the
/// command; `None` to fall through to the default `IRCall` path.
#[must_use]
pub fn try_lower_hook(
    cmd: &LoweringCommand<'_>,
    aliases: &CommandAliasMap,
    registry: &CommandRegistry,
) -> Option<Statement> {
    let arg_refs: Vec<&str> = cmd.args.iter().map(String::as_str).collect();
    let resolved = registry.resolve_call(cmd.name, &arg_refs, DialectSet::empty())?;
    let hook = resolved.lowering_hook?;
    dispatch_lowering_hook(hook, cmd, aliases)
}

/// Dispatch a typed [`LoweringHookId`] to its implementation.
///
/// Public so external dispatchers (tests, future LSP feature
/// providers, peephole experiments) can reuse the same per-hook
/// implementations without duplicating the table. Per-command
/// algorithms now live under [`crate::lowering::hooks`]; this
/// function is the single point that maps registry-typed hook IDs
/// to the implementation modules.
#[must_use]
pub fn dispatch_lowering_hook(
    hook: LoweringHookId,
    cmd: &LoweringCommand<'_>,
    aliases: &CommandAliasMap,
) -> Option<Statement> {
    match hook {
        LoweringHookId::Expr => crate::lowering::hooks::control::try_lower_expr(cmd),
        LoweringHookId::Return => Some(crate::lowering::hooks::control::try_lower_return(
            cmd, aliases,
        )),
        LoweringHookId::Set => Some(lower_set(cmd, aliases)),
        LoweringHookId::Incr => Some(crate::lowering::hooks::incr::try_lower_incr(cmd)),
        LoweringHookId::AppendOrLappend => lower_append_lappend(cmd),
        LoweringHookId::Unset => Some(lower_unset(cmd)),
        LoweringHookId::Global => lower_global(cmd),
        LoweringHookId::Variable => Some(lower_variable(cmd)),
        LoweringHookId::Upvar => lower_upvar(cmd),
        // C43 structured-command hooks: the lowerer's per-command
        // methods (`lower_proc`, `lower_when`, `lower_if`,
        // `lower_switch`, `lower_for`, `lower_while`,
        // `lower_foreach`, `lower_catch`, `lower_try`, `lower_dict`,
        // `try_lower_eval_static`, `try_lower_uplevel_static`,
        // `lower_namespace_eval`) require `&mut self` to thread the
        // const-map / proc-depth / dead-code-depth / module state
        // through; the static dispatcher here can't call them.
        // Return `None` so `lower_command` falls back to its
        // existing string-pattern dispatch, which calls the
        // matching method directly.  The registry-typed
        // `LoweringHookId` lands so downstream consumers (LSP /
        // compiler explorer / coverage audit) can discover the
        // canonical hook for each spec without re-parsing names;
        // the wiring step that routes runtime dispatch through the
        // hook ID instead of the string match is the next C43
        // sub-strip.
        LoweringHookId::Proc
        | LoweringHookId::When
        | LoweringHookId::NamespaceEval
        | LoweringHookId::If
        | LoweringHookId::Switch
        | LoweringHookId::For
        | LoweringHookId::While
        | LoweringHookId::Foreach
        | LoweringHookId::Lmap
        | LoweringHookId::Catch
        | LoweringHookId::Try
        | LoweringHookId::Dict
        | LoweringHookId::Eval
        | LoweringHookId::Uplevel => None,
    }
}

/// Whether this command has `{*}` expansion on any argument.
///
/// `pub(crate)` so per-command hook modules under
/// [`crate::lowering::hooks`] can share the single canonical
/// expansion check rather than re-implementing it inline (which
/// would drift over time as the `LoweringCommand` shape evolves).
pub(crate) fn has_expansion(cmd: &LoweringCommand<'_>) -> bool {
    cmd.expand_word.is_some_and(|ew| ew.iter().any(|&e| e))
}

// ── expr ──────────────────────────────────────────────────────────
//
// Moved to `crate::lowering::hooks::control::try_lower_expr` (chunk
// **C43**). The dispatcher above delegates the `"expr"` case to
// the per-hook module.

// ── return ────────────────────────────────────────────────────────
//
// Moved to `crate::lowering::hooks::control::try_lower_return`
// (chunk **C43**). The dispatcher above delegates the `"return"`
// case to the per-hook module.

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
                // Only fold to the parsed decimal form when the
                // source spelling matches the canonical decimal
                // form **exactly** — including leading / trailing
                // whitespace.  ``set arg 0005`` must store
                // ``0005`` verbatim, and ``set x " 5"`` must store
                // ``" 5"`` verbatim — Tcl preserves the source
                // string repr; the value only shimmers to int 5
                // when used as an integer.  Folding eagerly here
                // would destroy the original spelling and break
                // ``puts $arg``, ``string length $arg``, and
                // ``expr "0005"`` which all care about it.
                // ``parse_decimal_int`` itself trims its input, so
                // comparing against ``value.trim()`` would still
                // accept leading-whitespace shapes — Copilot
                // review on PR #376 caught this.  Tighten to
                // ``value == int_val`` so any whitespace or sign
                // form different from the canonical produces no
                // fold.  Mirrors upstream commit ``31f5357f``
                // (PR #341).
                if let Some(int_val) = parse_decimal_int(value) {
                    if value.as_str() == int_val {
                        return Statement::AssignConst {
                            span: cmd.span,
                            name: name.clone(),
                            value: int_val,
                        };
                    }
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
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: vec![name],
        reads: vec![],
        reads_own_defs: true,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
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
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: !nocomplain,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
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
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
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
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
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
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: my_vars,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    })
}

// ── Helpers ───────────────────────────────────────────────────────

/// Build a generic `Statement::Call` from a lowering command.
///
/// Shared with the per-command hook modules under
/// [`crate::lowering::hooks`] so every fallback site produces the
/// same `Call` shape and stays in sync if `Statement::Call` grows
/// new fields.
pub(crate) fn make_call(cmd: &LoweringCommand<'_>) -> Statement {
    Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    }
}

/// Extract the single expression argument from a `[expr ...]` command.
///
/// Returns `Some(expr_text)` if the text is `expr <one-word>` (or an
/// expr alias), `None` otherwise.
///
/// `pub(crate)` so per-command hook modules under
/// [`crate::lowering::hooks`] (e.g. `control::try_lower_return`) can
/// share the same single-arg-extraction logic as `lower_set` here.
pub(crate) fn extract_single_expr_arg(
    text: &str,
    expr_aliases: &HashSet<String>,
) -> Option<String> {
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
    fn lower_set_preserves_leading_zeros() {
        // ``set arg 0005`` must store ``0005`` verbatim — folding
        // to ``5`` would destroy the source-level repr that
        // ``puts $arg`` / ``string length $arg`` / ``expr "0005"``
        // depend on.  Mirrors upstream commit ``31f5357f`` (PR #341).
        // The leading-zero rejection in `parse_decimal_int` already
        // kicks the value to the `AssignValue` path; the
        // `int_val == value.trim()` guard added in the same sync
        // is a redundant safety check that catches any future
        // loosening of `parse_decimal_int`.
        let m = crate::lowering::lower_to_ir("set arg 0005", &CommandRegistry::build_default());
        let value = match &m.top_level.statements[0] {
            Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. } => {
                value.clone()
            }
            other => panic!("expected AssignConst or AssignValue, got {other:?}"),
        };
        assert_eq!(value, "0005");
    }

    #[test]
    fn lower_set_preserves_leading_whitespace_in_value() {
        // ``parse_decimal_int`` trims its input, so the
        // ``int_val == value.trim()`` check in ``lower_set`` would
        // still accept leading-whitespace shapes; the tightened
        // ``value == int_val`` check rejects them and sends ``" 5"``
        // through the ``AssignValue`` path with the spelling
        // preserved (Copilot review on PR #376).  Use the brace
        // form so the segmenter passes the literal text through
        // without trimming on its end.
        let m = crate::lowering::lower_to_ir("set arg { 5}", &CommandRegistry::build_default());
        let value = match &m.top_level.statements[0] {
            Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. } => {
                value.clone()
            }
            other => panic!("expected AssignConst or AssignValue, got {other:?}"),
        };
        assert!(
            value.starts_with(' '),
            "expected leading whitespace preserved, got {value:?}",
        );
    }

    #[test]
    fn lower_set_folds_canonical_decimal_form() {
        // Canonical decimal spelling — ``5`` parses to ``5`` and
        // can fold without losing repr fidelity.
        let m = crate::lowering::lower_to_ir("set arg 5", &CommandRegistry::build_default());
        if let Statement::AssignConst { value, .. } = &m.top_level.statements[0] {
            assert_eq!(value, "5");
        } else {
            panic!("expected AssignConst");
        }
    }

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

    // `lower_return_simple` lived here in pre-C43 layouts; origin/rust
    // moved the algorithm to `crate::lowering::hooks::control` in
    // chunk C43, where the per-hook module owns its own test surface.

    /// Registry must drive the lowering-hook dispatch — `set`'s
    /// hook ID must come from [`tcl_registry::CommandSpec`], not a
    /// compiler-side name table.
    #[test]
    fn registry_set_spec_carries_lowering_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("set").expect("set is registered");
        assert_eq!(spec.lowering_hook, Some(LoweringHookId::Set));
    }

    /// Same check for `expr` — the registry-side hook ID must be
    /// the canonical source of truth, not a compiler dispatch table.
    #[test]
    fn registry_expr_spec_carries_lowering_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("expr").expect("expr is registered");
        assert_eq!(spec.lowering_hook, Some(LoweringHookId::Expr));
    }

    /// `incr`'s hook is also stamped on the registry side.
    #[test]
    fn registry_incr_spec_carries_lowering_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("incr").expect("incr is registered");
        assert_eq!(spec.lowering_hook, Some(LoweringHookId::Incr));
    }

    /// C43: structured-command specs declare their canonical
    /// `LoweringHookId` so downstream consumers (LSP / compiler
    /// explorer / coverage audit) can dispatch through the
    /// registry-typed identifier rather than re-parsing names.
    /// The dispatcher returns `None` for these hooks (the lowerer's
    /// per-command methods need `&mut self`); runtime dispatch
    /// continues to flow through `lower_command`'s string-pattern
    /// match until the wiring follow-up.
    #[test]
    fn structured_specs_declare_lowering_hook_ids() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let pairs: &[(&str, LoweringHookId)] = &[
            ("proc", LoweringHookId::Proc),
            ("when", LoweringHookId::When),
            ("if", LoweringHookId::If),
            ("switch", LoweringHookId::Switch),
            ("for", LoweringHookId::For),
            ("while", LoweringHookId::While),
            ("foreach", LoweringHookId::Foreach),
            ("lmap", LoweringHookId::Lmap),
            ("catch", LoweringHookId::Catch),
            ("try", LoweringHookId::Try),
            ("dict", LoweringHookId::Dict),
            ("eval", LoweringHookId::Eval),
            ("uplevel", LoweringHookId::Uplevel),
        ];
        for (name, hook) in pairs {
            let spec = registry.get(name).expect("registered");
            assert_eq!(
                spec.lowering_hook,
                Some(*hook),
                "{name} should declare {hook:?}",
            );
        }
        // `namespace eval` is subcommand-scoped.
        let ns = registry.get("namespace").expect("namespace registered");
        let eval_sub = ns.subcommand("eval").expect("namespace eval");
        assert_eq!(eval_sub.lowering_hook, Some(LoweringHookId::NamespaceEval),);
    }

    /// End-to-end: routing `set` through `try_lower_hook` returns
    /// the typed `AssignConst` form via the registry-resolved hook.
    #[test]
    fn try_lower_hook_routes_set_via_registry() {
        let registry = CommandRegistry::build_default();
        let aliases = CommandAliasMap::new();
        let args = vec!["x".to_string(), "hello".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Str];
        let cmd = LoweringCommand {
            span: Span::new(0, 13),
            name: "set",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
        };
        let result = try_lower_hook(&cmd, &aliases, &registry);
        assert!(
            matches!(result, Some(Statement::AssignConst { .. })),
            "expected AssignConst from registry-driven dispatch; got {result:?}",
        );
    }

    /// A command with no `lowering_hook` must return `None` from
    /// `try_lower_hook` so the caller falls through to the generic
    /// `IRCall` path.
    #[test]
    fn try_lower_hook_returns_none_for_uncovered_command() {
        let registry = CommandRegistry::build_default();
        let aliases = CommandAliasMap::new();
        let args = vec!["greetings".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Esc];
        let cmd = LoweringCommand {
            span: Span::new(0, 4),
            name: "puts",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
        };
        assert!(try_lower_hook(&cmd, &aliases, &registry).is_none());
    }
}
