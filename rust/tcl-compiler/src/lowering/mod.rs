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

//! Lower Tcl source to structured analysis IR.
//!
//! Translates a flat token stream (via the segmenter) into the tree of
//! IR nodes defined in [`crate::ir`]. Each Tcl command is pattern-matched by
//! name and converted to a typed IR statement.

use std::collections::{HashMap, HashSet};

use tcl_lexer::TokenType;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::prelude::DialectSet;
use tcl_registry::{ArgRole, CommandRegistry};

use crate::alias::{CommandAliasMap, detect_interp_alias, detect_rename, resolve_alias};
use crate::ir::{
    CommandTokens, ForeachIterator, MethodDef, MethodKind, Module, Procedure, Script, Statement,
};
use crate::lowering_hooks::{ArgTokenKind, LoweringCommand, try_lower_hook};
use crate::naming::{normalise_qualified_name, normalise_var_name};
use crate::segmenter::{
    SegmentedCommand, segment_commands, segment_commands_with_offset_and_config,
};

pub(crate) mod hooks;
mod structured;

/// Stand-in `Script` for a body past [`MAX_LOWER_NEST_DEPTH`]: a single
/// [`Statement::Barrier`] spanning `[base_offset, base_offset + len)`, so
/// downstream passes treat the unanalysed region as having unknown effects
/// (matching how a dynamic `eval`/`uplevel` body is already modelled)
/// rather than as empty/dead code.
fn over_depth_script(base_offset: u32, len: usize) -> Script {
    let end = base_offset.saturating_add(u32::try_from(len).unwrap_or(u32::MAX));
    Script::from_statements(vec![Statement::Barrier {
        span: tcl_lexer::Span::new(base_offset, end),
        reason: "nesting depth exceeds analysis limit".to_owned(),
        command: String::new(),
        canonical_command: None,
        args: Vec::new(),
        tokens: None,
    }])
}

/// Map token kind to the simplified `ArgTokenKind` used by lowering hooks.
fn arg_token_kind(kind: TokenType) -> ArgTokenKind {
    match kind {
        TokenType::Str => ArgTokenKind::Str,
        TokenType::Esc => ArgTokenKind::Esc,
        TokenType::Cmd => ArgTokenKind::Cmd,
        TokenType::Var => ArgTokenKind::Var,
        _ => ArgTokenKind::Other,
    }
}

/// Join a parent namespace with a child name.
fn join_namespace(parent: &str, child: &str) -> String {
    if child.starts_with("::") {
        return normalise_qualified_name(child);
    }
    if parent == "::" {
        return normalise_qualified_name(&format!("::{child}"));
    }
    normalise_qualified_name(&format!("{parent}::{child}"))
}

/// The namespace a procedure body lowers in — everything up to the
/// last `::` of its qualified name, or `::` for a global proc.
fn proc_namespace(qname: &str) -> String {
    let n = normalise_qualified_name(qname);
    match n.rfind("::") {
        Some(0) | None => "::".to_string(),
        Some(idx) => n[..idx].to_string(),
    }
}

/// Recognise the OO definition-command spellings. Returns
/// `Some("oo::class")` / `Some("oo::define")` when the command is one
/// of those (with or without a leading `::`), else `None`.
///
/// The stock property-/instantiation-restricting metaclasses
/// (`oo::configurable`, `oo::abstract`, `oo::singleton`) create a class with
/// the *identical* `METACLASS create NAME { body }` shape as `oo::class`, so
/// they map to the `oo::class` form — their bodies are lowered and their
/// methods lifted like any class's, rather than falling through to a barrier
/// (issue #797: a `Device` class defined with `oo::configurable` left every
/// method body unanalysed, so no object-collection types flowed).
fn oo_definition_form(command: &str, canonical: Option<&str>) -> Option<&'static str> {
    let c = canonical.unwrap_or(command);
    let c = c.strip_prefix("::").unwrap_or(c);
    match c {
        "oo::class" | "oo::configurable" | "oo::abstract" | "oo::singleton" => Some("oo::class"),
        "oo::define" => Some("oo::define"),
        _ => None,
    }
}

/// True iff the command carries the class/define shape with a static
/// braced body block:
///
/// * `oo::class create Name { body }` — body at argv index 3.
/// * `oo::define Name { body }` — body at argv index 2.
///
/// Only static braced bodies qualify — the single-method
/// `oo::define Name method m {...} {...}` and dynamic-body forms are
/// left to the default lowering. `texts` / `kinds` / `single` are the
/// full per-word arrays (index 0 = command word).
fn is_oo_definition_shape(
    form: &str,
    texts: &[String],
    kinds: &[TokenType],
    single: &[bool],
) -> bool {
    match form {
        "oo::class" => {
            texts.len() >= 4
                && texts.get(1).is_some_and(|s| s == "create")
                && word_is_static_braced(kinds, single, 3)
        }
        "oo::define" => texts.len() >= 3 && word_is_static_braced(kinds, single, 2),
        _ => false,
    }
}

/// Full-argv index of the body word for an OO definition form
/// (`oo::class create Name {body}` → 3, `oo::define Name {body}` → 2).
fn oo_body_word_idx(form: &str) -> usize {
    if form == "oo::class" { 3 } else { 2 }
}

/// True iff `command` (`::`-stripped) is `namespace` and the args are
/// the `eval CHILD {static-braced-body}` shape — the form whose body
/// the Rust lowerer evaluates inline and discards (it emits a
/// `Barrier`), so the OO post-pass must re-segment it to find any
/// classes defined directly inside the namespace.
fn is_namespace_eval_shape(
    command: &str,
    texts: &[String],
    kinds: &[TokenType],
    single: &[bool],
) -> bool {
    command.strip_prefix("::").unwrap_or(command) == "namespace"
        && texts.len() >= 4
        && texts.get(1).is_some_and(|s| s == "eval")
        && word_is_static_braced(kinds, single, 3)
}

/// True iff word `idx` (full argv index) is a single braced-literal
/// (`Str`) token.
fn word_is_static_braced(kinds: &[TokenType], single: &[bool], idx: usize) -> bool {
    idx < kinds.len() && idx < single.len() && single[idx] && kinds[idx] == TokenType::Str
}

/// `SegmentedCommand` variant of [`word_is_static_braced`]: word
/// `idx` is a single braced-literal (`Str`) token.
fn seg_word_is_static_braced(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg.argv.get(idx).is_some_and(|t| t.kind == TokenType::Str)
}

/// Word `idx` is a single substitution-free literal: a `{braced}` (`Str`) word
/// or a plain bareword (`Esc`) — exactly the body words C's `TclCompileIfCmd`
/// inlines. A word carrying `$var` / `[cmd]` substitution, or a multi-token
/// concatenation like `$x1$x2`, is not (it must be substituted then evaluated
/// as a script by the runtime `if` command).
fn seg_word_is_static_literal(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg
            .argv
            .get(idx)
            .is_some_and(|t| matches!(t.kind, TokenType::Str | TokenType::Esc))
}

/// True iff `nm` is a static instance-variable name (not a `$var` /
/// `[cmd]` substitution and not an option flag). Dynamic names are
/// skipped — conservative.
fn is_instance_var_name(nm: &str) -> bool {
    !nm.is_empty() && !nm.contains('$') && !nm.contains('[') && !nm.starts_with('-')
}

/// True iff a statement's command (preferring its canonical form,
/// `::`-stripped) equals `bare`, tolerating the IR's
/// `canonical_command == None` for un-aliased commands.
fn canonical_matches(command: &str, canonical: Option<&str>, bare: &str) -> bool {
    let c = canonical.unwrap_or(command);
    c.strip_prefix("::").unwrap_or(c) == bare
}

/// Qualify a procedure name relative to a namespace.
fn qualify_proc_name(namespace: &str, proc_name: &str) -> String {
    if proc_name.starts_with("::") {
        return normalise_qualified_name(proc_name);
    }
    if namespace == "::" {
        return normalise_qualified_name(&format!("::{proc_name}"));
    }
    normalise_qualified_name(&format!("{namespace}::{proc_name}"))
}

/// Parse a Tcl parameter / variable list into its names.
///
/// The list is a braced word, so a `\<newline>` line continuation collapses to
/// a single space first (the one substitution braces permit) — without it a
/// multi-line var-list (`foreach {a b\<nl>  c} …`, a wrapped `proc` param list)
/// would keep the `\` glued to the preceding name (`b\`) and bind the wrong
/// variable.
fn parse_param_names(param_str: &str) -> Vec<String> {
    let collapsed = tcl_syntax::backslash::collapse_brace_continuations_str(param_str);
    let mut params = Vec::new();
    let text = collapsed.trim();
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Skip whitespace.
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        if bytes[i] == b'{' {
            // Braced parameter — find matching close brace.
            let mut level = 1i32;
            i += 1;
            let start = i;
            while i < bytes.len() && level > 0 {
                match bytes[i] {
                    b'{' => level += 1,
                    b'}' => level -= 1,
                    _ => {}
                }
                i += 1;
            }
            let inner = &text[start..i.saturating_sub(1)].trim();
            if !inner.is_empty()
                && let Some(name) = inner.split_whitespace().next()
            {
                params.push(name.to_owned());
            }
        } else {
            // Bare word parameter.
            let start = i;
            while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                i += 1;
            }
            let word = &text[start..i];
            if !word.is_empty() {
                params.push(word.to_owned());
            }
        }
    }
    params
}

/// Return `Some(true)` / `Some(false)` if *`expr_text`* is a
/// literal Tcl boolean, or `None` when the condition is not a
/// simple literal. Tolerates surrounding whitespace and is
/// case-insensitive (matches `Tcl_GetBoolean`).
pub(crate) fn static_bool(expr_text: &str) -> Option<bool> {
    let stripped = expr_text.trim();
    match stripped {
        "0" => Some(false),
        "1" => Some(true),
        // Boolean words, incl. unique-prefix spellings (`tr`, `ye`, `of`).
        _ => tcl_syntax::boolean::parse_boolean_word(stripped),
    }
}

/// Parse the level argument of an `uplevel` call into a frame
/// shift. Accepts the canonical positive-integer form (`uplevel 1
/// body`, `uplevel 3 body`) and the global form (`#0` / `#N`),
/// returning `None` when the argument is dynamic (`$lvl`, `[expr
/// {...}]`) or otherwise unparseable.
///
/// The returned shift is normalised so callers can decide whether to
/// route the call through [`Statement::UpFrame`] (positive shifts)
/// or fall back to a barrier.
fn parse_uplevel_level(text: &str) -> Option<i32> {
    if let Some(rest) = text.strip_prefix('#') {
        return rest.parse::<i32>().ok().map(|n| -n);
    }
    text.parse::<i32>().ok()
}

/// Return `(name, literal)` when *seg* is `set name {literal}`.
///
/// The LHS must be a plain bareword `Esc` token (no substitutions,
/// array index, or namespace qualifier) so we only ever track
/// proc-local scalars. The RHS must be a single brace-string `Str`
/// token.
fn set_literal_body(seg: &SegmentedCommand) -> Option<(String, String)> {
    if seg.name() != "set" || seg.args().len() != 2 {
        return None;
    }
    let arg_tokens = seg.arg_tokens();
    if arg_tokens.len() < 2 {
        return None;
    }
    if !seg.single_token_word.iter().take(3).all(|&b| b) {
        return None;
    }
    let name_tok = arg_tokens[0];
    let value_tok = arg_tokens[1];
    if value_tok.kind != TokenType::Str {
        return None;
    }
    if name_tok.kind != TokenType::Esc {
        return None;
    }
    let name = &seg.args()[0];
    if name.is_empty()
        || name.contains('$')
        || name.contains('[')
        || name.contains('(')
        || name.contains("::")
    {
        return None;
    }
    let value = seg.args()[1].clone();
    Some((normalise_var_name(name).to_string(), value))
}

/// If *`cmd_text`* is `list lit1 lit2 ...` with all-literal
/// arguments, return the body text the list would evaluate to —
/// otherwise `None`.
///
/// Literal means `Esc` / `Str` token only: no `$var` substitution,
/// no nested command substitution. `Str` (`{...}`) tokens are
/// re-braced in the synthesised body so list-canonicalisation
/// stays correct (we trust the segmenter's `single_token_word`
/// flag plus the absence of `$` / `[` in `Esc` text).
fn eval_list_literal_body(cmd_text: &str) -> Option<String> {
    let inner = segment_commands(cmd_text);
    if inner.len() != 1 {
        return None;
    }
    let inner_cmd = &inner[0];
    if inner_cmd.texts.is_empty() || inner_cmd.texts[0] != "list" {
        return None;
    }
    let argv = inner_cmd.arg_tokens();
    let texts = inner_cmd.args();
    let single = &inner_cmd.single_token_word;
    // Each element after ``list`` must be a single-token literal.
    for (i, tok) in argv.iter().enumerate() {
        // ``single`` is per-word over the full argv (including
        // the command word at index 0) — argv here starts at
        // index 1 of the original argv, so the matching single-
        // token-word index is ``i + 1``.
        if !single.get(i + 1).copied().unwrap_or(false) {
            return None;
        }
        if !matches!(tok.kind, TokenType::Esc | TokenType::Str) {
            return None;
        }
        if tok.kind == TokenType::Esc {
            let text = &texts[i];
            if text.contains('$') || text.contains('[') {
                return None;
            }
        }
    }
    let mut parts: Vec<String> = Vec::with_capacity(argv.len());
    for (i, tok) in argv.iter().enumerate() {
        let text = &texts[i];
        if tok.kind == TokenType::Str {
            parts.push(format!("{{{text}}}"));
        } else {
            parts.push(text.clone());
        }
    }
    Some(parts.join(" "))
}

/// Drop const-map entries that *stmt* may have overwritten.
/// Straight-line assignments pop just the named variable;
/// structured IR and barriers conservatively clear the whole map
/// because their child scopes (or runtime side effects) could
/// touch any tracked name.
/// Token-level check that a relaxed-eval /
/// relaxed-uplevel body is free of nested dynamic-shape barriers.
///
/// When the eval/uplevel hooks consider relaxing a braced-literal
/// body to inline IR, they first walk the body's command words and
/// reject any nested `eval`/`uplevel` whose own body argument is
/// substitution-bearing (`$var` / `[cmd]` / multi-token).  Without
/// this gate, a static braced `eval {uplevel 1 $x}` would relax to
/// IR that runs a compiled `uplevel` with a still-dynamic body.
///
/// The walk is deliberately shallow and token-based:
///
/// 1. Segment the body into commands.
/// 2. For each command whose name is in the dynamic-barrier set,
///    inspect its own body-shaped argument (last arg).  If it isn't
///    a `Str` token (a braced literal), poison.
/// 3. Recurse into nested braced bodies and into braced-arg shapes
///    of non-barrier commands so a nested
///    `if { … } { eval $x }` still trips the gate.
///
/// Returns `true` when the body contains a nested dynamic-shape
/// barrier (the caller should fall back to `IRBarrier`); `false`
/// when the body is safe to relax.
fn body_has_dynamic_barrier(body_text: &str, registry: &CommandRegistry) -> bool {
    use tcl_lexer::TokenType;
    let commands = segment_commands(body_text);
    for sc in &commands {
        if sc.argv.is_empty() || sc.texts.is_empty() {
            continue;
        }
        let name = sc.texts[0].as_str();
        // A "dynamic barrier" command evaluates a script in another
        // frame (`eval` / `uplevel`).  Sourced from the registry's
        // `EVALUATES_CODE` trait (stamped on exactly those two)
        // rather than a hardcoded name list; strip any leading `::`
        // so the fully-qualified spellings resolve too.
        let is_barrier = registry
            .get(name.strip_prefix("::").unwrap_or(name))
            .is_some_and(|s| {
                s.traits
                    .contains(tcl_registry::prelude::Traits::EVALUATES_CODE)
            });
        if !is_barrier {
            // Recurse into braced args of non-barrier commands so
            // nested barriers still trip the gate.
            for (i, tok) in sc.argv.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                if tok.kind != TokenType::Str {
                    continue;
                }
                let arg_text = sc.texts.get(i).map_or("", String::as_str);
                if body_has_dynamic_barrier(arg_text, registry) {
                    return true;
                }
            }
            continue;
        }
        // Name is a barrier — inspect its own body.
        let args = &sc.texts[1..];
        let arg_tokens = &sc.argv[1..];
        if args.is_empty() {
            // Malformed: no body. Poison so the outer hook falls
            // back to IRBarrier (runtime can report the error).
            return true;
        }
        // For ``uplevel`` skip the level arg if literal.
        let body_idx = if name == "uplevel" || name == "::uplevel" {
            let level = &args[0];
            let level_is_int = !level.is_empty()
                && (level.starts_with('#')
                    || level
                        .trim_start_matches('-')
                        .chars()
                        .all(|c| c.is_ascii_digit()));
            if level_is_int {
                if args.len() < 2 {
                    return true;
                }
                let level_tok = &arg_tokens[0];
                if level_tok.kind != TokenType::Esc {
                    return true;
                }
                args.len() - 1
            } else {
                args.len() - 1
            }
        } else {
            args.len() - 1
        };
        let body_tok_nested = &arg_tokens[body_idx];
        if body_tok_nested.kind != TokenType::Str {
            return true;
        }
        // Recurse into the literal nested body.
        if body_has_dynamic_barrier(&args[body_idx], registry) {
            return true;
        }
    }
    false
}

fn invalidate_const_map_for(stmt: &Statement, scope: &mut HashMap<String, String>) {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => {
            scope.remove(normalise_var_name(name));
        }
        Statement::Call { defs, .. } => {
            for v in defs {
                scope.remove(normalise_var_name(v));
            }
        }
        Statement::Barrier { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => {
            scope.clear();
        }
        Statement::Return { .. } | Statement::ExprEval { .. } => {}
    }
}

/// A memoised per-procedure body-lowering callback: `(offset-0 body text,
/// namespace) → offset-0 body [`Script`]`.  See [`Lowerer::with_body_cache`].
type BodyCacheFn<'a> = dyn Fn(&str, &str) -> Script + 'a;

/// Which lowering pass a [`Lowerer`] performs.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompileTarget {
    /// The analysis IR: structured control flow kept everywhere, every
    /// registry-driven hook active. What LSP consumers (diagnostics,
    /// navigation) lower against.
    #[default]
    Analysis,
    /// The VM-faithful bytecode pass ([`lower_to_ir_for_bytecode`]):
    /// constructs the bytecode backend can't compile correctly (a bare
    /// `try`, a directly-nested `foreach`/`lmap`) barrier to a runtime
    /// dispatch instead of their structured IR; every other hook active.
    Bytecode,
    /// The bytecode pass with every registry-driven inline/structured
    /// lowering hook ALSO suppressed ([`lower_to_ir_traced`]): every command
    /// falls straight to [`Lowerer::lower_default`] — a plain runtime
    /// dispatch (`Statement::Call`/`Statement::Barrier` with a non-empty
    /// `command`, both of which [`crate::codegen`] emits as `push words;
    /// INVOKE`), never a typed statement (`AssignConst`/`Incr`/`Return`/…)
    /// or a structured control-flow block (`If`/`While`/`Foreach`/`Switch`/
    /// `Catch`/`Try`/…). Used to recompile a proc/script body so execution
    /// traces (`enterstep`/`leavestep`) observe every command in it — the
    /// compiler-side half of issue #946's step-trace parity fix; see
    /// [`tcl_runtime_api::CompileService::compile_traced`]. Never set for
    /// the primary module compile — only for a standalone "compile this body
    /// text as a script" call, so a proc's own `proc name args body`
    /// DEFINITION always module-compiles normally; only its *body* is
    /// affected when recompiled under this target.
    BytecodeTraced,
}

impl CompileTarget {
    /// Whether this pass targets the bytecode backend (barriers
    /// backend-incompatible structured forms) — [`Self::Bytecode`] or
    /// [`Self::BytecodeTraced`].
    pub(crate) fn is_bytecode(self) -> bool {
        matches!(self, Self::Bytecode | Self::BytecodeTraced)
    }

    /// Whether this pass suppresses every inline/structured hook —
    /// [`Self::BytecodeTraced`] only.
    pub(crate) fn is_trace_visible(self) -> bool {
        self == Self::BytecodeTraced
    }
}

