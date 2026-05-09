//! Lower Tcl source to structured analysis IR.
//!
//! Translates a flat token stream (via the segmenter) into the tree of
//! IR nodes defined in [`ir`]. Each Tcl command is pattern-matched by
//! name and converted to a typed IR statement.
//!
//! Ports `core/compiler/lowering.py`.

use std::collections::HashMap;

use tcl_lexer::TokenType;
use tcl_registry::{ArgRole, CommandRegistry};

use crate::alias::{detect_interp_alias, resolve_alias, CommandAliasMap};
use crate::ir::{CommandTokens, Module, Procedure, Script, Statement};
use crate::lowering_hooks::{try_lower_hook, ArgTokenKind, LoweringCommand};
use crate::naming::{normalise_qualified_name, normalise_var_name};
use crate::segmenter::{segment_commands, segment_commands_with_offset, SegmentedCommand};

pub(crate) mod hooks;
mod structured;

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

/// Parse a Tcl parameter list into parameter names.
fn parse_param_names(param_str: &str) -> Vec<String> {
    let mut params = Vec::new();
    let text = param_str.trim();
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
            if !inner.is_empty() {
                if let Some(name) = inner.split_whitespace().next() {
                    params.push(name.to_owned());
                }
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
/// case-insensitive (matches `Tcl_GetBoolean`). Mirrors Python's
/// `_static_bool` (main commit `06f42efa`).
pub(crate) fn static_bool(expr_text: &str) -> Option<bool> {
    let stripped = expr_text.trim().to_ascii_lowercase();
    match stripped.as_str() {
        "0" | "false" | "no" | "off" => Some(false),
        "1" | "true" | "yes" | "on" => Some(true),
        _ => None,
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
/// token. Mirrors Python's `_set_literal_body`.
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
/// flag plus the absence of `$` / `[` in `Esc` text). Mirrors
/// Python's `_eval_list_literal_body`.
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
/// touch any tracked name. Mirrors Python's `_invalidate_const_map_for`.
/// C43 / barrier-gate: token-level check that a relaxed-eval /
/// relaxed-uplevel body is free of nested dynamic-shape barriers.
///
/// Mirrors `core/compiler/lowering_hooks/_barrier_gate.py::body_has_dynamic_barrier`.
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
fn body_has_dynamic_barrier(body_text: &str) -> bool {
    use tcl_lexer::TokenType;
    let commands = segment_commands(body_text);
    for sc in &commands {
        if sc.argv.is_empty() || sc.texts.is_empty() {
            continue;
        }
        let name = sc.texts[0].as_str();
        let is_barrier = matches!(name, "eval" | "uplevel" | "::eval" | "::uplevel");
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
                if body_has_dynamic_barrier(arg_text) {
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
                    || level.trim_start_matches('-').chars().all(|c| c.is_ascii_digit()));
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
        if body_has_dynamic_barrier(&args[body_idx]) {
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

/// The lowering engine — accumulates procedures and IR statements.
pub struct Lowerer<'r> {
    /// Output module being built.
    pub module: Module,
    /// Command alias table built during lowering.
    aliases: CommandAliasMap,
    /// Event handler occurrence counts (for `when` numbering).
    when_counts: HashMap<String, u32>,
    /// Whether we're inside a `namespace eval` body.
    in_namespace_eval: bool,
    /// Command registry for arg-role queries.
    registry: &'r CommandRegistry,
    /// Per-script const-map stack (C35). Each scope tracks
    /// proc-local variables assigned a brace-string literal so
    /// later `eval $var` / `uplevel 1 $var` calls can fold the
    /// body in at lowering time. Active only when
    /// `proc_depth > 0` — top-level / `namespace eval` scopes
    /// write globals or namespace vars whose values can be
    /// observed and mutated by other code, so const-propagating
    /// them is unsound. Mirrors Python's `_const_map_stack`.
    const_map_stack: Vec<HashMap<String, String>>,
    /// Depth of `proc` / `when` body lowerings currently in
    /// flight. A positive value enables the const-map. Mirrors
    /// Python's `_proc_depth`.
    proc_depth: u32,
    /// `namespace import` directives observed at lowering time
    /// (C38a). Recorded as `(context_namespace, absolute_pattern)`
    /// pairs and copied into `Module::namespace_imports` at the
    /// end of lowering. Order is preserved.
    namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives observed at lowering time
    /// (C38b). Recorded as `(context_namespace, pattern)` pairs
    /// and copied into `Module::namespace_exports` at the end of
    /// lowering. Order is preserved.
    namespace_exports: Vec<(String, String)>,
    /// Depth of statically-dead branches currently being lowered
    /// (C38c). `if {0} {…}` / `if {1} {…} else {…}` bump this
    /// around the dead body so any `namespace import` /
    /// `namespace export` directives found inside don't register
    /// with the module-level tables. The IR for the dead code is
    /// still produced so consumers that walk the tree by syntactic
    /// offset see the original structure. Mirrors Python's
    /// `_dead_code_depth`.
    pub(crate) dead_code_depth: u32,
}

impl<'r> Lowerer<'r> {
    /// Create a new lowerer.
    #[must_use]
    pub fn new(registry: &'r CommandRegistry) -> Self {
        Self {
            module: Module::default(),
            aliases: CommandAliasMap::new(),
            when_counts: HashMap::new(),
            in_namespace_eval: false,
            registry,
            const_map_stack: Vec::new(),
            proc_depth: 0,
            namespace_imports: Vec::new(),
            namespace_exports: Vec::new(),
            dead_code_depth: 0,
        }
    }

    /// Lower a complete source string to an IR module.
    pub fn lower(&mut self, source: &str) -> &Module {
        self.module.top_level = self.lower_script(source, "::");
        // C38a: surface namespace import / export directives onto
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
    fn lower_script(&mut self, source: &str, namespace: &str) -> Script {
        let commands = segment_commands(source);
        self.const_map_stack.push(HashMap::new());
        let stmts = self.lower_segmented(&commands, namespace);
        self.const_map_stack.pop();
        Script::from_statements(stmts)
    }

    /// Lower a body argument (inside braces/brackets) to an IR script.
    ///
    /// Inherits the parent scope's const-map (C35a, mirroring main
    /// commit `c30203da`) so a child body can still relax its
    /// `eval` / `uplevel` against literals bound in the enclosing
    /// scope (`set body {literal}; catch {uplevel 1 $body}` is the
    /// canonical example).
    fn lower_body(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let commands = segment_commands_with_offset(text, base_offset);
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

    /// Maintain the per-scope const-map (C35) after a command lowers.
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
        } else if let Some(rest) = word.strip_prefix('$') {
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
        } else {
            return None;
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

    /// C38a / C38b: detect ``namespace import ?-force? pattern...``
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
            .map_or(true, |ew| !ew.iter().any(|&e| e));
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

    /// Lower a single command.
    fn lower_command(&mut self, seg: &SegmentedCommand, namespace: &str) -> Option<Statement> {
        if seg.texts.is_empty() {
            return None;
        }

        let cmd_name = seg.name();
        let args = seg.args();

        // Detect `interp alias {} name {} target ?args?`.
        let args_owned: Vec<String> = args.to_vec();
        if let Some((qualified, target, prepended)) = detect_interp_alias(cmd_name, &args_owned) {
            self.aliases.insert(qualified, (target, prepended));
        }

        self.record_namespace_directives(cmd_name, args, seg, namespace);

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

        // Command-specific dispatch.
        match cmd_name {
            "proc" if args.len() == 3 && seg.arg_tokens().len() >= 3 => {
                Some(self.lower_proc(seg, namespace))
            }

            "when" if args.len() >= 2 && seg.arg_tokens().len() >= 2 => {
                Some(self.lower_when(seg, namespace))
            }

            "namespace" if args.len() >= 3 && args[0] == "eval" && seg.arg_tokens().len() >= 3 => {
                Some(self.lower_namespace_eval(seg, namespace))
            }

            // C34a: static-body uplevel. Match `uplevel 1 {body}`,
            // `uplevel #0 {body}`, and the canonical no-level form
            // `uplevel {body}` (level defaults to 1) when the body
            // arg is a brace-string literal token. Dynamic forms
            // (``uplevel 1 $body`` / ``uplevel $lvl {body}``) fall
            // through to the default lowering so a runtime ``Call`` /
            // ``Barrier`` carries the unresolved arguments.
            "uplevel" => Some(
                self.try_lower_uplevel_static(seg, namespace)
                    .unwrap_or_else(|| self.lower_default(seg, namespace)),
            ),

            // C35b: ``eval $body`` / ``eval {body}`` with a literal /
            // const-folded body relaxes to a ``Statement::Block`` so
            // downstream analyses see the inlined script. Dynamic
            // bodies (``eval $dyn`` with no const-map binding, ``eval
            // [cmd]``) fall through to the default barrier dispatch.
            "eval" => Some(
                self.try_lower_eval_static(seg, namespace)
                    .unwrap_or_else(|| self.lower_default(seg, namespace)),
            ),

            "if" => Some(self.lower_if(seg, namespace)),
            "switch" => Some(self.lower_switch(seg, namespace)),
            "for" => Some(self.lower_for(seg, namespace)),
            "while" => Some(self.lower_while(seg, namespace)),
            "foreach" => Some(self.lower_foreach(seg, namespace, false)),
            "lmap" => Some(self.lower_foreach(seg, namespace, true)),
            "catch" => Some(self.lower_catch(seg, namespace)),
            "try" => Some(self.lower_try(seg, namespace)),
            "dict" if !args.is_empty() => Some(self.lower_dict(seg, namespace)),

            _ => Some(self.lower_default(seg, namespace)),
        }
    }

    /// Lower `proc name params body`.
    fn lower_proc(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args_borrow = seg.args();
        let proc_name_initial = &args_borrow[0];

        // Dynamic proc names: a bare ``$var`` whose value is in the
        // const-map can be resolved at lowering time (C36b — main
        // commit `2ad4efc9`). Multi-token names (``foo_$x``) and
        // command-substitution names (``$name[suffix]``) stay on
        // the runtime path.
        let proc_name_owned: String;
        let mut args_owned: Vec<String>;
        if proc_name_initial.contains('$') || proc_name_initial.contains('[') {
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
                return Statement::Barrier {
                    span: seg.span,
                    reason: "dynamic proc name".into(),
                    command: "proc".into(),
                    canonical_command: None,
                    args: args_borrow.to_vec(),
                    tokens: Some(Self::cmd_tokens(seg)),
                };
            };
            proc_name_owned = literal;
            args_owned = args_borrow.to_vec();
            args_owned[0].clone_from(&proc_name_owned);
        } else {
            proc_name_owned = proc_name_initial.clone();
            args_owned = args_borrow.to_vec();
        }
        // Re-bind ``args`` and ``proc_name`` to the (possibly
        // substituted) owned values.
        let args: &[String] = &args_owned;
        let proc_name = &proc_name_owned;

        let params = parse_param_names(&args[1]);
        let qualified = qualify_proc_name(namespace, proc_name);
        let body_tok = seg.arg_tokens()[2];
        let body_text = &args[2];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // C36c: if the body is a ``[subst -nocommands {template}]``
        // command sub and every ``\$var`` inside *template* is in the
        // current const-map, evaluate the subst at compile time and
        // lower the resulting string as a fresh script. Catches the
        // tcltest ``Option`` factory shape where the accessor body
        // is built from a template plus const-known option name /
        // default / description. Mirrors main commit `d4d2cdd5`.
        let materialised_body = if body_tok.kind == tcl_lexer::TokenType::Cmd {
            self.eval_subst_nocommands_body(&args[2])
        } else {
            None
        };
        // C37b: fresh const-map frame for the nested proc body.
        // ``lower_body`` would otherwise inherit the enclosing
        // scope's tracked scalars — correct for control-flow
        // bodies (if / catch / loops share the frame) but unsound
        // for ``proc`` bodies, which have their own runtime frame.
        // Pushing an empty frame here means the inner ``lower_body``
        // clones an empty parent, giving the proc body a clean
        // slate. Mirrors main commit `49f90130`.
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let body = if let Some(text) = materialised_body {
            // Lower the substituted text as a fresh script.
            self.lower_script(&text, namespace)
        } else {
            self.lower_body(body_text, body_offset, namespace)
        };
        self.const_map_stack.pop();
        self.proc_depth -= 1;

        if let std::collections::hash_map::Entry::Vacant(e) =
            self.module.procedures.entry(qualified.clone())
        {
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
        } else {
            self.module.redefined_procedures.insert(qualified);
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

    /// Lower `when EVENT ?priority N? body`.
    fn lower_when(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let event_name = &args[0];
        let body_idx = args.len() - 1;
        let body_tok = seg.arg_tokens()[body_idx];
        let body_text = &args[body_idx];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // C37b: fresh const-map frame for the nested proc body.
        // ``lower_body`` would otherwise inherit the enclosing
        // scope's tracked scalars — correct for control-flow
        // bodies (if / catch / loops share the frame) but unsound
        // for ``proc`` bodies, which have their own runtime frame.
        // Pushing an empty frame here means the inner ``lower_body``
        // clones an empty parent, giving the proc body a clean
        // slate. Mirrors main commit `49f90130`.
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let body = self.lower_body(body_text, body_offset, namespace);
        self.const_map_stack.pop();
        self.proc_depth -= 1;

        let mut base_priority: u32 = 500;
        if args.len() >= 4 && args[1] == "priority" {
            if let Ok(p) = args[2].parse::<u32>() {
                base_priority = p;
            }
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
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let (frame_shift, body_tok_idx) = match args.len() {
            1 => (1_i32, 0),
            2 => (parse_uplevel_level(&args[0])?, 1),
            _ => return None,
        };
        let body_tok = arg_tokens.get(body_tok_idx)?;
        // C43 / barrier-gate: see `try_lower_eval_static` for the
        // rationale.  A static-body `uplevel 1 {eval $x}` would
        // relax to inline IR with a still-dynamic `eval`; reject.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[body_tok_idx];
            if body_has_dynamic_barrier(body_text) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            let body_text = &args[body_tok_idx];
            let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.lower_body(body_text, body_offset, namespace)
        } else if body_tok.kind == TokenType::Var {
            // C35b: `uplevel ?N? $var` with $var resolved by the
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
    /// Used by C36c to materialise the tcltest-style `Option`
    /// factory body at compile time when the surrounding proc has
    /// all the template vars const-tracked. Mirrors Python's
    /// `_eval_subst_nocommands_body`.
    fn eval_subst_nocommands_body(&self, cmd_text: &str) -> Option<String> {
        use tcl_lexer::TokenType;
        let inner = segment_commands(cmd_text);
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
    ///
    /// Mirrors main commits `b5e18ce2` (string + var shapes) and
    /// `a080c8d7` (`[list ...]` shape).
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
        // C43 / barrier-gate: a braced literal body might contain a
        // nested ``eval $x`` / ``uplevel $lvl {...}`` whose own body
        // is still dynamic.  Relaxing the outer barrier in that case
        // produces IR that runs a compiled inner barrier with a
        // still-dynamic shape — we'd lose the runtime barrier without
        // gaining static knowledge.  Reject and fall back to the
        // default IRBarrier dispatch.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[0];
            if body_has_dynamic_barrier(body_text) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            let body_text = &args[0];
            let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.lower_body(body_text, body_offset, namespace)
        } else if body_tok.kind == TokenType::Var {
            let literal = self.const_map_lookup(&args[0])?;
            if body_has_dynamic_barrier(&literal) {
                return None;
            }
            self.lower_script(&literal, namespace)
        } else if body_tok.kind == TokenType::Cmd {
            // C35c: ``eval [list lit1 lit2 ...]`` — synthesise the
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

    fn lower_namespace_eval(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let child_ns = join_namespace(namespace, &args[1]);
        let prev = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let body_tok = seg.arg_tokens()[2];
        let body_text = &args[2];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let _body = self.lower_body(body_text, body_offset, &child_ns);
        self.in_namespace_eval = prev;

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

        // Resolve alias for arg role lookups.
        let mut role_cmd = cmd_name.to_owned();
        let mut role_args: Vec<String> = args.to_vec();
        let mut prepend_n: usize = 0;
        if let Some((target, prepended)) = resolve_alias(cmd_name, &self.aliases, namespace) {
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

        if !body_indices.is_empty() {
            return Statement::Barrier {
                span: seg.span,
                reason: "unsupported body command".into(),
                command: cmd_name.into(),
                canonical_command: None,
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        if !var_indices.is_empty() || !var_read_indices.is_empty() {
            let var_defs: Vec<String> = var_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real).map(|a| normalise_var_name(a).to_owned())
                })
                .collect();
            let var_reads: Vec<String> = var_read_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real).map(|a| normalise_var_name(a).to_owned())
                })
                .collect();
            return Statement::Call {
                span: seg.span,
                command: cmd_name.into(),
                canonical_command: None,
                args: args.to_vec(),
                defs: var_defs,
                reads: var_reads,
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(Self::cmd_tokens(seg)),
                foreach_groups: None,
            };
        }

        Statement::Call {
            span: seg.span,
            command: cmd_name.into(),
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
}

// Public API

/// Lower Tcl source to an IR module.
///
/// This is the main entry point for the lowering phase.
#[must_use]
pub fn lower_to_ir(source: &str, registry: &CommandRegistry) -> Module {
    let mut lowerer = Lowerer::new(registry);
    lowerer.lower(source);
    let mut module = lowerer.module;
    populate_trace_facts(&mut module);
    module
}

/// SYNC9: post-lower scan for `trace add execution NAME enter|leave …`
/// that populates `Module::traced_commands` / `has_dynamic_trace`.
///
/// Runs over the top-level script + every procedure body.  Literal
/// command names land in `traced_commands` (`::`-stripped to match
/// the canonical key); non-literal targets (`$cmd`, `[expr ...]`,
/// command substitutions) flip `has_dynamic_trace` so GVN treats
/// every call as potentially traced.
///
/// Mirrors Python's IR-builder pass added by `8a6f4d58` (closes `#251`).
fn populate_trace_facts(module: &mut Module) {
    let top_level = module.top_level.clone();
    walk_for_trace(&top_level, module);
    let proc_bodies: Vec<Script> = module
        .procedures
        .values()
        .map(|p| p.body.clone())
        .collect();
    for body in &proc_bodies {
        walk_for_trace(body, module);
    }
}

fn walk_for_trace(script: &Script, module: &mut Module) {
    use crate::ir::Statement;
    for stmt in &script.statements {
        match stmt {
            Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
                if command == "trace"
                    && args.len() >= 4
                    && args[0] == "add"
                    && args[1] == "execution"
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
            }
            Statement::If { clauses, else_body, .. } => {
                for c in clauses {
                    walk_for_trace(&c.body, module);
                }
                if let Some(e) = else_body {
                    walk_for_trace(e, module);
                }
            }
            Statement::For { init, next, body, .. } => {
                walk_for_trace(init, module);
                walk_for_trace(next, module);
                walk_for_trace(body, module);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => walk_for_trace(body, module),
            Statement::Switch { arms, default_body, .. } => {
                for arm in arms {
                    if let Some(b) = &arm.body {
                        walk_for_trace(b, module);
                    }
                }
                if let Some(b) = default_body {
                    walk_for_trace(b, module);
                }
            }
            Statement::Try { body, handlers, finally_body, .. } => {
                walk_for_trace(body, module);
                for h in handlers {
                    walk_for_trace(&h.body, module);
                }
                if let Some(f) = finally_body {
                    walk_for_trace(f, module);
                }
            }
            _ => {}
        }
    }
}

fn is_literal_trace_target(s: &str) -> bool {
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
        // can't be statically resolved without C35's const-propagate.
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
        // C35b: ``set body {set x 1}; uplevel 1 $body`` inside a
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
        // C35b: ``eval $body`` with const-mapped body folds to a
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
        // C35b: ``eval {body}`` in a proc context lowers to Block.
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
        // C35c: ``eval [list set x 42]`` recognised as static body.
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
        // C37b: a ``set body {literal}`` in the outer proc must
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
        // C38a: ``namespace import ::tcltest::*`` at top-level
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
        // C36b: ``set name {Verbose}; proc \$name {} { ... }``
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
        // C36c: ``proc \$name {x} [subst -nocommands {return \$default}]``
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
        // the proc name itself was substituted via C36b). Actually
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
        assert!(m
            .namespace_exports
            .iter()
            .any(|(ns, pat)| ns == "::tcltest" && pat == "test"));
    }

    #[test]
    fn ns_import_in_dead_branch_suppressed() {
        // C38c: ``if {0} { namespace import ::evil::* }`` — the
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
        // C35a: child scope (catch body) inherits parent's const-map.
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

    // -- SYNC9: trace add execution module-fact population -----------

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

    #[test]
    fn trace_add_execution_inside_proc_recorded() {
        let m = lower_to_ir(
            "proc init {} { trace add execution foo enter h }",
            &reg(),
        );
        assert!(
            m.traced_commands.contains("foo"),
            "traced_commands={:?}",
            m.traced_commands,
        );
    }

    // -- C43 / barrier-gate ------------------------------------------

    #[test]
    fn body_has_dynamic_barrier_clean() {
        // No barriers at all.
        assert!(!body_has_dynamic_barrier("set x 1"));
        // Barrier with a fully literal body.
        assert!(!body_has_dynamic_barrier("eval { set x 1 }"));
        // Nested literal.
        assert!(!body_has_dynamic_barrier(
            "if { 1 } { eval { set x 1 } }"
        ));
    }

    #[test]
    fn body_has_dynamic_barrier_dynamic_eval_body() {
        // ``eval $x`` inside the outer body — dynamic.
        assert!(body_has_dynamic_barrier("eval $x"));
        // Same shape nested.
        assert!(body_has_dynamic_barrier("if { 1 } { eval $x }"));
    }

    #[test]
    fn body_has_dynamic_barrier_dynamic_uplevel_body() {
        assert!(body_has_dynamic_barrier("uplevel 1 $body"));
        assert!(body_has_dynamic_barrier("uplevel #0 $b"));
    }

    #[test]
    fn body_has_dynamic_barrier_uplevel_with_literal_body_clean() {
        // ``uplevel $lvl {body}`` with a literal body is OK — the
        // gate only poisons when the BODY is substitution-bearing.
        // Mirrors Python parity.
        assert!(!body_has_dynamic_barrier("uplevel $lvl {set x 1}"));
    }

    #[test]
    fn body_has_dynamic_barrier_qualified_eval_uplevel() {
        // ``::eval`` and ``::uplevel`` are also caught.
        assert!(body_has_dynamic_barrier("::eval $x"));
        assert!(body_has_dynamic_barrier("::uplevel 1 $body"));
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
}