/// The lowering engine — accumulates procedures and IR statements.
pub struct Lowerer<'r> {
    /// Output module being built.
    pub module: Module,
    /// Command alias table built during lowering.
    aliases: CommandAliasMap,
    /// Event handler occurrence counts (for `when` numbering).
    when_counts: HashMap<String, u32>,
    /// Monotonic counter for synthetic *body unit* names (`apply` lambdas and
    /// `namespace eval` bodies), so each registers under a unique qualified
    /// name in [`Module::body_units`]. Deterministic in source order, so the
    /// fresh and incremental lowerings produce identical names.
    body_unit_count: u32,
    /// Whether we're inside a `namespace eval` body.
    in_namespace_eval: bool,
    /// Command registry for arg-role queries.
    registry: &'r CommandRegistry,
    /// Per-script const-map stack. Each scope tracks
    /// proc-local variables assigned a brace-string literal so
    /// later `eval $var` / `uplevel 1 $var` calls can fold the
    /// body in at lowering time. Active only when
    /// `proc_depth > 0` — top-level / `namespace eval` scopes
    /// write globals or namespace vars whose values can be
    /// observed and mutated by other code, so const-propagating
    /// them is unsound.
    const_map_stack: Vec<HashMap<String, String>>,
    /// Depth of `proc` / `when` body lowerings currently in
    /// flight. A positive value enables the const-map.
    proc_depth: u32,
    /// `namespace import` directives observed at lowering time.
    /// Recorded as `(context_namespace, absolute_pattern)`
    /// pairs and copied into `Module::namespace_imports` at the
    /// end of lowering. Order is preserved.
    namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives observed at lowering time.
    /// Recorded as `(context_namespace, pattern)` pairs
    /// and copied into `Module::namespace_exports` at the end of
    /// lowering. Order is preserved.
    namespace_exports: Vec<(String, String)>,
    /// Depth of statically-dead branches currently being lowered.
    /// `if {0} {…}` / `if {1} {…} else {…}` bump this
    /// around the dead body so any `namespace import` /
    /// `namespace export` directives found inside don't register
    /// with the module-level tables. The IR for the dead code is
    /// still produced so consumers that walk the tree by syntactic
    /// offset see the original structure.
    pub(crate) dead_code_depth: u32,
    /// `true` while lowering a `TclOO` method body. A `proc`
    /// (or `namespace eval`-lifted proc) defined inside a method
    /// body is created at method-call time in the global namespace,
    /// NOT at class-definition time — so it must not be lifted into
    /// `module.procedures` (codegen would otherwise emit it
    /// unconditionally at script load). The method body is still
    /// lowered for analysis; only the global registration is
    /// suppressed.
    suppress_proc_register: bool,
    /// Lexer config for the document's dialect, threaded into every
    /// body re-segmentation so `{*}` expansion (off for Tcl 8.4 /
    /// iRules) and the iRules `}{` ghost SEP are honoured rather than
    /// always assuming the Tcl-8.5+ default.  Defaults to
    /// `LexerConfig::default()`; production callers thread the active
    /// dialect via [`Lowerer::with_config`] / [`lower_to_ir_with_config`].
    config: tcl_lexer::LexerConfig,
    /// Which lowering pass this instance performs — folds what would
    /// otherwise be two related bool fields (`for_bytecode`, `trace_visible`)
    /// into one three-state enum (`clippy::struct_excessive_bools`); see
    /// [`CompileTarget`].
    pub(crate) target: CompileTarget,
    /// Optional memoised per-procedure body lowering (SRV-INCREMENTAL Task 3).
    /// When set, a **top-level** `proc`'s static literal body is lowered through
    /// this callback `(offset-0 body text, namespace) -> offset-0 body Script`
    /// (the caller rebases by the body offset) instead of `lower_body`, so an
    /// unchanged proc's body IR is reused across edits.  Set only for
    /// **context-free** files (no `namespace eval`/`import`/`export`, alias,
    /// `oo::`, `when`, or nested `proc`) where isolated lowering is byte-identical;
    /// `None` ⇒ the normal whole-file lowering.  See [`lower_to_ir_with_body_cache`].
    body_cache: Option<&'r BodyCacheFn<'r>>,
    /// Current `lower_script` / `lower_body` recursion depth, bounded by
    /// [`MAX_LOWER_NEST_DEPTH`] so deeply-nested bodies cannot overflow the
    /// stack. Mirrors [`crate::cfg_builder::CfgBuilder`]'s `depth` field —
    /// lowering runs *before* CFG construction, so an unguarded recursion
    /// here reaches the same crash first and makes the CFG builder's own
    /// cap moot (issue #996).
    nest_depth: usize,
}

/// Maximum nesting depth for the recursive `lower_script` / `lower_body`
/// descent (`lower_script` ↔ `lower_body` ↔ `lower_segmented` ↔
/// `lower_command` are mutually recursive with one Rust frame group per
/// braced-body nesting level). Matches
/// [`crate::cfg_builder::MAX_LOWER_DEPTH`] and
/// `tcl_compiler::analyser::commands::MAX_BODY_DEPTH` — all three
/// independently-recursive walkers over the same source cap at the same
/// depth, so no consumer of this crate depends on one pass reaching a
/// deeper nesting level than another.
const MAX_LOWER_NEST_DEPTH: usize = 256;

impl<'r> Lowerer<'r> {
    /// Create a new lowerer with the default (Tcl-8.5+) lexer config.
    #[must_use]
    pub fn new(registry: &'r CommandRegistry) -> Self {
        Self::with_config(registry, tcl_lexer::LexerConfig::default())
    }

    /// Create a lowerer with an explicit dialect [`tcl_lexer::LexerConfig`]
    /// (see the `config` field).
    #[must_use]
    pub fn with_config(registry: &'r CommandRegistry, config: tcl_lexer::LexerConfig) -> Self {
        Self {
            module: Module::default(),
            aliases: CommandAliasMap::new(),
            when_counts: HashMap::new(),
            body_unit_count: 0,
            in_namespace_eval: false,
            registry,
            const_map_stack: Vec::new(),
            proc_depth: 0,
            namespace_imports: Vec::new(),
            namespace_exports: Vec::new(),
            dead_code_depth: 0,
            suppress_proc_register: false,
            config,
            target: CompileTarget::Analysis,
            body_cache: None,
            nest_depth: 0,
        }
    }

    /// Install a memoised per-procedure body-lowering callback (see
    /// [`body_cache`](Self::body_cache) and [`lower_to_ir_with_body_cache`]).
    #[must_use]
    pub fn with_body_cache(mut self, cache: &'r BodyCacheFn<'r>) -> Self {
        self.body_cache = Some(cache);
        self
    }

    /// Mark this as the bytecode/VM compile path, barriering constructs the
    /// backend can't compile (see [`CompileTarget::Bytecode`]). A no-op if
    /// [`Self::trace_visible`] already upgraded this to
    /// [`CompileTarget::BytecodeTraced`] (still bytecode-backend, just also
    /// trace-visible) — order-independent whichever builder call comes first.
    #[must_use]
    pub fn for_bytecode_backend(mut self) -> Self {
        if self.target == CompileTarget::Analysis {
            self.target = CompileTarget::Bytecode;
        }
        self
    }

    /// Suppress every inline/structured lowering hook so the whole lowering
    /// pass produces plain runtime dispatches (see
    /// [`CompileTarget::BytecodeTraced`]). Implies the bytecode backend.
    #[must_use]
    pub fn trace_visible(mut self) -> Self {
        self.target = CompileTarget::BytecodeTraced;
        self
    }

    /// Lower a complete source string to an IR module.
    pub fn lower(&mut self, source: &str) -> &Module {
        self.module.top_level = self.lower_script(source, "::");
        // Surface namespace import / export directives onto
        // the module for downstream consumers (codegen import
        // resolution, future warning passes).
        self.module.namespace_imports = std::mem::take(&mut self.namespace_imports);
        self.module.namespace_exports = std::mem::take(&mut self.namespace_exports);
        &self.module
    }

    /// Lower a source-text literal into a [`Script`] without
    /// installing it as the module top-level. Used by passes that
    /// need to lower a sub-script (e.g. the
    /// [`crate::inline_uplevel`] rewriter materialising a
    /// brace-literal callsite body).
    pub fn lower_into_script(&mut self, source: &str, namespace: &str) -> Script {
        self.lower_script(source, namespace)
    }

    /// Lower a source string to an IR script.
    ///
    /// Depth-guarded entry to the recursive lowering, alongside
    /// [`Self::lower_body`] — every nested body re-enters through one of
    /// these two, so bounding both here caps the whole `lower_script` ↔
    /// `lower_body` ↔ `lower_segmented` ↔ `lower_command` recursion. At the
    /// cap we stop descending and lower the remaining text as a single
    /// opaque [`Statement::Barrier`] rather than silently dropping it —
    /// downstream passes (SCCP, dead-code elimination, …) must treat
    /// unanalysed content as having unknown effects, not as empty/dead.
    fn lower_script(&mut self, source: &str, namespace: &str) -> Script {
        self.nest_depth += 1;
        if self.nest_depth > MAX_LOWER_NEST_DEPTH {
            self.nest_depth -= 1;
            return over_depth_script(0, source.len());
        }
        let result = self.lower_script_inner(source, namespace);
        self.nest_depth -= 1;
        result
    }

    fn lower_script_inner(&mut self, source: &str, namespace: &str) -> Script {
        let commands = segment_commands_with_offset_and_config(source, 0, self.config);
        self.const_map_stack.push(HashMap::new());
        let stmts = self.lower_segmented(&commands, namespace);
        self.const_map_stack.pop();
        Script::from_statements(stmts)
    }

    /// Lower a body argument (inside braces/brackets) to an IR script.
    ///
    /// Inherits the parent scope's const-map so a child body can
    /// still relax its
    /// `eval` / `uplevel` against literals bound in the enclosing
    /// scope (`set body {literal}; catch {uplevel 1 $body}` is the
    /// canonical example).
    ///
    /// Depth-guarded the same way as [`Self::lower_script`] — see its doc
    /// comment.
    fn lower_body(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        self.nest_depth += 1;
        if self.nest_depth > MAX_LOWER_NEST_DEPTH {
            self.nest_depth -= 1;
            return over_depth_script(base_offset, text.len());
        }
        let result = self.lower_body_inner(text, base_offset, namespace);
        self.nest_depth -= 1;
        result
    }

    fn lower_body_inner(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let commands = segment_commands_with_offset_and_config(text, base_offset, self.config);
        let inherited = self.const_map_stack.last().cloned().unwrap_or_default();
        self.const_map_stack.push(inherited);
        let stmts = self.lower_segmented(&commands, namespace);
        self.const_map_stack.pop();
        Script::from_statements(stmts)
    }

    /// Lower a list of segmented commands to IR statements.
    fn lower_segmented(
        &mut self,
        segments: &[SegmentedCommand],
        namespace: &str,
    ) -> Vec<Statement> {
        let mut stmts = Vec::new();
        for seg in segments {
            if seg.is_partial {
                stmts.push(Statement::Barrier {
                    span: seg.span,
                    reason: "incomplete command".into(),
                    command: String::new(),
                    canonical_command: None,
                    args: vec![],
                    tokens: None,
                });
                continue;
            }
            if let Some(stmt) = self.lower_command(seg, namespace) {
                self.update_const_map(seg, &stmt);
                stmts.push(stmt);
            }
        }
        stmts
    }

    /// Maintain the per-scope const-map after a command lowers.
    ///
    /// Populates a `name → braced-literal` entry when *seg* is a
    /// `set var {literal}` shape with a plain bareword LHS;
    /// otherwise invalidates entries that *stmt* may have written.
    /// Gated on `proc_depth > 0` — globals / namespace vars at
    /// top-level or inside `namespace eval` cannot be safely
    /// const-propagated.
    fn update_const_map(&mut self, seg: &SegmentedCommand, stmt: &Statement) {
        if self.proc_depth == 0 {
            return;
        }
        let Some(scope) = self.const_map_stack.last_mut() else {
            return;
        };

        if let Some((name, value)) = set_literal_body(seg) {
            scope.insert(name, value);
            return;
        }

        invalidate_const_map_for(stmt, scope);
    }

    /// Resolve a `$var` / `${var}` body word against the current
    /// const-map and return the bound literal, or `None` if the
    /// word is not a pure single-token variable reference, the
    /// variable has no known literal binding, or we are not inside
    /// a `proc` body (top-level / `namespace eval` scopes are out
    /// of scope for this optimisation).
    fn const_map_lookup(&self, word: &str) -> Option<String> {
        if self.proc_depth == 0 {
            return None;
        }
        let scope = self.const_map_stack.last()?;
        let inner = if let Some(rest) = word.strip_prefix("${") {
            let inner = rest.strip_suffix('}')?;
            if inner.contains('$')
                || inner.contains('[')
                || inner.contains('{')
                || inner.contains('(')
            {
                return None;
            }
            inner
        } else {
            let rest = word.strip_prefix('$')?;
            if rest.is_empty()
                || rest.contains('(')
                || rest.contains('$')
                || rest.contains('[')
                || rest.contains('{')
                || rest.starts_with(':')
            {
                return None;
            }
            rest
        };
        scope.get(inner).cloned()
    }

    /// Build a `CommandTokens` snapshot from a segmented command.
    fn cmd_tokens(seg: &SegmentedCommand) -> CommandTokens {
        CommandTokens {
            argv: seg.argv.iter().map(|t| t.span).collect(),
            argv_texts: seg.texts.clone(),
            argv_kinds: seg.argv.iter().map(|t| t.kind).collect(),
            single_token_word: seg.single_token_word.clone(),
            all_tokens: seg.all_tokens.iter().map(|t| t.span).collect(),
            expand_word: seg.expand_word.clone(),
        }
    }

    /// Extract arg token kinds for the lowering hooks.
    fn arg_kinds(seg: &SegmentedCommand) -> Vec<ArgTokenKind> {
        seg.argv
            .iter()
            .skip(1)
            .map(|t| arg_token_kind(t.kind))
            .collect()
    }

    /// Detect ``namespace import ?-force? pattern...``
    /// and ``namespace export pattern...``. Records absolute
    /// patterns only.  Skips `{*}`-expanded calls and statically-
    /// dead branches.
    fn record_namespace_directives(
        &mut self,
        cmd_name: &str,
        args: &[String],
        seg: &SegmentedCommand,
        namespace: &str,
    ) {
        if cmd_name != "namespace" || args.len() < 2 || self.dead_code_depth != 0 {
            return;
        }
        let no_expand = seg
            .expand_word
            .as_ref()
            .is_none_or(|ew| !ew.iter().any(|&e| e));
        if !no_expand {
            return;
        }
        if args[0] == "import" {
            let mut i = 1usize;
            while i < args.len() && args[i].starts_with('-') {
                i += 1;
            }
            for pat in &args[i..] {
                if pat.starts_with("::") && pat[2..].contains("::") {
                    self.namespace_imports
                        .push((namespace.to_string(), pat.clone()));
                }
            }
        } else if args[0] == "export" {
            let mut i = 1usize;
            // ``-clear`` is the only flag for ``namespace export``.
            while i < args.len() && args[i].starts_with('-') {
                i += 1;
            }
            for pat in &args[i..] {
                self.namespace_exports
                    .push((namespace.to_string(), pat.clone()));
            }
        }
    }

    /// `{*}` expansion on structured commands lowers to a barrier so
    /// downstream analyses can't reason about the expanded form.
    /// Returns `Some(barrier)` when the gate trips; `None` otherwise.
    fn structured_expand_barrier(
        cmd_name: &str,
        args: &[String],
        seg: &SegmentedCommand,
    ) -> Option<Statement> {
        let structured = matches!(
            cmd_name,
            "proc"
                | "when"
                | "namespace"
                | "if"
                | "switch"
                | "for"
                | "while"
                | "foreach"
                | "foreach_in_collection"
        );
        if !structured {
            return None;
        }
        if !seg
            .expand_word
            .as_ref()
            .is_some_and(|ew| ew.iter().any(|&e| e))
        {
            return None;
        }
        Some(Statement::Barrier {
            span: seg.span,
            reason: format!("{cmd_name} with argument expansion"),
            command: cmd_name.into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        })
    }

    /// Try to dispatch *`cmd_name`* through the registry's typed
    /// [`LoweringHookId`] for structured commands dispatched from
    /// [`lower_command`](Self::lower_command).  Returns
    /// `Some(stmt)` when the hook ID matches a structured form;
    /// returns `None` otherwise so `lower_command` falls back to
    /// [`lower_default`](Self::lower_default).
    ///
    /// Every structured form routes through
    /// `resolve_call().lowering_hook`, and unmatched commands flow
    /// straight to `lower_default`.  Forms with shape preconditions
    /// (`proc`, `namespace eval`, `foreach`, `lmap`, `dict`,
    /// `when`, `foreachLine`) return `None` on a failed
    /// precondition so `lower_default` catches them.
    ///
    /// Dispatching through the hook ID picks up two benefits over a
    /// bare name match:
    ///
    /// 1. Canonical resolution via [`CommandRegistry::resolve_call`]
    ///    instead of bare name comparison, so any future spec
    ///    that aliases an existing form (e.g. an iRules-specific
    ///    `eval` variant) automatically dispatches correctly
    ///    when its `lowering_hook` is stamped.
    /// 2. The hook ID is the canonical key consumed by downstream
    ///    audit / LSP / compiler-explorer surfaces.
    fn try_dispatch_structured_hook(
        &mut self,
        cmd_name: &str,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Statement> {
        let args = seg.args();
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let resolved = self
            .registry
            .resolve_call(cmd_name, &arg_refs, DialectSet::empty())?;
        match resolved.lowering_hook? {
            // Static-body uplevel.  Match `uplevel 1 {body}`,
            // `uplevel #0 {body}`, and the canonical no-level form
            // `uplevel {body}` (level defaults to 1) when the body
            // arg is a brace-string literal token.  Dynamic forms
            // (`uplevel 1 $body` / `uplevel $lvl {body}`) fall
            // through to the default lowering so a runtime
            // `Call` / `Barrier` carries the unresolved arguments.
            LoweringHookId::Uplevel => Some(
                self.try_lower_uplevel_static(seg, namespace)
                    .unwrap_or_else(|| self.lower_default(seg, namespace)),
            ),

            // `eval $body` / `eval {body}` with a literal /
            // const-folded body relaxes to a `Statement::Block` so
            // downstream analyses see the inlined script.  Dynamic
            // bodies (`eval $dyn` with no const-map binding,
            // `eval [cmd]`) fall through to the default barrier
            // dispatch.
            LoweringHookId::Eval => Some(
                self.try_lower_eval_static(seg, namespace)
                    .unwrap_or_else(|| self.lower_default(seg, namespace)),
            ),

            // `apply {{params} body ?ns?} …` — walk the braced body so nested
            // definitions register (like `namespace eval`), keeping the call a
            // runtime barrier because the body runs in a separate frame.
            LoweringHookId::Apply => Some(self.lower_apply(seg, namespace)),

            // `array for {k v} arr body` (Tcl 9.0) — iterate array entries.
            // Like `apply`, the call stays a runtime barrier (C Tcl invokes
            // `::tcl::array::for` with the body as an unparsed literal), but a
            // braced literal body is walked in a fresh frame bound to the two
            // loop variables so it is analysable.
            LoweringHookId::ArrayFor => Some(self.lower_array_for(seg, namespace)),

            // Straightforward control-flow forms.  Each is a
            // single-method dispatch with no arity / shared-method /
            // subcommand complications.
            LoweringHookId::If => Some(self.lower_if(seg, namespace)),
            LoweringHookId::Switch => Some(self.lower_switch(seg, namespace)),
            LoweringHookId::For => Some(self.lower_for(seg, namespace)),
            LoweringHookId::While => Some(self.lower_while(seg, namespace)),
            LoweringHookId::Catch => Some(self.lower_catch(seg, namespace)),
            LoweringHookId::Try => Some(self.lower_try(seg, namespace)),

            // Forms with shape preconditions.  Calls with the wrong
            // arity / shape must fall through to `lower_default`
            // instead of crashing the dedicated lowerer, so each arm
            // returns `None` on precondition failure and
            // `lower_command` then falls through to `lower_default`.
            //
            // `proc name params body` — exactly three args and
            // at least three token slices (the body needs to
            // be a real token, not synthesised whitespace).
            LoweringHookId::Proc => {
                if args.len() == 3 && seg.arg_tokens().len() >= 3 {
                    Some(self.lower_proc(seg, namespace))
                } else {
                    None
                }
            }
            // `namespace eval ns body` — the subcommand match
            // is already handled by `resolve_call`, so the
            // hook only fires for the `eval` subcommand.  Keep
            // the arity / token-count guards (a tokenless body
            // would derail body lowering).
            LoweringHookId::NamespaceEval => {
                if args.len() >= 3 && seg.arg_tokens().len() >= 3 {
                    Some(self.lower_namespace_eval(seg, namespace))
                } else {
                    None
                }
            }
            // `foreach vars list body` / `lmap vars list body`
            // — share `lower_foreach(... is_lmap)`.  The
            // dedicated lowerer handles its own shape errors,
            // so no precondition here.
            LoweringHookId::Foreach => Some(self.lower_foreach(seg, namespace, false)),
            LoweringHookId::Lmap => Some(self.lower_foreach(seg, namespace, true)),
            // `dict <subcommand> ...` — must have at least one
            // arg so the subcommand can be picked.  Bare `dict`
            // falls through to `lower_default`.
            LoweringHookId::Dict => {
                if args.is_empty() {
                    None
                } else {
                    Some(self.lower_dict(seg, namespace))
                }
            }

            // `when EVENT ?priority N? body` — iRules event
            // handler.  The hook stamp lives on the `when` spec
            // in `tcl-registry/src/commands/irules/when.rs`,
            // which `load_dialect(DialectSet::IRULES)` brings
            // into the registry.  Production callers (LSP
            // server / Python bindings) load the active dialect
            // before lowering; tests that lower iRule code call
            // `registry.load_irules()` explicitly (see
            // `irules_checks::tests::registry`).  Callers that
            // lower iRule code against a vanilla `build_default()`
            // registry now silently flow through to
            // `lower_default` — that path was always a
            // misconfiguration; the dialect needs to match the
            // source.
            LoweringHookId::When => {
                if args.len() >= 2 && seg.arg_tokens().len() >= 2 {
                    Some(self.lower_when(seg, namespace))
                } else {
                    None
                }
            }
            // `foreachLine varName filename body` — Tcl 9.0
            // (TIP 670).  Always registered in `build_default()`
            // via `tcl::foreachline::spec`, so no dialect-load
            // dance is needed.  The `lower_foreach_line`
            // emitter handles its own shape errors and
            // dynamic-body fallback, but the `args.len() == 3`
            // guard keeps an under- or over-argued `foreachLine`
            // flowing to `lower_default` instead of triggering a
            // barrier inside the dedicated emitter.
            LoweringHookId::ForeachLine => {
                if args.len() == 3 {
                    Some(self.lower_foreach_line(seg, namespace))
                } else {
                    None
                }
            }

            // Non-structured hooks (`Expr` / `Return` / `Set` /
            // `Incr` / `AppendOrLappend` / `Unset` / `Global` /
            // `Variable` / `Upvar`) are handled by
            // [`try_lower_hook`] before [`lower_command`] reaches
            // this dispatcher.  They can still reach here when
            // their static dispatcher returned `None` for the
            // input shape (e.g. `try_lower_expr` rejects
            // multi-arg `expr`); fall through to `lower_default`.
            LoweringHookId::Expr
            | LoweringHookId::Return
            | LoweringHookId::Set
            | LoweringHookId::Incr
            | LoweringHookId::AppendOrLappend
            | LoweringHookId::Unset
            | LoweringHookId::Global
            | LoweringHookId::Variable
            | LoweringHookId::Upvar => None,
        }
    }

    /// Lower a single command.
    fn lower_command(&mut self, seg: &SegmentedCommand, namespace: &str) -> Option<Statement> {
        if seg.texts.is_empty() {
            return None;
        }

        let cmd_name = seg.name();
        let args = seg.args();

        // Detect `interp alias {} name {} target ?args?` and static
        // `rename oldName newName` — both feed the same alias table, since
        // calling through a renamed name and calling through an `interp
        // alias` are indistinguishable for arg-role / body-form resolution,
        // taint sink dispatch, and canonical-command typing (`interp alias {}
        // myEval {} eval` and `rename eval myEval` both make `myEval $x` reach
        // the `eval` sink). `interp alias` pre-qualifies its name at the global
        // root, but `rename` binds NEW in the *current* namespace, so qualify
        // it against `namespace` — an unqualified `rename set myset` inside
        // `namespace eval ::ns` binds `::ns::myset`, not global `::myset` (and
        // global `myset` is then invalid). `join_namespace` leaves an
        // already-`::`-qualified NEW rooted globally, matching how
        // `resolve_alias` looks the name back up (current namespace, then
        // global) and `command_binding`'s own namespace-relative candidates.
        let args_owned: Vec<String> = args.to_vec();
        match self
            .registry
            .command_table_effect(cmd_name, args_owned.first().map(String::as_str))
        {
            Some(tcl_registry::CommandTableEffect::CreatesAliases) => {
                if let Some((qualified, target, prepended)) = detect_interp_alias(&args_owned) {
                    self.aliases.insert(qualified, (target, prepended));
                }
            }
            Some(tcl_registry::CommandTableEffect::RenamesCommands) => {
                if let Some((old, new)) = detect_rename(&args_owned) {
                    self.aliases
                        .insert(join_namespace(namespace, &new), (old, Vec::new()));
                }
            }
            // `proc` feeds the alias table nothing — its definition is
            // lowered by its own hook below.
            _ => {}
        }

        self.record_namespace_directives(cmd_name, args, seg, namespace);

        // Trace-visible compilation (`CompileTarget::BytecodeTraced`)
        // suppresses every inline/structured hook unconditionally: every
        // command in this pass becomes a plain runtime dispatch, so an
        // execution trace observes it. The alias-table/namespace-directive
        // bookkeeping above still runs (a traced body's own `rename`/
        // `interp alias` commands must still affect how later commands in it
        // resolve).
        if self.target.is_trace_visible() {
            return Some(self.lower_default(seg, namespace));
        }

        // Try registered lowering hooks first.
        let hook_cmd = LoweringCommand {
            span: seg.span,
            name: cmd_name,
            args,
            single_token_word: &seg.single_token_word,
            expand_word: seg.expand_word.as_deref(),
            tokens: Some(Self::cmd_tokens(seg)),
            arg_kinds: &Self::arg_kinds(seg),
        };
        if let Some(stmt) = try_lower_hook(&hook_cmd, &self.aliases, self.registry) {
            return Some(stmt);
        }

        if let Some(barrier) = Self::structured_expand_barrier(cmd_name, args, seg) {
            return Some(barrier);
        }

        // Registry-driven hook-ID dispatch covers all 15
        // structured command forms — every typed
        // `LoweringHookId` (Proc, When, NamespaceEval, If,
        // Switch, For, While, Foreach, Lmap, ForeachLine, Catch,
        // Try, Dict, Eval, Uplevel) flows through
        // [`try_dispatch_structured_hook`].  Commands that
        // aren't in the registry, or whose hook is `None`, fall
        // through to [`lower_default`] below.
        if let Some(stmt) = self.try_dispatch_structured_hook(cmd_name, seg, namespace) {
            return Some(stmt);
        }

        Some(self.lower_default(seg, namespace))
    }

    /// Lower `proc name params body`.
    ///
    /// Sequential phases:
    ///   1. Empty-simple-name barrier (`proc ::ns:: {…} {…}`).
    ///   2. Dynamic name resolution (`proc $x …` / `proc [cmd] …`).
    ///   3. Dynamic body / params check, with const-map body
    ///      materialisation when possible.
    ///   4. IR registration + emit the runtime `proc` call.
    fn lower_proc(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args_borrow = seg.args();
        let proc_name_initial = &args_borrow[0];

        // Phase 1: empty-simple-name procs (`proc ::ns:: {args} {body}`).
        // Tcl 9 lets a trailing `::` register a command named `""` inside
        // the target namespace (`TclGetNamespaceForQualName` returns
        // `simpleName=""` rather than NULL for trailing colons on
        // cmd/var lookups — tclNamesp.c:2493).  Our static
        // `normalise_qualified_name` strips the trailing colons
        // (`"a::".split("::")` filters the empty trailing element),
        // so an AOT-lifted empty-name proc would register under the
        // namespace name itself — a different command than the
        // runtime lookup needs.  Route the whole shape through the
        // runtime `proc` builtin via `Statement::Barrier`.  Carries
        // namespace-old-1.27 / 2.1 / 2.2 and namespace-14.11.
        if proc_name_initial.ends_with("::") {
            return Statement::Barrier {
                span: seg.span,
                reason: "empty-simple-name proc".into(),
                command: "proc".into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        // Phase 2: dynamic proc name resolution.
        let (proc_name_owned, args_owned, name_was_substituted) =
            match self.resolve_dynamic_proc_name(seg, args_borrow) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };
        let args: &[String] = &args_owned;
        let proc_name = &proc_name_owned;

        // Phase 3: dynamic body / params check, plus body
        // materialisation via the const-map.
        let (materialised_body, body_is_dynamic, body_offset) =
            match self.check_proc_body_dynamic(seg, args, name_was_substituted) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };

        // Phase 4: lower the body (materialised, static, or empty
        // for dynamic-with-resolved-name), register the IRProcedure,
        // and emit the runtime `proc` Call.
        let params = parse_param_names(&args[1]);
        let qualified = qualify_proc_name(namespace, proc_name);
        let body_text = &args[2];
        // Fresh const-map frame for the nested proc body.
        // ``lower_body`` would otherwise inherit the enclosing
        // scope's tracked scalars — correct for control-flow
        // bodies (if / catch / loops share the frame) but unsound
        // for ``proc`` bodies, which have their own runtime frame.
        // Pushing an empty frame here means the inner ``lower_body``
        // clones an empty parent, giving the proc body a clean
        // slate.
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let body = if let Some(text) = materialised_body {
            self.lower_script(&text, namespace)
        } else if body_is_dynamic {
            // Dynamic body, no materialisation possible — but the
            // proc name resolved via the const-map (otherwise we'd
            // have bailed above).  Leave the IRProcedure body empty
            // so static analysis doesn't try to compile the literal
            // ``$body`` source text as a script.  The runtime
            // ``proc`` IRCall below carries the actual body bytes.
            Script::default()
        } else if let Some(cache) = self
            .body_cache
            .filter(|_| self.proc_depth == 1 && body_cache_eligible(body_text))
        {
            // SRV-INCREMENTAL Task 3: a top-level proc's static body is lowered in
            // isolation through the memo (offset 0) and rebased to its real offset.
            // Gated per-body on [`body_cache_eligible`]: a body free of the
            // cross-item constructs the isolated lowering would drop lowers
            // byte-identically to the in-place `lower_body` (the const-map is a fresh
            // empty frame here, so a literal body has no cross-item dependency) — so a
            // context-carrying sibling no longer disables the cache for this one.
            // Guarded by the corpus differential gates (`file_analysis_corpus` /
            // `compiler_check_corpus` / `per_item_corpus`).
            let mut s = cache(body_text, namespace);
            crate::lattice_rebase::rebase_script(&mut s, i64::from(body_offset));
            s
        } else {
            self.lower_body(body_text, body_offset, namespace)
        };
        self.const_map_stack.pop();
        self.proc_depth -= 1;

        // A `proc` defined inside a TclOO method body is created
        // at method-call time in the global namespace, not at script
        // load — so it must not be lifted into `module.procedures`
        // (codegen would otherwise emit it unconditionally). The body
        // was still lowered above for analysis; only the global
        // registration is suppressed.
        if !self.suppress_proc_register {
            match self.module.procedures.entry(qualified.clone()) {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(Procedure {
                        name: proc_name.clone(),
                        qualified_name: qualified,
                        params,
                        span: seg.span,
                        body,
                        params_raw: args[1].clone(),
                        body_source: Some(args[2].clone()),
                        namespace_scoped: self.in_namespace_eval,
                        base_priority: 500,
                    });
                }
                std::collections::hash_map::Entry::Occupied(_) => {
                    self.module.redefined_procedures.insert(qualified);
                }
            }
        }

        Statement::Call {
            span: seg.span,
            command: "proc".into(),
            canonical_command: None,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    /// Resolve a proc name that may be a `$var` / `[cmd]` substitution.
    ///
    /// Returns the resolved name, the (possibly rewritten) args
    /// vector, and a `name_was_substituted` flag.  A literal name
    /// short-circuits to `Ok` with the original args.  A dynamic
    /// name that can't be resolved via the const-map returns `Err`
    /// with a "dynamic proc name" barrier — the caller propagates
    /// it as the statement.
    ///
    /// Multi-token names (`foo_$x`) and command-substitution names
    /// (`$name[suffix]`) stay on the runtime path.
    fn resolve_dynamic_proc_name(
        &mut self,
        seg: &SegmentedCommand,
        args_borrow: &[String],
    ) -> Result<(String, Vec<String>, bool), Box<Statement>> {
        let proc_name_initial = &args_borrow[0];
        if !proc_name_initial.contains('$') && !proc_name_initial.contains('[') {
            return Ok((proc_name_initial.clone(), args_borrow.to_vec(), false));
        }

        let arg_tokens = seg.arg_tokens();
        let single_token_proc_name = seg.single_token_word.get(1).copied().unwrap_or(false);
        let resolved = if !proc_name_initial.contains('[')
            && single_token_proc_name
            && arg_tokens
                .first()
                .is_some_and(|t| t.kind == tcl_lexer::TokenType::Var)
        {
            self.const_map_lookup(proc_name_initial)
        } else {
            None
        };

        let Some(literal) = resolved else {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: "dynamic proc name".into(),
                command: "proc".into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            }));
        };

        let mut args_owned = args_borrow.to_vec();
        args_owned[0].clone_from(&literal);
        Ok((literal, args_owned, true))
    }

    /// Check whether the proc body and/or params are dynamic, and
    /// attempt to materialise the body from the const-map.
    ///
    /// Returns:
    ///   * `Ok((Some(text), _, body_offset))` — body materialised
    ///     successfully; caller should lower `text` as a fresh
    ///     script.
    ///   * `Ok((None, body_is_dynamic, body_offset))` — body is
    ///     static (lower from source) or dynamic-with-name-resolved
    ///     (lower as empty Script).
    ///   * `Err(barrier)` — params dynamic, or body dynamic without
    ///     name substitution.  Caller propagates the barrier.
    fn check_proc_body_dynamic(
        &mut self,
        seg: &SegmentedCommand,
        args: &[String],
        name_was_substituted: bool,
    ) -> Result<(Option<String>, bool, u32), Box<Statement>> {
        let arg_tokens = seg.arg_tokens();
        let params_tok = arg_tokens[1];
        let body_tok = arg_tokens[2];
        // `single_token_word[0]` covers the command itself; args[0..]
        // live at indices `[1..]`.  See `SegmentedCommand::arg_single_token`.
        let body_is_single = seg.single_token_word.get(3).copied().unwrap_or(false);
        let params_is_single = seg.single_token_word.get(2).copied().unwrap_or(false);
        let body_is_dynamic = matches!(
            body_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !body_is_single;
        let params_is_dynamic = matches!(
            params_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !params_is_single;
        let materialised_body: Option<String> =
            if body_is_single && body_tok.kind == tcl_lexer::TokenType::Cmd {
                // `args[2]` is the `word_piece`-reconstructed source
                // — for a Cmd token that's `[subst -nocommands {…}]`
                // (with the brackets re-added).
                // `eval_subst_nocommands_body` segments its input as
                // a top-level Tcl command stream and expects the
                // inner content, so strip the outer `[…]` before
                // handing it over.
                args[2]
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .and_then(|inner| self.eval_subst_nocommands_body(inner))
            } else if body_is_single && body_tok.kind == tcl_lexer::TokenType::Var {
                self.const_map_lookup(&args[2])
            } else {
                None
            };
        if params_is_dynamic
            || (body_is_dynamic && materialised_body.is_none() && !name_was_substituted)
        {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: if params_is_dynamic {
                    "dynamic proc params"
                } else {
                    "dynamic proc body"
                }
                .into(),
                command: "proc".into(),
                canonical_command: None,
                args: seg.args().to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            }));
        }
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        Ok((materialised_body, body_is_dynamic, body_offset))
    }

    /// Lower `when EVENT ?priority N? body`.
    fn lower_when(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let event_name = &args[0];
        let body_idx = args.len() - 1;
        let body_tok = seg.arg_tokens()[body_idx];
        let body_text = &args[body_idx];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // Fresh const-map frame for the nested proc body.
        // ``lower_body`` would otherwise inherit the enclosing
        // scope's tracked scalars — correct for control-flow
        // bodies (if / catch / loops share the frame) but unsound
        // for ``proc`` bodies, which have their own runtime frame.
        // Pushing an empty frame here means the inner ``lower_body``
        // clones an empty parent, giving the proc body a clean
        // slate.
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let body = self.lower_body(body_text, body_offset, namespace);
        self.const_map_stack.pop();
        self.proc_depth -= 1;

        let mut base_priority: u32 = 500;
        if args.len() >= 4
            && args[1] == "priority"
            && let Ok(p) = args[2].parse::<u32>()
        {
            base_priority = p;
        }

        let n = self
            .when_counts
            .get(event_name.as_str())
            .copied()
            .unwrap_or(0);
        *self.when_counts.entry(event_name.clone()).or_insert(0) += 1;
        let qualified = if n == 0 {
            format!("::when::{event_name}")
        } else {
            format!("::when::{event_name}#{n}")
        };

        self.module.procedures.insert(
            qualified.clone(),
            Procedure {
                name: event_name.clone(),
                qualified_name: qualified,
                params: vec![],
                span: seg.span,
                body,
                params_raw: String::new(),
                body_source: None,
                namespace_scoped: false,
                base_priority,
            },
        );

        Statement::Call {
            span: seg.span,
            command: "when".into(),
            canonical_command: None,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    /// Lower `namespace eval ns body`.
    /// Try to lower `uplevel ?level? {body}` to a static-body
    /// [`Statement::UpFrame`] when:
    ///
    /// 1. The body argument is a brace-string token (`TokenType::Str`),
    ///    and
    /// 2. The level argument (if present) parses as a positive integer
    ///    or `#0` / `#N` global form.
    ///
    /// Returns `None` if the call doesn't match the static shape, in
    /// which case the caller falls back to [`Self::lower_default`]
    /// (producing a runtime [`Statement::Barrier`]).
    fn try_lower_uplevel_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;

        // Static-body `uplevel` lowers to `Statement::UpFrame` so the analysers
        // (interproc purity, code-sinking, var-escape, the inline_uplevel pass)
        // see a structured frame-shift with a lowered body instead of an opaque
        // runtime barrier. Runtime correctness is preserved at codegen: a
        // surviving `UpFrame` re-emits the original `uplevel <level> {body}`
        // invoke from its preserved command tokens (see
        // `codegen::statements`), producing byte-identical bytecode to the old
        // barrier path and dispatching through the `uplevel` builtin's correct
        // frame semantics. The whole-callee inline_uplevel splice (which would
        // need frame-shift opcodes to run a *bare* UpFrame against the right
        // activation) is an opt-in optimiser pass, not wired into codegen.
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let (frame_shift, body_tok_idx) = match args.len() {
            1 => (1_i32, 0),
            2 => (parse_uplevel_level(&args[0])?, 1),
            _ => return None,
        };
        let body_tok = arg_tokens.get(body_tok_idx)?;
        // Barrier gate: see `try_lower_eval_static` for the
        // rationale.  A static-body `uplevel 1 {eval $x}` would
        // relax to inline IR with a still-dynamic `eval`; reject.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[body_tok_idx];
            if body_has_dynamic_barrier(body_text, self.registry) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            let body_text = &args[body_tok_idx];
            let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.lower_body(body_text, body_offset, namespace)
        } else if body_tok.kind == TokenType::Var {
            // `uplevel ?N? $var` with $var resolved by the
            // const-map to a brace-string literal — fold the literal
            // in and lower as a static UpFrame.
            let literal = self.const_map_lookup(&args[body_tok_idx])?;
            self.lower_script(&literal, namespace)
        } else {
            return None;
        };
        Some(Statement::UpFrame {
            span: seg.span,
            frame_shift,
            body,
            tokens: Some(Self::cmd_tokens(seg)),
        })
    }

    /// If *`cmd_text`* is `subst -nocommands {template}` (in any
    /// flag order) AND every `$var` inside *template* is in the
    /// current const-map, return the substituted string. Otherwise
    /// `None` so the caller falls back to runtime dispatch.
    ///
    /// Used to materialise the tcltest-style `Option` factory body
    /// at compile time when the surrounding proc has all the
    /// template vars const-tracked.
    fn eval_subst_nocommands_body(&self, cmd_text: &str) -> Option<String> {
        use tcl_lexer::TokenType;
        let inner = segment_commands_with_offset_and_config(cmd_text, 0, self.config);
        if inner.len() != 1 {
            return None;
        }
        let inner_cmd = &inner[0];
        if inner_cmd.texts.is_empty() || inner_cmd.texts[0] != "subst" {
            return None;
        }
        let argv = inner_cmd.arg_tokens();
        let texts = inner_cmd.args();
        let single = &inner_cmd.single_token_word;

        let mut saw_nocommands = false;
        let mut template_text: Option<&str> = None;
        for (i, tok) in argv.iter().enumerate() {
            let text = &texts[i];
            if text == "-nocommands" {
                saw_nocommands = true;
                continue;
            }
            if text == "-nobackslashes" || text == "-novariables" {
                // Either flag changes the semantics our evaluator
                // assumes — refuse.
                return None;
            }
            if text.starts_with('-') {
                return None;
            }
            if !single.get(i + 1).copied().unwrap_or(false) {
                return None;
            }
            if tok.kind != TokenType::Str {
                return None;
            }
            if template_text.is_some() {
                // Multiple positionals — not the shape we recognise.
                return None;
            }
            template_text = Some(text.as_str());
        }
        if !saw_nocommands {
            return None;
        }
        let template = template_text?;
        if self.proc_depth == 0 {
            return None;
        }
        let scope = self.const_map_stack.last()?;
        crate::subst_nocommands::subst_nocommands(template, scope)
    }

    /// Try to lower `eval ?body?` to a static-body
    /// [`Statement::Block`] when the body is a brace-string literal,
    /// a const-mapped `$var`, or an `eval [list lit1 lit2 ...]`
    /// command-substitution shape. Returns `None` for dynamic
    /// bodies so the caller falls through to runtime barrier
    /// dispatch.
    fn try_lower_eval_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        // Single-body shape only: ``eval $body`` / ``eval {body}``.
        // The list form (``eval cmd arg1 arg2``) keeps runtime
        // semantics — joining the words with spaces is observable.
        if args.len() != 1 || arg_tokens.is_empty() {
            return None;
        }
        let body_tok = arg_tokens[0];
        // Barrier gate: a braced literal body might contain a
        // nested ``eval $x`` / ``uplevel $lvl {...}`` whose own body
        // is still dynamic.  Relaxing the outer barrier in that case
        // produces IR that runs a compiled inner barrier with a
        // still-dynamic shape — we'd lose the runtime barrier without
        // gaining static knowledge.  Reject and fall back to the
        // default IRBarrier dispatch.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[0];
            if body_has_dynamic_barrier(body_text, self.registry) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            let body_text = &args[0];
            let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.lower_body(body_text, body_offset, namespace)
        } else if body_tok.kind == TokenType::Var {
            let literal = self.const_map_lookup(&args[0])?;
            if body_has_dynamic_barrier(&literal, self.registry) {
                return None;
            }
            self.lower_script(&literal, namespace)
        } else if body_tok.kind == TokenType::Cmd {
            // ``eval [list lit1 lit2 ...]`` — synthesise the
            // body by joining the list's literal arguments and
            // re-lowering. The bracket-substitution text retains
            // the surrounding ``[...]``; strip them via
            // ``content_offset`` if present, otherwise the helper
            // strips them itself.
            let inner_text = if body_tok.content_offset > 0 {
                let start = u32::from(body_tok.content_offset) as usize;
                &args[0][start..args[0].len() - start]
            } else {
                args[0].trim_start_matches('[').trim_end_matches(']')
            };
            let synthesised = eval_list_literal_body(inner_text)?;
            self.lower_script(&synthesised, namespace)
        } else {
            return None;
        };
        Some(Statement::Block {
            span: seg.span,
            body,
            namespace: namespace.to_string(),
            tokens: Some(Self::cmd_tokens(seg)),
        })
    }

    /// `apply {{params} body ?ns?} ?arg ...?` — an anonymous lambda.
    ///
    /// The body runs in a *separate* frame (its own locals, bound parameters),
    /// so — like `namespace eval` — the `apply` itself stays a runtime
    /// [`Statement::Barrier`]: codegen executes it via the runtime `apply` and
    /// the body's frame-local effects are conservatively opaque to the caller's
    /// CFG. But a braced literal body is still walked through [`Self::lower_body`]
    /// (in a fresh const-map / proc-depth frame, as `lower_proc` does) so nested
    /// `proc` definitions and other module-level effects register — where the
    /// old `lower_default` barrier discarded the body wholesale, losing them.
    ///
    /// A `$var` / `[cmd]` lambda, or a lambda whose body element is not a braced
    /// literal, stays fully opaque (nothing statically walkable).
    ///
    /// The body is walked in the *lambda's* namespace — element 2 of the lambda
    /// if present, else the global namespace `::` — not the caller's namespace,
    /// so a nested `proc` registers under the same qualified name Tcl's runtime
    /// `apply` would give it (per the `apply` manual).
    /// Register an already-lowered fresh-frame body (an `apply` lambda or a
    /// `namespace eval` block) as a synthetic *body unit* in
    /// [`Module::body_units`], so the static-analysis pipeline reaches inside
    /// it. `label` names the construct (`apply` / `namespace-eval`), `params`
    /// are the frame's bound names (empty for `namespace eval`), and `span`
    /// covers the body text (used by the complexity guard). Purely additive —
    /// the caller still emits the runtime `Statement::Barrier` that executes
    /// the body, so bytecode is unchanged.
    /// `label` is either a bare marker (`"apply"`, reducing to the global
    /// namespace once `#N`-suffixed — matching `apply`'s own default
    /// resolution namespace for the common 2-element-lambda form) or an
    /// already-namespace-qualified prefix (`"::ns::namespace-eval"`, from
    /// [`Self::lower_namespace_eval`]) — so the resulting `qualified` name's
    /// *enclosing* namespace (everything before the last `::`, the same
    /// convention every proc/method qname uses) is the namespace this body
    /// actually executes in, not always the global namespace. This is what
    /// lets [`crate::compilation_unit::build_extra_call_site_scan_contexts`]
    /// resolve a bare call inside the body against the *correct* namespace
    /// via [`crate::interprocedural::resolve_internal_call`], rather than
    /// silently defaulting every body unit to global.
    fn register_body_unit(
        &mut self,
        label: &str,
        params: Vec<String>,
        span: tcl_lexer::Span,
        body: Script,
    ) {
        let n = self.body_unit_count;
        self.body_unit_count += 1;
        let prefix = label.strip_prefix("::").unwrap_or(label);
        let qualified = format!("::{prefix}#{n}");
        // `name` stays the short leaf marker (`"apply"` / `"namespace-eval"`)
        // even when `label` carries a namespace prefix — matching every
        // other `Procedure::name`'s "short name" contract.
        let short_name = prefix.rsplit("::").next().unwrap_or(prefix).to_string();
        self.module.body_units.insert(
            qualified.clone(),
            Procedure {
                name: short_name,
                qualified_name: qualified,
                params,
                span,
                body,
                params_raw: String::new(),
                body_source: None,
                namespace_scoped: false,
                base_priority: 500,
            },
        );
    }

    fn lower_apply(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();

        // Flatten the braced literal lambda into its list elements (index 0 =
        // params, 1 = body, 2 = optional namespace). A newline splits *commands*
        // but not *list elements*, matching the analyser's `handle_apply_command`.
        let lambda_elems: Vec<(tcl_lexer::Token, String)> = match (args.first(), arg_tokens.first())
        {
            (Some(lambda_text), Some(&lambda_tok)) if lambda_tok.kind == TokenType::Str => {
                let base = lambda_tok.span.start() + u32::from(lambda_tok.content_offset);
                segment_commands_with_offset_and_config(lambda_text, base, self.config)
                    .iter()
                    .flat_map(|c| c.argv.iter().copied().zip(c.texts.iter().cloned()))
                    .collect()
            }
            _ => Vec::new(),
        };

        // The body element (index 1) must itself be a braced literal to walk.
        let body_info: Option<(String, u32)> = lambda_elems
            .get(1)
            .filter(|(tok, _)| tok.kind == TokenType::Str)
            .map(|(tok, text)| {
                (
                    text.clone(),
                    tok.span.start() + u32::from(tok.content_offset),
                )
            });

        if let Some((body_text, body_offset)) = body_info {
            // `apply` evaluates the body in the namespace named by lambda element
            // 2, or the *global* namespace when it is absent — never the caller's
            // namespace (Tcl `apply` manual). So a nested `proc` registers
            // globally unless the lambda pins a namespace; a non-`::`-qualified
            // pin resolves relative to the caller (as `TclGetNamespaceFromObj`).
            let body_ns = match lambda_elems.get(2).map(|(_, t)| t.as_str()) {
                Some(ns) if !ns.is_empty() && !ns.starts_with('$') && !ns.starts_with('[') => {
                    join_namespace(namespace, ns)
                }
                _ => "::".to_string(),
            };
            // Fresh frame for the lambda body: its own runtime frame means the
            // caller's tracked scalars must not fold into it (a bare `$x` in the
            // body is an unbound local, not the caller's `x`).  Mirror
            // `lower_proc`'s const-map / proc-depth push.
            self.proc_depth += 1;
            self.const_map_stack.push(HashMap::new());
            let body = self.lower_body(&body_text, body_offset, &body_ns);
            self.const_map_stack.pop();
            self.proc_depth -= 1;

            // The lambda's bound parameters are element 0 of the lambda list, so
            // a `$param` read inside the body resolves to the param (not a
            // caller scalar). A body defined inside a TclOO method body is not
            // registered globally (`suppress_proc_register`), but a body unit is
            // analysis-only (never codegen-emitted), so it is always safe to
            // record for coverage.
            let params = lambda_elems
                .first()
                .map(|(_, t)| parse_param_names(t))
                .unwrap_or_default();
            let span = tcl_lexer::Span::new(
                body_offset,
                body_offset + u32::try_from(body_text.len()).unwrap_or(u32::MAX),
            );
            self.register_body_unit("apply", params, span, body);
        }

        Statement::Barrier {
            span: seg.span,
            reason: "unsupported body command".into(),
            command: seg.name().into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// `array for {keyVar valueVar} arrayName body` (Tcl 9.0).
    ///
    /// The body runs in the caller's frame with the two loop variables bound
    /// per entry. C Tcl compiles this to an `invokeStk` of `::tcl::array::for`
    /// with the body pushed as an unparsed literal — it does not compile the
    /// body — so, exactly like [`Self::lower_apply`], the call itself stays a
    /// runtime [`Statement::Barrier`] (codegen unchanged, byte-identical to C
    /// Tcl). But a braced literal body is walked in a fresh frame bound to the
    /// loop variables and recorded as a body unit so static analysis reaches
    /// inside it. A `$var` / `[cmd]` body stays fully opaque.
    fn lower_array_for(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        // `array for {k v} arr body` → args = [for, {k v}, arr, body].
        // The body (index 3) must be a single braced-literal token to walk.
        let body_is_braced = arg_tokens.get(3).is_some_and(|t| t.kind == TokenType::Str)
            && arg_single.get(3).copied() == Some(true);
        if args.len() == 4 && body_is_braced {
            let body_tok = &arg_tokens[3];
            // The body runs in the *caller's* frame (`vm.eval_source` in place),
            // so lower it inline — inheriting the caller's const-map — into a
            // loop-variable-bound `Foreach` rather than isolating it in a
            // fresh-frame body unit. The analysis CFG inlines this into the
            // caller unit (so a body `$re` resolves to a caller literal for
            // regex-source / taint), while codegen barriers it to the byte-
            // identical `::tcl::array::for` invoke — see `lower_foreach_dispatch`.
            let body = self.lower_body_from_tok(&args[3], Some(body_tok), namespace);
            let vars = parse_param_names(&args[1]);
            return Statement::Foreach {
                span: seg.span,
                iterators: vec![ForeachIterator {
                    vars,
                    list_arg: args[2].clone(),
                }],
                body,
                body_span: body_tok.span,
                is_lmap: false,
                raw_args: args.to_vec(),
                is_dict_iteration: false,
                is_array_iteration: true,
                raw_tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        // A dynamic (`$body` / `[cmd]`) body stays fully opaque: the runtime
        // barrier re-emits `array for …`.
        self.lower_default(seg, namespace)
    }

    fn lower_namespace_eval(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let child_ns = join_namespace(namespace, &args[1]);
        let prev = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let body_tok = seg.arg_tokens()[2];
        let body_text = &args[2];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let body = self.lower_body(body_text, body_offset, &child_ns);
        self.in_namespace_eval = prev;

        // A `namespace eval` body runs in the child namespace's own frame — its
        // straight-line statements are a real analysable script, so record them
        // as a body unit (no bound params: namespace vars, not locals).
        let span = tcl_lexer::Span::new(
            body_offset,
            body_offset + u32::try_from(body_text.len()).unwrap_or(u32::MAX),
        );
        // Prefix the body unit's own qualified name with `child_ns` (not just
        // the bare "namespace-eval" marker): a bare command inside this body
        // resolves against `child_ns`, never the caller's own namespace, so
        // the body unit's qname must encode that for
        // `interprocedural::resolve_internal_call` to get it right (see
        // `register_body_unit`'s doc). tclsh8.6-confirmed:
        // `namespace eval ::foo { helper }` calls `::foo::helper` even when
        // nested inside an unrelated proc, never a same-named proc in the
        // caller's own namespace.
        self.register_body_unit(
            &join_namespace(&child_ns, "namespace-eval"),
            vec![],
            span,
            body,
        );

        Statement::Barrier {
            span: seg.span,
            reason: "namespace eval".into(),
            command: "namespace".into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// Default lowering: generic `IRCall` with registry-based arg roles.
    fn lower_default(&self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let cmd_name = seg.name();
        let args = seg.args();

        // Resolve alias for arg role lookups.  When `cmd_name`
        // resolves to a different alias target, populate
        // `canonical_command` with the resolved name so downstream
        // dispatch (codegen hook lookup, side-effect classification,
        // GVN purity, var-escape) can key off the canonical target
        // instead of re-resolving from the source spelling.
        let mut role_cmd = cmd_name.to_owned();
        let mut role_args: Vec<String> = args.to_vec();
        let mut prepend_n: usize = 0;
        let mut canonical: Option<String> = None;
        if let Some((target, prepended)) = resolve_alias(cmd_name, &self.aliases, namespace) {
            // Only record canonical_command when the alias resolved
            // to a different target — ``cmd_name == target`` means
            // the source already names the canonical form.
            if target != cmd_name {
                canonical = Some(target.clone());
            }
            role_cmd = target;
            let mut new_args: Vec<String> = prepended;
            new_args.extend_from_slice(args);
            prepend_n = new_args.len() - args.len();
            role_args = new_args;
        }

        let role_args_ref: Vec<&str> = role_args.iter().map(String::as_str).collect();
        let body_indices =
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::Body);
        let var_indices =
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::VarWrite);
        let var_read_indices =
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::VarRead);
        // Read-modify-write commands (`lset` / `lpop` / `ledit` — like
        // `incr` / `append` / `lappend`, which are hook-lowered) read the
        // current value of their target before rewriting it, so the prior
        // definition is live. Carry that as `reads_own_defs` so dead-store /
        // unused-variable analysis does not treat a feeding `set` as dead.
        let reads_before_write = self
            .registry
            .get(&role_cmd)
            .is_some_and(|s| s.traits.contains(tcl_registry::Traits::READS_BEFORE_WRITE));

        if !body_indices.is_empty() {
            return Statement::Barrier {
                span: seg.span,
                reason: "unsupported body command".into(),
                command: cmd_name.into(),
                canonical_command: canonical.clone(),
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        if !var_indices.is_empty() || !var_read_indices.is_empty() {
            let var_defs: Vec<String> = var_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real)
                        .map(|a| crate::naming::element_var_name(a).to_owned())
                })
                .collect();
            let var_reads: Vec<String> = var_read_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real)
                        .map(|a| crate::naming::element_var_name(a).to_owned())
                })
                .collect();
            return Statement::Call {
                span: seg.span,
                command: cmd_name.into(),
                canonical_command: canonical.clone(),
                args: args.to_vec(),
                defs: var_defs,
                reads: var_reads,
                reads_own_defs: reads_before_write,
                safe_on_uninit: false,
                tokens: Some(Self::cmd_tokens(seg)),
                foreach_groups: None,
            };
        }

        Statement::Call {
            span: seg.span,
            command: cmd_name.into(),
            canonical_command: canonical,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    // TclOO method-body lowering (method purity for O126)

    /// Populate `module.methods` from the fully-assembled IR.
    ///
    /// Runs as a cache-independent post-pass (mirroring
    /// [`populate_trace_facts`]): clone the top-level script and proc
    /// bodies, then walk them — plus any `namespace eval` blocks — for
    /// `oo::class create` / `oo::define` barriers and lift each
    /// `method` / `constructor` / `destructor` body to an
    /// [`MethodDef`]. Method bodies are lowered for analysis only —
    /// codegen never reads `module.methods`, and the barrier emitted
    /// for the class command is unchanged, so bytecode is
    /// byte-identical.
    pub fn extract_oo_methods_pass(&mut self) {
        let top_level = self.module.top_level.clone();
        self.walk_for_oo_methods(&top_level.statements, "::");
        let procs: Vec<(String, Script)> = self
            .module
            .procedures
            .iter()
            .map(|(q, p)| (q.clone(), p.body.clone()))
            .collect();
        for (qname, body) in &procs {
            let proc_ns = proc_namespace(qname);
            self.walk_for_oo_methods(&body.statements, &proc_ns);
        }
    }

    /// Recursive walk for `oo::class` / `oo::define` definition
    /// barriers. Descends `namespace eval` blocks (tracking the active
    /// namespace) and extracts methods from any class/define command
    /// carrying a static braced body. The lowerer evaluates a
    /// `namespace eval` body inline and discards it (emitting a
    /// `Barrier`), so rather than descending a preserved block this
    /// walk re-segments the barrier's body token instead (see
    /// [`Self::walk_segments_for_oo`]).
    fn walk_for_oo_methods(&mut self, statements: &[Statement], namespace: &str) {
        for stmt in statements {
            match stmt {
                // Folded `eval` / `uplevel` bodies survive as Blocks.
                Statement::Block {
                    body,
                    namespace: block_ns,
                    ..
                } => {
                    let ns = if block_ns.is_empty() {
                        namespace
                    } else {
                        block_ns.as_str()
                    };
                    self.walk_for_oo_methods(&body.statements, ns);
                }
                Statement::Call {
                    command,
                    canonical_command,
                    tokens: Some(ct),
                    ..
                }
                | Statement::Barrier {
                    command,
                    canonical_command,
                    tokens: Some(ct),
                    ..
                } => {
                    if let Some(form) = oo_definition_form(command, canonical_command.as_deref()) {
                        if is_oo_definition_shape(
                            form,
                            &ct.argv_texts,
                            &ct.argv_kinds,
                            &ct.single_token_word,
                        ) {
                            let idx = oo_body_word_idx(form);
                            self.extract_oo_methods(
                                form,
                                &ct.argv_texts,
                                ct.argv[idx].start() + 1,
                                namespace,
                            );
                        }
                    } else if is_namespace_eval_shape(
                        command,
                        &ct.argv_texts,
                        &ct.argv_kinds,
                        &ct.single_token_word,
                    ) {
                        // The body was lowered inline and discarded;
                        // re-segment it to find classes defined directly
                        // inside the namespace.
                        let child_ns = join_namespace(namespace, &ct.argv_texts[2]);
                        let body_off = ct.argv[3].start() + 1;
                        let segments = segment_commands_with_offset_and_config(
                            &ct.argv_texts[3],
                            body_off,
                            self.config,
                        );
                        self.walk_segments_for_oo(&segments, &child_ns);
                    }
                }
                _ => {}
            }
        }
    }

    /// Segment-level counterpart of [`Self::walk_for_oo_methods`] used
    /// for `namespace eval` bodies (which the lowerer discards). Walks
    /// re-segmented commands, recursing through nested `namespace eval`
    /// and extracting methods from `oo::class` / `oo::define` blocks.
    fn walk_segments_for_oo(&mut self, segments: &[SegmentedCommand], namespace: &str) {
        for seg in segments {
            if seg.is_partial || seg.texts.is_empty() {
                continue;
            }
            let kinds: Vec<TokenType> = seg.argv.iter().map(|t| t.kind).collect();
            let cmd = seg.texts[0].as_str();
            if let Some(form) = oo_definition_form(cmd, None) {
                if is_oo_definition_shape(form, &seg.texts, &kinds, &seg.single_token_word) {
                    let idx = oo_body_word_idx(form);
                    let off = seg.argv[idx].span.start() + u32::from(seg.argv[idx].content_offset);
                    self.extract_oo_methods(form, &seg.texts, off, namespace);
                }
            } else if is_namespace_eval_shape(cmd, &seg.texts, &kinds, &seg.single_token_word) {
                let child_ns = join_namespace(namespace, &seg.texts[2]);
                let off = seg.argv[3].span.start() + u32::from(seg.argv[3].content_offset);
                let sub = segment_commands_with_offset_and_config(&seg.texts[3], off, self.config);
                self.walk_segments_for_oo(&sub, &child_ns);
            }
        }
    }

    /// Lift `method` / `classmethod` / `constructor` / `destructor`
    /// bodies inside an `oo::class create` / `oo::define` block to
    /// per-method [`MethodDef`] entries keyed by
    /// `{class_qname}::{method_name}` (constructors / destructors use
    /// the synthetic names `<constructor>` / `<destructor>`). `texts`
    /// is the class command's
    /// full per-word text array; `body_content_offset` is the absolute
    /// source offset of the first byte inside the body's braces.
    fn extract_oo_methods(
        &mut self,
        form: &str,
        texts: &[String],
        body_content_offset: u32,
        namespace: &str,
    ) {
        // `is_oo_definition_shape` guarantees the indices below exist.
        let (class_simple, body_text) = match form {
            "oo::class" => (texts[2].as_str(), texts[3].as_str()),
            // oo::define
            _ => (texts[1].as_str(), texts[2].as_str()),
        };
        // Dynamic class names can't be resolved statically.
        if class_simple.contains('$') || class_simple.contains('[') {
            return;
        }
        let class_qname = qualify_proc_name(namespace, class_simple);
        let segments =
            segment_commands_with_offset_and_config(body_text, body_content_offset, self.config);

        // Class-level instance-variable declarations (`variable a b
        // ...`) are auto-linked into every method. The TclOO
        // class-body `variable` slot is names-only (`variable a b c`
        // declares three instance vars — verified vs tclsh 9.0 — NOT
        // name/value pairs), so every literal trailing word is a name.
        let mut class_ivars: HashSet<String> = HashSet::new();
        for seg in &segments {
            if !seg.is_partial && seg.texts.len() >= 2 && seg.texts[0] == "variable" {
                for nm in &seg.texts[1..] {
                    if is_instance_var_name(nm) {
                        class_ivars.insert(normalise_var_name(nm).to_string());
                    }
                }
            }
        }

        for seg in &segments {
            if seg.is_partial || seg.texts.is_empty() {
                continue;
            }
            let head = seg.texts[0].as_str();
            let (name, params_str, b_idx, kind): (&str, &str, usize, &str) = match head {
                "method" if seg.texts.len() >= 4 => {
                    (seg.texts[1].as_str(), seg.texts[2].as_str(), 3, "method")
                }
                "classmethod" if seg.texts.len() >= 4 => (
                    seg.texts[1].as_str(),
                    seg.texts[2].as_str(),
                    3,
                    "classmethod",
                ),
                "constructor" if seg.texts.len() >= 3 => {
                    ("<constructor>", seg.texts[1].as_str(), 2, "constructor")
                }
                "destructor" if seg.texts.len() >= 2 => ("<destructor>", "", 1, "destructor"),
                _ => continue,
            };
            // Dynamic method names / non-static bodies are left
            // un-lowered (the optimiser stays conservative for them).
            if name.contains('$') || name.contains('[') {
                continue;
            }
            if !seg_word_is_static_braced(seg, b_idx) {
                continue;
            }
            let params = if params_str.is_empty() {
                Vec::new()
            } else {
                parse_param_names(params_str)
            };
            // Lower the method body in its own frame (fresh const-map +
            // proc-depth) so the enclosing scope's tracked scalars
            // don't leak into the body's barrier-relaxation gate —
            // exactly as the `proc` case does — and suppress
            // nested-proc registration while doing so.
            let body_tok = seg.argv[b_idx];
            let body_off = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.proc_depth += 1;
            self.const_map_stack.push(HashMap::new());
            let prev_suppress = self.suppress_proc_register;
            self.suppress_proc_register = true;
            let body_script = self.lower_body(seg.texts[b_idx].as_str(), body_off, namespace);
            self.suppress_proc_register = prev_suppress;
            self.const_map_stack.pop();
            self.proc_depth -= 1;

            // Instance vars in scope: class-level decls plus this
            // method's own top-level `variable` declarations. A write
            // to any of these mutates object state (impure for O126).
            let mut method_ivars = class_ivars.clone();
            for st in &body_script.statements {
                if let Statement::Call {
                    command,
                    canonical_command,
                    args,
                    ..
                } = st
                    && canonical_matches(command, canonical_command.as_deref(), "variable")
                {
                    for nm in args {
                        if is_instance_var_name(nm) {
                            method_ivars.insert(normalise_var_name(nm).to_string());
                        }
                    }
                }
            }

            let method_qname = format!("{class_qname}::{name}");
            // First definition wins for the stored body (matches proc
            // registration), but a redefinition (a later `oo::define`
            // or a duplicate in-body `method`) replaces the body at
            // runtime — we can't statically know which body a given
            // dispatch runs, so flag it impure to keep O126 sound.
            if self.module.methods.contains_key(&method_qname) {
                self.module.redefined_methods.insert(method_qname);
                continue;
            }
            self.module.methods.insert(
                method_qname,
                MethodDef {
                    class_name: class_qname.clone(),
                    method_name: name.to_string(),
                    params,
                    body: body_script,
                    kind: MethodKind::from_str_lossy(kind),
                    span: Some(seg.span),
                    instance_vars: method_ivars,
                },
            );
        }
    }
}

// Public API

/// Lower Tcl source to an IR module.
///
/// This is the main entry point for the lowering phase.  Lexes with the
/// default (Tcl-8.5+) config; use [`lower_to_ir_with_config`] to honour a
/// document's dialect.
#[must_use]
pub fn lower_to_ir(source: &str, registry: &CommandRegistry) -> Module {
    lower_to_ir_with_config(source, registry, tcl_lexer::LexerConfig::default())
}

/// Lower a single `proc` body in isolation at `proc_depth == 1` (a fresh
/// [`Lowerer`] with an empty const-map frame), returning the **offset-0** body
/// [`Script`].  Replicates the body-lowering setup `lower_proc` performs for a
/// static literal body (`proc_depth += 1`; push an empty const-map;
/// `lower_body(text, 0, namespace)`).
///
/// For a body free of cross-item context (no nested `proc` / `namespace
/// import`/`export` / command alias / const-map materialisation), this is
/// byte-identical to the body the whole-file lowering produces for that
/// procedure, normalised to offset 0 — the seam the SRV-INCREMENTAL per-procedure
/// lowering memo (Task 3) keys on the offset-0 body text and feeds back through
/// [`Lowerer::with_body_cache`].
#[must_use]
pub fn lower_proc_body_isolated(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Script {
    let mut lowerer = Lowerer::with_config(registry, config);
    lowerer.proc_depth += 1;
    lowerer.const_map_stack.push(HashMap::new());
    let body = lowerer.lower_body(body_text, 0, namespace);
    lowerer.const_map_stack.pop();
    lowerer.proc_depth -= 1;
    body
}

/// Whether a single top-level `proc` body can be lowered in isolation (through
/// the SRV-INCREMENTAL Task 3 body-cache memo) byte-identically to the in-place
/// [`Lowerer::lower_body`].
///
/// The isolated lowering runs against a fresh [`Lowerer`] with an empty
/// const-map frame, so it drops every *cross-item* side effect `lower_body`
/// performs while lowering a body — registering a nested `IRProcedure`, tracking
/// `namespace import`/`export`, recording command aliases, and `TclOO` / `when`
/// definitions. A body qualifies exactly when it carries none of those
/// constructs, so its lowering is a pure function of `(body_text, namespace,
/// dialect, config)`.
///
/// This is a **per-body** gate: a context-carrying sibling `proc` (or top-level
/// `namespace eval` / OO / `when` code) no longer disqualifies the *other* bodies
/// in the same file. The scan is deliberately conservative — a false negative
/// only forgoes reuse (falling back to the identical in-place lowering); a false
/// positive would corrupt the IR — and is backstopped by the corpus differential
/// gates.
///
/// **Precondition:** the caller must also check [`source_may_alias_commands`] at
/// the *file* level. A command alias declared outside any body (`interp alias`)
/// populates the lowerer's alias table that `resolve_alias` consults while
/// lowering every body; this per-body scan cannot see it, so a file that
/// establishes aliases must not install the cache at all.
#[must_use]
pub fn body_cache_eligible(body: &str) -> bool {
    // Command keywords whose body-level use carries a cross-item effect the
    // isolated lowering drops. `proc` covers a nested procedure definition.
    // Matched as a word followed by Tcl inter-word whitespace (space, tab, CR,
    // newline, or a `\`-continuation), so a tab-separated `interp\talias` trips
    // it too.
    const WORD_DISQUALIFIERS: &[&str] = &[
        "namespace",
        "interp",
        "rename",
        "method",
        "when",
        "apply",
        "alias",
        "proc",
    ];
    // Substring markers (the token itself is the marker — no trailing arg).
    const SUBSTR_DISQUALIFIERS: &[&str] = &["oo::", "::oo", "itcl"];
    !SUBSTR_DISQUALIFIERS.iter().any(|kw| body.contains(kw))
        && !WORD_DISQUALIFIERS
            .iter()
            .any(|kw| contains_word_followed_by_ws(body, kw))
}

/// Whether `source` may establish a command alias (`interp alias {} name {}
/// target`, or a static `rename old new`) that a cached proc body could
/// reference.
///
/// `interp alias` and `rename` both populate the lowerer's alias table, and
/// `resolve_alias` consults it while lowering *every* subsequent body — but
/// the isolated body-cache lowering starts with an empty table, so an alias
/// declared at the top level (outside any body the per-body
/// [`body_cache_eligible`] scan inspects) would silently resolve differently
/// there. The whole file must therefore forgo the body cache when this
/// returns `true`. `interp` and `rename` are the only commands that feed the
/// alias table (see `detect_interp_alias` / `detect_rename`), so scanning for
/// either as a word is both sufficient and conservative.
#[must_use]
pub fn source_may_alias_commands(source: &str) -> bool {
    contains_word_followed_by_ws(source, "interp") || contains_word_followed_by_ws(source, "rename")
}

/// True when `word` occurs in `source` followed by Tcl inter-word whitespace —
/// i.e. used as a command with an argument. Ignores the character *before* the
/// word (a preceding non-boundary only over-disqualifies, which is safe).
fn contains_word_followed_by_ws(source: &str, word: &str) -> bool {
    source.match_indices(word).any(|(i, _)| {
        source
            .as_bytes()
            .get(i + word.len())
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\\'))
    })
}

#[cfg(test)]
mod body_cache_eligible_tests {
    use super::body_cache_eligible;

    #[test]
    fn plain_bodies_are_eligible() {
        assert!(body_cache_eligible("set x 1"));
        assert!(body_cache_eligible("puts \"hi $name\"\nreturn $x"));
        assert!(body_cache_eligible("if {$x} { foo } else { bar }"));
        assert!(body_cache_eligible(""));
    }

    #[test]
    fn cross_item_constructs_disqualify() {
        for body in [
            "namespace eval x { }",
            "namespace import ::ns::*",
            "interp alias {} x {} y",
            "rename set myset",
            "apply {{} { }}",
            "when HTTP_REQUEST { }",
            "oo::class create C { }",
            "itcl::class C { }",
            "proc inner {} { }",
        ] {
            assert!(!body_cache_eligible(body), "should disqualify: {body}");
        }
    }

    #[test]
    fn tab_separated_command_still_disqualifies() {
        assert!(!body_cache_eligible("interp\talias {} x {} y"));
        assert!(!body_cache_eligible("namespace\timport ::ns::*"));
    }

    #[test]
    fn substring_only_use_is_safe() {
        assert!(body_cache_eligible("set myproclist {a b c}"));
        assert!(body_cache_eligible("set renamed 1"));
    }

    #[test]
    fn source_may_alias_commands_flags_interp() {
        use super::source_may_alias_commands;
        // `interp alias` establishes an alias a cached body cannot see.
        assert!(source_may_alias_commands(
            "interp alias {} = {} expr\nproc f {} { = 1 }\n"
        ));
        assert!(source_may_alias_commands("interp\talias {} = {} expr"));
        // No `interp` -> safe to cache.
        assert!(!source_may_alias_commands(
            "proc f {x} { set y $x }\nproc g {} { return 1 }\n"
        ));
        // `interp` as a substring of a bareword does not trip it.
        assert!(!source_may_alias_commands("set interpreter 1"));
    }

    // Proves *why* `source_may_alias_commands` must guard the file (Codex #739
    // P2): with a top-level `interp alias {} = {} expr` in scope, lowering `f`'s
    // body through the isolated body cache (empty alias table) resolves `=` as an
    // unknown command, whereas the in-place whole-file lowering resolves it to
    // `expr` — so the module IRs differ. The db therefore must not install the
    // cache for such a file.
    #[test]
    fn alias_in_scope_makes_body_cache_diverge() {
        use tcl_registry::CommandRegistry;

        use super::{
            lower_proc_body_isolated, lower_to_ir_with_body_cache, lower_to_ir_with_config,
        };

        let reg = CommandRegistry::build_default();
        let cfg = tcl_lexer::LexerConfig::default();
        let src = "interp alias {} = {} expr\nproc f {x} { return [= {$x + 1}] }\n";
        let cache = |body: &str, ns: &str| lower_proc_body_isolated(body, ns, &reg, cfg);
        let cached = lower_to_ir_with_body_cache(src, &reg, cfg, &cache);
        let fresh = lower_to_ir_with_config(src, &reg, cfg);
        assert_ne!(
            format!("{cached:?}"),
            format!("{fresh:?}"),
            "an alias-in-scope body cache is expected to diverge from the in-place lowering"
        );
    }

    #[test]
    fn source_may_alias_commands_flags_rename() {
        use super::source_may_alias_commands;
        // A static `rename` establishes an alias a cached body cannot see —
        // same hazard as `interp alias`, see `detect_rename`.
        assert!(source_may_alias_commands(
            "rename puts myputs\nproc f {x} { myputs $x }\n"
        ));
        assert!(source_may_alias_commands("rename\tputs myputs"));
        // `rename` as a substring of a bareword does not trip it.
        assert!(!source_may_alias_commands("set renamed 1"));
    }

    // Same hazard as `alias_in_scope_makes_body_cache_diverge`, for a static
    // `rename`: `rename puts myputs` at the top level must make `f`'s body
    // resolve `myputs` to `puts`, which the isolated (empty-alias-table) body
    // cache cannot see.
    #[test]
    fn rename_in_scope_makes_body_cache_diverge() {
        use tcl_registry::CommandRegistry;

        use super::{
            lower_proc_body_isolated, lower_to_ir_with_body_cache, lower_to_ir_with_config,
        };

        let reg = CommandRegistry::build_default();
        let cfg = tcl_lexer::LexerConfig::default();
        let src = "rename puts myputs\nproc f {x} { myputs $x }\n";
        let cache = |body: &str, ns: &str| lower_proc_body_isolated(body, ns, &reg, cfg);
        let cached = lower_to_ir_with_body_cache(src, &reg, cfg, &cache);
        let fresh = lower_to_ir_with_config(src, &reg, cfg);
        assert_ne!(
            format!("{cached:?}"),
            format!("{fresh:?}"),
            "a rename-in-scope body cache is expected to diverge from the in-place lowering"
        );
    }
}

/// Like [`lower_to_ir_with_config`] but with a memoised per-procedure body-lowering
/// callback (SRV-INCREMENTAL Task 3): a top-level `proc`'s static body is lowered
/// through `body_cache` `(offset-0 body text, namespace) -> offset-0 Script` and
/// rebased, so an unchanged proc's body IR is reused across edits.  The caller must
/// only install a cache for **context-free** files (see [`Lowerer::body_cache`]);
/// byte-identity is guarded by the corpus differential gates.
#[must_use]
pub fn lower_to_ir_with_body_cache(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    body_cache: &dyn Fn(&str, &str) -> Script,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config).with_body_cache(body_cache),
        source,
    )
}

/// Like [`lower_to_ir`] but with an explicit dialect
/// [`tcl_lexer::LexerConfig`], threaded into every body re-segmentation
/// so `{*}` expansion (off for Tcl 8.4 / iRules) and the iRules `}{`
/// ghost SEP are honoured.
#[must_use]
pub fn lower_to_ir_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Module {
    lower_with(Lowerer::with_config(registry, config), source)
}

/// Like [`lower_to_ir`] but for the bytecode/VM compile path: constructs the
/// backend can't compile correctly (`try`, and a `foreach`/`lmap` directly
/// nesting another) are lowered to runtime-command barriers (see
/// [`Lowerer::for_bytecode`]). Analysis callers keep the structured IR via
/// [`lower_to_ir`].
#[must_use]
pub fn lower_to_ir_for_bytecode(source: &str, registry: &CommandRegistry) -> Module {
    lower_with(
        Lowerer::with_config(registry, tcl_lexer::LexerConfig::default()).for_bytecode_backend(),
        source,
    )
}

/// Like [`lower_to_ir_for_bytecode`] but with every inline/structured
/// lowering hook suppressed (see [`Lowerer::trace_visible`]) — every command
/// in `source` compiles to a plain runtime dispatch, so an execution trace
/// observes it. For recompiling a proc/script body once a step-capable
/// execution trace targets it (issue #946); see
/// [`tcl_runtime_api::CompileService::compile_traced`].
#[must_use]
pub fn lower_to_ir_traced(source: &str, registry: &CommandRegistry) -> Module {
    lower_with(
        Lowerer::with_config(registry, tcl_lexer::LexerConfig::default())
            .for_bytecode_backend()
            .trace_visible(),
        source,
    )
}

/// Lexer warning messages that correspond to a hard `Tcl_ParseCommand`
/// failure in C Tcl — i.e. a malformed word that C Tcl reports as a parse
/// error rather than tokenising leniently.
///
/// The lexer detects these but only *raises* them under `strict_quoting`
/// (off by default); in the VM compile path it records them as warnings and
/// recovers. C Tcl, however, aborts compilation of the script and defers the
/// error to a bytecode instruction that raises it at runtime — so a parse
/// error inside a `catch {…}` body (re-compiled at runtime) is catchable. See
/// [`first_fatal_parse_error`].
const FATAL_PARSE_MESSAGES: &[&str] = &[
    "extra characters after close-quote",
    "extra characters after close-brace",
    "missing close-brace",
    "missing close-brace for variable name",
    "missing \"",
    "missing )",
    "missing close-bracket",
];

/// Return the first hard parse-error message in `source`, in source order, or
/// `None` if `source` parses cleanly.
///
/// A VM compile front-end (`CompileService::compile`) calls this so that a
/// malformed script becomes a catchable runtime error carrying the **exact**
/// C Tcl message (e.g. `extra characters after close-quote`), matching C Tcl's
/// deferral of a compile-time parse error to a runtime instruction. The lexer
/// otherwise recovers from these for the benefit of editor diagnostics, so the
/// IR / bytecode never sees them.
///
/// Lexes with the default (Tcl-8.5+) dialect config — the same one
/// [`lower_to_ir_for_bytecode`] uses. Only the messages in
/// [`FATAL_PARSE_MESSAGES`] count; benign warnings are ignored.
#[must_use]
pub fn first_fatal_parse_error(source: &str) -> Option<String> {
    let lexer = tcl_lexer::Lexer::new(source);
    let (_tokens, warnings) = lexer.tokenise_all_with_warnings().ok()?;
    warnings
        .into_iter()
        .filter(|w| FATAL_PARSE_MESSAGES.contains(&w.message.as_str()))
        .min_by_key(|w| w.offset)
        .map(|w| w.message)
}

/// Drive a configured [`Lowerer`] to a finished [`Module`] (the shared tail of
/// the `lower_to_ir*` entry points).
fn lower_with(mut lowerer: Lowerer<'_>, source: &str) -> Module {
    lowerer.lower(source);
    // Extract TclOO method bodies from the fully-assembled
    // module (cache-independent — see `extract_oo_methods_pass`).
    lowerer.extract_oo_methods_pass();
    let registry = lowerer.registry;
    let mut module = lowerer.module;
    module.source = source.to_string();
    populate_trace_facts(&mut module, registry);
    module
}

/// Post-lower scan for `trace add ...` calls that populates
/// `Module::traced_commands` / `has_dynamic_trace` (execution traces)
/// and `Module::traced_variables` / `has_dynamic_variable_trace`
/// (variable traces).
///
/// Runs over the top-level script + every procedure body + every `TclOO`
/// method body (`module.methods`, already populated by
/// `extract_oo_methods_pass` before this runs).  Literal command names
/// land in `traced_commands` (`::`-stripped to match the canonical key);
/// non-literal targets (`$cmd`, `[expr ...]`, command substitutions) flip
/// `has_dynamic_trace` so GVN treats every call as potentially traced.
/// Literal variable names targeted by any `Traits::
/// ESTABLISHES_VARIABLE_TRACE` subcommand — the registry's
/// `ArgRole::VarWrite` resolution for `trace add|remove|variable|vdelete`,
/// covering both the modern and deprecated legacy spellings — land in
/// `traced_variables`; non-literal targets flip
/// `has_dynamic_variable_trace`.
fn populate_trace_facts(module: &mut Module, registry: &CommandRegistry) {
    let top_level = module.top_level.clone();
    walk_for_trace(&top_level, module, registry, 0);
    // Every statically-known frame, not just named procedures: a `trace`
    // call inside a `namespace eval` / `apply` body (`Module::body_units`)
    // or a `TclOO` method (`Module::methods`) is just as live as one inside
    // a `proc` — omitting either class here would silently under-populate
    // `traced_variables` the moment an optimiser pass starts reaching into
    // those bodies (`CompilationUnit::all_body_function_units`).
    let bodies: Vec<Script> = module
        .procedures
        .values()
        .map(|p| p.body.clone())
        .chain(module.body_units.values().map(|p| p.body.clone()))
        .chain(module.methods.values().map(|m| m.body.clone()))
        .collect();
    for body in &bodies {
        walk_for_trace(body, module, registry, 0);
    }
    let method_bodies: Vec<Script> = module.methods.values().map(|m| m.body.clone()).collect();
    for body in &method_bodies {
        walk_for_trace(body, module, registry, 0);
    }
}

/// Resolve a `trace add|remove` type word (`variable`/`command`/
/// `execution`) against C Tcl 9.0's `Tcl_GetIndexFromObj` abbreviation
/// rule: a unique, non-empty prefix is accepted (`trace add e foo enter
/// h` installs the same execution trace as the full spelling, checked
/// against tclsh 8.6.14). Mirrors
/// `tcl_registry::commands::tcl::trace`'s private resolver of the same
/// name — duplicated rather than exposed across the crate boundary for
/// one three-word list.
fn resolve_trace_type_word(word: &str) -> Option<&'static str> {
    const TYPES: &[&str] = &["variable", "command", "execution"];
    if word.is_empty() {
        return None;
    }
    let mut hits = TYPES.iter().copied().filter(|t| t.starts_with(word));
    let first = hits.next()?;
    if hits.next().is_some() {
        return None; // ambiguous prefix
    }
    Some(first)
}

/// `depth` is the nesting level of `script` — see [`MAX_LOWER_NEST_DEPTH`]
/// (this post-lower scan reuses the same cap `lower_script`/`lower_body`
/// already build every `Script` under, for consistency; every input this
/// walk sees is already transitively bounded to that depth by construction,
/// so this is defence-in-depth rather than a currently-reachable path).
fn walk_for_trace(script: &Script, module: &mut Module, registry: &CommandRegistry, depth: u32) {
    use crate::ir::Statement;
    if depth as usize > MAX_LOWER_NEST_DEPTH {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            // Canonical (alias-resolved) name, so `interp alias exampleTrace trace`
            // still lands in `traced_commands`/`traced_variables` instead of being
            // silently missed because the call site spells it `exampleTrace ...`.
            Statement::Call { args, .. } | Statement::Barrier { args, .. }
                if stmt.canonical_command_or_source().trim_start_matches("::") == "trace" =>
            {
                let command = stmt.canonical_command_or_source();
                let is_add = args.first().is_some_and(|w| {
                    registry
                        .get(command)
                        .and_then(|spec| spec.resolve_subcommand(w))
                        .is_some_and(|sub| sub.name == "add")
                });
                if args.len() >= 4
                    && is_add
                    && resolve_trace_type_word(&args[1]) == Some("execution")
                {
                    let target = &args[2];
                    if is_literal_trace_target(target) {
                        let canonical = target.trim_start_matches("::").to_string();
                        if !canonical.is_empty() {
                            module.traced_commands.insert(canonical);
                        }
                    } else {
                        module.has_dynamic_trace = true;
                    }
                }
                populate_variable_trace_facts(command, args, module, registry);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_for_trace(&c.body, module, registry, depth + 1);
                }
                if let Some(e) = else_body {
                    walk_for_trace(e, module, registry, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_for_trace(init, module, registry, depth + 1);
                walk_for_trace(next, module, registry, depth + 1);
                walk_for_trace(body, module, registry, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => walk_for_trace(body, module, registry, depth + 1),
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(b) = &arm.body {
                        walk_for_trace(b, module, registry, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    walk_for_trace(b, module, registry, depth + 1);
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_for_trace(body, module, registry, depth + 1);
                for h in handlers {
                    walk_for_trace(&h.body, module, registry, depth + 1);
                }
                if let Some(f) = finally_body {
                    walk_for_trace(f, module, registry, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Word indices of every variable-trace target that `command`/`args`
/// installs or removes an active trace on — any subcommand carrying
/// `Traits::ESTABLISHES_VARIABLE_TRACE` (`trace add`/`remove`/
/// `variable`/`vdelete` — not the read-only `info`/`vinfo` forms),
/// located via the registry's `ArgRole::VarWrite` resolution. Entirely
/// data-driven off the registry — no hardcoded knowledge of `trace`'s
/// subcommand grammar (`add` vs the legacy `variable` spelling, which
/// argument position holds the name, …). Shared by
/// [`populate_variable_trace_facts`] (the whole-module fact) and
/// `var_observability::stmt_gen` (the flow-sensitive `TRACED` mark), so
/// both derive the same trace-target positions from one query.
pub(crate) fn variable_trace_write_indices(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<usize> {
    let Some(spec) = registry.get(command) else {
        return Vec::new();
    };
    let Some(sub) = args.first().and_then(|s| spec.resolve_subcommand(s)) else {
        return Vec::new();
    };
    if !sub
        .traits
        .contains(tcl_registry::prelude::Traits::ESTABLISHES_VARIABLE_TRACE)
    {
        return Vec::new();
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    registry.arg_indices_for_role(command, &arg_strs, ArgRole::VarWrite)
}

/// Registry-driven half of [`walk_for_trace`]: record every literal
/// variable name a `trace` call targets via a
/// `Traits::ESTABLISHES_VARIABLE_TRACE` subcommand (`add`/`remove`/
/// `variable`/`vdelete` — not the read-only `info`/`vinfo` forms).
/// Dispatch is entirely data-driven off the registry's
/// `ArgRole::VarWrite` resolution — this function has no hardcoded
/// knowledge of `trace`'s subcommand grammar (`add` vs the legacy
/// `variable` spelling, which argument position holds the name, …).
fn populate_variable_trace_facts(
    command: &str,
    args: &[String],
    module: &mut Module,
    registry: &CommandRegistry,
) {
    for idx in variable_trace_write_indices(registry, command, args) {
        let Some(target) = args.get(idx) else {
            continue;
        };
        if is_literal_trace_target(target) {
            // `::`-stripped to match the canonical key — mirrors
            // `traced_commands`'s treatment of an execution-trace
            // target, so a top-level `set x 5` (chain key `x`) matches
            // a `trace add variable ::x ...` target the same way an
            // unqualified `trace add variable x ...` would.
            let canonical = crate::naming::normalise_var_name(&format!("${target}"))
                .trim_start_matches("::")
                .to_string();
            if !canonical.is_empty() {
                module.traced_variables.insert(canonical);
            }
        } else {
            module.has_dynamic_variable_trace = true;
        }
    }
}

/// True when a `trace` target word names a static variable literally —
/// no substitution, quoting, or whitespace.  Shared with the optimiser's
/// scope-alias / traced-global scans so every consumer applies one
/// literalness rule.
pub(crate) fn is_literal_trace_target(s: &str) -> bool {
    !s.is_empty()
        && !s.contains('$')
        && !s.contains('[')
        && !s.contains('{')
        && !s.contains('"')
        && !s.contains(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn static_bool_accepts_unique_prefix_booleans() {
        // A condition folds boolean words by unique prefix, like Tcl
        // (`if {tr}` runs, `if {of}` does not — verified against tclsh).
        assert_eq!(static_bool("tr"), Some(true));
        assert_eq!(static_bool(" ye "), Some(true));
        assert_eq!(static_bool("on"), Some(true));
        assert_eq!(static_bool("of"), Some(false));
        assert_eq!(static_bool("n"), Some(false));
        assert_eq!(static_bool("0"), Some(false));
        assert_eq!(static_bool("1"), Some(true));
        // Ambiguous `o` (on/off) is not a static boolean.
        assert_eq!(static_bool("o"), None);
        assert_eq!(static_bool("$x"), None);
    }

    #[test]
    fn lowering_is_dialect_aware_via_expand_syntax() {
        // Lowering re-segments each
        // body under the document dialect.  For `if {*}$cond { puts hi }`,
        // on 8.5+ the `{*}` expands the condition, so the structured `if`
        // trips the expand-barrier (an opaque `Statement::Barrier` — the
        // structure can't be reasoned about); on 8.4 `{*}` is a literal
        // word, so the `if` lowers normally (no barrier).
        let reg = reg();
        let src = "if {*}$cond { puts hi }";
        let m90 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
        let m84 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::for_dialect("tcl8.4"));
        let is_barrier = |m: &Module| {
            matches!(
                m.top_level.statements.first(),
                Some(Statement::Barrier { .. })
            )
        };
        assert!(
            is_barrier(&m90),
            "9.0 expands `{{*}}` → structured `if` becomes a barrier: {:?}",
            m90.top_level.statements,
        );
        assert!(
            !is_barrier(&m84),
            "8.4 treats `{{*}}` as literal → `if` lowers without a barrier: {:?}",
            m84.top_level.statements,
        );
    }

    #[test]
    fn empty_source() {
        let m = lower_to_ir("", &reg());
        assert!(m.top_level.statements.is_empty());
    }

    #[test]
    fn simple_set() {
        let m = lower_to_ir("set x 1", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::AssignConst { name, value, .. } if name == "x" && value == "1"
        ));
    }

    #[test]
    fn set_with_variable_value() {
        let m = lower_to_ir("set y $x", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::AssignValue { name, .. } if name == "y"
        ));
    }

    #[test]
    fn incr_command() {
        let m = lower_to_ir("incr i", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Incr { name, .. } if name == "i"
        ));
    }

    // The `extract_oo_methods_pass` post-pass
    // lifts TclOO method bodies into `module.methods`.

    #[test]
    fn extracts_oo_class_methods() {
        let src = "oo::class create Counter {\n\
                   \x20   variable n\n\
                   \x20   constructor {} { set n 0 }\n\
                   \x20   method bump {} { incr n }\n\
                   \x20   method get {} { return $n }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        let mut keys: Vec<&String> = m.methods.keys().collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                &"::Counter::<constructor>".to_string(),
                &"::Counter::bump".to_string(),
                &"::Counter::get".to_string(),
            ],
            "methods: {keys:?}",
        );
        let get = &m.methods["::Counter::get"];
        assert_eq!(get.method_name, "get");
        assert_eq!(get.kind, MethodKind::Method);
        assert_eq!(get.class_name, "::Counter");
        // `variable n` in the class body is an in-scope instance var
        // for every method (auto-linked).
        assert!(
            get.instance_vars.contains("n"),
            "get ivars: {:?}",
            get.instance_vars
        );
        // The method body is lowered for analysis (the `incr n` /
        // `return $n` statements are present, not an empty barrier).
        let bump = &m.methods["::Counter::bump"];
        assert!(
            !bump.body.statements.is_empty(),
            "bump body should be lowered"
        );
        let ctor = &m.methods["::Counter::<constructor>"];
        assert_eq!(ctor.kind, MethodKind::Constructor);
        assert!(m.redefined_methods.is_empty());
    }

    #[test]
    fn lowers_oo_configurable_class_body() {
        // `oo::configurable create` (and `oo::abstract` / `oo::singleton`)
        // share the `METACLASS create NAME { body }` shape, so their bodies
        // must be lowered and their methods lifted like `oo::class`'s — not left
        // as an unanalysed barrier (issue #797: a `Device` defined with
        // `oo::configurable` otherwise had every method body skipped).
        let src = "oo::configurable create Pin {\n\
                   \x20   property node\n\
                   \x20   method describe {} { return [my configure -node] }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.methods.contains_key("::Pin::describe"),
            "oo::configurable method must be lifted; methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
        assert!(
            !m.methods["::Pin::describe"].body.statements.is_empty(),
            "the method body should be lowered, not barriered"
        );
    }

    #[test]
    fn extracts_oo_define_methods_and_namespaced_class() {
        // `oo::define` adds methods to an existing class, and a class
        // created inside `namespace eval` qualifies under that ns.
        let src = "namespace eval ::app {\n\
                   \x20   oo::class create Widget {\n\
                   \x20       method draw {} { return ok }\n\
                   \x20   }\n\
                   }\n\
                   oo::define ::app::Widget {\n\
                   \x20   method hide {} { return done }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.methods.contains_key("::app::Widget::draw"),
            "methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
        assert!(
            m.methods.contains_key("::app::Widget::hide"),
            "methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn redefined_oo_method_is_flagged() {
        // A method redefined by a later `oo::define` keeps the first
        // body but is recorded in `redefined_methods` so purity stays
        // conservative.
        let src = "oo::class create C {\n\
                   \x20   method m {} { return 1 }\n\
                   }\n\
                   oo::define C {\n\
                   \x20   method m {} { return 2 }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(m.methods.contains_key("::C::m"));
        assert!(
            m.redefined_methods.contains("::C::m"),
            "redefined: {:?}",
            m.redefined_methods
        );
    }

    #[test]
    fn oo_method_nested_proc_not_registered_globally() {
        // A `proc` defined inside a method body is created at
        // method-call time, not script load — it must not leak into
        // `module.procedures` (codegen safety).
        let src = "oo::class create C {\n\
                   \x20   method m {} { proc helper {} { return 1 }; return 2 }\n\
                   }\n\
                   proc toplevel {} { return 3 }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.procedures.contains_key("::toplevel"),
            "procs: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        assert!(
            !m.procedures.contains_key("::helper"),
            "nested method proc must not be registered: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        // The method body still lowered the `proc helper ...` call.
        assert!(m.methods.contains_key("::C::m"));
    }

    #[test]
    fn apply_body_proc_registers_in_lambda_namespace_not_callers() {
        // `apply`'s body runs in the namespace named by the lambda — or the
        // *global* namespace when none is given — NOT the caller's namespace.
        // So a nested `proc` registers as `::helper`, never `::foo::helper`
        // (Tcl `apply` manual).
        let src = "namespace eval foo {\n\
                   \x20   apply {{} { proc helper {} { return 1 } }}\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.procedures.contains_key("::helper"),
            "apply body proc must register in the global namespace: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        assert!(
            !m.procedures.contains_key("::foo::helper"),
            "apply body proc must NOT inherit the caller namespace: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn apply_body_proc_uses_explicit_lambda_namespace() {
        // A lambda whose element 2 pins a namespace runs its body there.
        let src = "apply {{} { proc helper {} { return 1 } } ::bar}\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.procedures.contains_key("::bar::helper"),
            "explicit lambda namespace must be honoured: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_oo_methods_for_plain_script() {
        let m = lower_to_ir("proc greet {} { puts hi }", &reg());
        assert!(m.methods.is_empty());
        assert!(m.redefined_methods.is_empty());
    }

    #[test]
    fn proc_definition() {
        let m = lower_to_ir("proc greet {name} {puts $name}", &reg());
        // proc emits an IRCall + registers a procedure.
        assert!(m.procedures.contains_key("::greet"));
        let p = &m.procedures["::greet"];
        assert_eq!(p.params, vec!["name"]);
    }

    #[test]
    fn if_statement() {
        let m = lower_to_ir("if {1} {set x 1}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(&m.top_level.statements[0], Statement::If { .. }));
    }

    #[test]
    fn for_loop() {
        let m = lower_to_ir("for {set i 0} {$i < 10} {incr i} {puts $i}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(&m.top_level.statements[0], Statement::For { .. }));
    }

    #[test]
    fn while_loop() {
        let m = lower_to_ir("while {1} {puts loop}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::While { .. }
        ));
    }

    #[test]
    fn foreach_loop() {
        let m = lower_to_ir("foreach x {a b c} {puts $x}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Foreach { .. }
        ));
    }

    #[test]
    fn catch_statement() {
        let m = lower_to_ir("catch {error oops} result", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Catch { .. }
        ));
    }

    #[test]
    fn generic_command() {
        let m = lower_to_ir("puts hello", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "puts"
        ));
    }

    #[test]
    fn multiple_commands() {
        let m = lower_to_ir("set x 1\nset y 2\nputs $x", &reg());
        assert_eq!(m.top_level.statements.len(), 3);
    }

    #[test]
    fn return_statement() {
        let m = lower_to_ir("return 42", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Return { value: Some(v), .. } if v == "42"
        ));
    }

    #[test]
    fn lower_to_ir_public_api() {
        let r = reg();
        let m = lower_to_ir("set x 1\nproc foo {} {return 1}", &r);
        assert_eq!(m.top_level.statements.len(), 2);
        assert!(m.procedures.contains_key("::foo"));
    }

    #[test]
    fn parse_param_names_basic() {
        assert_eq!(parse_param_names("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_param_names_braced() {
        assert_eq!(parse_param_names("{x default} y"), vec!["x", "y"]);
    }

    #[test]
    fn parse_param_names_empty() {
        assert!(parse_param_names("").is_empty());
    }

    #[test]
    fn qualify_proc_name_global() {
        assert_eq!(qualify_proc_name("::", "foo"), "::foo");
    }

    #[test]
    fn qualify_proc_name_nested() {
        assert_eq!(qualify_proc_name("::ns", "bar"), "::ns::bar");
    }

    #[test]
    fn qualify_proc_name_already_qualified() {
        assert_eq!(qualify_proc_name("::ns", "::abs"), "::abs");
    }

    #[test]
    fn parse_uplevel_level_decimal() {
        assert_eq!(parse_uplevel_level("1"), Some(1));
        assert_eq!(parse_uplevel_level("3"), Some(3));
        assert_eq!(parse_uplevel_level("0"), Some(0));
    }

    #[test]
    fn parse_uplevel_level_hash_form() {
        assert_eq!(parse_uplevel_level("#0"), Some(0));
        assert_eq!(parse_uplevel_level("#3"), Some(-3));
    }

    #[test]
    fn parse_uplevel_level_dynamic_returns_none() {
        assert_eq!(parse_uplevel_level("$lvl"), None);
        assert_eq!(parse_uplevel_level("[expr {1+1}]"), None);
        assert_eq!(parse_uplevel_level("foo"), None);
    }

    #[test]
    fn uplevel_static_body_no_level() {
        let m = lower_to_ir("uplevel {set x 1}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        match &m.top_level.statements[0] {
            Statement::UpFrame {
                frame_shift, body, ..
            } => {
                assert_eq!(*frame_shift, 1);
                assert_eq!(body.statements.len(), 1);
            }
            other => panic!("expected UpFrame, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_static_body_with_level_one() {
        let m = lower_to_ir("uplevel 1 {set x 1}", &reg());
        match &m.top_level.statements[0] {
            Statement::UpFrame { frame_shift, .. } => assert_eq!(*frame_shift, 1),
            other => panic!("expected UpFrame, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_static_body_with_hash_zero() {
        let m = lower_to_ir("uplevel #0 {set x 1}", &reg());
        match &m.top_level.statements[0] {
            Statement::UpFrame { frame_shift, .. } => assert_eq!(*frame_shift, 0),
            other => panic!("expected UpFrame for #0, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_dynamic_body_falls_back_to_default() {
        // ``uplevel 1 $body`` body is a $var, not a brace literal —
        // can't be statically resolved without const-propagation.
        // Falls back to ``lower_default`` (Statement::Call).
        let m = lower_to_ir("uplevel 1 $body", &reg());
        assert!(matches!(
            m.top_level.statements[0],
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
    }

    #[test]
    fn uplevel_dynamic_level_falls_back_to_default() {
        // ``uplevel $lvl {body}`` — level is dynamic, can't pick a
        // ``frame_shift``. Falls back to default lowering.
        let m = lower_to_ir("uplevel $lvl {set x 1}", &reg());
        assert!(matches!(
            m.top_level.statements[0],
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
    }

    #[test]
    fn const_prop_uplevel_resolves_set_var_body() {
        // ``set body {set x 1}; uplevel 1 $body`` inside a
        // proc — the const-map records ``body`` and the uplevel
        // folds in the literal as an UpFrame.
        let m = lower_to_ir("proc f {} { set body {set x 1}\n uplevel 1 $body }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        // proc body: [AssignConst(body), UpFrame { body: [...] }]
        let last = proc.body.statements.last().expect("body has statements");
        match last {
            Statement::UpFrame { body, .. } => {
                assert!(!body.statements.is_empty(), "expected lowered body");
            }
            other => panic!("expected UpFrame, got {other:?}"),
        }
    }

    #[test]
    fn const_prop_eval_resolves_set_var_body() {
        // ``eval $body`` with const-mapped body folds to a
        // Statement::Block.
        let m = lower_to_ir("proc f {} { set body {set x 1}\n eval $body }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        let last = proc.body.statements.last().expect("body has statements");
        match last {
            Statement::Block { body, .. } => {
                assert!(!body.statements.is_empty(), "expected lowered block");
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn const_prop_eval_brace_body_emits_block() {
        // ``eval {body}`` in a proc context lowers to Block.
        let m = lower_to_ir("proc f {} { eval {set x 1} }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_disabled_at_top_level() {
        // The const-map is gated on ``proc_depth > 0``. At
        // top-level, ``set body {set x 1}; uplevel 1 $body``
        // does NOT relax — the uplevel falls back to default
        // dispatch (Call / Barrier).
        let m = lower_to_ir("set body {set x 1}\nuplevel 1 $body", &reg());
        // statements[0] = AssignConst(body), [1] = uplevel call
        assert!(matches!(
            m.top_level.statements[1],
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
    }

    #[test]
    fn const_prop_invalidated_on_reassignment() {
        // ``set body {a}; set body $other; uplevel 1 $body`` — the
        // second ``set`` invalidates the binding (RHS is a $var,
        // not a brace literal), so the uplevel can't fold.
        let m = lower_to_ir(
            "proc f {} { set body {a}\n set body $other\n uplevel 1 $body }",
            &reg(),
        );
        let proc = m.procedures.get("::f").expect("proc registered");
        let last = proc.body.statements.last().expect("body");
        assert!(
            !matches!(last, Statement::UpFrame { .. }),
            "expected fallback after re-assignment, got {last:?}",
        );
    }

    #[test]
    fn const_prop_eval_list_literal() {
        // ``eval [list set x 42]`` recognised as static body.
        let m = lower_to_ir("proc f {} { eval [list set x 42] }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_eval_list_with_dynamic_arg_rejected() {
        // ``eval [list set x $v]`` — dynamic ``\$v`` rejects the
        // list-literal shape. Falls back to runtime barrier.
        let m = lower_to_ir("proc f {} { eval [list set x $v] }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_eval_non_list_command_rejected() {
        // ``eval [foo arg]`` — inner command isn't ``list``;
        // can't synthesise a body. Falls back to runtime barrier.
        let m = lower_to_ir("proc f {} { eval [foo arg] }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_does_not_leak_into_nested_proc() {
        // A ``set body {literal}`` in the outer proc must
        // NOT appear to a nested ``proc inner``'s
        // barrier-relaxation gate as a tracked literal.
        let m = lower_to_ir(
            "proc outer {} {\n  set body {set x 1}\n  proc inner {} { uplevel 1 $body }\n}",
            &reg(),
        );
        let inner = m.procedures.get("::inner").expect("inner registered");
        // The inner uplevel must remain a Call/Barrier — NOT an
        // UpFrame. If the const-map leaked, the inner body would
        // be folded as UpFrame { body: [set x 1], .. }.
        let last = inner.body.statements.last().expect("body");
        assert!(
            !matches!(last, Statement::UpFrame { .. }),
            "outer scope's const-map must not leak into nested proc, got {last:?}",
        );
    }

    #[test]
    fn ns_import_recorded_with_context_namespace() {
        // ``namespace import ::tcltest::*`` at top-level
        // records (``::``, ``::tcltest::*``).
        let m = lower_to_ir("namespace import ::tcltest::*", &reg());
        assert_eq!(
            m.namespace_imports,
            vec![("::".to_string(), "::tcltest::*".to_string())]
        );
    }

    #[test]
    fn ns_import_skips_relative_pattern() {
        // Relative patterns (``foo::*`` without leading ``::``)
        // require runtime namespace-path walking; we skip them.
        let m = lower_to_ir("namespace import foo::*", &reg());
        assert!(m.namespace_imports.is_empty());
    }

    #[test]
    fn ns_import_handles_force_flag() {
        // ``-force`` is the documented option; the next word is
        // the pattern.
        let m = lower_to_ir("namespace import -force ::tcltest::*", &reg());
        assert_eq!(
            m.namespace_imports,
            vec![("::".to_string(), "::tcltest::*".to_string())]
        );
    }

    #[test]
    fn proc_dynamic_name_resolved_via_const_map() {
        // ``set name {Verbose}; proc \$name {} { ... }``
        // inside a proc — ``name`` is in the const-map, so the
        // inner proc registers as ``::Verbose``.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n proc $name {} { puts hi } }",
            &reg(),
        );
        assert!(
            m.procedures.contains_key("::Verbose"),
            "expected ::Verbose in {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn proc_dynamic_name_no_binding_stays_barrier() {
        // ``proc \$name`` with no const-map entry — bail to barrier.
        let m = lower_to_ir("proc factory {} { proc $name {} { puts hi } }", &reg());
        let factory = m.procedures.get("::factory").expect("factory registered");
        let last = factory.body.statements.last().expect("body");
        assert!(
            matches!(last, Statement::Barrier { .. }),
            "expected Barrier, got {last:?}"
        );
    }

    #[test]
    fn proc_with_trailing_colons_routes_through_barrier() {
        // Tcl 9 ``proc ::ns:: {args} {body}`` registers a command
        // named ``""`` inside ``::ns``.  Static normalisation would
        // strip the trailing ``::`` and register the proc under the
        // namespace name itself — wrong cmd.  Route through
        // Statement::Barrier so the runtime ``proc`` builtin handles
        // the registration.
        let m = lower_to_ir("proc ::ns:: {a b} { puts $a$b }", &reg());
        // No procedure entry should have been AOT-registered under
        // ``::ns`` (which is what the bug would produce) or any
        // empty-key variant.
        assert!(
            !m.procedures.contains_key("::ns")
                && !m.procedures.contains_key("::ns::")
                && !m.procedures.contains_key(""),
            "trailing-:: proc should not be AOT-registered; got {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        // The top-level statement should be a Barrier with the
        // empty-simple-name reason.
        let top = m
            .top_level
            .statements
            .last()
            .expect("at least one top-level statement");
        match top {
            Statement::Barrier {
                reason, command, ..
            } => {
                assert_eq!(command, "proc");
                assert!(
                    reason.contains("empty-simple-name"),
                    "unexpected barrier reason: {reason}"
                );
            }
            other => panic!("expected Statement::Barrier, got {other:?}"),
        }
    }

    #[test]
    fn proc_with_command_substitution_in_name_stays_barrier() {
        // ``proc \$name[suffix]`` — multi-token name with a command
        // substitution. The const-map gate only covers single-VAR
        // tokens; this stays on the runtime path.
        let m = lower_to_ir(
            "proc factory {} { set name {x}\n proc $name[suffix] {} { puts hi } }",
            &reg(),
        );
        // ``::factory`` registered; the inner proc must NOT be
        // registered under any literal name (``::x...``).
        assert!(!m.procedures.keys().any(|k| k.contains("suffix")));
    }

    #[test]
    fn proc_subst_nocommands_body_materialised() {
        // ``proc \$name {x} [subst -nocommands {return \$default}]``
        // with both ``name`` and ``default`` const-tracked materialises
        // the body to ``return 0`` and lowers it as a real script.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nocommands {return $default}] }",
            &reg(),
        );
        let inner = m.procedures.get("::Verbose").expect("::Verbose registered");
        // Body should contain a Return statement (or at least be
        // non-empty — the materialised body lowers to a real
        // statement, not a Barrier).
        assert!(
            !inner.body.statements.is_empty(),
            "expected lowered body, got empty"
        );
    }

    #[test]
    fn foreach_line_lowers_to_foreach_statement() {
        // `foreachLine var filename body` lowers
        // to a single-iterator IRForeach so variables assigned
        // inside the body propagate to the enclosing scope.  The
        // generic stdlib-proc path would route through IRBarrier and
        // lose that lattice information.
        let m = lower_to_ir(
            "foreachLine line /etc/hosts { set count [expr {$count + 1}] }",
            &reg(),
        );
        let stmts = &m.top_level.statements;
        let last = stmts.last().expect("at least one statement");
        match last {
            Statement::Foreach {
                iterators, is_lmap, ..
            } => {
                assert!(!is_lmap);
                assert_eq!(iterators.len(), 1);
                assert_eq!(iterators[0].vars, vec!["line".to_owned()]);
            }
            other => panic!("expected Statement::Foreach, got {other:?}"),
        }
    }

    #[test]
    fn foreach_line_var_body_routes_through_barrier() {
        // `foreachLine line file $body` — the body is a Var token,
        // which is single-token but NOT a braced literal.  Compiling
        // the literal `$body` text as a static loop body would
        // produce incorrect IR / data-flow; the lowering must fall
        // through to the runtime command via Barrier.
        let m = lower_to_ir("foreachLine line /etc/hosts $body", &reg());
        let last = m.top_level.statements.last().expect("top-level statement");
        match last {
            Statement::Barrier {
                reason, command, ..
            } => {
                assert_eq!(command, "foreachLine");
                assert!(
                    reason.contains("dynamic body"),
                    "unexpected reason: {reason}"
                );
            }
            other => panic!("expected Barrier for Var body, got {other:?}"),
        }
    }

    #[test]
    fn foreach_line_cmd_body_routes_through_barrier() {
        // Same guard for a `[cmd]` body — single-token but a Cmd
        // substitution, not a braced literal.
        let m = lower_to_ir("foreachLine line /etc/hosts [build-body]", &reg());
        let last = m.top_level.statements.last().expect("top-level statement");
        assert!(
            matches!(last, Statement::Barrier { .. }),
            "expected Barrier for Cmd body, got {last:?}"
        );
    }

    #[test]
    fn proc_var_body_const_map_materialised() {
        // A bare `$body` body word whose value
        // is in the const-map materialises into the IRProcedure body
        // (matching the `proc $name {x} $body` shape).
        let m = lower_to_ir(
            "proc factory {} { set name {Greeter}\n set body {return hi}\n proc $name {} $body }",
            &reg(),
        );
        let inner = m.procedures.get("::Greeter").expect("::Greeter registered");
        // The body was const-resolved to "return hi"; lowering it
        // produces a Return statement.
        assert!(
            !inner.body.statements.is_empty(),
            "expected lowered body, got empty"
        );
    }

    #[test]
    fn proc_var_body_unresolved_keeps_empty_body() {
        // `$body` has no const-map binding but `$name` does — the
        // proc name resolves, but the body stays empty (the runtime
        // proc call below carries the dynamic body text).
        let m = lower_to_ir(
            "proc factory {} { set name {Greeter}\n proc $name {} $body }",
            &reg(),
        );
        let inner = m.procedures.get("::Greeter").expect("::Greeter registered");
        assert!(
            inner.body.statements.is_empty(),
            "expected empty body for unresolved dynamic body, got {:?}",
            inner.body.statements
        );
    }

    #[test]
    fn proc_dynamic_params_routes_through_barrier() {
        // A dynamic params word — there's no static arg-list to
        // build a real IRProcedure from, so the whole proc shape
        // bails to a Barrier.  Even when the name resolves via the
        // const-map.
        let m = lower_to_ir(
            "proc factory {} { set name {x}\n proc $name $params { puts hi } }",
            &reg(),
        );
        // No inner procedure should be registered — params couldn't
        // be parsed statically.
        assert!(
            !m.procedures.contains_key("::x"),
            "dynamic-params shape must not AOT-register: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn proc_multi_token_body_word_is_dynamic() {
        // `proc foo {} "pre${body}post"` — the body is a quoted
        // multi-token word.  Even though the first token is Str,
        // single_token_word is false so the whole word is dynamic
        // and we route through Barrier (the multi-token guard).
        let m = lower_to_ir(
            r#"proc factory {} { set body {return hi}; proc inner {} "pre${body}post" }"#,
            &reg(),
        );
        // `::inner` would only be registered if the body was
        // materialised — which can't happen for multi-token words.
        // We allow either no entry or an empty-body entry; the key
        // assertion is that the literal source text isn't compiled
        // as a script.
        if let Some(inner) = m.procedures.get("::inner") {
            assert!(
                inner.body.statements.is_empty(),
                "multi-token body must not be statically lowered, got {:?}",
                inner.body.statements
            );
        }
    }

    #[test]
    fn proc_subst_nocommands_missing_var_skips_materialisation() {
        // ``\$default`` is not in the const-map — the materialiser
        // refuses, leaving the body to fall back to runtime
        // dispatch.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n proc $name {x} [subst -nocommands {return $default}] }",
            &reg(),
        );
        // Verbose not registered (because \$default missing means
        // we keep the dynamic body which routes via Barrier — but
        // the proc name itself was substituted). Actually
        // the proc IS registered with whatever the body lowering
        // produces; the assertion is that it's NOT the materialised
        // form.
        let inner = m.procedures.get("::Verbose");
        if let Some(p) = inner {
            // The body should not contain a Return whose value is
            // the literal "0" — that would be the materialised
            // form. It can be empty / contain a Barrier.
            // Conservative check: just verify the helper refused.
            // (The body might still lower the original CMD-token
            // text as a runtime call.)
            assert!(
                p.body.statements.is_empty()
                    || matches!(
                        p.body.statements[0],
                        Statement::Call { .. } | Statement::Barrier { .. }
                    )
            );
        }
    }

    #[test]
    fn proc_subst_nocommands_nobackslashes_refused() {
        // ``-nobackslashes`` flag — semantics differ from our
        // evaluator's default. Refuse and fall through.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nobackslashes -nocommands {return $default}] }",
            &reg(),
        );
        let inner = m.procedures.get("::Verbose");
        if let Some(p) = inner {
            // Body did not materialise — should be empty or have
            // a fallback shape.
            assert!(
                p.body.statements.is_empty()
                    || matches!(
                        p.body.statements[0],
                        Statement::Call { .. } | Statement::Barrier { .. }
                    )
            );
        }
    }

    #[test]
    fn proc_dynamic_name_picks_latest_set() {
        // Multiple ``set`` calls — the most recent literal wins.
        let m = lower_to_ir(
            "proc factory {} { set name {First}\n set name {Second}\n proc $name {} { puts hi } }",
            &reg(),
        );
        assert!(m.procedures.contains_key("::Second"));
        assert!(!m.procedures.contains_key("::First"));
    }

    #[test]
    fn ns_export_recorded() {
        let m = lower_to_ir("namespace eval ::tcltest { namespace export test }", &reg());
        // The export was inside a ``namespace eval`` body so the
        // context namespace is ``::tcltest``.
        assert!(
            m.namespace_exports
                .iter()
                .any(|(ns, pat)| ns == "::tcltest" && pat == "test")
        );
    }

    #[test]
    fn ns_import_in_dead_branch_suppressed() {
        // ``if {0} { namespace import ::evil::* }`` — the
        // import is inside a syntactically-dead branch so it must
        // NOT be recorded.
        let m = lower_to_ir("if {0} { namespace import ::evil::* }", &reg());
        assert!(
            m.namespace_imports.is_empty(),
            "imports inside dead if{{0}} branch must not be collected, got {:?}",
            m.namespace_imports,
        );
    }

    #[test]
    fn ns_import_in_static_true_else_suppressed() {
        // ``if {1} { ... } else { namespace import ::evil::* }``
        // — the else branch is dead.
        let m = lower_to_ir(
            "if {1} { namespace import ::good::* } else { namespace import ::evil::* }",
            &reg(),
        );
        // Only ``::good::*`` recorded.
        assert_eq!(m.namespace_imports.len(), 1);
        assert_eq!(m.namespace_imports[0].1, "::good::*");
    }

    #[test]
    fn const_prop_inherited_into_catch_body() {
        // Child scope (catch body) inherits parent's const-map.
        let m = lower_to_ir(
            "proc f {} { set body {set x 1}\n catch { uplevel 1 $body } }",
            &reg(),
        );
        let proc = m.procedures.get("::f").expect("proc registered");
        // Look for a Catch wrapping an UpFrame.
        let catch_stmt = proc
            .body
            .statements
            .iter()
            .find(|s| matches!(s, Statement::Catch { .. }))
            .expect("expected Catch");
        if let Statement::Catch { body, .. } = catch_stmt {
            assert!(
                body.statements
                    .iter()
                    .any(|s| matches!(s, Statement::UpFrame { .. })),
                "expected UpFrame inside catch body, got {body:?}",
            );
        }
    }

    // trace add execution module-fact population

    #[test]
    fn trace_add_execution_literal_recorded() {
        let m = lower_to_ir("trace add execution foo enter handler", &reg());
        assert!(m.traced_commands.contains("foo"));
        assert!(!m.has_dynamic_trace);
    }

    #[test]
    fn trace_add_execution_dynamic_widens() {
        let m = lower_to_ir("trace add execution $cmd enter handler", &reg());
        assert!(m.has_dynamic_trace);
        assert!(m.traced_commands.is_empty());
    }

    #[test]
    fn trace_add_execution_qualified_canonicalised() {
        let m = lower_to_ir("trace add execution ::ns::foo enter h", &reg());
        // Stripped of leading ``::`` so the GVN gate's
        // `command.trim_start_matches("::")` lookup hits.
        assert!(m.traced_commands.contains("ns::foo"));
    }

    #[test]
    fn trace_add_variable_does_not_record_execution_trace() {
        // `trace add variable` is a separate channel — should not
        // populate `traced_commands` (those are command traces only).
        let m = lower_to_ir("trace add variable x write h", &reg());
        assert!(m.traced_commands.is_empty());
        assert!(!m.has_dynamic_trace);
    }

    /// Regression coverage for issue #996: `walk_for_trace`'s post-lower
    /// scan recurses once per nested `if`/`for`/`while`/`foreach`/`catch`/
    /// `try`/`switch`/`Block`/`UpFrame` body, with no depth cap of its own
    /// before this fix. Transitively bounded to `MAX_LOWER_NEST_DEPTH`
    /// (256) by `lower_script`/`lower_body` (this same `lower_to_ir` call
    /// builds the `Script` `walk_for_trace` then scans), so this is
    /// defence-in-depth / consistency with every other full-tree walker in
    /// this crate, not a currently-reproducible crash. 1000 levels of
    /// source nesting is comfortably past the new cap; the assertion is
    /// that lowering (which runs `populate_trace_facts` ->
    /// `walk_for_trace`) returns at all, not what it returns. Spawns its
    /// own big-stack thread since the lexer/CST/segmenter stages upstream
    /// of `lower_script`'s own cap still walk the full un-truncated source
    /// nesting before that cap trims it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_walk_for_trace() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("trace add execution foo enter handler\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = lower_to_ir(&src, &reg());
            })
            .unwrap()
            .join()
            .unwrap();
    }

    // trace add/remove/variable/vdelete module-fact population

    #[test]
    fn trace_add_variable_literal_recorded() {
        let m = lower_to_ir("trace add variable x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
        assert!(!m.has_dynamic_variable_trace);
    }

    #[test]
    fn trace_remove_variable_literal_recorded() {
        // A `remove` only proves a trace *might* have existed — still
        // conservative to record, mirroring `trace add`.
        let m = lower_to_ir("trace remove variable x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_qualified_canonicalised() {
        let m = lower_to_ir("trace add variable ::x write h", &reg());
        // `::`-stripped so a top-level `set x 5` (chain key `x`) matches.
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_dynamic_widens() {
        let m = lower_to_ir("trace add variable $name write h", &reg());
        assert!(m.has_dynamic_variable_trace);
        assert!(m.traced_variables.is_empty());
    }

    #[test]
    fn trace_add_variable_inside_proc_recorded() {
        let m = lower_to_ir(
            "proc setup {} { trace add variable ::x read onread }",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_inside_method_recorded() {
        // TclOO method bodies are extracted into `module.methods` (a
        // separate map from `module.procedures`) before this scan runs —
        // must not be missed just because it isn't a plain proc.
        let m = lower_to_ir(
            "oo::class create C {\n method m {} { trace add variable v write cb }\n}",
            &reg(),
        );
        assert!(m.traced_variables.contains("v"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_legacy_variable_form_recorded() {
        // The deprecated `trace variable name ops command` spelling must
        // populate the same fact as `trace add variable` — no
        // hardcoded-per-form gap.
        let m = lower_to_ir("trace variable x r onread", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_legacy_vdelete_form_recorded() {
        let m = lower_to_ir("trace vdelete x r onread", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_info_variable_does_not_record() {
        // `trace info`/`vinfo` only query trace state — no
        // `ESTABLISHES_VARIABLE_TRACE` trait, so they must not widen the
        // module fact.
        let m = lower_to_ir("trace info variable x", &reg());
        assert!(m.traced_variables.is_empty());
        assert!(!m.has_dynamic_variable_trace);
    }

    #[test]
    fn trace_add_variable_through_interp_alias_recorded() {
        // `interp alias {} tracer {} trace` means `tracer add variable ...`
        // is really a `trace add variable ...` call — the whole-module scan
        // must key off the canonical (alias-resolved) command name, not the
        // source-surface `tracer` spelling, or this trace target is missed.
        let m = lower_to_ir(
            "interp alias {} tracer {} trace\ntracer add variable x write h",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_execution_through_interp_alias_recorded() {
        // Same alias-resolution requirement for the execution-trace channel:
        // `walk_for_trace` gates entry into both `traced_commands` and
        // `populate_variable_trace_facts` on one shared canonical-name check.
        let m = lower_to_ir(
            "interp alias {} tracer {} trace\ntracer add execution foo enter h",
            &reg(),
        );
        assert!(m.traced_commands.contains("foo"), "{:?}", m.traced_commands);
        assert!(!m.has_dynamic_trace);
    }

    #[test]
    fn trace_add_execution_does_not_record_variable_trace() {
        // The command-execution channel is separate — should not
        // populate `traced_variables`.
        let m = lower_to_ir("trace add execution foo enter h", &reg());
        assert!(m.traced_variables.is_empty());
        assert!(!m.has_dynamic_variable_trace);
    }

    #[test]
    fn trace_add_execution_inside_proc_recorded() {
        let m = lower_to_ir("proc init {} { trace add execution foo enter h }", &reg());
        assert!(
            m.traced_commands.contains("foo"),
            "traced_commands={:?}",
            m.traced_commands,
        );
    }

    #[test]
    fn trace_add_variable_abbreviated_type_word_recorded() {
        // `trace add v x write h` / `trace add var x write h` install the
        // same variable trace as the full `variable` spelling — C Tcl
        // accepts any unique prefix (checked against tclsh 8.6.14).
        let m = lower_to_ir("trace add v x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
        let m = lower_to_ir("trace add var x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_remove_variable_abbreviated_type_word_recorded() {
        let m = lower_to_ir("trace remove var x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_execution_abbreviated_type_word_not_variable() {
        // `e`/`exec` abbreviate `execution`, not `variable` — must not
        // widen `traced_variables`.
        let m = lower_to_ir("trace add e foo enter h", &reg());
        assert!(m.traced_variables.is_empty());
        assert!(m.traced_commands.contains("foo"), "{:?}", m.traced_commands);
    }

    #[test]
    fn trace_add_variable_inside_namespace_eval_body_recorded() {
        // A trace installed inside a `namespace eval` body (a synthetic
        // `Module::body_units` entry, not a named proc) must populate the
        // same whole-module fact as one inside a `proc`.
        let m = lower_to_ir(
            "namespace eval ::n { trace add variable ::x write h }",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_inside_apply_body_recorded() {
        let m = lower_to_ir("apply {{} { trace add variable ::x write h }}", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_inside_method_body_recorded() {
        let m = lower_to_ir(
            "oo::class create C { method m {} { trace add variable ::x write h } }",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    // barrier-gate

    #[test]
    fn body_has_dynamic_barrier_clean() {
        // No barriers at all.
        assert!(!body_has_dynamic_barrier("set x 1", &reg()));
        // Barrier with a fully literal body.
        assert!(!body_has_dynamic_barrier("eval { set x 1 }", &reg()));
        // Nested literal.
        assert!(!body_has_dynamic_barrier(
            "if { 1 } { eval { set x 1 } }",
            &reg()
        ));
    }

    #[test]
    fn body_has_dynamic_barrier_dynamic_eval_body() {
        // ``eval $x`` inside the outer body — dynamic.
        assert!(body_has_dynamic_barrier("eval $x", &reg()));
        // Same shape nested.
        assert!(body_has_dynamic_barrier("if { 1 } { eval $x }", &reg()));
    }

    #[test]
    fn body_has_dynamic_barrier_dynamic_uplevel_body() {
        assert!(body_has_dynamic_barrier("uplevel 1 $body", &reg()));
        assert!(body_has_dynamic_barrier("uplevel #0 $b", &reg()));
    }

    #[test]
    fn body_has_dynamic_barrier_uplevel_with_literal_body_clean() {
        // ``uplevel $lvl {body}`` with a literal body is OK — the
        // gate only poisons when the BODY is substitution-bearing.
        assert!(!body_has_dynamic_barrier("uplevel $lvl {set x 1}", &reg()));
    }

    #[test]
    fn body_has_dynamic_barrier_qualified_eval_uplevel() {
        // ``::eval`` and ``::uplevel`` are also caught.
        assert!(body_has_dynamic_barrier("::eval $x", &reg()));
        assert!(body_has_dynamic_barrier("::uplevel 1 $body", &reg()));
    }

    #[test]
    fn try_lower_eval_static_rejects_nested_dynamic_barrier() {
        // The relaxer would normally promote ``eval { ... }`` to a
        // ``Statement::Block``; with the barrier gate, the nested
        // ``eval $x`` poisons the relaxation and we fall back to
        // ``Statement::Barrier``.
        let m = lower_to_ir(r"eval { eval $x }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Barrier { .. }),
            "expected Barrier (gate triggered), got {stmt:?}",
        );
    }

    #[test]
    fn try_lower_eval_static_clean_body_relaxes() {
        // No nested barrier — relaxes to Block.
        let m = lower_to_ir("eval { set x 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Block { .. }),
            "expected Block (relaxed), got {stmt:?}",
        );
    }

    // The next three tests confirm `eval` and `uplevel` dispatch
    // through the registry-driven hook ID to the expected output.

    #[test]
    fn registry_dispatch_eval_static_body_lowers_to_block() {
        // `eval {body}` with a literal body should still relax
        // to `Statement::Block` when dispatched through the
        // registry-driven path.
        let m = lower_to_ir("eval { set y 2 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Block { .. }),
            "expected Block via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_uplevel_static_body_lowers_to_upframe() {
        // `uplevel 1 {body}` with a literal body should lower
        // to a UpFrame via the registry-driven path.
        let m = lower_to_ir("uplevel 1 { set z 3 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::UpFrame { .. }),
            "expected UpFrame via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_unknown_command_falls_through_to_default() {
        // `lower_command` has no per-
        // name fallback — commands that aren't in the registry
        // (or whose `lowering_hook` is `None`) route directly
        // to [`Lowerer::lower_default`], which emits a generic
        // `Statement::Call`.  Pin that contract with a clearly
        // unknown command name.
        let m = lower_to_ir("totallyMadeUpCommand arg1 arg2", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "totallyMadeUpCommand",
                "expected generic Call via lower_default, got command={command}",
            ),
            other => panic!("expected Call via lower_default, got {other:?}"),
        }
    }

    // These tests pin the registry-driven hook-ID dispatch of `if`,
    // `switch`, `for`, `while`, `catch`, `try` to the expected shape.

    #[test]
    fn registry_dispatch_if_lowers_to_if() {
        let m = lower_to_ir("if {1} { set q 4 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::If { .. }),
            "expected If via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_switch_lowers_to_switch() {
        let m = lower_to_ir("switch a { a { set q 1 } b { set q 2 } }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Switch { .. }),
            "expected Switch via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_for_lowers_to_for() {
        let m = lower_to_ir("for {set i 0} {$i < 3} {incr i} { set q $i }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::For { .. }),
            "expected For via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_while_lowers_to_while() {
        let m = lower_to_ir("while {0} { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::While { .. }),
            "expected While via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_catch_lowers_to_catch() {
        let m = lower_to_ir("catch { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Catch { .. }),
            "expected Catch via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_try_lowers_to_try() {
        let m = lower_to_ir("try { set q 1 } on error e { set q 2 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Try { .. }),
            "expected Try via registry hook, got {stmt:?}",
        );
    }

    // `proc`, `namespace eval`, `foreach`, `lmap`, `dict` dispatch
    // through registry-driven hook-ID, with shape preconditions
    // enforced inside each match arm.

    #[test]
    fn registry_dispatch_proc_lowers_to_proc_call_and_registers_procedure() {
        let m = lower_to_ir("proc greet {name} { puts hi }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "proc",
                "expected `proc` call via registry hook, got command={command}",
            ),
            other => panic!("expected Call via registry hook, got {other:?}"),
        }
        assert!(
            m.procedures.contains_key("::greet"),
            "expected ::greet to be registered in module.procedures, got keys={:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn registry_dispatch_proc_wrong_arity_falls_through_to_default() {
        // `proc` with two args — fails the `args.len() == 3`
        // precondition.  The registry-driven path returns None,
        // falls through to the residual string match's
        // `_ => lower_default(...)` arm, which produces a
        // runtime `Call` rather than registering a procedure.
        let m = lower_to_ir("proc greet onlytwo", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Call { .. }),
            "expected Call via lower_default fallback, got {stmt:?}",
        );
        assert!(
            !m.procedures.contains_key("::greet"),
            "expected ::greet NOT to be registered when arity is wrong",
        );
    }

    #[test]
    fn registry_dispatch_namespace_eval_lowers_to_barrier() {
        // `lower_namespace_eval` emits a `Statement::Barrier`
        // tagged with `reason: "namespace eval"`.
        let m = lower_to_ir("namespace eval ::myns { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Barrier { reason, .. } => assert_eq!(
                reason, "namespace eval",
                "expected `namespace eval` barrier via registry hook, got reason={reason}",
            ),
            other => panic!("expected Barrier via registry hook, got {other:?}"),
        }
    }

    #[test]
    fn registry_dispatch_namespace_non_eval_subcommand_falls_through() {
        // `namespace import` has no `lowering_hook` on its
        // subcommand spec, so `resolve_call` returns hook=None.
        // The registry-driven dispatcher returns None and the
        // residual string match's `_ => lower_default(...)`
        // arm produces a generic `Call`.
        let m = lower_to_ir("namespace import ::foo::*", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Call { .. }),
            "expected Call via lower_default fallback, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_foreach_lowers_to_foreach() {
        let m = lower_to_ir("foreach v {1 2 3} { set q $v }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Foreach { .. }),
            "expected Foreach via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_lmap_lowers_to_foreach() {
        // `lmap` shares `lower_foreach(... is_lmap=true)` with
        // `foreach`; both produce `Statement::Foreach`.
        let m = lower_to_ir("lmap v {1 2 3} { expr {$v + 1} }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Foreach { .. }),
            "expected Foreach (is_lmap=true) via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_dict_lowers_via_lower_dict() {
        // `dict set d k v` exercises the `lower_dict`
        // subcommand-dispatch path.  Pin the registry-driven
        // route by checking the canonical command name on the
        // emitted Call.
        let m = lower_to_ir("dict set d k v", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "dict",
                "expected `dict` call via registry hook, got command={command}",
            ),
            other => panic!("expected Call via registry hook, got {other:?}"),
        }
    }

    #[test]
    fn registry_dispatch_dict_empty_args_falls_through_to_default() {
        // Bare `dict` — fails the `!args.is_empty()`
        // precondition, falls through to `lower_default`.
        let m = lower_to_ir("dict", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Call { .. }),
            "expected Call via lower_default fallback, got {stmt:?}",
        );
    }

    // `when` and `foreachLine` dispatch through registry-driven
    // hook-ID.  `when` is dialect-gated (iRules), so its test
    // registry loads the dialect explicitly.  `foreachLine` is
    // a Tcl 9.0+ command always registered in `build_default()`.

    #[test]
    fn registry_dispatch_when_lowers_via_lower_when_with_irules_loaded() {
        // The iRules `when` spec carries `LoweringHookId::When`
        // and only enters the registry after
        // `load_dialect(IRULES)`.  Production callers (LSP
        // server, Python bindings) always pair `build_default()`
        // with the active dialect; this test mirrors that.
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let m = lower_to_ir("when HTTP_REQUEST { set q 1 }", &registry);
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "when",
                "expected `when` call via registry hook, got command={command}",
            ),
            other => panic!("expected Call via registry hook, got {other:?}"),
        }
        assert!(
            m.procedures.contains_key("::when::HTTP_REQUEST"),
            "expected ::when::HTTP_REQUEST procedure to be registered, got keys={:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn registry_dispatch_when_without_irules_loaded_falls_through() {
        // Without `load_irules()`, the registry has no `when`
        // spec, so `resolve_call` returns `None` and
        // `try_dispatch_structured_hook` falls through to
        // `lower_default`.  This is a misconfiguration on the
        // caller's side (the source uses iRules but the
        // registry doesn't know about it); the lowerer emits a
        // generic `Call` rather than silently treating it as
        // an event handler.  Pinning this behaviour catches
        // future drift if `build_default()` ever folds in
        // iRules.
        let m = lower_to_ir("when HTTP_REQUEST { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "when",
                "expected generic Call via lower_default, got command={command}",
            ),
            other => panic!("expected Call via lower_default, got {other:?}"),
        }
        assert!(
            !m.procedures.keys().any(|k| k.starts_with("::when::")),
            "expected NO ::when::* procedure registered without iRules loaded",
        );
    }

    #[test]
    fn registry_dispatch_foreach_line_lowers_via_lower_foreach_line() {
        // `foreachLine` is a Tcl 9.0+ command (TIP 670) and
        // `build_default()` registers it via the
        // `tcl::foreachline` spec.  No dialect-load dance is
        // needed.  The body is a brace-string literal here, so
        // the dedicated lowerer relaxes to a typed `Foreach`
        // (not a barrier).
        let m = lower_to_ir("foreachLine line readme.txt { set q $line }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Foreach { .. }),
            "expected Foreach via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_foreach_line_wrong_arity_falls_through_to_default() {
        // `foreachLine` with two args — fails the
        // `args.len() == 3` precondition, falls through to
        // `lower_default`.
        let m = lower_to_ir("foreachLine onlytwo args", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "foreachLine",
                "expected generic Call via lower_default, got command={command}",
            ),
            other => panic!("expected Call via lower_default, got {other:?}"),
        }
    }

    /// `if {1} { if {1} { ... } }`, `depth` levels deep.
    fn nested_if_source(depth: u32) -> String {
        let mut source = String::new();
        for _ in 0..depth {
            source.push_str("if {1} {\n");
        }
        for _ in 0..depth {
            source.push_str("}\n");
        }
        source
    }

    /// Depth-first search for a [`Statement::Barrier`] whose `reason`
    /// contains `needle`, anywhere in `script`'s tree (into `If` clauses and
    /// `else` bodies — the only nesting `nested_if_source` produces).
    fn contains_barrier_reason(script: &Script, needle: &str) -> bool {
        script.statements.iter().any(|stmt| match stmt {
            Statement::Barrier { reason, .. } => reason.contains(needle),
            Statement::If {
                clauses, else_body, ..
            } => {
                clauses
                    .iter()
                    .any(|c| contains_barrier_reason(&c.body, needle))
                    || else_body
                        .as_ref()
                        .is_some_and(|e| contains_barrier_reason(e, needle))
            }
            _ => false,
        })
    }

    /// Issue #996: `lower_script` / `lower_body` recurse one Rust frame
    /// group per `if` nesting level with no depth cap prior to this fix —
    /// unlike `analyse_body` (`tcl_compiler::analyser::commands::
    /// MAX_BODY_DEPTH`) and `cfg_builder::lower_script`
    /// (`MAX_LOWER_DEPTH`), both already guarded. A document whose IR
    /// lowering runs unguarded defeats the CFG builder's own guard
    /// downstream — the crash happens one stage earlier. Lowering 2000
    /// levels must neither hang nor overflow the stack, and must record the
    /// cap trip as a `Statement::Barrier` (unknown effects) rather than
    /// silently truncating to an empty, falsely-dead-looking script.
    #[test]
    fn deeply_nested_if_past_max_lower_depth_barriers_not_crashes() {
        let source = nested_if_source(2000);
        // `cargo test` runs each test on a small default-stack thread (the
        // same undersized budget issue #996 was actually caused by) — see
        // the identical helper's doc comment in `analyser::commands::tests`.
        let m = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || lower_to_ir(&source, &reg()))
            .expect("spawn big-stack test thread")
            .join()
            .expect("lower_to_ir on big-stack thread panicked");
        assert!(
            contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
            "expected a depth-cap Barrier somewhere in the lowered tree"
        );
    }

    /// Depth of the deepest chain of nested `If` statements in `script`.
    fn if_chain_depth(script: &Script) -> u32 {
        script
            .statements
            .iter()
            .map(|stmt| match stmt {
                Statement::If { clauses, .. } => {
                    1 + clauses.first().map_or(0, |c| if_chain_depth(&c.body))
                }
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn shallow_nested_if_lowers_with_no_barrier() {
        // False-positive guard: ordinary, hand-written nesting must lower
        // as real `If` statements all the way down, never barriered.
        let source = nested_if_source(10);
        let m = lower_to_ir(&source, &reg());
        assert!(
            !contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
            "shallow nesting must not trip the depth cap"
        );
        // 10 real nested `If`s, not a single opaque barrier standing in for
        // all of them.
        assert_eq!(if_chain_depth(&m.top_level), 10);
    }
}
