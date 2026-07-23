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

//! Per-command handlers for the variable-mutation commands.
//!
//! The variable-write trio:
//!
//! - [`Analyser::handle_set_command`] — `set var ?value?`
//! - [`Analyser::handle_variable_command`] /
//!   [`Analyser::handle_global_command`] —
//!   `variable name ?value? ...?` and `global name...`
//! - [`Analyser::handle_incr_command`] — `incr var ?amount?`
//!
//! Plus the remaining structural command handlers: proc-body
//! walking, `namespace eval`/`namespace ensemble`, `foreach`/`for`/
//! `switch`, `catch`/`try`, `interp alias`, `oo::objdefine`, and
//! alias resolution.

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

use crate::alias::{detect_interp_alias, resolve_alias};
use crate::signature_scan::types::SignatureCommandAlias;

use super::state::Analyser;
use super::types::{DefinedSymbol, ProcDef};
use super::utils::{param_name_spans_for_token, parse_param_list};

/// Tcl *library* procedures (defined in init.tcl / auto.tcl / history.tcl /
/// package.tcl / word.tcl) that are script-defined and documented as
/// user-replaceable — redefining one is the supported overlay idiom, not
/// shadowing a C built-in.  Genuine C commands that are not byte-compiled
/// but still dangerous to redefine (`clock`, `after`, `socket`, `glob`) are
/// deliberately excluded — they keep firing W113.
/// Memoised set of the redefinable Tcl library procs (`unknown`,
/// `auto_*`, `pkg_*`, `tclLog`, `tcl_findLibrary`, the word-boundary
/// helpers, …), sourced from the registry's
/// [`Traits::OVERRIDABLE_LIBRARY_PROC`] trait. Redefining one of these
/// must not fire W113. Cached once; the set is dialect-agnostic core Tcl.
fn overridable_library_procs() -> &'static std::collections::HashSet<String> {
    use std::sync::OnceLock;
    static SET: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        tcl_registry::CommandRegistry::build_default()
            .commands_with_trait(tcl_registry::Traits::OVERRIDABLE_LIBRARY_PROC)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Build a fully-qualified Tcl proc / class name from a namespace
/// prefix and a possibly-relative name.
///
/// `ns_prefix` is a **constructed** namespace key, rooted (`::a::b`, `::`)
/// or unrooted (`a::b`, empty = global); it is joined verbatim — only the
/// *written* `name`'s colon runs canonicalise (#934).
pub(super) fn qualify(ns_prefix: &str, name: &str) -> String {
    crate::naming::qualify(ns_prefix, name)
}

/// Normalise a literal `interp` path word to the interpreter-domain map
/// key: the path is a Tcl *list* naming a descent through child
/// interpreters (`{s t}` = child `t` of child `s`), so whitespace runs
/// collapse to one separator.  A single-element path (`s`) keys as
/// itself.
pub(super) fn interp_path_key(path: &str) -> String {
    path.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Condense a definition's description argument into a single-line outline
/// detail: the first non-empty line, trimmed, and length-capped so a verbose
/// multi-line description doesn't bloat the outline entry.
fn summarise_detail(description: &str) -> String {
    const MAX: usize = 80;
    let line = description
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .trim();
    if line.chars().count() > MAX {
        let truncated: String = line.chars().take(MAX - 1).collect();
        format!("{truncated}…")
    } else {
        line.to_string()
    }
}

impl Analyser {
    /// Handle the `set` command: `set var ?value?`.
    ///
    /// - **Two-arg form** (`set var value`) — defines the variable
    ///   in the scope at `scope_path` and tracks the value as a
    ///   constant string when the value is a single-token literal
    ///   (no interpolation, no command sub).
    /// - **One-arg form** (`set var`) — records a var read on the
    ///   variable.  Tcl `set` with no value returns the current
    ///   value, so this is a reference, not a definition.
    ///
    /// `single_token_word` parallels `args` and `arg_tokens` —
    /// `true` when the corresponding word is a single atomic
    /// token, i.e. when the word's text is the same as a single
    /// token's raw text.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Set`];
    /// only the argument-shape checks live here.
    pub fn handle_set_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        single_token_word: &[bool],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }

        // Arg-count branch: two-arg form defines, one-arg form reads.
        let Some(name_tok) = arg_tokens.first() else {
            return;
        };
        if args.len() >= 2 {
            self.define_var(&args[0], *name_tok, scope_path, true, None);
        } else {
            self.record_var_read(&args[0], name_tok.span, scope_path);
        }

        // Track constant-string assignments for regex propagation.
        // Skipped for the 1-arg read form (no value to track).
        if args.len() < 2 || arg_tokens.len() < 2 {
            return;
        }
        let value_token = arg_tokens[1];
        let value_is_single_token = single_token_word.get(1).copied().unwrap_or(false);
        let value_token_kind = value_token.kind;
        if value_is_single_token && matches!(value_token_kind, TokenType::Esc | TokenType::Str) {
            self.set_const_string(&args[0], args[1].clone(), value_token.span, scope_path);
        } else {
            self.clear_const_string(&args[0], scope_path);
        }
    }

    /// Handle a `global` declaration: a flat list of names; each gets
    /// a var binding with `warn_if_unused = false` (declared, not
    /// "set but unused").
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Global`].
    pub fn handle_global_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        // `global ::ns::v` makes the *unqualified tail* (`v`) a local alias
        // to the global variable, so define the tail name locally (matches
        // Tcl + completion's expectation of both `$v` and `$::ns::v`).
        for (i, name) in args.iter().enumerate() {
            let local = name.rsplit("::").next().unwrap_or(name);
            // A dynamic name (`global $dyn`) computes its name at runtime — not
            // a static declaration.
            if let Some(tok) = arg_tokens.get(i)
                && !crate::naming::is_dynamic_word(name)
            {
                self.define_var(local, *tok, scope_path, false, None);
                // `global v` aliases the global cell `::v`; `global ::ns::v`
                // aliases `::ns::v` as written.  Record the target so every
                // `global` alias and the global declaration unify.
                let target = if name.starts_with("::") {
                    name.clone()
                } else {
                    format!("::{name}")
                };
                self.set_var_link_target(local, scope_path, target);
            }
        }
    }

    /// Handle a `variable` declaration: alternating `name ?value?`
    /// pairs; only the names get bindings. The optional value words
    /// are skipped (the IR pass handles their assignment if the value
    /// form actually fires).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Variable`].
    pub fn handle_variable_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        // `variable name ?value? name ?value? ...`
        // Each `name` aliases the cell `<current-namespace>::<name>`; every
        // `variable name` across that namespace's procs, plus the namespace
        // -level declaration, shares that target and so unifies.
        let ns = self.command_resolution_namespace(scope_path);
        let ns_prefix = ns.trim_end_matches("::");
        let mut i = 0;
        while i < args.len() {
            // A dynamic name (`variable $dyn` / `variable [f]`) is computed at
            // runtime — its literal text is not the variable's name, so it is
            // not a static declaration.  Skip it rather than record the
            // substitution text as a variable.
            if let Some(tok) = arg_tokens.get(i)
                && !crate::naming::is_dynamic_word(&args[i])
            {
                // Mirror `global` above: the *unqualified tail* is the local
                // alias name, but the target keeps the FULL qualified path so a
                // relative `variable child::v` aliases `<ns>::child::v` (and an
                // absolute `variable ::x::v` aliases `::x::v`), never the
                // tail-collapsed `<ns>::v`.  Keying the link on the tail is what
                // lets a later `$v` reference share the target and unify.
                let local = args[i].rsplit("::").next().unwrap_or(&args[i]);
                self.define_var(local, *tok, scope_path, false, None);
                let target = if args[i].starts_with("::") {
                    args[i].clone()
                } else {
                    format!("{ns_prefix}::{}", args[i])
                };
                self.set_var_link_target(local, scope_path, target);
            }
            i += if i + 1 < args.len() { 2 } else { 1 };
        }
    }

    /// Record a lightweight named definition for a registry *symbol-definer*
    /// command (`tcltest::test NAME …`, …) so it appears in the document /
    /// workspace outline alongside procs, classes, and variables (issue #790).
    ///
    /// Everything command-specific is registry data: which argument holds the
    /// name, which holds the description, and the outline category all come from
    /// the command's [`tcl_registry::SymbolDef`] — there is no `cmd == "test"`
    /// arm here, so registering the next such command (a benchmark runner, a
    /// custom test wrapper) is a one-line spec change.
    ///
    /// The name is resolved through the analyser's constant-propagation lattice
    /// (`const_strings` / [`Self::lookup_const_string`]): a bare literal, a
    /// constant `$var`, or a quoted literal all resolve identically, and a
    /// genuinely dynamic name (`test $unknown …`) is skipped rather than
    /// recorded as the literal text `$unknown`.  This is a void handler — it
    /// records the symbol and returns, leaving the command's body recursion to
    /// the generic `ArgRole::Body` walk.
    pub fn handle_defines_symbol(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        let Some(sym) = self.resolve_symbol_definer(cmd_name, scope_path) else {
            return;
        };
        // A command that defines only in one form (the `testConstraint NAME
        // value` setter, not the `testConstraint NAME` getter) records a symbol
        // only when its defining argument is present.
        if let Some(req) = sym.requires_arg
            && args.get(req as usize).is_none()
        {
            return;
        }
        let name_idx = sym.name_arg as usize;
        let (Some(name_tok), Some(name_text), Some(name_single)) = (
            arg_tokens.get(name_idx).copied(),
            args.get(name_idx),
            arg_single.get(name_idx).copied(),
        ) else {
            return;
        };
        // Resolve the name through constant propagation; skip a dynamic name.
        let Some(name) = self.resolve_const_word(name_text, name_tok, name_single, scope_path)
        else {
            return;
        };
        if name.is_empty() {
            return;
        }

        // The description argument, but only when it too resolves to a constant.
        let detail = sym.detail_arg.and_then(|d| {
            let di = d as usize;
            let tok = arg_tokens.get(di).copied()?;
            let text = args.get(di)?;
            let single = arg_single.get(di).copied().unwrap_or(false);
            self.resolve_const_word(text, tok, single, scope_path)
                .map(|d| summarise_detail(&d))
        });

        // Fold range: the name token through the end of the last argument.
        let end = arg_tokens
            .last()
            .map_or(name_tok.span.end(), |t| t.span.end())
            .max(name_tok.span.end());
        let full_span = Span::new(name_tok.span.start(), end);

        let ns_prefix = self.namespace_from_scope_path(scope_path);
        let qualified = qualify(ns_prefix.trim_start_matches(':'), &name);

        let symbol = DefinedSymbol {
            name,
            qualified_name: qualified,
            kind: sym.kind,
            name_span: name_tok.span,
            full_span,
            detail,
        };
        self.result.all_defined_symbols.push(symbol.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.defined_symbols.push(symbol);
        }
    }

    /// Resolve `cmd_name` at `scope_path` to its [`tcl_registry::SymbolDef`], if
    /// the command (or an imported bare form of it) declares one.
    ///
    /// Mirrors the imported-command fallback the body-role walk uses so a bare
    /// `test` reached through `namespace import ::tcltest::*` resolves to the
    /// qualified `tcltest::test` spec — the import must be in effect at the call
    /// site (its own namespace, or a global-scope import via Tcl's `::`
    /// fallback).
    fn resolve_symbol_definer(
        &mut self,
        cmd_name: &str,
        scope_path: &[usize],
    ) -> Option<tcl_registry::SymbolDef> {
        let dialect = self.profile.availability_mask;
        // `namespace_from_scope_path` needs `&mut self`; compute it before the
        // shared `registry` borrow, and only when imports could matter.
        let cur_ns = if self.result.namespace_imports.is_empty() {
            String::new()
        } else {
            self.namespace_from_scope_path(scope_path)
        };
        // Does `cmd_name` (or an imported bare form of it) name a registry
        // definer?  The `registry` borrow is confined to this block so the
        // shadowing check below can re-borrow `self`.
        let sym = {
            let registry = self.registry?;
            if let Some(sym) = registry.defines_symbol(cmd_name, dialect) {
                Some(*sym)
            } else if cmd_name.contains("::") {
                None
            } else {
                self.result
                    .namespace_imports
                    .iter()
                    .filter(|imp| imp.ns == cur_ns || imp.ns == "::")
                    .find_map(|imp| {
                        let candidate = if let Some(prefix) = imp.pattern.strip_suffix('*') {
                            format!("{prefix}{cmd_name}")
                        } else if imp.pattern.rsplit("::").next() == Some(cmd_name) {
                            imp.pattern.clone()
                        } else {
                            return None;
                        };
                        registry.defines_symbol(&candidate, dialect).copied()
                    })
            }
        }?;
        // A user-defined proc of the same name shadows the imported / built-in
        // definer under Tcl's command resolution — a local `proc test` beats an
        // imported `::test` — so a bare call to it invokes that proc, not the
        // tcltest definer, and must not be recorded as a test symbol.
        if self.resolve_proc_call(cmd_name, scope_path).is_some() {
            return None;
        }
        Some(sym)
    }

    /// Resolve a single argument word to a constant string, or `None` when it is
    /// not statically constant.
    ///
    /// The analyser-level counterpart to the optimiser's SCCP: a plain literal
    /// word (its delimiters already stripped into `text`) is itself constant; a
    /// bare `$var` resolves through the constant-string lattice
    /// ([`Self::lookup_const_string`]); anything with embedded substitution or
    /// concatenation is not statically known.
    fn resolve_const_word(
        &self,
        text: &str,
        tok: Token,
        is_single: bool,
        scope_path: &[usize],
    ) -> Option<String> {
        if !is_single {
            return None;
        }
        match tok.kind {
            TokenType::Str | TokenType::Esc => Some(text.to_string()),
            TokenType::Var => {
                let sm = tcl_lexer::SourceMap::new(&self.source);
                let var_name = sm.token_text(tok);
                self.lookup_const_string(var_name, scope_path)
                    .map(str::to_string)
            }
            _ => None,
        }
    }

    /// Handle the `proc` command: `proc NAME PARAMS BODY`.
    ///
    /// Returns `true` when the command was handled (callers use the
    /// bool to decide whether further processing is needed), `false`
    /// when the input doesn't match the expected shape.
    ///
    /// Records the [`ProcDef`] in
    /// both `scope.procs` (keyed by simple name) and
    /// `result.all_procs` (keyed by qualified name).  When the
    /// body is a braced literal, opens a fresh
    /// [`super::types::ScopeKind::Proc`] child scope, defines each parameter
    /// in it, and re-segments the body via
    /// [`crate::segmenter::segment_commands_with_offset`] —
    /// every body command is dispatched through
    /// [`Analyser::process_command`] with the new scope path.
    /// Body recursion does **not** invoke segmenter recovery —
    /// that fires only at the top level.
    /// Dynamic bodies (`$body`, `[gen]`) skip the walk because
    /// they cannot be statically re-segmented; the proc record
    /// is still emitted so downstream consumers see the
    /// signature.
    ///
    /// Infer per-parameter traits for a proc body.  Always runs
    /// the shallow pass (`infer_param_traits`); when
    /// [`Self::deep_param_traits`] is set, also runs the
    /// recursive deep pass (`infer_param_traits_deep`) and
    /// unions both via [`super::param_traits::merge_traits`].
    /// Threads the analyser's pre-built dialect-aware registry
    /// through so iRules-only `arg_role_resolver` callbacks
    /// (e.g. `when`) fire on body args.  When [`Self::registry`]
    /// is `None` (outside an active `analyse` run, e.g. a unit-
    /// test harness) we skip the inference rather than pay the
    /// cost of a fresh `build_default` on every proc.
    fn infer_proc_param_traits(
        &self,
        params: &[crate::signature_scan::types::ParamDef],
        body_text: &str,
    ) -> std::collections::HashMap<String, std::collections::HashSet<super::types::ProcArgTrait>>
    {
        let Some(registry) = self.registry else {
            return std::collections::HashMap::new();
        };
        let overlay = self.stub_overlay.as_ref();
        let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        let config = self.lexer_config();
        let shallow = super::param_traits::infer_param_traits_with_config(
            &param_names,
            body_text,
            registry,
            overlay,
            config,
        );
        if self.deep_param_traits {
            let deep = super::param_traits::infer_param_traits_deep_with_config(
                &param_names,
                body_text,
                registry,
                overlay,
                config,
            );
            super::param_traits::merge_traits(shallow, deep)
        } else {
            shallow
        }
    }

    /// **W113** — a `proc` name shadows a built-in command.
    ///
    /// The check runs against both the unqualified `raw_name` and the
    /// fully-qualified form, with the leading `::` trimmed (the registry
    /// indexes by bare command name).  The inner borrow on
    /// `self.builtin_command_names()` is dropped before the diagnostic push so
    /// `self.result` is free to mutate.
    ///
    /// Only *core global* built-ins are shadow-worthy (unqualified `set` /
    /// `dict` / …).  A namespace-qualified match (`::base64::encode`,
    /// `::snit::type`) is a library/package command living in its own namespace
    /// — its own definition or a deliberate override, not shadowing a built-in.
    /// And the overridable Tcl *library* procedures (`unknown`, `history`,
    /// `auto_*`, …) are script-defined, documented user-replaceable overlays.
    fn emit_w113_proc_shadows_builtin(&mut self, raw_name: &str, qualified: &str, name_span: Span) {
        let normalised_proc: String = raw_name.trim_start_matches(':').to_string();
        let normalised_qual: String = qualified.trim_start_matches(':').to_string();
        let shadow_name: Option<String> = {
            let builtins = self.builtin_command_names();
            // A `tcl::mathop` operator (`%`, `+`, `eq`, …) carries a bare-name
            // registry entry only so an `import`ed / `namespace path`-reachable
            // call resolves; the command itself lives in `::tcl::mathop`, not the
            // global namespace, so `proc %` shadows nothing reachable. Detect it
            // by its qualified spelling and treat it like the other namespaced
            // (non-core-global) commands the check already skips.
            let is_mathop = |n: &str| builtins.contains(&format!("tcl::mathop::{n}"));
            if builtins.contains(&normalised_proc) && !is_mathop(&normalised_proc) {
                Some(normalised_proc.clone())
            } else if builtins.contains(&normalised_qual) && !is_mathop(&normalised_qual) {
                Some(normalised_qual.clone())
            } else {
                None
            }
        };
        let shadow_name = shadow_name.filter(|name| {
            !name.contains("::") && !overridable_library_procs().contains(name.as_str())
        });
        if shadow_name.is_some() {
            // The permissive fallback profile means "no specific dialect" —
            // no parenthetical label (the old empty-string contract).
            let dialect_label = if self.profile.is_fallback() {
                String::new()
            } else {
                format!(" ({})", self.dialect())
            };
            let message = format!("Procedure '{raw_name}' shadows built-in command{dialect_label}");
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W113,
                span: name_span,
                message,
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W314.** The definition's name has no absolute (fully-qualified)
    /// written form (issue #934).  A written colon run of two or more is a
    /// namespace separator (the whole run collapses, `TclGetNamespaceForQualName`,
    /// invariant C 8.4→9.1), so an **all-colon simple name** — only `:` is
    /// writable — can never be spelled absolutely: `namespace which :` renders
    /// `:::`, which parses back to the global `{}` command, ensembles cannot
    /// dispatch an exported `:`, and `interp alias` / callback qualification
    /// break the same way.  The command remains reachable by *relative*
    /// lookup only.  (tclsh 8.6/9.0-pinned.)
    pub(super) fn emit_w314_no_absolute_name(&mut self, raw_name: &str, name_span: Span) {
        // The written simple name is the last colon-run-delimited segment.  A
        // name *ending* in a run names the `{}` command, which IS addressable
        // (`::x::`); an all-colon segment can only be a lone `:` (a run of ≥2
        // is consumed as the separator).
        let unaddressable = !tcl_syntax::naming::ends_with_separator(raw_name.as_bytes())
            && crate::naming::qualifier_segments(raw_name.as_bytes())
                .last()
                .is_some_and(|seg| *seg == b":");
        if !unaddressable {
            return;
        }
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W314,
            span: name_span,
            message: format!(
                "'{raw_name}' has no absolute (fully-qualified) name — a written colon run \
                 is a namespace separator, so this definition is reachable only by \
                 unqualified/relative lookup ([namespace which] output will not resolve, \
                 and ensembles cannot dispatch it)"
            ),
            severity: super::types::Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W314**, namespace flavour: a `namespace eval` (or sibling) whose
    /// written name carries an **all-colon segment** (`namespace eval :`)
    /// creates a namespace no absolute path can spell — its entire contents
    /// are reachable only relatively (`namespace inscope : :`), and
    /// `namespace current` inside renders an unresolvable `:::`.
    pub(super) fn emit_w314_unaddressable_namespace(&mut self, written_ns: &str, span: Span) {
        let has_all_colon_segment = crate::naming::qualifier_segments(written_ns.as_bytes())
            .into_iter()
            .any(|seg| seg == b":");
        if !has_all_colon_segment {
            return;
        }
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W314,
            span,
            message: format!(
                "namespace '{written_ns}' has no absolute (fully-qualified) path — a \
                 written colon run is a namespace separator, so everything defined \
                 inside is reachable only by relative lookup"
            ),
            severity: super::types::Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W218.** `args` declared anywhere but the final parameter position
    /// is an ordinary parameter named `args` — C Tcl sets `VAR_IS_ARGS`
    /// only on the last formal (`tclProc.c`), so the variadic
    /// collect-the-rest meaning is silently lost.  Anchors at the
    /// parameter's own name span; `fallback_tok` is used when the name
    /// span could not be recovered.  Shared by the `proc`, `apply`, and
    /// OO-method param walks.
    pub(super) fn emit_w218_args_not_final(
        &mut self,
        params: &[crate::signature_scan::types::ParamDef],
        param_spans: &[tcl_lexer::Span],
        fallback_tok: Token,
    ) {
        let Some(last) = params.len().checked_sub(1) else {
            return;
        };
        for (i, p) in params.iter().enumerate() {
            if i == last || p.name != "args" {
                continue;
            }
            let span = param_spans.get(i).copied().unwrap_or(fallback_tok.span);
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W218,
                span,
                message: "`args` here is an ordinary parameter — it only has its special                           collect-the-rest meaning as the final parameter. Move it last, or                           rename it if a plain parameter is intended."
                    .to_string(),
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// Handle a `proc name params body` definition: record the procedure
    /// (name, parameter list, body, and harvested doc-comment) in the
    /// analysis result and run per-parameter trait inference. Returns `true`
    /// when the definition had the full three-argument shape this handler
    /// consumes.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Proc`].
    pub fn handle_proc_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 || arg_tokens.len() < 3 {
            return false;
        }

        let raw_name = &args[0];
        // Home the proc to the *command-resolution* namespace, not the purely
        // lexical one: a proc defined inside another proc's body homes to that
        // enclosing proc's **defining** namespace (the prefix of its qualified
        // name), the way Tcl resolves the `proc` command at run time.  The
        // lexical `namespace_from_scope_path` skips proc scopes and so homed a
        // nested `proc helper` under `proc a::outer` to `::helper`, overwriting
        // the real global `::helper` in `all_procs`; the command-resolution
        // namespace homes it to `::a::helper`.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // `qualify` takes the constructed (rooted) namespace key verbatim and
        // `key_tail` inverts the construction — a `char`-pattern colon trim or
        // an `rsplit("::")` here would collapse a lone-colon name (#934).
        let qualified = qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_tok = arg_tokens[0];
        let name_span = name_tok.span;
        let body_tok = arg_tokens[2];
        let body_span = body_tok.span;

        // **W113** — proc name shadows a built-in command.
        self.emit_w113_proc_shadows_builtin(raw_name, &qualified, name_span);
        // **W314** — the name has no absolute written form (#934).
        self.emit_w314_no_absolute_name(raw_name, name_span);

        let params = parse_param_list(&args[1]);
        // Doc string: prefer the preceding-comment harvest from
        // the segmenter; fall back to ``extract_body_docstring``
        // (leading comment block at the top of the body).
        let mut doc = std::mem::take(&mut self.last_comment);
        if doc.is_empty() && args.len() >= 3 {
            doc = super::utils::extract_body_docstring(&args[2]);
        }

        // When a user defines ``proc unknown ...`` (or
        // ``::tcl::unknown``), inspect the body to determine which
        // commands the handler can resolve.  The result gates
        // W123 (unresolved command) — if the user provided their
        // own ``unknown`` we can't statically prove a command is
        // truly unresolved.
        if matches!(simple.as_str(), "unknown") || qualified == "::tcl::unknown" {
            let info = self.extract_unknown_proc_info(&args[2], &params);
            self.result.unknown_proc_info = Some(info);
        }

        let body_text = &args[2];
        let param_traits = self.infer_proc_param_traits(&params, body_text);

        let proc = ProcDef {
            name: simple,
            qualified_name: qualified.clone(),
            params: params.clone(),
            name_span,
            body_span,
            doc,
            param_traits,
        };

        // Register globally and in the current scope. ``scope.procs``
        // is keyed by the *simple* (unqualified) proc name (so
        // per-scope lookup and shadowing rules work locally), while
        // ``result.all_procs`` is keyed by the fully-qualified
        // name. The full qualified name is still on
        // ``ProcDef.qualified_name`` for callers that need it.
        self.result
            .all_procs
            .insert(qualified.clone(), proc.clone());
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(simple_key.clone(), proc);
        }
        // A `proc` defined anywhere inside an `interp eval` body creates a
        // real command in that interpreter's ordinary command table,
        // independent of the hidden set the safe-interp gate consults —
        // see `mark_locally_defined_in_enclosing_interp`'s doc comment.
        self.mark_locally_defined_in_enclosing_interp(&simple_key);

        // Walk the body in a fresh proc scope when the body is a
        // braced literal. ``raw_name`` is used as the proc-scope
        // name because ``define_var`` keys ``result.all_variables``
        // on ``"<scope_name>::<var>"``, so the scope name must be
        // the proc name to key that map consistently.
        if body_tok.kind == TokenType::Str {
            let proc_scope_idx = {
                let parent = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
                    .expect("scope_path resolved when registering proc must still resolve");
                let mut child =
                    super::types::Scope::new(super::types::ScopeKind::Proc, raw_name.clone());
                child.body_span = Some(body_span);
                parent.children.push(child);
                parent.children.len() - 1
            };
            let mut child_path = path.clone();
            child_path.push(proc_scope_idx);

            // Parameters become locals in the proc scope. Each param's
            // definition range is anchored to its *name* in the param-list
            // literal (issue #727) so go-to-definition / references / rename on
            // a formal parameter resolve to the parameter, not the proc name.
            // The spans are recovered from the raw param-list word token
            // (`arg_tokens[1]`); any param whose name can't be located falls
            // back to the proc name token.
            let params_tok = arg_tokens[1];
            let param_spans = param_name_spans_for_token(&self.source, params_tok);
            for (i, p) in params.iter().enumerate() {
                self.define_var(
                    &p.name,
                    name_tok,
                    &child_path,
                    false,
                    param_spans.get(i).copied(),
                );
            }
            self.emit_w218_args_not_final(&params, &param_spans, params_tok);

            // Save / restore last_comment around the body walk so
            // a doc-comment inside the proc body doesn't bleed to
            // whatever follows the proc at the outer scope.
            let saved_comment = std::mem::take(&mut self.last_comment);

            // Body recursion via the shared helper.  Re-segments
            // the body (no recovery — top-level only) and
            // dispatches each command at the new proc scope path.
            // Per-item shell pass: defer the body (its scope is already
            // created with params; a second pass fills it in place).
            let body_text = args[2].clone();
            if self.defer_proc_bodies {
                let safe_interp_ctx = self.safe_interp_ctx_snapshot();
                self.deferred_bodies.push(super::per_item::DeferredBody {
                    body_text,
                    body_tok,
                    scope_path: child_path.clone(),
                    is_method: false,
                    oo_global_resolution: false,
                    namespace: ns_prefix.clone(),
                    scope_name: raw_name.clone(),
                    params: params.clone(),
                    class_variables: Vec::new(),
                    safe_interp_ctx,
                });
            } else {
                self.analyse_body(&body_text, body_tok, &child_path);
            }

            self.last_comment = saved_comment;
        }

        true
    }

    /// Split an `apply` call's lambda-literal first argument
    /// (`{{params} body ?ns?}`) into its list elements — `(token, text)`
    /// pairs carrying absolute source spans, in declaration order (params,
    /// body, and an optional target-namespace pin).
    ///
    /// Returns `None` when there are no arguments or the first argument
    /// isn't a *braced* literal (a `$var` / `[cmd]` / quoted lambda is
    /// opaque — its element boundaries can't be split statically).  Both
    /// callers gate on [`tcl_registry::hooks::AnalyserHookId::Apply`]
    /// first. Shared by [`Self::handle_apply_command`] (body
    /// / scope walk) and the `apply` direct-call arity check
    /// ([`super::diagnostics::validity::Analyser::emit_arity_diagnostics`])
    /// so both consumers agree on exactly what counts as a
    /// statically-inspectable lambda, rather than each re-implementing the
    /// brace-literal guard and segmentation independently.
    pub(in crate::analyser) fn parse_apply_lambda_elements(
        &self,
        args: &[String],
        arg_tokens: &[Token],
    ) -> Option<Vec<(Token, String)>> {
        if args.is_empty() || arg_tokens.is_empty() {
            return None;
        }
        let lambda_tok = arg_tokens[0];
        // Only a *braced* literal lambda can be split and offset-mapped safely
        // (its content is verbatim source).
        let lambda_start = lambda_tok.span.start() as usize;
        if lambda_tok.kind != TokenType::Str
            || self.source.as_bytes().get(lambda_start) != Some(&b'{')
        {
            return None;
        }

        // Split the lambda literal into its list elements. Re-segmenting the
        // brace-stripped content (`args[0]`) at the lambda's absolute content
        // offset yields the elements as command words carrying absolute-span
        // tokens; flattening across commands keeps params / body paired even
        // when a multi-line lambda puts them on separate lines (a newline
        // splits *commands*, not *list elements*).
        let base = lambda_tok.span.start() + u32::from(lambda_tok.content_offset);
        let segmented = crate::segmenter::segment_commands_with_offset_and_config(
            &args[0],
            base,
            self.lexer_config(),
        );
        Some(
            segmented
                .iter()
                .flat_map(|c| c.argv.iter().copied().zip(c.texts.iter().cloned()))
                .collect(),
        )
    }

    /// Handle an `apply {{params} body ?ns?} ?arg ...?` invocation.
    ///
    /// `apply`'s first argument is a *lambda* (an anonymous procedure), **not**
    /// a plain script body: element 0 is the parameter list and element 1 is
    /// the body. The `apply` registry spec marks the argument `ArgRole::Body`,
    /// so without this handler the generic body-recursion in
    /// [`Analyser::dispatch_body_arguments`] treats the whole `{{params} body}`
    /// literal as a *script* — mis-reading the parameter list as a command
    /// (`apply {{a} {…}}` ⇒ a spurious W123 "unknown command 'a'") and never
    /// linting the real body.
    ///
    /// This models the lambda like a `proc`: the parameters bind as locals in a
    /// fresh proc scope and element 1 is walked as the body (deferred during the
    /// per-item shell pass, inline otherwise — mirroring
    /// [`Analyser::handle_proc_command`], so the incremental and full paths stay
    /// byte-identical). The body is walked in the *lambda's* namespace — element
    /// 2 of the lambda, or the global namespace `::` when absent — not the
    /// caller's, so a nested `proc` registers under the qualified name the
    /// runtime `apply` gives it (`::p`, not `::caller::p`). Returns `true` when
    /// the command was an `apply` with a
    /// statically-inspectable braced lambda literal, so the caller skips the
    /// generic body recursion. A dynamic lambda (`apply $lambda …`) returns
    /// `false` and falls through to the generic path (whose `analyse_body` is a
    /// no-op on the non-braced word), preserving its existing behaviour.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Apply`].
    pub fn handle_apply_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        let Some(elements) = self.parse_apply_lambda_elements(args, arg_tokens) else {
            return false;
        };
        // A lambda needs at least a parameter list and a body.
        if elements.len() < 2 {
            return true;
        }
        let (params_tok, params_text) = (elements[0].0, elements[0].1.as_str());
        let (body_tok, body_text) = (elements[1].0, elements[1].1.as_str());

        // The body must itself be a braced literal to walk statically (matching
        // `analyse_body`'s own `TokenType::Str` guard); a bare / substituted
        // body is left un-analysed, identically on the inline and per-item
        // paths so the two never diverge.
        if body_tok.kind != TokenType::Str
            || self.source.as_bytes().get(body_tok.span.start() as usize) != Some(&b'{')
        {
            return true;
        }

        // `apply` runs the lambda body in the namespace named by lambda element
        // 2, or the *global* namespace when it is absent — never the caller's
        // namespace (Tcl `apply` manual). Derive the body namespace so a nested
        // `proc` registers under the qualified name the runtime `apply` would
        // give it (`::p`, not `::caller::p`). A non-`::`-qualified pin resolves
        // relative to the caller (as `TclGetNamespaceFromObj`); absent → global.
        let body_ns = match elements.get(2).map(|(_, t)| t.as_str()) {
            Some(ns) if !ns.is_empty() && !ns.starts_with('$') && !ns.starts_with('[') => {
                let caller_ns = self.namespace_from_scope_path(scope_path);
                qualify(caller_ns.trim_start_matches(':'), ns)
            }
            _ => "::".to_string(),
        };

        let params = parse_param_list(params_text);
        let body_text = body_text.to_string();
        let body_span = body_tok.span;
        // Anonymous, but keyed by source position so two lambdas never collide
        // in `all_variables` (keyed `"<scope_name>::<var>"`).
        let scope_name = format!("apply@{}", arg_tokens[0].span.start());

        // Root the lambda scope at `body_ns` under the global scope — NOT under
        // the caller — via the same `reconstruct_proc_scope` the per-item path
        // uses, so the inline (full) and per-item (incremental) walks build
        // byte-identical structure and nested definitions resolve to `body_ns`.
        let child_path = super::per_item::reconstruct_proc_scope(
            &mut self.result.global_scope,
            &body_ns,
            &scope_name,
            super::types::ScopeKind::Proc,
        );
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &child_path)
        {
            scope.body_span = Some(body_span);
        }

        // Parameters become locals, each anchored to its name in the param-list
        // literal (issue #727) so go-to-definition / references / rename on a
        // formal resolve to the parameter, not the `apply` call.
        let param_spans = param_name_spans_for_token(&self.source, params_tok);
        for (i, p) in params.iter().enumerate() {
            self.define_var(
                &p.name,
                params_tok,
                &child_path,
                false,
                param_spans.get(i).copied(),
            );
        }
        self.emit_w218_args_not_final(&params, &param_spans, params_tok);

        // Save / restore last_comment around the body walk, as `proc` does, so a
        // doc-comment inside the lambda body doesn't bleed to what follows.
        let saved_comment = std::mem::take(&mut self.last_comment);
        if self.defer_proc_bodies {
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                body_text,
                body_tok,
                scope_path: child_path.clone(),
                is_method: false,
                oo_global_resolution: false,
                namespace: body_ns,
                scope_name,
                params,
                class_variables: Vec::new(),
                safe_interp_ctx,
            });
        } else {
            self.analyse_body(&body_text, body_tok, &child_path);
        }
        self.last_comment = saved_comment;
        true
    }

    /// Handle `namespace eval`: opens a new namespace scope and
    /// schedules its body for analysis.
    ///
    /// Returns `true` when the command was handled.
    ///
    /// Creates the child namespace scope so downstream handlers see
    /// qualified names resolve through it, then recurses into the
    /// body.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceEval`] (stamped
    /// on `namespace`'s `eval` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_eval_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 2 {
            return false;
        }
        let ns_name = args[1].clone();
        // **W314** — an all-colon segment (`namespace eval :`) creates a
        // namespace no absolute path can spell (#934).
        if let Some(ns_tok) = arg_tokens.get(1) {
            self.emit_w314_unaddressable_namespace(&ns_name, ns_tok.span);
        }
        let body_span = arg_tokens.get(2).map(|t| t.span);
        let body_text = args.get(2).cloned();
        let body_tok = arg_tokens.get(2).copied();

        // A dynamic target (`namespace eval $name { … }`, the irc.tcl
        // per-connection idiom) can't be resolved to a real namespace path —
        // and, unlike a literal one, two lexically unrelated occurrences
        // that happen to write the *same* variable name (an unremarkable
        // choice for this exact idiom) must never collapse into one scope:
        // each occurrence gets its own synthetic, per-call-site domain,
        // keyed by this argument token's own source offset (unique per
        // occurrence, deterministic, and — like `@interp@` — unrepresentable
        // in real Tcl, so it can never collide with a literal namespace of
        // the same written text). Mirrors `interp eval`'s dynamic-path
        // handling a few hooks below, which is conservatively isolated the
        // same way rather than merged by raw text.
        let scope_name = if crate::naming::is_dynamic_word(&ns_name) {
            arg_tokens.get(1).map_or_else(
                || ns_name.clone(),
                |tok| format!("@dynns@{}", tok.span.start()),
            )
        } else {
            ns_name
        };

        let path = scope_path.to_vec();
        let child_scope_idx = {
            let mut child =
                super::types::Scope::new(super::types::ScopeKind::Namespace, scope_name);
            child.body_span = body_span;
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path;
        child_path.push(child_scope_idx);

        // Body recursion lets procs and classes declared inside
        // ``namespace eval`` register with the correct namespace
        // prefix.
        if let (Some(text), Some(tok)) = (body_text, body_tok) {
            self.analyse_body(&text, tok, &child_path);
        }
        true
    }

    /// Handle `interp eval PATH SCRIPT`: the script runs in a **child**
    /// interpreter — a separate command / variable space — so its `proc` /
    /// `oo::class` / variable definitions and calls must not merge into the
    /// parent namespace (a parent `rename foo` must not rewrite a child `proc
    /// foo`; [`tcl_vm::interp`] isolates the child at run time).
    ///
    /// Only the single-script form (`interp eval child { … }`) is isolated: an
    /// **empty** path (`interp eval {} script`) targets the *current*
    /// interpreter — its definitions belong here — and a multi-word /
    /// non-literal script cannot be statically re-assembled, so both fall back
    /// to the generic body recursion by returning `false`.  Otherwise an
    /// isolated child scope is opened, named for the interpreter path so the
    /// child's definitions home under it (`::<path>::foo`) and the child's own
    /// calls still resolve within the block; a dynamic path (`$i`) can't
    /// collide with a real namespace, so it stays conservatively isolated too.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpEval`]
    /// (stamped on `interp`'s `eval` subcommand); `args[0]` is the subcommand
    /// word.
    pub fn handle_interp_eval_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        // `interp eval PATH arg ?arg ...?` — the subcommand word, the path,
        // and one or more script words (C concatenates them).
        if args.len() < 3 {
            return false;
        }
        // An empty path runs in the current interpreter — not isolated.
        if args[1].is_empty() {
            return false;
        }
        // The interpreter path is a *list* relative to the current
        // interpreter — inside a child's eval body it is relative to that
        // child — so qualify against the walk's interpreter-path stack.
        let literal_path = !crate::naming::is_dynamic_word(&args[1]);
        let key = self.qualified_interp_key(&args[1]);
        if literal_path {
            // Interpreter existence (issue #945 fault 8): evaluating into a
            // literal child this file never creates raises `could not find
            // interpreter` at run time.  Abstains when any interp operation
            // in the file used a dynamic path (existence then unknowable).
            if !self.interpreters.contains_key(&key)
                && !self.dynamic_interp_ops
                && let Some(tok) = arg_tokens.get(1)
            {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: tcl_core_types::DiagCode::W140,
                    span: tok.span,
                    message: format!(
                        "interpreter '{}' is never created in this file — \
                         `interp eval` will raise `could not find interpreter`",
                        args[1]
                    ),
                    severity: super::types::Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
        // Multiple script words concatenate at run time (`Tcl_ConcatObj`),
        // so commands can span word boundaries — no per-word walk is sound.
        // Consume the command *without* walking, keeping the parent domain
        // clean (the old fall-through analysed the words in the parent
        // scope — issue #945 fault 8); W312 separately flags the shape.
        if args.len() > 3 {
            return true;
        }
        let Some(body_tok) = arg_tokens.get(2).copied() else {
            return true;
        };
        self.isolate_interp_eval_body(&key, &args[2], body_tok, scope_path);
        true
    }

    /// Isolate `body_text` — already known to belong to interpreter `key` —
    /// into a synthetic `@interp@<key>` child scope, so its `proc` /
    /// `oo::class` / variable definitions and calls don't merge into the
    /// parent namespace. Shared core of [`Self::handle_interp_eval_command`]
    /// (literal `interp eval PATH { … }`) and
    /// [`Self::handle_interp_handle_eval_command`] (`NAME eval { … }` via the
    /// interpreter's own object command) — both recognise the *same*
    /// isolated-body shape, just via different call-site spellings, so both
    /// must isolate it identically rather than maintaining two copies of
    /// this logic that could drift apart.
    ///
    /// Returns `false` (never isolated) only when the scope-tree path has
    /// gone stale — shouldn't happen during a healthy walk.
    fn isolate_interp_eval_body(
        &mut self,
        key: &str,
        body_text: &str,
        body_tok: Token,
        scope_path: &[usize],
    ) -> bool {
        let outer = scope_path.to_vec();
        let child_scope_idx = {
            // The child's global namespace is its own domain, not a parent
            // namespace — home the scope under a synthetic
            // `@interp@<path>` name (unrepresentable in Tcl, mirroring
            // `@objdefine@`) so a real parent namespace of the same name
            // can never collide.  Repeated evals into the same live
            // interpreter share one domain name (their definitions
            // accumulate, as in C); the name carries the path's deletion
            // *epoch*, so a deleted-and-recreated interpreter is a fresh
            // domain that never merges with its predecessor's definitions
            // (issue #945 fault 8's temporal identity).
            let mut child = super::types::Scope::new(
                super::types::ScopeKind::Namespace,
                self.interp_domain_name(key),
            );
            child.body_span = Some(body_tok.span);
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &outer)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = outer;
        child_path.push(child_scope_idx);

        // A safe target interpreter evaluates the body with the unsafe
        // command set hidden, and any interpreter with explicit `interp
        // hide`s carries those deltas (issue #945 fault 7): push the
        // visibility context so the per-command gate flags hidden calls
        // (W129) and builds no effects from them.  A tainted state (dynamic
        // hide/expose) abstains.
        let safe_ctx = self.interpreters.get(key).and_then(|st| {
            (!st.tainted && (st.safe || !st.hidden.is_empty())).then(|| {
                super::state::SafeInterpCtx {
                    base_hidden: st.safe,
                    hidden_extra: st.hidden.clone(),
                    exposed: st.exposed.clone(),
                }
            })
        });
        let pushed = if let Some(ctx) = safe_ctx {
            self.safe_interp_stack.push(ctx);
            true
        } else {
            false
        };
        // `interp` operations inside the body name paths relative to *this*
        // child — push its path so they qualify correctly.
        self.interp_path_stack.push(key.to_string());
        self.analyse_body(body_text, body_tok, &child_path);
        self.interp_path_stack.pop();
        if pushed {
            self.safe_interp_stack.pop();
        }
        true
    }

    /// Handle `NAME eval SCRIPT` — a far more common real-world spelling of
    /// `interp eval NAME SCRIPT` than the `interp eval` form itself: `interp
    /// create` binds the child interpreter's *own* object command to its
    /// create-time name (`interp create sandbox` makes `sandbox` itself
    /// callable; `sandbox eval { … }` dispatches exactly like `interp eval
    /// sandbox { … }`). Without this, the body is neither isolated nor
    /// walked at all: no symbols are produced for procs defined inside it,
    /// and hover/go-to-definition on a call inside falls back to a
    /// scope-blind, file-wide "any proc anywhere with this bare name" match
    /// — which can resolve to a same-named proc in a completely unrelated
    /// interpreter.
    ///
    /// Recognised purely from analysis state — `cmd_name` is looked up
    /// against [`Self::interpreters`] (built by
    /// [`Self::handle_interp_create_command`]), never matched against a
    /// hardcoded name — so an ordinary proc that happens to be named `eval`
    /// as its first argument (`foo eval bar`, `foo` an unrelated command) is
    /// untouched. Only the single-script form is isolated, mirroring
    /// [`Self::handle_interp_eval_command`]; every other shape (no `eval`
    /// first word, wrong arity, an untracked head) falls through to the
    /// generic per-command dispatch by returning `false`.
    ///
    /// Deliberately narrower than the mined idiom's `$handle eval { … }`
    /// spelling: `cmd_name` is the literal, unsubstituted head text, so a
    /// `$`-prefixed handle (the value of a variable, not a name this file
    /// spells out) never matches an entry in [`Self::interpreters`] (which is
    /// keyed by literal `interp create`/`interp eval` path text) and falls
    /// through untouched — resolving a *variable's* interpreter value is the
    /// separate, harder value-flow problem `interp alias`'s cross-domain
    /// tracking already has its own narrower literal-path requirement for
    /// (issue #945 fault 8), not one this handler takes on.
    ///
    /// Dispatched from [`Self::dispatch_analyser_hook`]'s hookless fallback
    /// chain — `cmd_name` never matches a registry command (it's a
    /// user-chosen interpreter name), so this can't be reached via a
    /// registry hook the way the literal `interp eval` form is.
    pub fn handle_interp_handle_eval_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() != 2 || args[0] != "eval" {
            return false;
        }
        let key = self.qualified_interp_key(cmd_name);
        if !self.interpreters.contains_key(&key) {
            return false;
        }
        let Some(body_tok) = arg_tokens.get(1).copied() else {
            return false;
        };
        self.isolate_interp_eval_body(&key, &args[1], body_tok, scope_path)
    }

    /// Qualify a literal `interp` path operand against the enclosing
    /// `interp eval` bodies on the walk stack: paths are relative to the
    /// *current* interpreter, so `interp create t` inside
    /// `interp eval s {…}` names `s t`.  A dynamic operand is returned
    /// key-normalised only (its stack context cannot make it literal).
    fn qualified_interp_key(&self, path: &str) -> String {
        let local = interp_path_key(path);
        match self.interp_path_stack.last() {
            Some(enclosing) if !local.is_empty() => format!("{enclosing} {local}"),
            _ => local,
        }
    }

    /// The synthetic namespace name for an interpreter domain: the
    /// current epoch of `key` is folded in so a deleted-and-recreated
    /// interpreter never shares its predecessor's definitions.
    fn interp_domain_name(&self, key: &str) -> String {
        match self.interp_epochs.get(key) {
            Some(epoch) if *epoch > 0 => format!("@interp@{key}#{epoch}"),
            _ => format!("@interp@{key}"),
        }
    }

    /// Handle `interp create ?-safe? ?--? ?path?` — record the child
    /// interpreter's existence and safe state in the interpreter-domain
    /// map (issue #945 faults 7–8).  A dynamic path makes interpreter
    /// existence unknowable file-wide; a missing path auto-generates a
    /// name this file cannot reference literally, so nothing is recorded.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpCreate`];
    /// `args[0]` is the subcommand word.
    pub fn handle_interp_create_command(&mut self, args: &[String]) {
        let mut safe = false;
        let mut path: Option<&str> = None;
        let mut past_flags = false;
        for word in &args[1..] {
            if !past_flags && word == "-safe" {
                safe = true;
                continue;
            }
            if !past_flags && word == "--" {
                past_flags = true;
                continue;
            }
            path = Some(word);
            break;
        }
        let Some(path) = path else { return };
        if crate::naming::is_dynamic_word(path) {
            self.dynamic_interp_ops = true;
            return;
        }
        let state = super::state::InterpState {
            safe,
            ..Default::default()
        };
        let key = self.qualified_interp_key(path);
        self.interpreters.insert(key, state);
    }

    /// Handle `interp delete ?path ...?` — remove the recorded state and
    /// bump the path's **epoch**, so a later re-creation is a fresh
    /// domain (issue #945 fault 8's temporal identity).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpDelete`].
    pub fn handle_interp_delete_command(&mut self, args: &[String]) {
        for word in &args[1..] {
            if crate::naming::is_dynamic_word(word) {
                self.dynamic_interp_ops = true;
                continue;
            }
            let key = self.qualified_interp_key(word);
            if self.interpreters.remove(&key).is_some() {
                *self.interp_epochs.entry(key).or_insert(0) += 1;
            }
        }
    }

    /// Handle `interp hide path cmdName ?hiddenName?` — mark the command
    /// hidden in the target interpreter's domain.  The optional third word
    /// only renames the hidden-table entry for `invokehidden` — it doesn't
    /// change `cmdName`'s ordinary-lookup visibility, which is all this
    /// gate tracks — so it's not read here.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpHide`].
    pub fn handle_interp_hide_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, true);
    }

    /// Handle `interp expose path hiddenName ?exposedName?` — re-expose a
    /// hidden command in the target interpreter's domain, visible under
    /// `exposedName` when given (`hiddenName` itself stays absent from
    /// ordinary lookup — tclsh 9.0.4-verified).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpExpose`].
    pub fn handle_interp_expose_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, false);
    }

    /// Shared body of the `interp hide` / `interp expose` handlers: apply
    /// the visibility delta for a literal `(path, command)` pair; a dynamic
    /// operand taints the interpreter's state (its visible set becomes
    /// unknowable, so the safe-context gate abstains for it).
    fn apply_interp_visibility_delta(&mut self, args: &[String], hide: bool) {
        let (Some(path), Some(cmd)) = (args.get(1), args.get(2)) else {
            return;
        };
        if crate::naming::is_dynamic_word(path) {
            self.dynamic_interp_ops = true;
            return;
        }
        let key = self.qualified_interp_key(path);
        let Some(state) = self.interpreters.get_mut(&key) else {
            return;
        };
        if crate::naming::is_dynamic_word(cmd) {
            state.tainted = true;
            return;
        }
        if hide {
            state.hidden.insert(cmd.clone());
            state.exposed.remove(cmd);
            return;
        }
        // `interp expose path hiddenName ?exposedName?` restores the
        // hidden command under `exposedName` when given — `hiddenName`
        // itself stays unavailable to ordinary lookup (tclsh 9.0.4:
        // `interp expose s source src` leaves `source` absent and makes
        // `src` callable).  A dynamic `exposedName` makes the resulting
        // visible name unknowable, so it taints like a dynamic `cmd`
        // would.
        let exposed_name = match args.get(3) {
            Some(name) if crate::naming::is_dynamic_word(name) => {
                state.tainted = true;
                return;
            }
            Some(name) => name.clone(),
            None => cmd.clone(),
        };
        state.exposed.insert(exposed_name);
        state.hidden.remove(cmd);
    }

    /// Record that `bare_name` is now a real, locally-defined command in
    /// the interpreter body currently being walked (a `proc` (re)definition
    /// — issue #945 fault 7 follow-up).  C creates it in the ordinary
    /// command table, a separate table from the hidden set entirely, so it
    /// is callable independent of any hide the base safe set or an earlier
    /// `interp hide` applied (tclsh 9.0.4-verified).  Updates both the
    /// interpreter's persistent state (so a *later*, separate `interp eval`
    /// into the same path also sees it) and the live gate context on top of
    /// the walk stack (so a call later in *this same* body sees it too — the
    /// stack entry is a snapshot taken before the body walk began).  A
    /// no-op outside any interpreter body.
    pub(super) fn mark_locally_defined_in_enclosing_interp(&mut self, bare_name: &str) {
        let Some(key) = self.interp_path_stack.last() else {
            return;
        };
        if let Some(state) = self.interpreters.get_mut(key) {
            state.exposed.insert(bare_name.to_string());
        }
        if let Some(ctx) = self.safe_interp_stack.last_mut() {
            ctx.exposed.insert(bare_name.to_string());
        }
    }

    /// Handle `namespace path {…}`: record the current namespace's
    /// command-resolution search path for the post-walk settlement
    /// ([`Self::finalise_invocation_resolutions`]).
    ///
    /// Only the two-word set form
    /// with a *literal* list is recorded — the one-word query form mutates
    /// nothing, and a dynamic list (`$var` / `[cmd]`) is statically
    /// unknowable, so it keeps the conservative empty path. Each
    /// declaration replaces the namespace's whole path, as in C Tcl, so
    /// the lexically-last one wins (settlement is call-time / whole-file,
    /// like the rest of the resolution model).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespacePath`] (stamped
    /// on `namespace`'s `path` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_path_command(&mut self, args: &[String], scope_path: &[usize]) {
        if args.len() != 2 {
            return;
        }
        if tcl_syntax::naming::is_dynamic_word(&args[1]) {
            return;
        }
        // The `namespace path` command-resolution tier is an 8.5 addition
        // (`NamespacePathCmd`, tclNamesp.c); 8.4 has no path tier, so a bare
        // call there never reaches a path namespace.  Recording the path under
        // a pre-8.5 dialect would make command resolution / definition / hover
        // falsely settle a call onto a path entry the runtime never consults —
        // so skip it, matching the `namespace path` subcommand's own dialect
        // gate (which already flags the command W002 there).
        if !self
            .profile
            .availability_mask
            .intersects(tcl_dialect::DialectSet::TCL85_PLUS)
        {
            return;
        }
        let ns = self.command_resolution_namespace(scope_path);
        let entries = args[1].split_whitespace().map(str::to_string).collect();
        self.namespace_paths.insert(ns, entries);
    }

    /// Handle `uplevel #0 { body }`: the script runs in the global
    /// frame, so the body's locals belong to a global-rooted scope
    /// rather than the enclosing proc.  Open an [`ScopeKind::Uplevel`]
    /// child scope (nested under the current scope, but tagged so
    /// completion / definition treat it as a global frame and ignore the
    /// proc's locals) and analyse the body there.
    ///
    /// Returns `true` for every single-braced-body `uplevel` form; a
    /// multi-word / non-literal script is left to the generic recursion (the
    /// W301 injection check already flags that shape).
    ///
    /// The frame the body runs in depends on the level word: `#0` is the global
    /// frame; `N` / `#N` / an implicit level is a *caller* frame that is
    /// statically unknown.  Either way the body's locals do **not** belong to
    /// the enclosing proc, so both open an [`ScopeKind::Uplevel`] child scope
    /// (tagged with the level word so variable resolution can tell global-frame
    /// from unknown-frame: `#0` resolves outward to the global namespace, a
    /// non-`#0` level abstains — the true frame can't be named).  Without the
    /// child scope the body's variables merged into the enclosing proc's
    /// locals, silently unifying two variables in different frames.
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Uplevel`];
    /// the level word is a shape check, not a command name.
    pub fn handle_uplevel_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        // Two literal-body shapes: `uplevel LEVEL {body}` (level word + single
        // braced body) and `uplevel {body}` (implicit level 1).  A `#0` level
        // records the global frame; anything else records an unknown caller
        // frame.  Multi-word scripts (`args.len() > 2`) fall through.
        let (level_word, body_tok, body_text) = match args.len() {
            2 => {
                let Some(bt) = arg_tokens.get(1).copied() else {
                    return false;
                };
                (args[0].clone(), bt, args[1].clone())
            }
            1 => {
                let Some(bt) = arg_tokens.first().copied() else {
                    return false;
                };
                ("1".to_owned(), bt, args[0].clone())
            }
            _ => return false,
        };
        if body_tok.kind != TokenType::Str {
            return false;
        }

        let path = scope_path.to_vec();
        let child_idx = {
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
            else {
                return false;
            };
            // The scope name carries the level word so `lookup_var_in_scope_chain`
            // can distinguish the `#0` global frame (resolve outward) from a
            // non-`#0` unknown caller frame (abstain outward).
            let mut child = super::types::Scope::new(super::types::ScopeKind::Uplevel, level_word);
            child.body_span = Some(body_tok.span);
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path;
        child_path.push(child_idx);
        self.analyse_body(&body_text, body_tok, &child_path);
        true
    }

    /// Handle `namespace ensemble create` — record the namespace as
    /// an ensemble so its tail names become valid commands, plus an
    /// explicit `-command name` override when present.
    ///
    /// The implicit form (`namespace eval ::ens { namespace ensemble
    /// create }`) dispatches through a command named after the enclosing
    /// namespace; `-command NAME` instead creates the ensemble's dispatch
    /// command under an arbitrary, possibly differently-namespaced, name.
    /// Without recording that name too, a call through it (`myEns
    /// subcmd …`) resolves to nothing the analyser knows — drawing a
    /// spurious W123 and abstaining from arity checking for the wrong
    /// reason (an unresolved name) rather than the right one (a
    /// dynamically-defined ensemble the analyser can't see the
    /// subcommand map of).
    ///
    /// `namespace ensemble create`'s options are registry data
    /// (`ENSEMBLE_CREATE_OPTIONS` in `tcl-registry`'s `namespace_`
    /// module), not a hardcoded name list — walking by each option's
    /// declared value arity (rather than a bare `opt == "-command"` scan
    /// over every word) is what keeps another option's *value* word
    /// (`-map`'s dict, `-subcommands`' list, …) from ever being misread as
    /// `-command`'s own flag or value.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceEnsemble`]
    /// (stamped on `namespace`'s `ensemble` subcommand); `args[0]` is
    /// still the subcommand word, and only the `create` form (checked
    /// on `args[1]`) mutates anything.
    pub fn handle_namespace_ensemble(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() < 2 {
            return;
        }
        // `create` operates on the *current* namespace, with no explicit name
        // argument; `configure NAME ...` (issue #1001 follow-up — the ensemble
        // side of the `namespace ensemble configure -map` gap, previously
        // silently ignored here entirely) names an *existing* ensemble
        // command explicitly, resolved like any other command reference.
        // Both share the same `-command`/`-map`/`-subcommands` option set.
        let is_create = args[1] == "create";
        let is_configure = args[1] == "configure";
        if !is_create && !is_configure {
            return;
        }
        let ns = self.namespace_from_scope_path(scope_path);
        let ns_prefix = ns.trim_start_matches(':').to_owned();

        let (opts, opt_tokens, configure_target) = if is_create {
            if !ns.is_empty() && ns != "::" {
                self.ensemble_namespaces.insert(ns.clone());
            }
            (&args[2..], arg_tokens.get(2..).unwrap_or(&[]), None)
        } else {
            let Some(name) = args.get(2) else { return };
            if crate::naming::is_dynamic_word(name) {
                return;
            }
            let target = self.resolve_command_qualified_name(name, scope_path);
            (
                args.get(3..).unwrap_or(&[]),
                arg_tokens.get(3..).unwrap_or(&[]),
                Some(target),
            )
        };

        // The ensemble's own resolved command name (issue #1001 follow-up):
        // `-command NAME`, when present, *replaces* the default naming
        // entirely (tclsh 8.6.14-verified: `namespace ensemble create
        // -command ::alt` creates only `::alt`, never the enclosing
        // namespace's own name too) — so scan for it before recording any
        // `-map`, rather than defaulting eagerly the way `ensemble_namespaces`
        // (a broader, over-inclusive "valid command name" recovery set) does.
        let explicit_command = is_create
            .then(|| {
                opts.iter()
                    .enumerate()
                    .find_map(|(i, o)| (o == "-command").then(|| opts.get(i + 1)).flatten())
            })
            .flatten()
            .filter(|v| !v.is_empty() && !crate::naming::is_dynamic_word(v))
            .map(|v| qualify(&ns_prefix, v));
        let ensemble_key = configure_target
            .or(explicit_command)
            .or_else(|| (is_create && !ns.is_empty() && ns != "::").then(|| ns.clone()));

        let option_specs: Vec<&tcl_registry::hover::OptionSpec> = self
            .registry
            .and_then(|r| r.get("namespace"))
            .and_then(|spec| spec.subcommand("ensemble").map(|sub| (spec, sub)))
            .map(|(spec, sub)| {
                use tcl_registry::ProfileQueries;
                self.profile.available_sub_option_specs(spec, sub)
            })
            .unwrap_or_default();

        let mut i = 0usize;
        while i < opts.len() {
            let Some(spec) = option_specs.iter().find(|o| o.matches(opts[i].as_str())) else {
                i += 1;
                continue;
            };
            let value = opts.get(i + 1);
            let value_tok = opt_tokens.get(i + 1).copied();
            match spec.name {
                // `-command NAME` names the ensemble command — its namespace
                // is recorded so `<ns> sub` calls resolve.  A dynamic value
                // (`$var` / `[cmd]`) can't be resolved statically.
                "-command" => {
                    if let Some(value) = value
                        && !value.is_empty()
                        && !value.starts_with('$')
                        && !value.starts_with('[')
                    {
                        self.ensemble_namespaces.insert(qualify(&ns_prefix, value));
                    }
                }
                // `-map {sub target sub target …}` — every *target* (an
                // odd-indexed element) is a command the ensemble dispatches to,
                // recorded so it is reached by references / definition / rename
                // — and, keyed by the ensemble's own resolved command name, so
                // the W129 safe-interpreter gate can resolve a call through
                // this redirect to the target (issue #1001 follow-up).
                "-map" => {
                    if let (Some(value), Some(tok)) = (value, value_tok) {
                        self.record_ensemble_map_targets(
                            value,
                            tok,
                            scope_path,
                            ensemble_key.as_deref(),
                        );
                    }
                }
                // `-subcommands {a b c}` — each subcommand `a` dispatches to
                // the command `<ns>::a` in the ensemble's namespace.
                "-subcommands" => {
                    if let (Some(value), Some(tok)) = (value, value_tok) {
                        self.record_ensemble_subcommands(value, tok, &ns_prefix);
                    }
                }
                _ => {}
            }
            i += 1 + spec.value_word_count(opts, i);
        }
    }

    /// The `(element, span)` pairs of a whitespace-separated list word, with
    /// each element's span located inside the token's content (`content_offset`
    /// skips the opening delimiter).  Shared by the ensemble `-map` /
    /// `-subcommands` extraction; a dynamic element is left for the caller to
    /// skip.
    fn list_word_elements(list_text: &str, tok: Token) -> Vec<(String, Span)> {
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let mut out = Vec::new();
        let mut search_start = 0usize;
        for elem in list_text.split_whitespace() {
            if let Some(rel) = list_text[search_start..].find(elem) {
                let idx = search_start + rel;
                let start = content_start + u32::try_from(idx).unwrap_or(0);
                let end = start + u32::try_from(elem.len()).unwrap_or(0);
                out.push((elem.to_owned(), Span::new(start, end)));
                search_start = idx + elem.len();
            }
        }
        out
    }

    /// Record every `-map` target (the odd elements of the `sub target …`
    /// list) as a command reference resolved in the caller's namespace, and
    /// — when `ensemble_key` is `Some` (the ensemble's own resolved
    /// qualified command name, computed by [`Self::handle_namespace_ensemble`])
    /// — also record each `sub -> target` pair into
    /// `self.ensemble_command_maps`, so the W129 safe-interpreter gate can
    /// resolve a call reaching a hidden command only through this ensemble's
    /// redirect (issue #1001 follow-up; `None` when the ensemble's own name
    /// couldn't be resolved statically, matching every other guard in this
    /// handler).
    ///
    /// The map stores the *raw written* target text (`"source"`), not
    /// `resolved` (the namespace-qualified form used for the reference
    /// below) — [`Self::check_ensemble_redirect_hiding`] hands it straight
    /// to [`Self::safe_interp_visibility_gate`], which — like the direct
    /// literal-head case and every other indirection path this fix adds —
    /// checks the bare written spelling against the registry, not a
    /// namespace-qualified path. This matters concretely: a `-map` target is
    /// namespace-qualified relative to the ensemble's own (possibly
    /// synthetic, interp-domain-rooted) home namespace by real Tcl's own
    /// rule (tclsh 8.6.14-verified: `-map {go source}` inside `namespace
    /// eval myns {…}` really dispatches `go` to `::myns::source`, not the
    /// global builtin, and raises its own unrelated `invalid command name
    /// ::myns::source` in every interpreter, safe or not, when no such proc
    /// exists) — using the qualified form here would make the check depend
    /// on the interp-domain namespace model lining up with the registry's
    /// flat, unqualified command-name keying, which it structurally can't.
    /// The raw-text check this uses instead only fires for a target that is
    /// unqualified or explicitly `::`-rooted, matching the shape a
    /// `SAFE_INTERP_HIDDEN` registry command's name actually takes; a
    /// locally shadowing `proc` by the same bare name, anywhere in the
    /// tracked safe interpreter body, still suppresses it via the *existing*
    /// `ctx.exposed` check inside `safe_interp_visibility_gate` (populated
    /// by `mark_locally_defined_in_enclosing_interp`, which is namespace-
    /// blind in exactly the same way already, for the direct-call case).
    fn record_ensemble_map_targets(
        &mut self,
        list_text: &str,
        tok: Token,
        scope_path: &[usize],
        ensemble_key: Option<&str>,
    ) {
        for pair in Self::list_word_elements(list_text, tok).chunks(2) {
            let [(sub, _), (target, span)] = pair else {
                continue;
            };
            if crate::naming::is_dynamic_word(target) {
                continue;
            }
            if let Some(key) = ensemble_key {
                self.ensemble_command_maps
                    .entry(key.to_string())
                    .or_default()
                    .insert(sub.clone(), target.clone());
            }
            let resolved = self.resolve_command_qualified_name(target, scope_path);
            self.push_command_reference(target.clone(), *span, resolved, None);
        }
    }

    /// Record each `-subcommands` name as a reference to the command
    /// `<ns>::<name>` the ensemble maps it to.
    fn record_ensemble_subcommands(&mut self, list_text: &str, tok: Token, ns_prefix: &str) {
        for (elem, span) in Self::list_word_elements(list_text, tok) {
            if crate::naming::is_dynamic_word(&elem) {
                continue;
            }
            let resolved = qualify(ns_prefix, &elem);
            self.push_command_reference(elem, span, resolved, None);
        }
    }

    /// Define a list of variables from a varList token (e.g. the
    /// loop-variable list of `foreach`).
    ///
    /// Each name gets its **own** definition span, located inside the
    /// varList token's content (the token's `content_offset` skips a
    /// leading `{`/`"`). This — crucially — gives same-token bindings
    /// like `foreach {b a}` / `dict for {k v}` distinct,
    /// declaration-ordered spans, so downstream offset-sorted consumers
    /// (the `symbols` / `symbolgraph` CLI verbs) stay deterministic and
    /// source-ordered. A name not found in the content text (escapes,
    /// line continuations) falls back to the whole-token span.
    fn define_vars_from_list(&mut self, var_list_text: &str, tok: Token, scope_path: &[usize]) {
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let mut search_start = 0usize;
        for name in var_list_text.split_whitespace() {
            if let Some(rel) = var_list_text[search_start..].find(name) {
                let idx = search_start + rel;
                let start = content_start + u32::try_from(idx).unwrap_or(0);
                let end = start + u32::try_from(name.len()).unwrap_or(0);
                self.define_var(name, tok, scope_path, true, Some(Span::new(start, end)));
                search_start = idx + name.len();
            } else {
                self.define_var(name, tok, scope_path, true, None);
            }
        }
    }

    /// Handle `foreach var list body` (and the `foreach_in_collection`
    /// dialect variant, whose spec carries the same hook).
    ///
    /// Defines the loop-variable list in the active scope, then
    /// recurses into the body so vars defined inside the loop land
    /// in the enclosing scope.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Foreach`].
    pub fn handle_foreach_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 {
            return false;
        }
        if let Some(tok) = arg_tokens.first() {
            self.define_vars_from_list(&args[0], *tok, scope_path);
        }
        // The body is always the last argument; recurse so vars
        // defined inside the loop land in the enclosing scope.
        if let (Some(body_text), Some(body_tok)) = (args.last(), arg_tokens.last().copied()) {
            self.analyse_body(body_text, body_tok, scope_path);
        }
        true
    }

    /// Handle `for init test next body`.
    ///
    /// Recurses into init / next / body so locals defined inside any
    /// of the three statement positions land in the enclosing scope's
    /// variable set.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::For`].
    pub fn handle_for_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 4 {
            return false;
        }
        // init body
        if let Some(tok) = arg_tokens.first().copied() {
            self.analyse_body(&args[0], tok, scope_path);
        }
        // next body
        if let Some(tok) = arg_tokens.get(2).copied() {
            self.analyse_body(&args[2], tok, scope_path);
        }
        // main body
        if let Some(tok) = arg_tokens.get(3).copied() {
            self.analyse_body(&args[3], tok, scope_path);
        }
        true
    }

    /// Handle `switch ?options? string ?pattern body? ...`.
    ///
    /// Arity checking lives in `compiler_checks::arity_checks` via
    /// the IR; this handler walks each arm body so locals defined
    /// inside an arm land in the enclosing scope.
    ///
    /// Switch has two forms:
    ///
    /// 1. ``switch ?options? string pattern body ?pattern body? ...``
    ///    — pattern and body args alternate inline.
    /// 2. ``switch ?options? string {pattern body ?pattern body? ...}``
    ///    — pattern/body pairs live inside a single braced
    ///    block.  See [`crate::segmenter::flatten_clause_list_elements`]
    ///    for how that braced form is split.
    ///
    /// Bodies that are literally ``-`` are fall-through markers
    /// (the next arm's body fires) and are skipped — recursing
    /// into the literal ``-`` would produce a useless command.
    ///
    /// `-regexp` pattern recording: each non-`default`
    /// pattern arm is recorded as a `RegexPattern` with
    /// ``command = "switch"`` when its token is a literal
    /// (`Esc` / `Str`) or a `Var` token whose name resolves via
    /// `lookup_const_string_with_span` to a constant string set
    /// earlier in the same scope.  Command-substitution patterns
    /// are skipped (runtime-computed values can't be statically
    /// resolved).
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Switch`].
    pub fn handle_switch_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 2 {
            return false;
        }

        // Scan and skip option flags. ``--`` ends the option
        // section explicitly (and is consumed).
        let mut i = 0;
        let mut is_regexp = false;
        while i < args.len() && args[i].starts_with('-') {
            if args[i] == "-regexp" {
                is_regexp = true;
            }
            if args[i] == "--" {
                i += 1;
                break;
            }
            i += 1;
        }
        // Skip the ``string`` argument that follows the options.
        i += 1;

        if i >= args.len() {
            return true;
        }

        if i == args.len() - 1 {
            // Form 2 — single braced body containing all pairs.
            let body_text = args[i].clone();
            let Some(body_tok) = arg_tokens.get(i).copied() else {
                return true;
            };
            let elements = crate::segmenter::flatten_clause_list_elements(&body_text, body_tok);
            let mut j = 0;
            while j + 1 < elements.len() {
                let (pat_text, pat_tok) = &elements[j];
                let (body_text, body_tok) = &elements[j + 1];
                if is_regexp {
                    self.record_switch_regexp_pattern(pat_text, *pat_tok, scope_path);
                }
                if body_text != "-" {
                    self.analyse_body(body_text, *body_tok, scope_path);
                }
                j += 2;
            }
        } else {
            // Form 1 — pattern/body pairs inline in args/arg_tokens.
            while i + 1 < args.len() {
                if is_regexp && let Some(pat_tok) = arg_tokens.get(i).copied() {
                    self.record_switch_regexp_pattern(&args[i], pat_tok, scope_path);
                }
                let body_text = &args[i + 1];
                if let Some(body_tok) = arg_tokens.get(i + 1).copied()
                    && body_text != "-"
                {
                    self.analyse_body(body_text, body_tok, scope_path);
                }
                i += 2;
            }
        }
        true
    }

    /// Record one ``switch -regexp`` arm's pattern.  Skipped for
    /// the ``default`` keyword (Tcl's catch-all).  Literal tokens
    /// are recorded verbatim; `Var` tokens are resolved via the
    /// `const_strings` map (the ``regex-vars`` propagation);
    /// `Cmd` substitutions are skipped (can't statically
    /// resolve).
    fn record_switch_regexp_pattern(&mut self, pattern: &str, tok: Token, scope_path: &[usize]) {
        if pattern == "default" {
            return;
        }
        self.record_regex_pattern_token(pattern, tok, "switch", scope_path);
    }

    /// Handle `catch SCRIPT ?RESULTVAR? ?OPTIONSVAR?`.
    ///
    /// Defines the optional `RESULTVAR` and `OPTIONSVAR` bindings
    /// (they receive values when the body throws / completes) and
    /// bumps `conditional_depth` for the duration of the body.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Catch`].
    pub fn handle_catch_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        // The script body (args[0]) is evaluated by `catch`, so walk it like
        // every other body-bearing command — otherwise the per-command
        // syntactic checks (W100 unbraced `expr`, W104, W304, …) never reach
        // inside a `catch { … }`, under-reporting. `analyse_body` no-ops on a
        // dynamic body (`catch $cmd`).
        //
        // The body is a *guarded probe*: `catch { package require Foo }` is the
        // idiomatic optional-dependency check, so facts recorded inside it
        // (package requirements, …) must be marked conditional. Bump
        // `conditional_depth` for the walk, matching `if`/`try` bodies (and this
        // handler's own contract — see the doc comment above).
        if let Some(body_tok) = arg_tokens.first().copied() {
            self.conditional_depth += 1;
            self.analyse_body(&args[0], body_tok, scope_path);
            self.conditional_depth -= 1;
        }
        // The result-var / options-var positions come from the registry's
        // `ArgRole::VarWrite` rows on the `catch` spec, matching the nested
        // `[catch …]` path in `dispatch_nested_segment`. The literal spec
        // name is sound here: `AnalyserHookId::Catch` dispatch already
        // resolved the head (qualified spellings included) to this spec.
        if let Some(registry) = self.registry {
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            for i in registry.arg_indices_for_role(
                "catch",
                &arg_strs,
                tcl_registry::arg_role::ArgRole::VarWrite,
            ) {
                if let (Some(name), Some(tok)) = (args.get(i), arg_tokens.get(i)) {
                    let name = name.clone();
                    self.define_var(&name, *tok, scope_path, false, None);
                }
            }
        }
        true
    }

    /// Bind the output variables of any command that writes results into
    /// named arguments — `lassign`, `scan`, `regexp`, `regsub`, `gets`,
    /// `binary scan`, `vwait`, `chan gets`, `file stat`, `dict set`,
    /// `info default`, … — every command whose registry spec marks a
    /// `VarWrite`-role argument (an empty role set no-ops).
    ///
    /// The registry's `VarWrite` arg-role resolver already encodes each
    /// command's option/positional shape (leading `-switches`, the `--`
    /// terminator, the pattern / format / subSpec positionals, and the
    /// variadic capture tail), so this reuses it rather than re-deriving the
    /// layout per command.  Binding these makes the destructured / captured
    /// names visible to completion, hover, and go-to-definition in the
    /// enclosing scope.  `warn_if_unused = false`: like `catch`'s result
    /// variable, the binding is a documented side effect, not a "set but never
    /// read" target, so it must not raise W211.
    ///
    /// Two trait families opt out:
    ///
    /// - [`tcl_registry::Traits::CREATES_SCOPE_ALIAS`] (`global` /
    ///   `variable` / `upvar`): their dedicated handlers
    ///   ([`Self::handle_var_declaration_command`] /
    ///   [`Self::handle_upvar_command`]) own the alias layout —
    ///   tail-stripping (`global ::ns::v` binds the local alias `v`, not
    ///   the qualified name) and name/value pairing — which a flat
    ///   `VarWrite` walk would get wrong.
    /// - [`tcl_registry::Traits::DESTROYS_VARIABLE`] (`unset`): its
    ///   `VarWrite` role marks a *removal* target (an SSA def that kills
    ///   the value), not a binding to record.
    ///
    /// Commands whose dedicated handlers already bind the same name
    /// (`set` / `incr` / `append` / `lappend`, the `dict` subforms) stay
    /// in: those handlers run first and [`Self::define_var`]'s
    /// re-definition path is idempotent for the same token span — no
    /// duplicate reference is pushed and this call's
    /// `warn_if_unused = false` never downgrades an earlier `true`.
    ///
    /// Void-returning (self-guards on the role set) so it composes with the
    /// other side-effect handlers — `regexp` / `regsub` also feed
    /// `handle_regex_pattern_capture`, which must still run.
    pub fn handle_var_binding_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let Some(registry) = self.registry else {
            return;
        };
        if registry.get(cmd_name).is_none_or(|spec| {
            spec.traits.intersects(
                tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::DESTROYS_VARIABLE,
            )
        }) {
            return;
        }
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let indices = registry.arg_indices_for_role(
            cmd_name,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::VarWrite,
        );
        for idx in indices {
            let (Some(name), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                continue;
            };
            // Only a plain literal names a definite scope variable.  A computed
            // target (`scan $s $fmt $dyn`), an array element (`arr(i)`), or a
            // brace/bracket-bearing word is not a simple local to bind.
            if name.is_empty() || name.contains(['$', '[', ']', '(', ')', '{', '}', ' ']) {
                continue;
            }
            self.define_var(name, *tok, scope_path, false, None);
        }
    }

    /// Handle `try BODY ?on/trap CODE VARLIST BODY?... ?finally BODY?`.
    ///
    /// Walks the main try body and every handler / finally clause;
    /// arity checking lives in `compiler_checks::arity_checks`
    /// already.
    ///
    /// Clause shapes:
    ///
    /// - ``finally BODY`` (2 words) — recurse into ``BODY``.
    /// - ``on CODE VARLIST BODY`` / ``trap PATTERN VARLIST BODY``
    ///   (4 words) — define the handler's ``VARLIST`` (e.g.
    ///   ``{result options}``), then recurse into ``BODY``.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Try`].
    pub fn handle_try_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        // Main try body at args[0].
        if let Some(body_tok) = arg_tokens.first().copied() {
            self.analyse_body(&args[0], body_tok, scope_path);
        }
        // Walk handler / finally clauses.
        let mut i = 1;
        while i < args.len() {
            let kw = args[i].as_str();
            if kw == "finally" && i + 1 < args.len() {
                if let Some(body_tok) = arg_tokens.get(i + 1).copied() {
                    self.analyse_body(&args[i + 1], body_tok, scope_path);
                }
                i += 2;
            } else if matches!(kw, "on" | "trap") && i + 3 < args.len() {
                // `on CODE {msg opts} body` / `trap PAT {msg opts} body` — the
                // var-list at i+2 binds the result message + options dict in
                // the handler body, so define them before walking it.
                if let Some(vl_tok) = arg_tokens.get(i + 2).copied() {
                    self.define_vars_from_list(&args[i + 2], vl_tok, scope_path);
                }
                // A handler body of literal `-` is a fallthrough marker (shares
                // the next handler's body, like `switch`); it is not a script,
                // so it must not be re-lexed as one — otherwise the solo `-`
                // reads as a zero-arg `-` command and trips a spurious arity
                // error (issue #703). Mirrors the `switch` arm handling above.
                if let Some(body_tok) = arg_tokens.get(i + 3).copied()
                    && args[i + 3] != "-"
                {
                    self.analyse_body(&args[i + 3], body_tok, scope_path);
                }
                i += 4;
            } else {
                i += 1;
            }
        }
        true
    }

    /// Register the local-alias names introduced by `upvar`.
    ///
    /// `upvar ?level? otherVar myVar ?otherVar myVar ...?` — an optional
    /// leading level makes the arg count odd; each pair after it binds the
    /// local alias `myVar`.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Upvar`].
    pub fn handle_upvar_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let pair_start = usize::from(args.len() % 2 == 1);
        let mut i = pair_start + 1;
        while i < args.len() && i < arg_tokens.len() {
            // The local alias name may be dynamic (`upvar 1 x $local`) — skip
            // it rather than record the substitution text.
            if !crate::naming::is_dynamic_word(&args[i]) {
                self.define_var(&args[i], arg_tokens[i], scope_path, false, None);
            }
            i += 2;
        }
    }

    /// Register the local aliases introduced by `namespace upvar`.
    ///
    /// `namespace upvar nsname otherVar myVar ?otherVar myVar ...?` — `myVar`
    /// lives at indices 3, 5, 7, …
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceUpvar`] (stamped
    /// on `namespace`'s `upvar` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_upvar_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() < 4 {
            return;
        }
        // `namespace upvar nsname otherVar myVar ?otherVar myVar ...?`: `myVar`
        // (indices 3, 5, …) aliases `nsname::otherVar` (`otherVar` at 2, 4, …).
        // Resolve a relative `nsname` against the current namespace.
        let cur = self.command_resolution_namespace(scope_path);
        let cur_prefix = cur.trim_end_matches("::");
        let target_ns = if args[1].starts_with("::") {
            args[1].trim_end_matches("::").to_string()
        } else {
            format!("{cur_prefix}::{}", args[1].trim_end_matches("::"))
        };
        let mut i = 3;
        while i < args.len() && i < arg_tokens.len() {
            // Skip a dynamic local alias name (`namespace upvar ns x $local`).
            if !crate::naming::is_dynamic_word(&args[i]) {
                self.define_var(&args[i], arg_tokens[i], scope_path, false, None);
                // `otherVar` is resolved *within* the target namespace, so keep
                // its full path: `namespace upvar ::a b::c local` aliases `local`
                // to `::a::b::c`, not the tail-collapsed `::a::c`.  An absolute
                // `otherVar` names its cell directly.
                let other = &args[i - 1];
                let target = if other.starts_with("::") {
                    other.clone()
                } else {
                    format!("{target_ns}::{other}")
                };
                self.set_var_link_target(&args[i], scope_path, target);
            }
            i += 2;
        }
    }

    /// Register the loop variables of `dict for {keyVar valueVar}
    /// dictValue body`.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::DictFor`]
    /// (stamped on `dict`'s `for` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_dict_for_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() >= 4 && arg_tokens.len() >= 2 {
            self.define_vars_from_list(&args[1], arg_tokens[1], scope_path);
        }
    }

    /// Register the alias variables of `dict update dictVar key1 var1
    /// ?key2 var2 ...? body` — vars at 3, 5, 7, … (i.e. `len-2` last).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::DictUpdate`] (stamped on
    /// `dict`'s `update` subcommand).
    pub fn handle_dict_update_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() < 5 || !(args.len() - 3).is_multiple_of(2) {
            return;
        }
        let mut i = 3;
        while i + 1 < args.len() {
            if let Some(tok) = arg_tokens.get(i) {
                self.define_var(&args[i], *tok, scope_path, false, None);
            }
            i += 2;
        }
    }

    /// Register the key variables of `dict with dictVar body` — only
    /// the no-path case is statically resolvable, and only when the
    /// dict came from a const literal.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::DictWith`]
    /// (stamped on `dict`'s `with` subcommand).
    pub fn handle_dict_with_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() != 3 || arg_tokens.len() < 2 {
            return;
        }
        let Some(const_val) = self
            .lookup_const_string(&args[1], scope_path)
            .map(str::to_owned)
        else {
            return;
        };
        let elements = crate::tcl_expr_eval::split_tcl_list(&const_val);
        let mut i = 0;
        while i < elements.len() {
            if !elements[i].is_empty() {
                self.define_var(&elements[i], arg_tokens[1], scope_path, false, None);
            }
            i += 2;
        }
    }

    /// Handle `interp alias {} ALIAS {} TARGET ?ARG ...?` —
    /// records the alias for later argument-role resolution.
    ///
    /// Delegates the actual detection logic to
    /// `crate::alias::detect_interp_alias` (which already handles
    /// the canonical `interp alias {}` shape and the `args[5..]`
    /// prepended-args slice). `offset` is the command token's start,
    /// recorded in [`Analyser::alias_offsets`] for the same-file arity
    /// resolver's top-level order gate.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::InterpAlias`] (stamped on
    /// `interp`'s `alias` subcommand); `args[0]` is still the
    /// subcommand word the detectors expect.
    pub fn handle_interp_alias(&mut self, args: &[String], offset: u32) {
        if let Some(deleted) = crate::alias::detect_interp_alias_delete(args) {
            self.deleted_commands.insert(deleted, offset);
            return;
        }
        if let Some((qualified, target_cmd, prepended)) = detect_interp_alias(args) {
            self.record_interp_alias(qualified, target_cmd, prepended, offset);
            return;
        }
        // Cross-domain alias (issue #945 fault 8): `interp alias PATH name
        // TPATH target ?arg…?` deliberately crosses interpreter domains —
        // the alias `name` becomes callable in the *source* interpreter,
        // running `target` resolved in the *target* interpreter.  With
        // literal paths both sides home under their `@interp@` domains, so
        // calls of the alias inside the child's eval bodies resolve to the
        // target through the ordinary alias link machinery.
        if args.len() >= 5 {
            let (src_path, alias_name, target_path, target_cmd) =
                (&args[1], &args[2], &args[3], &args[4]);
            let literal = |w: &String| !crate::naming::is_dynamic_word(w);
            let plain_path = |w: &String| matches!(w.as_str(), "" | "{}");
            if literal(alias_name)
                && literal(target_cmd)
                && literal(src_path)
                && literal(target_path)
                && !(plain_path(src_path) && plain_path(target_path))
            {
                let domain_prefix = |path: &String| {
                    if plain_path(path) {
                        String::new()
                    } else {
                        let key = self.qualified_interp_key(path);
                        format!("::{}", self.interp_domain_name(&key))
                    }
                };
                let qualified = format!(
                    "{}::{}",
                    domain_prefix(src_path),
                    alias_name.trim_start_matches(':')
                );
                let target = format!(
                    "{}::{}",
                    domain_prefix(target_path),
                    target_cmd.trim_start_matches(':')
                );
                let prepended: Vec<String> = args[5..].to_vec();
                self.record_interp_alias(qualified, target, prepended, offset);
            }
        }
    }

    /// Record one resolved `interp alias` fact into the three alias
    /// tables (shared by the current-interp and cross-domain forms).
    fn record_interp_alias(
        &mut self,
        qualified: String,
        target_cmd: String,
        prepended: Vec<String>,
        offset: u32,
    ) {
        self.command_aliases
            .insert(qualified.clone(), (target_cmd.clone(), prepended.clone()));
        self.alias_offsets.insert(qualified.clone(), offset);
        self.result.alias_offsets.insert(qualified.clone(), offset);
        self.result.command_aliases.insert(
            qualified.clone(),
            SignatureCommandAlias {
                qualified_name: qualified,
                target: target_cmd,
                extras: prepended,
            },
        );
    }

    /// Handle `rename OLD NEW` — record a static rename so calls to
    /// `NEW` resolve to whatever `OLD` denoted (the same proc, unchanged
    /// signature — a rename is a pure name move, never an arity change).
    /// Also records that `OLD` itself is no longer a callable command
    /// from this point on (confirmed against tclsh 9.0.4: `OLD` fails
    /// "invalid command name" afterwards, not a "wrong # args" against
    /// its original signature).
    ///
    /// Returns `true` when the rename is *dynamic* (`rename $x y` /
    /// `rename x [y]`, per [`crate::naming::is_dynamic_word`]) and so
    /// could not be resolved statically — the caller widens
    /// `has_dynamic_providers` in that case, the same wildcard-collapse
    /// convention `command_binding.rs`'s flow-sensitive lattice uses for
    /// the identical shape. A malformed `rename` (wrong argument count,
    /// already flagged by the registry arity check) is not treated as
    /// dynamic — there is no new binding to widen for. `offset` is the
    /// command token's start, recorded in [`Analyser::rename_offsets`] /
    /// [`Analyser::deleted_commands`] for the same-file arity resolver's
    /// top-level order gate. A deleting `rename OLD {}` records only
    /// `OLD`'s deletion (confirmed against tclsh 9.0.4: also "invalid
    /// command name" afterwards) — there is no `NEW` to map it to.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Rename`].
    pub fn handle_rename(&mut self, args: &[String], arg_tokens: &[Token], offset: u32) -> bool {
        if args.len() != 2 {
            return false;
        }
        if crate::naming::is_dynamic_word(&args[0]) || crate::naming::is_dynamic_word(&args[1]) {
            return true;
        }
        let old = crate::naming::normalise_qualified_name(&args[0]);
        let new = crate::naming::normalise_qualified_name(&args[1]);
        if old.is_empty() {
            return false;
        }
        // A rename moves whatever real Tcl object `OLD` currently denotes —
        // including a live interpreter's own handle command — to `NEW` (or,
        // for a deleting `rename OLD {}`, off the command table entirely).
        // Keep `self.interpreters` in step so a later, unrelated definition
        // that reuses `OLD`'s freed name is never misidentified as still
        // being that interpreter's handle (confirmed against tclsh 9.0.4:
        // after `rename sandbox {}` the child interpreter is gone —
        // `interp slaves` no longer lists it — and a later `proc sandbox
        // {sub body} {...}` dispatches to the new proc, never to the
        // interpreter; `rename sandbox t` instead keeps the same
        // interpreter reachable, now only as `t`). Bumping the epoch on
        // both the vacated key and any interpreter the rename overwrites at
        // `NEW` mirrors `handle_interp_delete_command`, so a later
        // `interp create` recreating either name never merges with the
        // interpreter that used to be tracked there (issue #945 fault 8).
        let old_interp_key = self.qualified_interp_key(&args[0]);
        if let Some(state) = self.interpreters.remove(&old_interp_key) {
            *self.interp_epochs.entry(old_interp_key).or_insert(0) += 1;
            if !new.is_empty() {
                let new_interp_key = self.qualified_interp_key(&args[1]);
                if self.interpreters.remove(&new_interp_key).is_some() {
                    *self
                        .interp_epochs
                        .entry(new_interp_key.clone())
                        .or_insert(0) += 1;
                }
                self.interpreters.insert(new_interp_key, state);
            }
        }
        if new.is_empty() {
            self.deleted_commands.insert(old, offset);
            return false;
        }
        self.renamed_commands.insert(new.clone(), old.clone());
        self.rename_offsets.insert(new.clone(), offset);
        self.deleted_commands.insert(old.clone(), offset);
        // `OLD` (`args[0]`, token `arg_tokens[0]`) names the command being
        // moved — a reference to it that rename rewrites.
        if let Some(tok) = arg_tokens.first() {
            self.result
                .rename_target_spans
                .insert(new.clone(), tok.span);
        }
        self.result.renamed_commands.insert(new, old);
        false
    }

    /// Handle `oo::objdefine $obj …` — record the object variable
    /// (so W308 can suppress unknown-method false positives from
    /// per-instance extensions) **and** walk the per-object
    /// definition so its method bodies are analysed exactly like an
    /// `oo::define` class's.
    ///
    /// `oo::objdefine` shares the `oo::define` member grammar, so the
    /// body / inline forms are parsed with the same helpers.  The
    /// members are collected into a *throwaway* `ClassDef` whose only
    /// purpose is to drive [`Self::parse_oo_definition_body`]'s
    /// method-body walk (variable / command resolution and in-body
    /// diagnostics light up as a side effect).  The `ClassDef` is
    /// deliberately **not** registered in `all_classes`: a per-object
    /// extension is not a class and must never surface in class
    /// listings, hover, rename, or completion.  Its method bodies
    /// home under a private synthetic name so the duplicate detector
    /// never confuses them with the object's real class methods.
    ///
    /// Returns `true` when it owns the command's body walk (an object
    /// name is present), mirroring [`Self::handle_oo_define_command`],
    /// so the generic body recursion does not also descend the body.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::OoObjdefine`].
    pub fn handle_oo_objdefine(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let mut obj_name = args[0].trim().to_string();
        if let Some(stripped) = obj_name.strip_prefix('$') {
            obj_name = stripped.trim_matches(|c| c == '{' || c == '}').to_string();
        }
        if !obj_name.is_empty() {
            self.objdefined_vars.insert(obj_name.clone());
        }

        // `oo::objdefine $obj` with no definition script — the object variable
        // is recorded above; there is nothing more to walk.
        if args.len() < 2 {
            return true;
        }

        let cmd_name = "oo::objdefine";
        // A throwaway holder for the walked members.  The method bodies home
        // under a private synthetic name (`@objdefine@…`, unrepresentable in
        // real Tcl) so a per-object `greet` never collides with the same-named
        // class method in the duplicate detector or the scope-name key.
        let synthetic = if obj_name.is_empty() {
            "::@objdefine@".to_string()
        } else {
            format!("::@objdefine@::{obj_name}")
        };
        let mut object_class = super::types::ClassDef {
            name: obj_name.clone(),
            qualified_name: synthetic,
            ..Default::default()
        };

        // Body-form vs inline-form is the same registry-grammar test as
        // `oo::define`: `args[1]` being a known member keyword means the
        // inline form (`oo::objdefine $o method m {} {}`).
        let inline_form = self
            .definition_grammar(cmd_name)
            .is_some_and(|g| g.is_member(&args[1]));

        if inline_form {
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            if let Some(grammar) = self.definition_grammar(cmd_name) {
                super::oo::parse_oo_define_inline(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    &mut object_class,
                );
                // The inline form names commands too (`oo::objdefine $o mixin
                // M`); record them the same way the body walk does so
                // references / rename reach the class.
                self.record_member_command_references(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    scope_path,
                );
            }
        } else if let Some(body_tok) = arg_tokens.get(1).copied() {
            let grammar = self.definition_grammar(cmd_name);
            let definer_disabled = self.command_dialect_disabled(cmd_name);
            self.parse_oo_definition_body(
                &args[1],
                body_tok,
                &mut object_class,
                scope_path,
                grammar,
                definer_disabled,
            );
        }

        // Record the per-object method declarations so `$obj m` navigation
        // resolves the per-object override ahead of a same-named class method.
        // Accumulate across multiple `oo::objdefine` blocks on the same object.
        // Each record carries the objdefine site's receiver offset, so
        // consumers key it by the receiver's *binding identity* — never by
        // the textual tail alone (issue #945 fault 5).
        if !obj_name.is_empty() && !object_class.methods.is_empty() {
            let objdefine_offset = arg_tokens.first().map_or(0, |t| t.span.start());
            self.result
                .object_methods
                .entry(obj_name)
                .or_default()
                .extend(object_class.methods.into_values().map(|def| {
                    super::types::ObjectMethodDef {
                        def,
                        objdefine_offset,
                    }
                }));
        }

        true
    }

    /// Handle ``package require`` (and ``package provide``) —
    /// record the package dependency so later passes can gate
    /// W123 (unresolved-command) suppression and dynamic-
    /// provider detection.
    ///
    /// Two shapes are recognised:
    ///
    /// - ``package require ?-exact? NAME ?VERSION?`` — appends a
    ///   ``SignaturePackageRequire`` record to
    ///   ``result.package_requires`` and flips
    ///   ``has_dynamic_providers`` when the name argument is a
    ///   ``$``-substitution / ``[…]``-substitution token.
    /// - ``package provide NAME ?VERSION?`` — consumed silently;
    ///   there's no field to record it on yet.
    ///
    /// The conditional flag is ``self.conditional_depth > 0``.
    ///
    /// `cmd_tok` is the command-head token (the ``package``
    /// word).  The recorded
    /// [`SignaturePackageRequire::range`](crate::signature_scan::types::SignaturePackageRequire::range)
    /// uses its span so code-action / quick-fix UX points at the
    /// ``package`` keyword rather than at the ``require``
    /// subcommand word.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageRequire`] (stamped
    /// on `package`'s `require` subcommand — other ``package``
    /// subcommands like ``ifneeded`` / ``forget`` / ``vsatisfies``
    /// carry no hook and aren't recorded); `args[0]` is still the
    /// subcommand word.
    pub fn handle_package_require(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if args.len() < 2 {
            return;
        }
        // ``package require -exact NAME ?VERSION?`` —
        // strip the flag and shift the name index.
        let (name_idx, name_text) = if args[1] == "-exact" && args.len() >= 3 {
            (2usize, args[2].clone())
        } else {
            (1usize, args[1].clone())
        };
        let version_idx = name_idx + 1;
        let version = if version_idx < args.len() {
            Some(args[version_idx].clone())
        } else {
            None
        };

        // Dynamic-provider detection — non-literal package
        // name suppresses W123 unknown-command emission
        // because the dynamic provider may register the
        // missing command at runtime.
        if let Some(name_tok) = arg_tokens.get(name_idx)
            && (matches!(name_tok.kind, TokenType::Var | TokenType::Cmd)
                || name_text.contains('$')
                || name_text.contains('['))
        {
            self.result.has_dynamic_providers = true;
        }

        self.result
            .package_requires
            .push(crate::signature_scan::types::SignaturePackageRequire {
                name: name_text,
                version,
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
            });
    }

    /// Record ``package provide NAME ?VERSION?``.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageProvide`] (stamped
    /// on `package`'s `provide` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_provide(&mut self, cmd_tok: Token, args: &[String]) {
        if args.len() < 2 {
            return;
        }
        let name = args[1].clone();
        let version = if args.len() >= 3 {
            Some(args[2].clone())
        } else {
            None
        };
        self.result
            .package_provides
            .push(super::types::PackageProvide {
                name,
                version,
                range: cmd_tok.span,
            });
    }

    /// Resolve a command alias to `(target_cmd, effective_args)`.
    ///
    /// Returns `(cmd_name, args)` unchanged if no alias matches;
    /// otherwise returns the target command and the prepended-args +
    /// original args list. Delegates to `crate::alias::resolve_alias` for the
    /// namespace-aware lookup.
    #[must_use]
    pub fn resolve_alias(
        &mut self,
        cmd_name: &str,
        args: &[String],
        scope_path: &[usize],
    ) -> (String, Vec<String>) {
        let ns = self.namespace_from_scope_path(scope_path);
        // `alias::resolve_alias` accepts `CommandAliasMap` (alias map
        // keyed by qualified alias name) — the same shape as
        // `self.command_aliases` already uses.
        if let Some((target_cmd, prepended)) = resolve_alias(cmd_name, &self.command_aliases, &ns) {
            let mut effective: Vec<String> = prepended;
            effective.extend(args.iter().cloned());
            (target_cmd, effective)
        } else {
            (cmd_name.to_string(), args.to_vec())
        }
    }

    /// The definition-body grammar attached to `cmd_name`'s spec, if it is a
    /// definer command.  Registry-sourced, so member recognition / argument
    /// structure in the body walkers never hardcodes a keyword list — a new
    /// definer is picked up the moment its spec carries a `definition_body`.
    pub(super) fn definition_grammar(
        &self,
        cmd_name: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        self.registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|s| s.definition_body)
    }

    /// Handle `oo::class create NAME ?BODY?` — record the class.
    ///
    /// Records a [`super::types::ClassDef`] in ``result.all_classes``
    /// (walking the body when present to populate method / mixin /
    /// superclass info) so consumers see the class in the workspace
    /// index. Returns `true` when the command shape matched.
    pub fn handle_oo_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        // Every metaclass name behaves the same shape — ``cmd_name
        // create Name ?body?`` — so the cmd-name guard widens to the full set
        // and the recorded ``metaclass`` field still distinguishes
        // them downstream (hover / outline / class-hierarchy).
        // A fully-qualified head (`::oo::class`) names the same global command
        // as its bare form, so strip a single leading `::` before matching (and
        // use the bare form for the recorded `metaclass`, so both spellings
        // produce an identical `ClassDef`).
        let cmd_name = cmd_name.strip_prefix("::").unwrap_or(cmd_name);
        // Which commands are TclOO class *creators* is registry data, not a
        // hardcoded name list: the `IS_OO_METACLASS` trait (marks the
        // `create`/`new`/`createWithNamespace` manufacturers) *and* a
        // `TclOo`-family definition-body grammar.  Both are required:
        //  - the trait alone includes `oo::object`, which has it but makes
        //    *instances*, not classes (no definition body);
        //  - the grammar alone includes `oo::define` / `oo::objdefine`, which
        //    *extend* an existing class named at `args[0]` — so `oo::define
        //    create method …` (a class literally named `create`) must not be
        //    mistaken for a creation and stolen from `handle_oo_define_command`.
        // define/objdefine carry no `IS_OO_METACLASS`, so they fall through.
        let is_class_definer = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .is_some_and(|s| {
                s.traits.contains(tcl_registry::Traits::IS_OO_METACLASS)
                    && s.definition_body
                        .is_some_and(|g| g.family == tcl_registry::definer::DefinerFamily::TclOo)
            });
        if !is_class_definer || args.len() < 2 || arg_tokens.len() < 2 {
            return false;
        }
        // `create Name ?body?`, `new ?body?`, and `createWithNamespace Name ns
        // ?body?` all introduce a class.
        if !matches!(args[0].as_str(), "create" | "new" | "createWithNamespace") {
            return false;
        }
        let raw_name = &args[1];
        // Home the class to the command-resolution namespace (see the same
        // reasoning in `handle_proc_command`): a class created inside a
        // qualified-name proc's body homes to that proc's defining namespace,
        // not the lexical global, so it can't overwrite a same-named global
        // class in `all_classes`.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[1].span;
        // **W314** — the class name has no absolute written form (#934).
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok_opt = arg_tokens.get(2).copied();
        let body_span = body_tok_opt.map_or(arg_tokens[1].span, |t| t.span);
        let doc = std::mem::take(&mut self.last_comment);
        let mut class = super::types::ClassDef {
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span,
            metaclass: cmd_name.to_string(),
            doc,
            ..Default::default()
        };
        // Walk the class body when present — populates
        // ``superclasses`` / ``mixins`` / ``methods`` /
        // ``class_methods`` from the OO-define subcommands.
        if let (Some(body_text), Some(body_tok)) = (args.get(2), body_tok_opt) {
            let grammar = self.definition_grammar(cmd_name);
            let definer_disabled = self.command_dialect_disabled(cmd_name);
            self.parse_oo_definition_body(
                body_text,
                body_tok,
                &mut class,
                scope_path,
                grammar,
                definer_disabled,
            );
        }
        // Register globally and in the current scope, the same as
        // the proc registration path: ``result.all_classes`` is keyed
        // by the fully-qualified name; the per-scope
        // ``scope.classes`` map is keyed by the bare (unqualified)
        // name so per-scope lookups and shadowing rules work.
        let simple_key = class.name.clone();
        self.result.all_classes.insert(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple_key, class);
        }
        true
    }

    /// Handle `oo::define CLASS ?BODY?` — record an extension to
    /// an existing class.
    ///
    /// Looks up the class by qualified name in
    /// ``result.all_classes``; when found, walks the body or
    /// inline-form arguments via the OO walkers in
    /// [`super::oo`] to extend ``superclasses`` / ``mixins`` /
    /// ``methods`` / ``class_methods``.  When the class isn't
    /// in the index yet (e.g. the class definition lives in a
    /// separate file the workspace index hasn't reached), a
    /// stub ``ClassDef`` is created so subsequent
    /// ``oo::define`` calls + the workspace index see a
    /// consistent record.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::OoDefine`]; `cmd_name` is
    /// the invocation's own head (always the stamped `oo::define`
    /// spelling), used only to look its definition grammar back up.
    pub fn handle_oo_define_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let raw_class_name = &args[0];
        let ns_prefix = self.namespace_from_scope_path(scope_path);
        let ns_for_qualify = ns_prefix.trim_start_matches(':');
        // A dynamic target (`oo::define $class method ...`, the clay.tcl
        // `current_class`-based Ensemble DSL idiom) can't be resolved to the
        // real class it extends — and, unlike a literal target, two
        // lexically unrelated occurrences that happen to write the *same*
        // variable name (an unremarkable choice for this exact idiom) must
        // never merge their `ClassDef` state into one via a shared raw-text
        // key: each occurrence gets its own synthetic, per-call-site key,
        // keyed by this argument token's own source offset (unique per
        // occurrence, deterministic, and unrepresentable as a real Tcl class
        // name, so it can never collide with a literal class of the same
        // written text). Mirrors `namespace eval`'s identical dynamic-target
        // handling a few hundred lines above.
        let dyn_key = crate::naming::is_dynamic_word(raw_class_name).then(|| {
            arg_tokens.first().map_or_else(
                || raw_class_name.clone(),
                |tok| format!("@dynclass@{}", tok.span.start()),
            )
        });
        let qualified = qualify(ns_for_qualify, dyn_key.as_deref().unwrap_or(raw_class_name));

        // Distinguish body-form from inline-form by inspecting
        // ``args[1]``.  Body-form: ``oo::define Class { ... }``
        // — args[1] is a single body argument.  Inline-form:
        // ``oo::define Class method foo {} {}`` — args[1] is a
        // known define subcommand.
        if args.len() < 2 {
            return true;
        }

        // The set of known define subcommands *is* the definer's member
        // grammar (registry data), so recognition can't drift from the source
        // of truth.  Anything not a grammar member falls through to body-form
        // handling (the segmenter does the re-parse).
        let inline_form = self
            .definition_grammar("oo::define")
            .is_some_and(|g| g.is_member(&args[1]));

        // Look up or create the partial ClassDef in
        // ``result.all_classes``. The ``name`` field carries the
        // bare tail even when the source declared the class
        // qualified (``oo::define ::ns::Other``), the same
        // ``simple`` extraction as ``handle_oo_class_command``.
        let simple = crate::naming::key_tail(&qualified).to_string();
        let mut class_def = self
            .result
            .all_classes
            .remove(&qualified)
            .unwrap_or_else(|| {
                let name_span = arg_tokens.first().map_or(
                    super::types::Scope::default()
                        .body_span
                        .unwrap_or_else(|| tcl_lexer::Span::new(0, 0)),
                    |t| t.span,
                );
                // The stub's body span must cover the *whole* `oo::define`
                // invocation, not just the class-name token, so any member
                // this call adds (inline `method`/`property`/… words, or a
                // `{ … }` body) stays nested inside it for document-symbol
                // rendering — a name-token-sized span left every
                // subsequently-added member "escaping" its own parent's
                // range, a self-contradictory, checkable-from-source-alone
                // structural error.
                let body_span = arg_tokens.last().map_or(name_span, |last| {
                    tcl_lexer::Span::new(name_span.start(), last.span.end())
                });
                super::types::ClassDef {
                    name: simple,
                    qualified_name: qualified.clone(),
                    name_span,
                    body_span,
                    // An `oo::define` on a class not created in this file — a
                    // cross-file extension stub, not the class's definition.
                    via_define: true,
                    ..Default::default()
                }
            });

        if inline_form {
            // ``oo::define Class subcmd ...`` — args[1..] is
            // the subcommand + its args.  (Registry-gated `inline_form` above
            // guarantees the grammar is present here.)
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            if let Some(grammar) = self.definition_grammar(cmd_name) {
                super::oo::parse_oo_define_inline(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    &mut class_def,
                );
                // Inline `oo::define Class superclass Base` (or `mixin`, or a
                // `forward` target) names commands too; record them like the
                // body walk so references / rename reach the referenced class.
                self.record_member_command_references(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    scope_path,
                );
            }
        } else if let Some(body_tok) = arg_tokens.get(1).copied() {
            // ``oo::define Class { body }`` — args[1] is the
            // body text, arg_tokens[1] is the body token.
            let grammar = self.definition_grammar(cmd_name);
            let definer_disabled = self.command_dialect_disabled(cmd_name);
            self.parse_oo_definition_body(
                &args[1],
                body_tok,
                &mut class_def,
                scope_path,
                grammar,
                definer_disabled,
            );
        }

        // Same dual-registration as ``oo::class create`` — keep
        // ``all_classes`` and the per-scope ``scope.classes`` in
        // sync.  ``oo::define`` may be redefining an existing
        // class; the ``insert`` overwrites the prior entry.
        let simple_key = class_def.name.clone();
        self.result.all_classes.insert(qualified, class_def.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple_key, class_def);
        }
        true
    }

    /// Record `source ?-encoding ENC? FILE` invocations.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Source`].
    pub fn handle_source_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        let file_idx = if args[0] == "-encoding" && args.len() >= 3 {
            2
        } else {
            0
        };
        if file_idx >= args.len() || file_idx >= arg_tokens.len() {
            return;
        }
        let st = arg_tokens[file_idx];
        let path = &args[file_idx];
        let is_lit = !matches!(st.kind, TokenType::Var | TokenType::Cmd)
            && !path.contains('$')
            && !path.contains('[');
        // `source` evaluates the file in the caller's current namespace (M9):
        // record the command-resolution namespace at this call site so the
        // workspace index can re-home the sourced document's definitions.
        let site_namespace = self.command_resolution_namespace(scope_path);
        self.result
            .source_targets
            .push(crate::signature_scan::types::SignatureSource {
                raw_path: path.clone(),
                range: st.span,
                is_literal: is_lit,
                site_namespace,
            });
    }

    /// Record `namespace import ?-force? PATTERN ...` declarations.
    ///
    /// Literal patterns are recorded in ``result.namespace_imports`` (the
    /// W123 unresolved-command pass suppresses an unqualified call whose
    /// name glob-matches a recorded pattern's tail); a *dynamic* pattern
    /// (``$``/``[…]`` substitution) can't be qualified statically, so it
    /// flips ``has_dynamic_providers`` instead — the imported namespace may
    /// provide any name at runtime, which suppresses W120/W123 file-wide.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceImport`] (stamped
    /// on `namespace`'s `import` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_import_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        // Skip the subcommand word + an optional ``-force`` flag.
        let mut idx = 1;
        if idx < args.len() && args[idx] == "-force" {
            idx += 1;
        }
        let importing_ns = self.namespace_from_scope_path(scope_path);
        while idx < args.len() && idx < arg_tokens.len() {
            let pat_raw = args[idx].clone();
            // Patterns containing ``$`` / ``[`` substitutions can't be
            // statically qualified — a runtime-computed import makes the
            // available command set unknowable.
            if pat_raw.contains('$') || pat_raw.contains('[') {
                self.result.has_dynamic_providers = true;
                idx += 1;
                continue;
            }
            // Patterns that already start with ``::`` are
            // absolute; relative patterns (``bar::*`` or
            // ``foo``) qualify against the *current* namespace
            // — inside ``namespace eval my { namespace import
            // bar::* }`` this becomes ``::my::bar::*``.
            let pat = if pat_raw.starts_with("::") {
                pat_raw
            } else if importing_ns == "::" {
                format!("::{pat_raw}")
            } else {
                format!("{importing_ns}::{pat_raw}")
            };
            self.result.namespace_imports.push(
                crate::signature_scan::types::SignatureNamespaceImport {
                    ns: importing_ns.clone(),
                    pattern: pat,
                    range: arg_tokens[idx].span,
                    conjectured: false,
                },
            );
            idx += 1;
        }
    }

    /// Handle `namespace unknown HANDLER` — installing a per-namespace
    /// unknown handler (TIP 181, `NamespaceUnknownCmd` in `tclNamesp.c`)
    /// makes command resolution unknowable, exactly like a dynamic user
    /// `proc unknown`: the handler runs for every failed lookup and may
    /// resolve anything.  Flips ``has_dynamic_providers``, which suppresses
    /// W120/W123 file-wide (matching the proc-unknown behaviour).
    ///
    /// Only the *setter* form suppresses: the bare query (`namespace
    /// unknown`) installs nothing, and an empty handler (`namespace unknown
    /// {}`) resets to the default lookup (`Tcl_SetNamespaceUnknownHandler`
    /// treats an empty list as a reset).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceUnknown`]
    /// (stamped on `namespace`'s `unknown` subcommand); `args[0]` is
    /// still the subcommand word.
    pub fn handle_namespace_unknown_command(&mut self, args: &[String]) {
        if args.len() < 2 {
            return;
        }
        if args[1].trim().is_empty() {
            return;
        }
        self.result.has_dynamic_providers = true;
    }

    /// Recognise the tcllib ``<NS>::import <ALIAS>`` wrapper idiom
    /// and record a conjectured `NamespaceImport`.
    ///
    /// A common tcllib convention is a proc ``some::ns::import`` that
    /// ``uplevel``s ``namespace eval <alias> {namespace import some::ns::*}``.
    /// Statically detecting the wrapper bodies is out of scope, but
    /// the call shape is unambiguous — its qualified name ends in
    /// ``::import`` and its single argument is a static namespace
    /// identifier.
    pub fn handle_tcllib_import_wrapper(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        args: &[String],
        scope_path: &[usize],
    ) {
        // Wrapper shape: ``X::import <alias>`` with exactly one
        // static argument.  Tcl's own ``namespace import`` is
        // handled separately and never falls into this branch
        // because ``cmd_name`` is ``namespace``, not ``…::import``.
        if !cmd_name.ends_with("::import") {
            return;
        }
        if args.len() != 1 {
            return;
        }
        let alias = &args[0];
        if alias.is_empty() || alias.contains('$') || alias.contains('[') {
            return;
        }
        // Strip the trailing ``::import`` to get the source
        // namespace; absolute-prefix it if missing the leading
        // ``::``.
        let stripped = &cmd_name[..cmd_name.len() - "::import".len()];
        let source_ns = if stripped.starts_with("::") {
            stripped.to_string()
        } else {
            format!("::{stripped}")
        };
        // Relative aliases live under the current namespace —
        // ``namespace eval outer { some::ns::import vt }``
        // creates ``::outer::vt``, not ``::vt``.
        let current_ns = self.namespace_from_scope_path(scope_path);
        let alias_ns = if alias.starts_with("::") {
            alias.clone()
        } else if current_ns == "::" {
            format!("::{alias}")
        } else {
            format!("{current_ns}::{alias}")
        };
        self.result.namespace_imports.push(
            crate::signature_scan::types::SignatureNamespaceImport {
                ns: alias_ns,
                pattern: format!("{source_ns}::*"),
                range: cmd_tok.span,
                conjectured: true,
            },
        );
    }

    /// Record a `lappend auto_path PATH...` mutation.
    ///
    /// Dispatched from the
    /// [`tcl_registry::hooks::AnalyserHookId::Lappend`] arm alongside
    /// the ordinary variable handling — the `auto_path` check is a
    /// *variable-name* shape check, not a command guard.  Any
    /// ``auto_path`` mutation flips ``has_dynamic_providers``:
    /// packages discovered at runtime can register commands the static
    /// analyser can't see, so W123 unknown-command diagnostics
    /// suppress on the document.
    pub fn handle_auto_path_lappend(&mut self, args: &[String], arg_tokens: &[Token]) {
        if args.first().map(String::as_str) != Some("auto_path") {
            return;
        }
        for (i, path) in args.iter().enumerate().skip(1) {
            let Some(tok) = arg_tokens.get(i) else {
                continue;
            };
            self.result
                .auto_path_entries
                .push(super::types::AutoPathEntry {
                    raw_path: path.clone(),
                    range: tok.span,
                });
        }
        self.result.has_dynamic_providers = true;
    }

    /// Record a `set auto_path PATH` mutation.
    ///
    /// Dispatched from the [`tcl_registry::hooks::AnalyserHookId::Set`]
    /// arm; see [`Self::handle_auto_path_lappend`] for why any
    /// ``auto_path`` mutation flips ``has_dynamic_providers``.
    pub fn handle_auto_path_set(&mut self, args: &[String], arg_tokens: &[Token]) {
        if args.first().map(String::as_str) != Some("auto_path") || args.len() < 2 {
            return;
        }
        if let Some(tok) = arg_tokens.get(1) {
            self.result
                .auto_path_entries
                .push(super::types::AutoPathEntry {
                    raw_path: args[1].clone(),
                    range: tok.span,
                });
            self.result.has_dynamic_providers = true;
        }
    }

    /// Record the pattern arguments of regex-pattern commands
    /// (`PatternType::Regex` specs — `regexp` / `regsub`) for syntax
    /// highlighting.
    ///
    /// Literal patterns (`Esc` / `Str` tokens) are recorded
    /// verbatim; `Var` tokens are resolved via the
    /// `const_strings` map (populated by `set var "..."`) so
    /// `regexp $p $line` records the literal stored in `p`.
    /// `Cmd`-substitution patterns are skipped — runtime-computed
    /// patterns can't be statically resolved.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::RegexPatternCapture`]
    /// (stamped on both `regexp` and `regsub`); `cmd_name` is data —
    /// it labels the recorded pattern — not a guard.
    pub fn handle_regex_pattern_capture(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        // Skip leading option flags (`-nocase`, `-line`, `-all`, `-indices`,
        // `-start INDEX`, …) to the pattern arg — the one canonical option-skip.
        let Some(idx) = crate::regex_source::regexp_pattern_index(args) else {
            return;
        };
        if idx >= arg_tokens.len() {
            return;
        }
        let tok = arg_tokens[idx];
        self.record_regex_pattern_token(&args[idx], tok, cmd_name, scope_path);
    }

    /// Record one regex-pattern token for `regexp` / `regsub` /
    /// `switch -regexp`.  Var tokens are resolved against
    /// `const_strings`; `Cmd` substitutions are skipped (no
    /// static resolution); literals are recorded verbatim.
    fn record_regex_pattern_token(
        &mut self,
        text: &str,
        tok: Token,
        command: &str,
        scope_path: &[usize],
    ) {
        match tok.kind {
            TokenType::Cmd => {
                // Runtime-computed patterns can't be statically
                // resolved — skip.  Only ``Var`` tokens are
                // resolved as a substitution branch.
            }
            TokenType::Var => {
                let sm = tcl_lexer::SourceMap::new(&self.source);
                let var_name = sm.token_text(tok).to_string();
                if let Some((const_val, def_span)) =
                    self.lookup_const_string_with_span(&var_name, scope_path)
                {
                    let pattern = const_val.to_string();
                    // Record the use site (the `$var` token).
                    self.result.regex_patterns.push(super::types::RegexPattern {
                        range: tok.span,
                        pattern: pattern.clone(),
                        command: command.to_string(),
                    });
                    // Also record the defining ``set`` value's
                    // range so the semantic-token provider can
                    // highlight the literal.
                    self.result.regex_patterns.push(super::types::RegexPattern {
                        range: def_span,
                        pattern,
                        command: command.to_string(),
                    });
                    self.regex_vars.insert((scope_path.to_vec(), var_name));
                }
            }
            _ => {
                self.result.regex_patterns.push(super::types::RegexPattern {
                    range: tok.span,
                    pattern: text.to_string(),
                    command: command.to_string(),
                });
            }
        }
    }

    /// Handle the `incr` command: `incr var ?amount?`.
    ///
    /// `incr` is safe-on-uninit (it initialises the variable to 0 if
    /// not yet set), so the var binding is created with
    /// `warn_if_unused = true` — the diagnostic emitter will
    /// still flag a `set`-only-no-read variable, but won't flag
    /// an `incr`-only-no-read one.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Incr`].
    pub fn handle_incr_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if let (Some(name), Some(tok)) = (args.first(), arg_tokens.first()) {
            self.define_var(name, *tok, scope_path, true, None);
        }
    }

    /// Handle `append VARNAME ?value ...?` / `lappend VARNAME ?value ...?`.
    ///
    /// Both read-modify-write their first argument, creating it if absent, so
    /// the target is a variable *definition* for symbol/scope purposes and must
    /// surface in `symbols` / completion / hover (it previously did not).
    /// `warn_if_unused = false` because the command itself reads the prior
    /// value, so an `append`/`lappend` target is never "set but never used"
    /// (no W211 for it).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Append`]
    /// and [`tcl_registry::hooks::AnalyserHookId::Lappend`] (the two
    /// commands share this handler; the `Lappend` arm additionally
    /// records `lappend auto_path …`).
    pub fn handle_append_lappend_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if let (Some(name), Some(tok)) = (args.first(), arg_tokens.first()) {
            self.define_var(name, *tok, scope_path, false, None);
        }
    }

    /// Resolve a command name to the `ProcDef` that implements it, following
    /// Tcl's real bareword command resolution via
    /// [`crate::naming::bareword_resolution_candidates`] — the current
    /// namespace (computed by [`Self::command_resolution_namespace`]) first,
    /// then global, exactly two levels, never every enclosing ancestor
    /// namespace on the scope chain (Tcl's own command lookup does not walk
    /// intermediate namespaces absent an explicit `namespace path`).
    ///
    /// Shared with the optimiser's identical same-file resolution
    /// (`resolve_proc_qname`) and the disabled-command / arity suppression
    /// checks' resolution (`UserResolutionFacts::resolves_to_user`) so the
    /// three can't diverge on the same rule.
    ///
    /// Returns the first matching ``ProcDef`` (by reference into
    /// ``result.all_procs``), or `None` if no candidate is known.
    #[must_use]
    pub fn resolve_proc_call(
        &self,
        cmd_name: &str,
        scope_path: &[usize],
    ) -> Option<&super::types::ProcDef> {
        if cmd_name.is_empty() {
            return None;
        }
        let ns = self.command_resolution_namespace(scope_path);
        crate::naming::bareword_resolution_candidates(&ns, cmd_name)
            .into_iter()
            .find_map(|qname| self.result.all_procs.get(&qname))
    }

    /// Static element count for a `{*}`-expanded word.
    ///
    /// Used by the proc-call arity checker (which lives in
    /// `compiler_checks::arity_checks`) to decide whether a
    /// ``{*}``-expanded argument contributes a statically-known
    /// number of runtime arguments.
    ///
    /// - **Braced literal** (`Str` token, ``{a b c}``) — split
    ///   the token's inner text as a list and return its length.
    /// - **Pure variable reference** (`Var` token, ``$x``) — if
    ///   the variable has a known constant value in the current
    ///   scope chain, split that value and return its length.
    /// - **Anything else** — `None`: count not statically known.
    ///
    /// Refinement is only attempted when ``single_token`` is
    /// `true`; for concatenated words like ``{*}$x$y`` or
    /// ``{*}{a b}$suffix`` the segmenter exposes only the *first*
    /// token, which would otherwise be misinterpreted as a pure
    /// literal or pure var ref.  Token text is resolved via
    /// [`tcl_lexer::SourceMap::token_text`] — the same helper the
    /// rest of the analyser uses — so the inner-content stripping
    /// rules (kind-specific delimiter handling) stay in one
    /// place.
    #[must_use]
    pub fn resolve_expansion_count(
        &self,
        tok: Token,
        single_token: bool,
        scope_path: &[usize],
    ) -> Option<usize> {
        use tcl_lexer::SourceMap;
        if !single_token {
            return None;
        }
        let sm = SourceMap::new(&self.source);
        match tok.kind {
            TokenType::Str => {
                let inner = sm.token_text(tok);
                Some(crate::codegen::helpers::split_list_simple(inner).len())
            }
            TokenType::Var => {
                let var_name = sm.token_text(tok);
                let const_val = self.lookup_const_string(var_name, scope_path)?;
                Some(crate::codegen::helpers::split_list_simple(const_val).len())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;

    fn esc_tok(span: Span) -> Token {
        Token::new(TokenType::Esc, span)
    }

    fn str_tok(span: Span) -> Token {
        Token {
            kind: TokenType::Str,
            span,
            content_offset: 1,
            in_quote: false,
        }
    }

    fn span(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    // handle_set_command

    #[test]
    fn handle_set_defines_variable() {
        let mut a = Analyser::new();
        a.handle_set_command(
            &["x".to_string(), "1".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 3))],
            &[true, true],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn handle_set_tracks_single_token_literal_value() {
        let mut a = Analyser::new();
        a.handle_set_command(
            &["x".to_string(), "hello".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 7))],
            &[true, true],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello"));
    }

    #[test]
    fn handle_set_tracks_braced_string_value() {
        let mut a = Analyser::new();
        a.handle_set_command(
            &["x".to_string(), "hello world".to_string()],
            &[esc_tok(span(0, 1)), str_tok(span(2, 15))],
            &[true, true],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello world"));
    }

    #[test]
    fn handle_set_clears_const_string_for_interpolated_value() {
        let mut a = Analyser::new();
        // Pre-seed a constant tracking entry.
        a.set_const_string("x", "old".to_string(), span(0, 0), &[]);
        // Re-assign with a multi-token (interpolation) value —
        // single_token_word[1] is false, so const_string is cleared.
        a.handle_set_command(
            &["x".to_string(), "$other".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 8))],
            &[true, false],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn handle_set_no_value_records_read_not_definition() {
        // ``set x`` (one-arg form) is a *read*, not a definition —
        // Tcl returns the current value of ``x``.
        let mut a = Analyser::new();
        // Pre-define x so the read records a reference.
        a.define_var("x", esc_tok(span(0, 1)), &[], false, None);
        a.handle_set_command(&["x".to_string()], &[esc_tok(span(10, 11))], &[true], &[]);
        // The read appended a reference; no second definition.
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert_eq!(
            a.result.global_scope.variables["x"].references,
            vec![span(10, 11)],
        );
        // No const-string tracking for the 1-arg form.
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn handle_set_no_value_undefined_var_is_silent() {
        // ``set x`` on an undefined variable is still a read; the
        // record_var_read helper silently no-ops when the name
        // isn't in scope, so no spurious binding lands.
        let mut a = Analyser::new();
        a.handle_set_command(&["x".to_string()], &[esc_tok(span(0, 1))], &[true], &[]);
        assert!(!a.result.global_scope.variables.contains_key("x"));
    }

    // handle_var_declaration_command

    #[test]
    fn handle_global_defines_each_name() {
        let mut a = Analyser::new();
        a.handle_global_command(
            &["x".to_string(), "y".to_string(), "z".to_string()],
            &[
                esc_tok(span(0, 1)),
                esc_tok(span(2, 3)),
                esc_tok(span(4, 5)),
            ],
            &[],
        );
        for name in ["x", "y", "z"] {
            assert!(a.result.global_scope.variables.contains_key(name));
            assert!(!a.result.global_scope.variables[name].warn_if_unused);
        }
    }

    #[test]
    fn handle_variable_defines_only_names_skipping_values() {
        let mut a = Analyser::new();
        // `variable x 1 y 2 z` — names at 0, 2, 4; values at 1, 3.
        a.handle_variable_command(
            &[
                "x".to_string(),
                "1".to_string(),
                "y".to_string(),
                "2".to_string(),
                "z".to_string(),
            ],
            &[
                esc_tok(span(0, 1)),
                esc_tok(span(2, 3)),
                esc_tok(span(4, 5)),
                esc_tok(span(6, 7)),
                esc_tok(span(8, 9)),
            ],
            &[],
        );
        for name in ["x", "y", "z"] {
            assert!(a.result.global_scope.variables.contains_key(name));
        }
        // Numbers should NOT be variable names.
        assert!(!a.result.global_scope.variables.contains_key("1"));
        assert!(!a.result.global_scope.variables.contains_key("2"));
    }

    #[test]
    fn handle_variable_single_name_no_value() {
        let mut a = Analyser::new();
        a.handle_variable_command(&["x".to_string()], &[esc_tok(span(0, 1))], &[]);
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    // handle_proc_command

    #[test]
    fn handle_proc_records_proc_at_global() {
        let mut a = Analyser::new();
        let handled = a.handle_proc_command(
            &["foo".to_string(), "a b".to_string(), "set x $a".to_string()],
            &[
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 25)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.all_procs.contains_key("::foo"));
        let proc = &a.result.all_procs["::foo"];
        assert_eq!(proc.name, "foo");
        assert_eq!(proc.qualified_name, "::foo");
        assert_eq!(proc.params.len(), 2);
        assert_eq!(proc.params[0].name, "a");
        assert_eq!(proc.params[1].name, "b");
    }

    #[test]
    fn handle_proc_qualifies_under_namespace() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        let handled = a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[0],
        );
        assert!(handled);
        assert!(a.result.all_procs.contains_key("::ns1::foo"));
    }

    #[test]
    fn handle_proc_keys_scope_procs_by_simple_name() {
        // ``scope.procs`` is keyed by the simple name so per-scope
        // lookups and shadowing rules work locally. The
        // qualified name lives on ``ProcDef.qualified_name`` and
        // is the key for ``result.all_procs``.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[0],
        );
        let scope = &a.result.global_scope.children[0];
        // Scope's procs map keyed by simple name, NOT qualified.
        assert!(
            scope.procs.contains_key("foo"),
            "scope.procs should be keyed by simple `foo`, got keys: {:?}",
            scope.procs.keys().collect::<Vec<_>>(),
        );
        assert!(
            !scope.procs.contains_key("::ns1::foo"),
            "scope.procs must not be keyed by qualified name",
        );
        // The qualified name is still on the ProcDef.
        assert_eq!(scope.procs["foo"].qualified_name, "::ns1::foo");
        // ...and result.all_procs is keyed by qualified name.
        assert!(a.result.all_procs.contains_key("::ns1::foo"));
    }

    #[test]
    fn handle_proc_absolute_name_rebases() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "outer"));
        let handled = a.handle_proc_command(
            &["::other::foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 17)),
                str_tok(span(18, 20)),
                str_tok(span(21, 23)),
            ],
            &[0],
        );
        assert!(handled);
        // Absolute name rebases — does NOT nest under outer.
        assert!(a.result.all_procs.contains_key("::other::foo"));
        assert!(!a.result.all_procs.contains_key("::outer::other::foo"));
    }

    #[test]
    fn handle_proc_consumes_last_comment_as_doc() {
        let mut a = Analyser::new();
        a.last_comment = "doc string".to_string();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(0, 3)),
                str_tok(span(4, 6)),
                str_tok(span(7, 9)),
            ],
            &[],
        );
        assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
        // last_comment is consumed.
        assert!(a.last_comment.is_empty());
    }

    #[test]
    fn handle_proc_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_proc_command(&["foo".to_string()], &[esc_tok(span(0, 3))], &[]);
        assert!(!handled);
        assert!(a.result.all_procs.is_empty());
    }

    // handle_proc_command W113 shadow check

    #[test]
    fn handle_proc_emits_w113_for_builtin_shadow() {
        // ``proc set {} {}`` — the proc name is a built-in.
        // W113 should anchor at the proc-name span and carry
        // the canonical message shape. A *real* dialect is named in the
        // parenthetical label; the permissive fallback (both the empty
        // string and its canonical "tcl" spelling resolve there —
        // dialect-profile-model.md §8, one sink) carries no label, covered
        // by the sibling no-label test below.
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("tcl8.6");
        a.handle_proc_command(
            &["set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        let w113s: Vec<&crate::analyser::types::Diagnostic> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W113)
            .collect();
        assert_eq!(w113s.len(), 1);
        assert_eq!(w113s[0].span, span(5, 8));
        assert!(w113s[0].message.contains("'set' shadows built-in"));
        assert!(w113s[0].message.contains("(tcl8.6)"));
        assert_eq!(w113s[0].severity, crate::analyser::types::Severity::Warning);
    }

    #[test]
    fn handle_proc_no_w113_for_non_builtin_name() {
        // ``foo`` is not a built-in — no W113.
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("tcl");
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "should NOT emit W113 for non-built-in name 'foo'",
        );
    }

    #[test]
    fn handle_proc_w113_matches_qualified_form() {
        // ``proc ::set {} {}`` — qualified form also shadows
        // ``set`` because the registry indexes by bare command
        // name (``::`` is trimmed at lookup).
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("tcl");
        a.handle_proc_command(
            &["::set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 10)),
                str_tok(span(11, 13)),
                str_tok(span(14, 16)),
            ],
            &[],
        );
        assert!(
            a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113)
        );
    }

    #[test]
    fn handle_proc_w113_no_dialect_label_when_dialect_empty() {
        // Empty dialect → no parenthetical label in the message.
        let mut a = Analyser::new();
        // dialect intentionally left empty
        a.handle_proc_command(
            &["set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        let w113 = a
            .result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W113)
            .expect("W113 expected");
        assert!(w113.message.contains("'set' shadows built-in"));
        assert!(!w113.message.contains('('), "no dialect label expected");
    }

    #[test]
    fn handle_proc_w113_dialect_specific_command_only_shadows_in_that_dialect() {
        // ``pool`` is an *unqualified* iRules-specific command; under the
        // ``f5-irules`` dialect a proc named ``pool`` shadows a built-in,
        // but under plain ``tcl`` it does not.  (A *namespace-qualified*
        // iRules command like ``HTTP::respond`` is never flagged, because a
        // qualified match is a library/package command living in its own
        // namespace, not a core-global shadow.)
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("f5-irules");
        a.handle_proc_command(
            &["pool".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 9)),
                str_tok(span(10, 12)),
                str_tok(span(13, 15)),
            ],
            &[],
        );
        assert!(
            a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "f5-irules dialect should treat pool as built-in",
        );

        // Same proc, plain tcl dialect → no W113.
        let mut b = Analyser::new();
        b.profile = tcl_dialect::DialectProfile::by_name("tcl");
        b.handle_proc_command(
            &["pool".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 9)),
                str_tok(span(10, 12)),
                str_tok(span(13, 15)),
            ],
            &[],
        );
        assert!(
            !b.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "plain tcl dialect should NOT flag pool",
        );
    }

    #[test]
    fn handle_proc_w113_silent_on_namespace_qualified_shadow() {
        // FP: a namespace-qualified match (`HTTP::respond` under f5-irules)
        // is a library/package command in its own namespace, not a
        // core-global shadow — never W113.
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("f5-irules");
        a.handle_proc_command(
            &["HTTP::respond".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 18)),
                str_tok(span(19, 21)),
                str_tok(span(22, 24)),
            ],
            &[],
        );
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "namespace-qualified HTTP::respond must not fire W113",
        );
    }

    // handle_proc_command body recursion

    #[test]
    fn handle_proc_creates_proc_scope_for_braced_body() {
        // ``proc foo {} {}`` — empty braced body still opens a
        // proc scope so subsequent body-walking handlers have a
        // place to record locals.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        assert_eq!(a.result.global_scope.children.len(), 1);
        let proc_scope = &a.result.global_scope.children[0];
        assert_eq!(proc_scope.kind, crate::analyser::types::ScopeKind::Proc);
        assert_eq!(proc_scope.name, "foo");
        assert_eq!(proc_scope.body_span, Some(span(12, 14)));
    }

    #[test]
    fn handle_proc_defines_params_in_proc_scope() {
        // ``proc foo {a b} {}`` — a, b become locals in the
        // proc scope, not in the outer scope.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), "a b".to_string(), String::new()],
            &[
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 17)),
            ],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(proc_scope.variables.contains_key("a"));
        assert!(proc_scope.variables.contains_key("b"));
        // Outer scope must be untouched.
        assert!(!a.result.global_scope.variables.contains_key("a"));
    }

    #[test]
    fn handle_proc_walks_body_set_defines_local() {
        // ``proc foo {} {set x 1}`` — body walk segments the
        // body and dispatches `set x 1` against the proc scope,
        // landing the local in proc_scope.variables, not global.
        // The body token's span must mirror the outer source so
        // the segmenter rebases correctly: source layout is
        // ``proc foo {} {set x 1}`` with the body occupying [13, 22].
        // ``content_offset = 1`` skips the leading ``{`` so the
        // re-segmented inner runs at base 14.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "set x 1".to_string()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(13, 22)),
            ],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(
            proc_scope.variables.contains_key("x"),
            "body walk should land 'x' in proc scope; vars: {:?}",
            proc_scope.variables.keys().collect::<Vec<_>>(),
        );
        assert!(!a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn handle_proc_walks_body_global_falls_into_proc_scope() {
        // ``proc foo {} {global a b}`` — the ``global`` handler
        // defines bindings in the proc scope so the body's later
        // reads/writes resolve correctly. Real ``global`` semantics
        // (link to outer var) live with diagnostic emission later.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "global a b".to_string()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(13, 25)),
            ],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(proc_scope.variables.contains_key("a"));
        assert!(proc_scope.variables.contains_key("b"));
    }

    #[test]
    fn handle_proc_nested_proc_creates_nested_scopes() {
        // Body-walk recursion must dispatch `proc` inside the body,
        // creating a nested proc scope under the outer proc.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &[
                "outer".to_string(),
                String::new(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(5, 10)),
                str_tok(span(11, 13)),
                str_tok(span(15, 33)),
            ],
            &[],
        );
        // Outer proc registered.
        assert!(a.result.all_procs.contains_key("::outer"));
        // Inner proc registered under outer's qualified prefix?
        // Namespace resolution skips proc scopes —
        // so an `inner` proc declared inside ``outer`` qualifies as
        // ``::inner`` (the outer proc is not a namespace).
        assert!(a.result.all_procs.contains_key("::inner"));
        // Outer's proc scope holds the nested proc scope as a child.
        let outer_scope = &a.result.global_scope.children[0];
        assert!(!outer_scope.children.is_empty());
        assert_eq!(
            outer_scope.children[0].kind,
            crate::analyser::types::ScopeKind::Proc,
        );
        assert_eq!(outer_scope.children[0].name, "inner");
    }

    #[test]
    fn handle_proc_dynamic_body_skips_walk() {
        // ``proc foo {} $body`` — the body is a Var token, not a
        // Str token; we cannot statically re-segment a dynamic
        // body, so the body walk is skipped. The proc record
        // itself still lands so downstream signature consumers see
        // the proc; only the inner walk is gated.
        let mut a = Analyser::new();
        let var_tok = Token::new(TokenType::Var, span(13, 18));
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "$body".to_string()],
            &[esc_tok(span(5, 8)), str_tok(span(9, 11)), var_tok],
            &[],
        );
        assert!(a.result.all_procs.contains_key("::foo"));
        // No proc scope opened — Str gate failed.
        assert!(a.result.global_scope.children.is_empty());
    }

    #[test]
    fn handle_proc_body_walk_increments_body_depth_temporarily() {
        // ``body_depth`` is bumped for the duration of the body
        // walk and restored on exit — top-level-only command
        // checks rely on the depth being zero outside any body.
        let mut a = Analyser::new();
        assert_eq!(a.body_depth, 0);
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        assert_eq!(a.body_depth, 0);
    }

    #[test]
    fn handle_proc_body_walk_does_not_leak_inner_doc_comment() {
        // A trailing comment inside the body should not bleed into
        // ``last_comment`` for whatever follows the proc at the
        // outer scope. The outer comment ("doc string") is consumed
        // as the proc's own doc; after the walk, ``last_comment``
        // is restored to empty.
        let mut a = Analyser::new();
        a.last_comment = "doc string".to_string();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
        assert!(a.last_comment.is_empty());
    }

    // handle_namespace_eval_command

    // handle_namespace_path_command

    /// The two-word literal set form records entries under the declaring
    /// namespace; a later declaration replaces the whole path (C Tcl
    /// semantics), and entries are kept as written (rooting happens in the
    /// shared candidate builder).
    #[test]
    fn handle_namespace_path_records_and_replaces() {
        let mut a = Analyser::new();
        a.handle_namespace_path_command(&["path".to_string(), "::u ::v".to_string()], &[]);
        assert_eq!(
            a.namespace_paths.get("::").map(Vec::as_slice),
            Some(&["::u".to_string(), "::v".to_string()][..]),
        );
        a.handle_namespace_path_command(&["path".to_string(), "inner".to_string()], &[]);
        assert_eq!(
            a.namespace_paths.get("::").map(Vec::as_slice),
            Some(&["inner".to_string()][..]),
            "each namespace path declaration replaces the whole path",
        );
    }

    /// The query form (no list) and a dynamic list (`$var` / `[cmd]`)
    /// record nothing — the conservative empty path stands.
    #[test]
    fn handle_namespace_path_skips_query_and_dynamic_forms() {
        let mut a = Analyser::new();
        a.handle_namespace_path_command(&["path".to_string()], &[]);
        a.handle_namespace_path_command(&["path".to_string(), "$entries".to_string()], &[]);
        a.handle_namespace_path_command(&["path".to_string(), "[current_path]".to_string()], &[]);
        assert!(a.namespace_paths.is_empty());
    }

    /// A declaration inside a namespace scope keys to that namespace's
    /// accumulated name, so settlement applies it to the right call sites.
    #[test]
    fn handle_namespace_path_keys_to_declaring_namespace() {
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(crate::analyser::types::Scope::new(
                crate::analyser::types::ScopeKind::Namespace,
                "outer",
            ));
        a.handle_namespace_path_command(&["path".to_string(), "::helpers".to_string()], &[0]);
        assert_eq!(
            a.namespace_paths.get("::outer").map(Vec::as_slice),
            Some(&["::helpers".to_string()][..]),
        );
    }

    /// The `namespace path` resolution tier is 8.5+: a bare call under a path
    /// gains the path namespace as a candidate from 8.5 on, but under 8.4 it
    /// must not (8.4 has no path tier, so the call never reaches it).
    #[test]
    fn bare_call_honours_namespace_path_only_from_8_5() {
        let src = "namespace eval ::app { namespace path ::mymod\n    proc run {} { helper } }\n";
        let candidates = |dialect: &str| {
            let mut a = Analyser::new();
            a.analyse(src, dialect)
                .command_invocations
                .iter()
                .find(|i| i.name == "helper")
                .map(|i| i.resolution_candidates.clone())
                .unwrap_or_default()
        };
        assert!(
            candidates("tcl8.6").iter().any(|c| c == "::mymod::helper"),
            "8.6 should add the path namespace as a candidate: {:?}",
            candidates("tcl8.6"),
        );
        assert!(
            !candidates("tcl8.4").iter().any(|c| c == "::mymod::helper"),
            "8.4 has no path tier, so it must not: {:?}",
            candidates("tcl8.4"),
        );
    }

    /// `interp eval child { proc foo }` runs in a child interpreter, so `foo`
    /// is isolated from the parent's `::foo` — the two stay distinct commands,
    /// so a parent rename of `::foo` can never reach the child body.
    #[test]
    fn interp_eval_child_isolates_definitions_from_the_parent() {
        // Runnable Tcl (tclsh 9.0.4): the child is created before the
        // eval.  The child's `foo` homes under the synthetic
        // `@interp@child` domain — unrepresentable as a real namespace,
        // so a parent namespace literally named `child` can never
        // collide with the interpreter's definitions (issue #945
        // fault 8's child-global-vs-parent-namespace identity split).
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create child\nproc foo {} {}\ninterp eval child { proc foo {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
        assert!(
            r.all_procs.contains_key("::@interp@child::foo"),
            "child proc isolated under the interpreter domain: {keys:?}",
        );
        assert!(
            !r.all_procs.contains_key("::child::foo"),
            "the child's global is not the parent namespace `::child`: {keys:?}",
        );
    }

    /// `sandbox eval { proc foo }` via the interpreter's own object command
    /// (`interp create sandbox` binds `sandbox` itself as a callable
    /// command) isolates definitions from the parent exactly like the
    /// literal `interp eval sandbox { … }` form above — the far more common
    /// real-world spelling (`docstrip_util.tcl`'s `$c eval {...}`, reduced to
    /// a literal name for a statically-resolvable repro).
    #[test]
    fn interp_handle_eval_isolates_definitions_from_the_parent() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\nproc foo {} {}\nsandbox eval { proc foo {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
        assert!(
            r.all_procs.contains_key("::@interp@sandbox::foo"),
            "child proc isolated under the interpreter domain: {keys:?}",
        );
    }

    #[test]
    fn renamed_away_interp_handle_reused_as_a_proc_is_not_treated_as_interp_eval() {
        // TP — regression for a bug found by Codex review of PR #963:
        // `self.interpreters` is only cleared by `interp delete`, so a plain
        // `rename sandbox {}` (deleting the interpreter's own object
        // command — confirmed against tclsh9.0: afterwards `info commands
        // sandbox` is empty and the child interpreter is only reachable
        // through whatever name, if any, it was renamed *to*) left a stale
        // `sandbox` entry. A later, wholly unrelated `proc sandbox {sub
        // body} {...}` reusing the freed name would then have any
        // `sandbox eval …` call misidentified as isolated interpreter-eval
        // (walking the literal second argument as a *script*) instead of an
        // ordinary two-arg proc call — confirmed against tclsh9.0 that the
        // real dispatch reaches the new proc, never the (now nameless,
        // orphaned) interpreter.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\n\
             rename sandbox {}\n\
             proc sandbox {sub body} {}\n\
             sandbox eval { proc shouldNotBeIsolated {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            !r.all_procs
                .contains_key("::@interp@sandbox::shouldNotBeIsolated"),
            "the call must not be isolated into the (deleted) interpreter's \
             domain once `sandbox` has been renamed away and reused: {keys:?}",
        );
    }

    #[test]
    fn renamed_interp_handle_still_isolates_under_its_new_name() {
        // TP — the other half of the same fix: a rename that *moves* the
        // interpreter handle (`NEW` non-empty) rather than deleting it must
        // keep the tracking, now keyed by `NEW` — confirmed against
        // tclsh9.0 that `rename sandbox t; t eval {...}` still talks to the
        // same child interpreter. A name-only invalidation (removing `OLD`
        // without transferring state to `NEW`) would wrongly stop isolating
        // `t eval { ... }` too (falling through to an ordinary, unisolated
        // command call). The domain name itself is always derived from
        // whichever key the call site actually writes — `@interp@t`, not
        // the original `@interp@sandbox` — exactly like every other
        // interpreter-domain lookup in this module (e.g. the literal
        // `interp eval` form re-derives its domain from its own `PATH`
        // argument text the same way); this test only asserts that the
        // call is isolated *at all* under the new name, not which domain
        // string it lands in.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\n\
             rename sandbox t\n\
             t eval { proc shouldBeIsolated {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@t::shouldBeIsolated"),
            "the renamed handle must still be tracked as an interpreter, \
             isolating the eval body under its new name's domain: {keys:?}",
        );
    }

    #[test]
    fn interp_handle_eval_and_literal_interp_eval_agree_on_the_same_domain() {
        // The two spellings of the same operation must home under the
        // *same* synthetic domain — the analyser can't tell them apart at
        // run time (both dispatch on the identical interpreter object
        // command), so it must not tell them apart statically either.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\ninterp eval sandbox { proc viaLiteral {} {} }\n\
             sandbox eval { proc viaHandle {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@sandbox::viaLiteral"),
            "{keys:?}"
        );
        assert!(
            r.all_procs.contains_key("::@interp@sandbox::viaHandle"),
            "{keys:?}"
        );
    }

    #[test]
    fn two_interp_handles_never_cross_contaminate_same_named_procs() {
        // TP — the exact scenario differential audit confirmed: two
        // separate safe child interpreters, each independently defining
        // their own same-named `helper`, must never merge — a call inside
        // one script must resolve to *that* interpreter's helper, never
        // the other's.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandboxA\ninterp create sandboxB\n\
             interp eval sandboxA { proc helper {} { return A } }\n\
             sandboxB eval { proc helper {} { return B } }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@sandboxA::helper"),
            "{keys:?}"
        );
        assert!(
            r.all_procs.contains_key("::@interp@sandboxB::helper"),
            "{keys:?}"
        );
        // Each interpreter's own `helper` is a genuinely distinct
        // ProcDef, not the same entry aliased twice.
        assert_ne!(
            r.all_procs["::@interp@sandboxA::helper"].body_span,
            r.all_procs["::@interp@sandboxB::helper"].body_span,
        );
    }

    #[test]
    fn untracked_bareword_eval_is_not_isolated() {
        // TN / FN guard — a bareword shaped exactly like the tracked-handle
        // idiom (`NAME eval { script }`) but never created via `interp
        // create` must fall through to the generic dispatch untouched: no
        // isolated scope, no crash.
        let mut a = Analyser::new();
        let r = a.analyse("untracked eval { proc foo {} {} }\n", "tcl8.6");
        assert!(
            !r.all_procs.keys().any(|k| k.contains("@interp@")),
            "an untracked head must never open an interpreter domain: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn ordinary_proc_with_eval_as_sole_argument_is_untouched() {
        // FP guard — an unrelated proc whose single argument merely happens
        // to be the literal text `eval` must resolve as an ordinary call,
        // never trip the arity/shape check into isolating anything.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {a} { return $a }\nfoo eval\n", "tcl8.6");
        assert!(
            !r.all_procs.keys().any(|k| k.contains("@interp@")),
            "a 1-arg call must never be treated as NAME eval SCRIPT: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn interp_eval_into_uncreated_interp_warns_and_multiword_stays_isolated_945() {
        // `interp eval ghost { … }` with no `interp create ghost` anywhere
        // raises `could not find interpreter` at run time — W140 (issue
        // #945 fault 8: interpreter existence).
        let mut a = Analyser::new();
        let r = a.analyse("interp eval ghost { proc foo {} {} }\n", "tcl8.6");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W140),
            "eval into a never-created interpreter warns: {:?}",
            r.diagnostics
        );
        // A dynamic create makes existence unknowable — W140 abstains.
        let mut a2 = Analyser::new();
        let r2 = a2.analyse(
            "interp create $name\ninterp eval ghost { proc foo {} {} }\n",
            "tcl8.6",
        );
        assert!(
            !r2.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W140),
            "dynamic interp creation abstains: {:?}",
            r2.diagnostics
        );
        // Multi-word scripts concatenate at run time — the words must not
        // be analysed in the *parent* scope (the old fall-through), nor
        // walked per-word (commands span word boundaries).
        let mut a3 = Analyser::new();
        let r3 = a3.analyse(
            "interp create c\ninterp eval c {proc leak {} {}} {puts hi}\n",
            "tcl8.6",
        );
        assert!(
            !r3.all_procs.contains_key("::leak"),
            "multi-word eval must not leak definitions into the parent: {:?}",
            r3.all_procs.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn deleted_and_recreated_interp_is_a_fresh_domain_945() {
        // C: `interp delete s; interp create s` starts with an empty
        // command table — the old definitions are gone.  The epoch-stamped
        // domain keeps the two lifetimes apart: the first eval's `foo`
        // homes under `@interp@s`, the recreated interpreter's under
        // `@interp@s#1`, and they never merge (issue #945 fault 8).
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create s\ninterp eval s { proc foo {} {} }\n\
             interp delete s\ninterp create s\n\
             interp eval s { proc bar {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@s::foo"),
            "first lifetime's proc: {keys:?}"
        );
        assert!(
            r.all_procs.contains_key("::@interp@s#1::bar"),
            "recreated interpreter is a fresh domain: {keys:?}"
        );
        assert!(
            !r.all_procs.contains_key("::@interp@s::bar"),
            "the recreated child never merges with its predecessor: {keys:?}"
        );
    }

    #[test]
    fn nested_interp_ops_qualify_against_the_enclosing_child_945() {
        // `interp create t` inside `interp eval s {…}` creates `s t` —
        // paths are relative to the *current* interpreter, so a top-level
        // `interp eval {s t} {…}` reaches the grandchild without W140 and
        // its definitions home under the composed domain.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create s\n\
             interp eval s { interp create t }\n\
             interp eval {s t} { proc deep {} {} }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W140),
            "the nested create makes `s t` exist: {:?}",
            r.diagnostics
        );
        assert!(
            r.all_procs.contains_key("::@interp@s t::deep"),
            "grandchild definitions home under the composed domain: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn cross_domain_interp_alias_links_child_calls_to_the_parent_target_945() {
        // `interp alias s helper {} ::parent_helper` makes `helper`
        // callable *inside the child* running the parent's proc — the
        // alias deliberately crosses interpreter domains while
        // definitions do not (issue #945 fault 8).  The child-side call
        // resolves through the alias link to the parent target.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc parent_helper {} {}\ninterp create s\n\
             interp alias s helper {} parent_helper\n\
             interp eval s { helper }\n",
            "tcl8.6",
        );
        let alias = r
            .command_aliases
            .get("::@interp@s::helper")
            .unwrap_or_else(|| {
                panic!(
                    "child-domain alias recorded: {:?}",
                    r.command_aliases.keys().collect::<Vec<_>>()
                )
            });
        assert_eq!(
            alias.target, "::parent_helper",
            "the alias target resolves in the parent domain",
        );
    }

    #[test]
    fn safe_interp_hides_unsafe_commands_and_expose_restores_945() {
        // tclsh 9.0.4: a `-safe` child hides `source` (and the rest of the
        // non-CMD_IS_SAFE set) — calling it raises `invalid command name`.
        // The safe-context walk flags the call (W129) and, because the
        // command never executes, builds no source edge from it.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `source` in a safe interp warns: {:?}",
            r.diagnostics
        );
        assert!(
            r.source_targets.is_empty(),
            "no source edge may be built from a call that never executes: {:?}",
            r.source_targets
        );
        // A safe command (`puts`) draws no W129; a non-safe interp draws
        // none either.
        let mut a2 = Analyser::new();
        let r2 = a2.analyse(
            "interp create -safe s\ninterp eval s { puts hi }\n\
             interp create n\ninterp eval n { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r2.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "safe commands and non-safe interps draw no W129: {:?}",
            r2.diagnostics
        );
        // `interp expose s source` restores the command.
        let mut a3 = Analyser::new();
        let r3 = a3.analyse(
            "interp create -safe s\ninterp expose s source\n\
             interp eval s { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r3.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an exposed command is callable again: {:?}",
            r3.diagnostics
        );
        // `interp hide n source` hides it in a *normal* interp too.
        let mut a4 = Analyser::new();
        let r4 = a4.analyse(
            "interp create n\ninterp hide n source\n\
             interp eval n { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            r4.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an explicitly hidden command warns in a normal interp: {:?}",
            r4.diagnostics
        );
    }

    #[test]
    fn safe_interp_child_redefinition_of_a_hidden_builtin_is_callable_945() {
        // tclsh 9.0.4: `proc source {} {…}` inside a safe interp creates a
        // real command in the ordinary command table — a table entirely
        // separate from the hidden set — so calling `source` afterwards
        // runs the user's proc, never `invalid command name`.  The gate
        // must not draw W129 for the call, and the call must still be
        // fully analysed (resolving to the local proc), not skipped as an
        // effect-free hidden call.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { proc source {} { return ok }; source }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a locally-redefined name is callable, not hidden: {:?}",
            r.diagnostics
        );
        assert!(
            r.all_procs.contains_key("::@interp@s::source"),
            "the local proc is recorded, not skipped: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        // A call *preceding* the redefinition is unaffected by this file's
        // whole-body approximation in the other direction — still hidden
        // until the walk has seen a local definition — is not asserted
        // here (the analyser's per-body model is not that finely ordered);
        // what matters is that a real, common redefinition idiom never
        // produces a false-positive W129 that also drops the call's facts.
    }

    #[test]
    fn interp_expose_with_a_new_name_tracks_the_new_name_945() {
        // tclsh 9.0.4: `interp expose s source src` restores the hidden
        // `source` implementation under the name `src` — `source` itself
        // stays absent from ordinary lookup, and `src` becomes callable.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp expose s source src\n\
             interp eval s { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "the hidden name itself stays hidden after an exposed rename: {:?}",
            r.diagnostics
        );
        let mut a2 = Analyser::new();
        let r2 = a2.analyse(
            "interp create -safe s\ninterp expose s source src\n\
             interp eval s { src b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r2.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "the new exposed name is callable: {:?}",
            r2.diagnostics
        );
    }

    // -- issue #1001: W129 through `[...]` bracket-substitution indirection --

    /// TP (the reported repro): `package ifneeded`'s script argument is
    /// `ArgRole::Body`-tagged (`Structural` — runs later, via `uplevel #0`,
    /// never the definer's frame), so a `[list apply {…} $dir]`
    /// deferred-command idiom sitting there is genuinely invoked later —
    /// a hidden `source` nested inside the lambda body must draw W129, the
    /// same way it would if `apply {dir {source …}} $dir` were written
    /// directly (no `[list …]` wrapper).
    #[test]
    fn safe_interp_w129_list_quoted_apply_lambda_body_reports_hidden_source_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded myPackage 1.0 [list apply {dir {\n\
                     source [file join $dir font.tcl]\n\
                 }} $dir]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `source` nested inside a list-quoted apply lambda body \
             (reached only via `package ifneeded`'s deferred script) warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the same `[list apply {…} $x]` idiom in an `ArgRole::CommandPrefix`
    /// position (`trace add variable … command CALLBACK`) — a different
    /// registry role than `package ifneeded`'s `Body`, exercising the same
    /// deferred-call resolution through a different gate.
    #[test]
    fn safe_interp_w129_list_quoted_apply_in_command_prefix_position_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 trace add variable x write [list apply {{a b c} {\n\
                     exec ls\n\
                 }} $x]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `exec` nested inside a list-quoted apply lambda body \
             reached via a CommandPrefix (`trace add … command`) position warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: `after idle [list apply {…} $x]` — the `after`/`after idle`
    /// deferred-callback idiom the issue calls out by name.
    #[test]
    fn safe_interp_w129_list_quoted_apply_after_idle_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 after idle [list apply {x {\n\
                     file delete $x\n\
                 }} val]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `file` nested inside a list-quoted apply lambda body \
             reached via `after idle` warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: `[list source $file]` / `[list exec …]` directly — no `apply` at
    /// all, `list` command-quoting a hidden command straight.
    #[test]
    fn safe_interp_w129_list_quoted_hidden_command_directly_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { after idle [list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a bare `[list source …]` in a deferred-call position warns \
             directly, with no `apply` indirection needed: {:?}",
            r.diagnostics
        );
    }

    /// FP guard (mirrors #954's `set data [list apply {…} value]`
    /// non-invocation case, adapted to W129): `[list apply {…} value]`
    /// sitting in ordinary `set` data — not a `Body` / `LambdaLiteral` /
    /// `CommandPrefix` argument position — is never invoked, so it must
    /// never draw W129 even though its lambda body contains a hidden
    /// command.
    #[test]
    fn safe_interp_w129_list_quoted_apply_in_plain_data_is_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 set data [list apply {x { source $x }} value]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a `[list apply …]` value that is only ever stored, never \
             invoked, must not warn: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: the exact same list-quoted-apply-with-a-hidden-command
    /// shape, but with **no** enclosing safe interpreter at all, must not
    /// warn — and, since this fix's whole mechanism is gated on a
    /// non-empty `safe_interp_stack`, must not create any new scope either
    /// (no `apply@…` proc scope, no collateral diagnostics of any other
    /// kind) — this stays exactly as un-analysed as it was before #1001.
    #[test]
    fn list_quoted_apply_lambda_outside_any_safe_interp_is_untouched_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "package ifneeded myPackage 1.0 [list apply {dir {source [file join $dir x]}} $dir]\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "no safe interpreter is involved, so no W129 can ever fire: {:?}",
            r.diagnostics
        );
        assert!(
            r.all_procs.is_empty(),
            "this fix must not widen the general analyser's scope — \
             the list-quoted lambda body stays un-analysed outside a \
             safe-interpreter context, exactly as before #1001: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    /// TP: a direct nested `[…]` bracket substitution (no `list`-quoting at
    /// all) — `set x [source b.tcl]` — is an *immediate* invocation
    /// (bracket substitution always evaluates its content right away,
    /// wherever it appears), so it must warn exactly like a bare top-level
    /// `source b.tcl` statement would.
    #[test]
    fn safe_interp_w129_direct_nested_bracket_substitution_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { set x [source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a directly nested `[source …]` substitution warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the same direct-nested shape, reached through a deeper `[…]` /
    /// braced-body combination (`if {$c} { [exec ls] }` inside another
    /// substitution) — pins that the fix covers arbitrary nesting depth,
    /// not just one level.
    #[test]
    fn safe_interp_w129_direct_nested_bracket_substitution_deep_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { if {[catch {set y [exec ls]} err]} { puts $err } }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a deeply nested `[exec …]` substitution still warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: `{*}[list source $file]` as the *whole* statement — `{*}`
    /// expansion splices `list`'s result into this statement's own argv, so
    /// the command's effective head becomes `source`, even though the
    /// literal head word is the substitution text (never itself a
    /// registry name).
    #[test]
    fn safe_interp_w129_expand_list_quoted_head_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { {*}[list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "`{{*}}[list source …]` used as the whole statement warns \
             on the effective (expanded) head: {:?}",
            r.diagnostics
        );
    }

    /// TN: `{*}$cmdList` — an opaque variable expansion. Unlike
    /// `{*}[list source $file]`, the value isn't statically known, so this
    /// must NOT warn (matches this codebase's "prefer a miss over a false
    /// positive" stance for dynamic dispatch — the same policy `$cmd $file`
    /// direct dynamic dispatch already gets).
    #[test]
    fn safe_interp_w129_expand_dynamic_var_head_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { set cmdList [list source b.tcl]; {*}$cmdList }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an opaque `{{*}}$var` expansion is not statically resolvable \
             and must not warn: {:?}",
            r.diagnostics
        );
    }

    /// TN: a bare dynamic dispatch via a variable (`set cmd source; $cmd
    /// $file`) is not statically provable and must stay unflagged, matching
    /// this codebase's existing precedent for dynamic command dispatch.
    #[test]
    fn safe_interp_w129_dynamic_variable_dispatch_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { set cmd source; $cmd b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "dynamic dispatch through a variable is not statically \
             resolvable and must not warn: {:?}",
            r.diagnostics
        );
    }

    /// TP: `eval [list source $file]` — combining `eval` (a `Body`-role
    /// command) with list-quoting; `eval` evaluates the built string as a
    /// script, immediately invoking `source`.
    #[test]
    fn safe_interp_w129_eval_list_quoted_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { eval [list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "`eval [list source …]` warns on the resolved head: {:?}",
            r.diagnostics
        );
    }

    /// TP: `uplevel [list source $file]` — the same combination via
    /// `uplevel` instead of `eval`.
    #[test]
    fn safe_interp_w129_uplevel_list_quoted_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { uplevel [list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "`uplevel [list source …]` warns on the resolved head: {:?}",
            r.diagnostics
        );
    }

    /// TN: a *safe* command wrapped the same list-quoted-apply way must not
    /// warn — this fix widens W129's recall, it must not start flagging
    /// ordinary, allowed calls just because they are reached via `[list
    /// apply …]`.
    #[test]
    fn safe_interp_w129_list_quoted_apply_safe_command_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { after idle [list apply {x { puts $x }} val] }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a safe command (`puts`) reached via list-quoted apply must \
             not warn: {:?}",
            r.diagnostics
        );
    }

    /// A locally-redefined hidden-builtin name (issue #945 fault 7 follow-up
    /// — see `safe_interp_child_redefinition_of_a_hidden_builtin_is_callable_945`)
    /// stays callable through this fix's new indirection paths too: once
    /// `proc source {} {…}` has run earlier in the same interpreter body,
    /// `source` is a real, locally-defined command — independent of the
    /// hidden-command table — so a later `[list apply {…} $x]`-nested call
    /// to it must not warn.
    #[test]
    fn safe_interp_w129_redefined_command_not_flagged_through_indirection_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 proc source {} { return ok }\n\
                 after idle [list apply {{} { source }}]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a locally-redefined name reached via list-quoted apply is \
             callable, not hidden: {:?}",
            r.diagnostics
        );
    }

    /// The standard safe-interpreter delegation pattern — the trusted
    /// parent creates `interp alias s foo {} source`, bridging its *own*
    /// (non-hidden) `source` into the child under a new name — must not
    /// warn when `foo` is called inside the child, including through this
    /// fix's new indirection paths: `foo` is never itself a
    /// `SAFE_INTERP_HIDDEN` registry name, so the gate correctly leaves it
    /// alone (rename/`interp alias` cannot resurrect a *hidden* command's
    /// callability from within the child — confirmed against the `tcl-vm`
    /// runtime, which resolves both only through the ordinary, hidden-
    /// command-free lookup table).
    #[test]
    fn safe_interp_w129_alias_bridged_command_not_flagged_through_indirection_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp alias s foo {} source\n\
             interp eval s { after idle [list apply {{} { foo }}] }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an alias bridging a capability in from the trusted parent is \
             not itself a hidden registry name, so it must not warn: {:?}",
            r.diagnostics
        );
    }

    /// `namespace inscope ::x { proc foo }` runs the body in `::x`, so `foo`
    /// homes to `::x::foo` — the same namespace frame as `namespace eval`, not
    /// the caller's scope.
    #[test]
    fn namespace_inscope_runs_the_body_in_the_named_namespace() {
        let mut a = Analyser::new();
        let r = a.analyse("namespace inscope ::x { proc foo {} {} }\n", "tcl8.6");
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::x::foo"),
            "inscope body should home to the named namespace: {keys:?}",
        );
        assert!(
            !r.all_procs.contains_key("::foo"),
            "the body must not resolve in the caller's global scope: {keys:?}",
        );
    }

    /// `namespace code { proc foo }` captures the *current* namespace, so its
    /// script is analysed in this scope — inside `::x`, `foo` homes to
    /// `::x::foo`.
    #[test]
    fn namespace_code_analyses_the_script_in_the_current_namespace() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval ::x { namespace code { proc foo {} {} } }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::x::foo"),
            "namespace code script should analyse in the current namespace: {keys:?}",
        );
    }

    /// `interp eval {} { proc foo }` targets the *current* interpreter, so
    /// `foo` is the parent's `::foo` (no isolation, no synthetic namespace).
    #[test]
    fn interp_eval_empty_path_is_the_current_interpreter() {
        let mut a = Analyser::new();
        let r = a.analyse("interp eval {} { proc foo {} {} }\n", "tcl8.6");
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::foo"),
            "current-interp proc: {keys:?}"
        );
        assert!(
            !keys.iter().any(|k| k.contains("::::")),
            "an empty path must not open a synthetic namespace: {keys:?}",
        );
    }

    #[test]
    fn handle_namespace_eval_creates_child_scope() {
        let mut a = Analyser::new();
        let handled = a.handle_namespace_eval_command(
            &[
                "eval".to_string(),
                "ns1".to_string(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
        );
        assert!(handled);
        assert_eq!(a.result.global_scope.children.len(), 1);
        assert_eq!(a.result.global_scope.children[0].name, "ns1");
        assert_eq!(
            a.result.global_scope.children[0].kind,
            crate::analyser::types::ScopeKind::Namespace,
        );
    }

    #[test]
    fn handle_namespace_eval_records_body_span() {
        let mut a = Analyser::new();
        a.handle_namespace_eval_command(
            &["eval".to_string(), "ns1".to_string(), String::new()],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
        );
        assert_eq!(
            a.result.global_scope.children[0].body_span,
            Some(span(19, 35))
        );
    }

    #[test]
    fn handle_namespace_eval_dynamic_target_gets_a_synthetic_span_keyed_name() {
        // TP — regression for a bug found by differential audit against
        // irc.tcl's per-connection `namespace eval $name { … }` idiom: a
        // dynamic target must never become the scope's `.name` verbatim (two
        // unrelated occurrences sharing the same variable name would then
        // collapse into one scope), so it's replaced with a synthetic name
        // keyed on this occurrence's own token offset.
        let mut a = Analyser::new();
        a.handle_namespace_eval_command(
            &[
                "eval".to_string(),
                "$name".to_string(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 20)),
                str_tok(span(21, 37)),
            ],
            &[],
        );
        assert_eq!(a.result.global_scope.children[0].name, "@dynns@15");
    }

    #[test]
    fn handle_namespace_eval_literal_target_keeps_its_written_name() {
        // FN guard — a literal (non-dynamic) target must still use its own
        // written text verbatim, exactly as before the fix.
        let mut a = Analyser::new();
        a.handle_namespace_eval_command(
            &["eval".to_string(), "ns1".to_string(), String::new()],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
        );
        assert_eq!(a.result.global_scope.children[0].name, "ns1");
    }

    #[test]
    fn two_dynamic_namespace_eval_blocks_sharing_a_variable_name_never_collide() {
        // TP — the full-pipeline shape of the irc.tcl bug: two lexically
        // unrelated procs each open `namespace eval $name { … }` with a
        // same-named local `name` (an unremarkable choice for this exact
        // idiom) and each define their own `helper`. Before the fix both
        // blocks' scope shared the literal text "$name", so the second
        // `helper` silently overwrote the first in the flat `all_procs` map.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc setupA {} {\n    set name connA\n    namespace eval $name {\n        proc helper {} { return A }\n    }\n}\n\
             proc setupB {} {\n    set name connB\n    namespace eval $name {\n        proc helper {} { return B }\n    }\n}\n",
            "tcl8.6",
        );
        // drift-ok: test assertion counting distinct entries by simple name
        // to prove no cross-occurrence merge, not a resolution decision.
        let helpers: Vec<_> = r
            .all_procs
            .iter()
            .filter(|(_, p)| p.name == "helper")
            .collect();
        assert_eq!(
            helpers.len(),
            2,
            "both blocks' helper must survive as distinct entries: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
        assert_ne!(
            helpers[0].0, helpers[1].0,
            "distinct call sites must get distinct qualified keys: {helpers:?}",
        );
        assert_ne!(
            helpers[0].1.body_span, helpers[1].1.body_span,
            "each is genuinely its own definition, not the same one twice: {helpers:?}",
        );
    }

    // handle_namespace_ensemble

    #[test]
    fn handle_namespace_ensemble_records_in_set() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(&["ensemble".to_string(), "create".to_string()], &[], &[0]);
        assert!(a.ensemble_namespaces.contains("::myns"));
    }

    #[test]
    fn handle_namespace_ensemble_command_option_recorded() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
        a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-command".to_string(),
                "::ens".to_string(),
            ],
            &[],
            &[0],
        );
        assert!(a.ensemble_namespaces.contains("::myns"));
        assert!(a.ensemble_namespaces.contains("::ens"));
    }

    #[test]
    fn handle_namespace_ensemble_command_option_qualifies_relative_name() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
        a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-command".to_string(),
                "ens".to_string(),
            ],
            &[],
            &[0],
        );
        assert!(a.ensemble_namespaces.contains("::myns::ens"));
    }

    #[test]
    fn handle_namespace_ensemble_command_option_dynamic_value_not_recorded() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-command".to_string(),
                "$dyn".to_string(),
            ],
            &[],
            &[0],
        );
        assert_eq!(
            a.ensemble_namespaces,
            std::collections::HashSet::from(["::myns".to_string()])
        );
    }

    #[test]
    fn namespace_ensemble_map_and_subcommands_record_command_references() {
        // `-map {get ::foo::getImpl}` — the odd element is the target command;
        // `-subcommands {show}` maps `show` to `::foo::show`.  Both become
        // command references so navigation reaches the implementing procs.
        let mut a = Analyser::new();
        let src = "namespace eval ::foo {\n    \
                   proc getImpl {} {}\n    \
                   proc show {} {}\n    \
                   namespace ensemble create -map {get ::foo::getImpl} -subcommands {show}\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            resolved.contains(&"::foo::getImpl"),
            "the -map target should be a command reference: {resolved:?}",
        );
        assert!(
            resolved.contains(&"::foo::show"),
            "the -subcommands name should map to `<ns>::show`: {resolved:?}",
        );
    }

    #[test]
    fn handle_namespace_ensemble_other_options_value_word_is_not_mistaken_for_command_flag() {
        // Regression: before the registry-driven option walk, the scan
        // checked *every* word for literal equality with `-command`,
        // including another option's own value word — `-map`'s value
        // here is (pathologically, but syntactically legally) the string
        // `-command`. A word-by-word scan misreads that value as the
        // `-command` flag itself and steals the *next* word (the real
        // `-command`'s own flag) as if it were a namespace name. Walking
        // by each option's declared value arity instead correctly skips
        // `-map`'s whole value before ever looking for `-command` again,
        // so only the genuine `-command ::real::target` is recorded.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-map".to_string(),
                "-command".to_string(),
                "-command".to_string(),
                "::real::target".to_string(),
            ],
            &[],
            &[0],
        );
        assert_eq!(
            a.ensemble_namespaces,
            std::collections::HashSet::from(["::myns".to_string(), "::real::target".to_string()]),
            "only the genuine -command flag's value must be recorded, \
             not -map's value word that happens to read \"-command\""
        );
    }

    #[test]
    fn handle_namespace_ensemble_global_scope_no_op() {
        let mut a = Analyser::new();
        a.handle_namespace_ensemble(&["ensemble".to_string(), "create".to_string()], &[], &[]);
        assert!(a.ensemble_namespaces.is_empty());
    }

    #[test]
    fn handle_namespace_ensemble_wrong_subcommand_no_op() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(&["eval".to_string(), "myns".to_string()], &[], &[0]);
        assert!(a.ensemble_namespaces.is_empty());
    }

    // handle_foreach_command

    #[test]
    fn handle_foreach_defines_single_loop_var() {
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            &[
                "i".to_string(),
                "{1 2 3}".to_string(),
                "puts $i".to_string(),
            ],
            &[
                esc_tok(span(8, 9)),
                str_tok(span(10, 17)),
                str_tok(span(18, 28)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.global_scope.variables.contains_key("i"));
    }

    #[test]
    fn handle_foreach_defines_multiple_loop_vars() {
        let mut a = Analyser::new();
        a.handle_foreach_command(
            &["k v".to_string(), "{a 1 b 2}".to_string(), String::new()],
            &[
                esc_tok(span(8, 11)),
                str_tok(span(12, 21)),
                str_tok(span(22, 24)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("k"));
        assert!(a.result.global_scope.variables.contains_key("v"));
        // Each name gets its own span inside the var-list content ("k v" at
        // bytes 8..11), not the shared whole-token span — so offset-sorted
        // consumers see them in declaration order (`k` before `v`).
        let k = &a.result.global_scope.variables["k"];
        let v = &a.result.global_scope.variables["v"];
        assert_eq!(k.definition_span, span(8, 9));
        assert_eq!(v.definition_span, span(10, 11));
        assert!(k.definition_span.start() < v.definition_span.start());
    }

    #[test]
    fn handle_foreach_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            &["i".to_string(), "{1 2}".to_string()],
            &[esc_tok(span(0, 1)), str_tok(span(2, 7))],
            &[],
        );
        assert!(!handled);
    }

    // handle_for_command

    #[test]
    fn handle_for_returns_true_for_canonical_shape() {
        let mut a = Analyser::new();
        let handled = a.handle_for_command(
            &[
                "set i 0".to_string(),
                "$i < 10".to_string(),
                "incr i".to_string(),
                "puts $i".to_string(),
            ],
            &[],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_for_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled =
            a.handle_for_command(&["set i 0".to_string(), "$i < 10".to_string()], &[], &[]);
        assert!(!handled);
    }

    // handle_switch_command

    #[test]
    fn handle_switch_returns_true_for_canonical_shape() {
        let mut a = Analyser::new();
        let handled = a.handle_switch_command(
            &["$x".to_string(), "{a {puts a} b {puts b}}".to_string()],
            &[esc_tok(span(7, 9)), str_tok(span(10, 36))],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_switch_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_switch_command(&["$x".to_string()], &[], &[]);
        assert!(!handled);
    }

    #[test]
    fn handle_switch_form1_walks_each_arm_body() {
        // Form 1: ``switch $x a {set y 1} b {set z 2}``.
        // Each arm body should land its ``set`` in the
        // surrounding scope.  Source layout has the bodies at
        // offsets 13..23 (``{set y 1}``) and 27..37 (``{set z 2}``).
        let mut a = Analyser::new();
        a.handle_switch_command(
            &[
                "$x".to_string(),
                "a".to_string(),
                "set y 1".to_string(),
                "b".to_string(),
                "set z 2".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 11)),
                str_tok(span(13, 22)),
                esc_tok(span(24, 25)),
                str_tok(span(27, 36)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_switch_form2_braced_body_walks_each_arm() {
        // Form 2: ``switch $x { a {set y 1} b {set z 2} }``.
        // The single braced body holds all pattern/body pairs;
        // ``flatten_clause_list_elements`` re-segments to surface
        // each pair, then each body recurses.
        let mut a = Analyser::new();
        let body_text = " a {set y 1} b {set z 2} ".to_string();
        // body span: outer source positions 10..(10 + len(body)+2).
        // body_text has 25 chars, plus surrounding braces → token
        // span 10..37, content_offset = 1 to skip the opening ``{``.
        a.handle_switch_command(
            &["$x".to_string(), body_text],
            &[esc_tok(span(7, 9)), str_tok(span(10, 37))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_switch_form1_skips_fallthrough_marker() {
        // ``switch $x a - b {set y 1}`` — the ``-`` body for
        // pattern ``a`` is fall-through (next arm fires); only
        // ``b``'s body should be walked.
        let mut a = Analyser::new();
        a.handle_switch_command(
            &[
                "$x".to_string(),
                "a".to_string(),
                "-".to_string(),
                "b".to_string(),
                "set y 1".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 11)),
                esc_tok(span(12, 13)),
                esc_tok(span(14, 15)),
                str_tok(span(17, 26)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_switch_recognises_dashdash_options_terminator() {
        // ``switch -- $x a {set y 1}`` — ``--`` ends the option
        // section; the string arg follows.  Walker still finds
        // the arm body and lands ``y``.
        let mut a = Analyser::new();
        a.handle_switch_command(
            &[
                "--".to_string(),
                "$x".to_string(),
                "a".to_string(),
                "set y 1".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 12)),
                esc_tok(span(13, 14)),
                str_tok(span(16, 25)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_switch_dynamic_form2_body_skips_walk() {
        // Form 2 with a dynamic body (``$body`` instead of a
        // braced literal) yields no elements; the walk no-ops.
        let mut a = Analyser::new();
        let var_tok = Token::new(TokenType::Var, span(10, 15));
        a.handle_switch_command(
            &["$x".to_string(), "$body".to_string()],
            &[esc_tok(span(7, 9)), var_tok],
            &[],
        );
        // No body walked → no vars defined.
        assert!(a.result.global_scope.variables.is_empty());
    }

    // handle_catch_command

    #[test]
    fn handle_catch_canonical_returns_true() {
        let mut a = Analyser::new();
        let handled = a.handle_catch_command(&["body".to_string()], &[esc_tok(span(0, 4))], &[]);
        assert!(handled);
    }

    #[test]
    fn handle_catch_with_result_var_defines_it() {
        let mut a = Analyser::new();
        // The binding positions come from the registry's VarWrite roles.
        a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
        a.handle_catch_command(
            &["body".to_string(), "res".to_string()],
            &[esc_tok(span(0, 4)), esc_tok(span(5, 8))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("res"));
    }

    #[test]
    fn handle_catch_with_options_var_defines_both() {
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
        a.handle_catch_command(
            &["body".to_string(), "res".to_string(), "opts".to_string()],
            &[
                esc_tok(span(0, 4)),
                esc_tok(span(5, 8)),
                esc_tok(span(9, 13)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("res"));
        assert!(a.result.global_scope.variables.contains_key("opts"));
    }

    #[test]
    fn handle_catch_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_catch_command(&[], &[], &[]);
        assert!(!handled);
    }

    // handle_try_command

    #[test]
    fn handle_try_canonical_returns_true() {
        let mut a = Analyser::new();
        let handled = a.handle_try_command(&["body".to_string()], &[str_tok(span(0, 4))], &[]);
        assert!(handled);
    }

    #[test]
    fn handle_try_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_try_command(&[], &[], &[]);
        assert!(!handled);
    }

    #[test]
    fn handle_try_walks_main_body() {
        // ``try {set y 1}`` — main body walks and lands ``y``.
        let mut a = Analyser::new();
        a.handle_try_command(&["set y 1".to_string()], &[str_tok(span(5, 14))], &[]);
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_try_walks_finally_body() {
        // ``try {} finally {set z 1}`` — finally clause body walks.
        let mut a = Analyser::new();
        a.handle_try_command(
            &[String::new(), "finally".to_string(), "set z 1".to_string()],
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 15)),
                str_tok(span(16, 25)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_try_walks_on_handler_body() {
        // ``try {} on error {result options} {set q 1}`` — the
        // handler body at offset i+3 walks; the varList at i+2
        // is *not* defined as a local.
        let mut a = Analyser::new();
        a.handle_try_command(
            &[
                String::new(),
                "on".to_string(),
                "error".to_string(),
                "result options".to_string(),
                "set q 1".to_string(),
            ],
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 10)),
                esc_tok(span(11, 16)),
                str_tok(span(17, 33)),
                str_tok(span(34, 43)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("q"));
        // The `on error {result options}` var-list binds the result message +
        // options dict in the handler body — both are defined (so completion
        // offers `$result` / `$options`).
        assert!(a.result.global_scope.variables.contains_key("result"));
        assert!(a.result.global_scope.variables.contains_key("options"));
    }

    #[test]
    fn handle_try_walks_trap_handler_body() {
        // ``try {} trap NONE {result} {set q 1}`` — same shape
        // as ``on``, but the keyword is ``trap``.
        let mut a = Analyser::new();
        a.handle_try_command(
            &[
                String::new(),
                "trap".to_string(),
                "NONE".to_string(),
                "result".to_string(),
                "set q 1".to_string(),
            ],
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 12)),
                esc_tok(span(13, 17)),
                str_tok(span(18, 26)),
                str_tok(span(27, 36)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("q"));
    }

    // resolve_proc_call

    #[test]
    fn resolve_proc_call_absolute_qualified_name() {
        // ``::foo`` resolves directly when registered.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        let resolved = a.resolve_proc_call("::foo", &[]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::foo");
    }

    #[test]
    fn resolve_proc_call_bare_name_walks_namespace_chain() {
        // ``foo`` declared inside ``ns1`` is found when resolved
        // from ``ns1``'s scope.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[0],
        );
        // Resolve from inside ns1 — should find ::ns1::foo.
        let resolved = a.resolve_proc_call("foo", &[0]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::ns1::foo");
    }

    #[test]
    fn resolve_proc_call_falls_back_to_global() {
        // Bare ``foo`` declared at global is found from any scope.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        // Resolve from inside ns1 — chain misses ::ns1::foo,
        // falls back to ::foo.
        let resolved = a.resolve_proc_call("foo", &[0]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::foo");
    }

    #[test]
    fn resolve_proc_call_qualified_relative_name() {
        // ``a::b`` (qualified but not absolute) prepends ``::``.
        let mut a = Analyser::new();
        a.result.all_procs.insert(
            "::a::b".to_string(),
            super::ProcDef {
                name: "b".to_string(),
                qualified_name: "::a::b".to_string(),
                params: Vec::new(),
                name_span: span(0, 0),
                body_span: span(0, 0),
                doc: String::new(),
                param_traits: std::collections::HashMap::new(),
            },
        );
        let resolved = a.resolve_proc_call("a::b", &[]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::a::b");
    }

    #[test]
    fn resolve_proc_call_relative_dotted_name_prefers_current_namespace() {
        // A relative dotted word (`ns2::inner`, containing `::` but not
        // starting with it) must resolve against the current namespace
        // first, not be rooted straight at global (confirmed against tclsh
        // 9.0.4 — see `bareword_resolution_candidates`).
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        for qname in ["::ns2::inner", "::ns::ns2::inner"] {
            a.result.all_procs.insert(
                qname.to_string(),
                super::ProcDef {
                    name: "inner".to_string(),
                    qualified_name: qname.to_string(),
                    params: Vec::new(),
                    name_span: span(0, 0),
                    body_span: span(0, 0),
                    doc: String::new(),
                    param_traits: std::collections::HashMap::new(),
                },
            );
        }
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns"));
        let resolved = a.resolve_proc_call("ns2::inner", &[0]);
        assert_eq!(
            resolved.unwrap().qualified_name,
            "::ns::ns2::inner",
            "must prefer the current-namespace proc over the root one",
        );
        // Falls back to the root proc when there is no current-namespace
        // candidate.
        let resolved = a.resolve_proc_call("ns2::inner", &[]);
        assert_eq!(resolved.unwrap().qualified_name, "::ns2::inner");
    }

    #[test]
    fn resolve_proc_call_does_not_walk_ancestor_namespaces() {
        // Real Tcl bareword resolution is exactly two levels — current
        // namespace, then global — absent an explicit `namespace path`. A
        // proc defined in a *grandparent* namespace must not resolve for a
        // bare call from a nested `::a::b::c` scope.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        let mut ns_a = Scope::new(ScopeKind::Namespace, "a");
        let mut ns_b = Scope::new(ScopeKind::Namespace, "b");
        ns_b.children.push(Scope::new(ScopeKind::Namespace, "c"));
        ns_a.children.push(ns_b);
        a.result.global_scope.children.push(ns_a);
        // scope_path [0] = ::a — define `foo` there.
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[0],
        );
        // Resolving from ::a::b::c (scope_path [0, 0, 0]) must NOT reach
        // the grandparent's ::a::foo.
        assert!(
            a.resolve_proc_call("foo", &[0, 0, 0]).is_none(),
            "a grandparent-namespace proc must not resolve for a bare call",
        );
        // Control: resolving directly from ::a still finds it.
        assert!(a.resolve_proc_call("foo", &[0]).is_some());
    }

    #[test]
    fn resolve_proc_call_unknown_name_returns_none() {
        let a = Analyser::new();
        assert!(a.resolve_proc_call("nope", &[]).is_none());
    }

    #[test]
    fn resolve_proc_call_empty_name_returns_none() {
        let a = Analyser::new();
        assert!(a.resolve_proc_call("", &[]).is_none());
    }

    // resolve_expansion_count

    #[test]
    fn resolve_expansion_count_braced_literal() {
        // ``{a b c}`` — Str token; inner content "a b c" splits
        // to three elements.
        let mut a = Analyser::new();
        a.source = "{a b c}".to_string();
        // Span covers ``{a b c`` (5 inner chars + opening brace),
        // content_offset = 1 to skip ``{``.  Closing ``}`` is
        // OUTSIDE the span by lexer convention for non-degenerate
        // STR tokens.
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 6),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(3));
    }

    #[test]
    fn resolve_expansion_count_braced_empty_list() {
        // ``{}`` — degenerate Str case; span extended to include
        // ``}``, token_text returns empty string.
        let mut a = Analyser::new();
        a.source = "{}".to_string();
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 2),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(0));
    }

    #[test]
    fn resolve_expansion_count_var_with_const_value() {
        // ``$xs`` where xs has known constant ``a b c`` — splits
        // to three elements.
        let mut a = Analyser::new();
        a.source = "$xs".to_string();
        a.set_const_string("xs", "a b c".to_string(), span(0, 5), &[]);
        // Var token: span covers ``xs`` (after `$`) by lexer
        // convention; content_offset = 0 because the lexer's
        // ``_start`` for VAR is set after the ``$``.
        // For testing, place the var name at offset 1..3 in source.
        let tok = Token {
            kind: TokenType::Var,
            span: span(1, 3),
            content_offset: 0,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(3));
    }

    #[test]
    fn resolve_expansion_count_var_without_const_value() {
        // Var with no known constant value → None.
        let mut a = Analyser::new();
        a.source = "$xs".to_string();
        let tok = Token {
            kind: TokenType::Var,
            span: span(1, 3),
            content_offset: 0,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), None);
    }

    #[test]
    fn resolve_expansion_count_concatenated_word_returns_none() {
        // ``single_token = false`` short-circuits to None.
        let mut a = Analyser::new();
        a.source = "{a b c}".to_string();
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 6),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, false, &[]), None);
    }

    #[test]
    fn resolve_expansion_count_other_token_kind_returns_none() {
        // Non-Str, non-Var token kinds aren't statically
        // resolvable.
        let a = Analyser::new();
        let tok = esc_tok(span(0, 4));
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), None);
    }

    // handle_interp_alias

    #[test]
    fn handle_interp_alias_records_canonical_form() {
        let mut a = Analyser::new();
        a.handle_interp_alias(
            &[
                "alias".to_string(),
                String::new(),
                "myset".to_string(),
                String::new(),
                "set".to_string(),
            ],
            42,
        );
        assert!(a.command_aliases.contains_key("::myset"));
        assert!(a.result.command_aliases.contains_key("::myset"));
        let (target, prepended) = &a.command_aliases["::myset"];
        assert_eq!(target, "set");
        assert!(prepended.is_empty());
        assert_eq!(a.alias_offsets.get("::myset"), Some(&42));
        // The offset is also promoted onto the finalised `AnalysisResult`
        // (not just the in-progress `Analyser`), so a cross-document
        // consumer such as `tcl_lsp_core::WorkspaceIndex` can tell an
        // unconditional alias apart from one nested in a proc/class body.
        assert_eq!(a.result.alias_offsets.get("::myset"), Some(&42));
    }

    #[test]
    fn handle_interp_alias_with_prepended_args() {
        let mut a = Analyser::new();
        a.handle_interp_alias(
            &[
                "alias".to_string(),
                String::new(),
                "logerr".to_string(),
                String::new(),
                "puts".to_string(),
                "stderr".to_string(),
            ],
            0,
        );
        let (target, prepended) = &a.command_aliases["::logerr"];
        assert_eq!(target, "puts");
        assert_eq!(prepended, &vec!["stderr".to_string()]);
    }

    #[test]
    fn handle_interp_alias_wrong_shape_no_op() {
        let mut a = Analyser::new();
        a.handle_interp_alias(&["alias".to_string()], 0);
        assert!(a.command_aliases.is_empty());
        assert!(a.alias_offsets.is_empty());
    }

    // handle_rename

    #[test]
    fn handle_rename_records_static_move() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["target".to_string(), "target_orig".to_string()], &[], 42);
        assert!(!dynamic, "a fully static rename is not dynamic");
        assert_eq!(
            a.renamed_commands.get("::target_orig").map(String::as_str),
            Some("::target")
        );
        assert_eq!(
            a.result
                .renamed_commands
                .get("::target_orig")
                .map(String::as_str),
            Some("::target")
        );
        assert_eq!(a.rename_offsets.get("::target_orig"), Some(&42));
        // The old name is recorded as deleted from this offset —
        // confirmed against tclsh 9.0.4: `target` becomes an unknown
        // command, not still callable under its original arity.
        assert_eq!(a.deleted_commands.get("::target"), Some(&42));
    }

    #[test]
    fn handle_rename_deletion_records_old_name_as_deleted() {
        // `rename OLD {}` deletes OLD — no new binding to record (it is
        // not a "dynamic" rename either, nothing to widen for), but OLD
        // itself must be recorded as gone (confirmed against tclsh
        // 9.0.4: also "invalid command name" afterwards).
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["target".to_string(), String::new()], &[], 7);
        assert!(!dynamic);
        assert!(a.renamed_commands.is_empty());
        assert_eq!(a.deleted_commands.get("::target"), Some(&7));
    }

    #[test]
    fn handle_rename_dynamic_old_name_reports_dynamic() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["$x".to_string(), "y".to_string()], &[], 0);
        assert!(dynamic, "rename $x y cannot be resolved statically");
        assert!(a.renamed_commands.is_empty());
        assert!(a.deleted_commands.is_empty());
    }

    #[test]
    fn handle_rename_dynamic_new_name_reports_dynamic() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["x".to_string(), "y[z]".to_string()], &[], 0);
        assert!(dynamic, "rename x y[z] cannot be resolved statically");
        assert!(a.renamed_commands.is_empty());
        assert!(a.deleted_commands.is_empty());
    }

    #[test]
    fn handle_rename_wrong_shape_no_op() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["onlyone".to_string()], &[], 0);
        assert!(!dynamic);
        assert!(a.renamed_commands.is_empty());
        assert!(a.deleted_commands.is_empty());
    }

    // handle_oo_objdefine

    #[test]
    fn handle_oo_objdefine_records_dollar_var() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine(&["$obj".to_string()], &[], &[]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn handle_oo_objdefine_records_braced_dollar_var() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine(&["${obj}".to_string()], &[], &[]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn handle_oo_objdefine_records_bare_name() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine(&["obj".to_string()], &[], &[]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn oo_objdefine_body_methods_are_analysed() {
        // Before: the `oo::objdefine` body was never parsed, so nothing inside
        // a per-object method resolved.  Now the body walks like any method
        // body — the call it makes is recorded as an invocation.
        let mut a = Analyser::new();
        let src = "proc helper {} {}\n\
                   oo::class create Foo {}\n\
                   set o [Foo new]\n\
                   oo::objdefine $o {\n    \
                       method greet {} { helper }\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(
            r.command_invocations.iter().any(|i| i.name == "helper"),
            "the call inside the per-object method body should be analysed: {:?}",
            r.command_invocations
                .iter()
                .map(|i| &i.name)
                .collect::<Vec<_>>(),
        );
    }

    // resolve_alias

    #[test]
    fn resolve_alias_passthrough_for_non_alias() {
        let mut a = Analyser::new();
        let (target, args) = a.resolve_alias("puts", &["hello".to_string()], &[]);
        assert_eq!(target, "puts");
        assert_eq!(args, vec!["hello".to_string()]);
    }

    #[test]
    fn resolve_alias_substitutes_target_and_prepended_args() {
        let mut a = Analyser::new();
        a.command_aliases.insert(
            "::logerr".to_string(),
            ("puts".to_string(), vec!["stderr".to_string()]),
        );
        let (target, args) = a.resolve_alias("logerr", &["hello".to_string()], &[]);
        assert_eq!(target, "puts");
        assert_eq!(args, vec!["stderr".to_string(), "hello".to_string()]);
    }

    // handle_oo_class_command

    #[test]
    fn handle_oo_class_create_records_class() {
        let mut a = Analyser::new();
        // Metaclass recognition is now registry-trait-driven (`IS_OO_METACLASS`).
        a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
        let handled = a.handle_oo_class_command(
            "oo::class",
            &["create".to_string(), "MyClass".to_string()],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 16)),
                esc_tok(span(17, 24)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.all_classes.contains_key("::MyClass"));
        let cls = &a.result.all_classes["::MyClass"];
        assert_eq!(cls.name, "MyClass");
    }

    #[test]
    fn handle_oo_class_create_with_body() {
        // arg_tokens stripped of cmd_name (matching the
        // ``process_command`` dispatch convention).
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_for_dialect("tcl"));
        let handled = a.handle_oo_class_command(
            "oo::class",
            &[
                "create".to_string(),
                "MyClass".to_string(),
                "method m {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 16)),
                esc_tok(span(17, 24)),
                str_tok(span(25, 41)),
            ],
            &[],
        );
        assert!(handled);
        assert_eq!(a.result.all_classes["::MyClass"].body_span, span(25, 41));
    }

    #[test]
    fn handle_oo_class_wrong_subcommand_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_class_command(
            "oo::class",
            &["destroy".to_string(), "MyClass".to_string()],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 17)),
                esc_tok(span(18, 25)),
            ],
            &[],
        );
        assert!(!handled);
        assert!(a.result.all_classes.is_empty());
    }

    // handle_oo_class_command body walking

    #[test]
    fn analyse_oo_class_body_records_superclass_and_methods() {
        // End-to-end: ``oo::class create Sub`` with a body
        // declaring a superclass and a method.  After analyse
        // ``::Sub`` should carry both fields.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create Sub { superclass ::Base\nmethod greet {} { puts hi } }",
            "tcl",
        );
        assert!(r.all_classes.contains_key("::Sub"));
        let cls = &r.all_classes["::Sub"];
        assert_eq!(cls.superclasses, vec!["::Base"]);
        assert!(cls.methods.contains_key("greet"));
        assert_eq!(cls.methods["greet"].kind, "method");
    }

    #[test]
    fn analyse_oo_class_body_records_classmethod_and_mixin() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { mixin ::M\nclassmethod build {} { return ok } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.mixins, vec!["::M"]);
        assert!(cls.class_methods.contains_key("build"));
        assert!(!cls.methods.contains_key("build"));
    }

    // handle_oo_define_command body walking

    #[test]
    fn analyse_oo_define_body_extends_existing_class() {
        // ``oo::class create C {}`` followed by ``oo::define
        // C { method m {} {} }`` — the method ends up in the
        // already-recorded class.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C {}\noo::define C { method m {} {} }",
            "tcl",
        );
        assert!(r.all_classes.contains_key("::C"));
        let cls = &r.all_classes["::C"];
        assert!(cls.methods.contains_key("m"));
    }

    #[test]
    fn oo_define_extends_class_named_like_a_create_subcommand() {
        // Regression: `oo::define` / `oo::objdefine` carry a TclOO
        // `definition_body` but no `IS_OO_METACLASS` trait, so a class literally
        // named `create` must be *extended* by `handle_oo_define_command`, not
        // stolen by `handle_oo_class_command` (which would record a bogus class
        // named `method`).  The class-creator check requires the metaclass
        // trait, which define/objdefine lack.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::define create method greet {} { return hi }", "tcl");
        // The real class is `create`, with method `greet` — not a bogus class.
        let cls = r
            .all_classes
            .get("::create")
            .expect("class `create` recorded by oo::define");
        assert!(
            cls.methods.contains_key("greet"),
            "method greet must be recorded on class `create`: {:?}",
            cls.methods.keys().collect::<Vec<_>>()
        );
        assert!(
            !r.all_classes.contains_key("::method"),
            "no bogus class named `method` should be created",
        );
    }

    #[test]
    fn analyse_oo_define_inline_form_extends_class() {
        // ``oo::define C method m {} {}`` — inline form,
        // single subcommand.  Works whether or not the class
        // was previously declared (creates a stub if absent).
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::define MyClass method greet {} { puts hi }", "tcl");
        assert!(r.all_classes.contains_key("::MyClass"));
        let cls = &r.all_classes["::MyClass"];
        assert!(cls.methods.contains_key("greet"));
    }

    #[test]
    fn two_dynamic_oo_define_targets_sharing_a_variable_name_never_merge() {
        // TP — regression for a bug found by differential audit against
        // clay.tcl's `current_class`-based Ensemble DSL: two lexically
        // unrelated procs each write `oo::define $class method ... ` with a
        // same-named local `class` (an unremarkable choice for this exact
        // idiom) and each add their own method. Before the fix both calls
        // computed the identical raw-text key `$class`, so the second
        // `remove`+`insert` round-trip silently merged the first proc's
        // method into the second's ClassDef (and vice versa via the
        // dual-registration into `scope.classes`), producing a document
        // symbol whose child method range sits outside its own parent's.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc addFoo {} {\n    set class ::A\n    oo::define $class method foo {} { return foo }\n}\n\
             proc addBar {} {\n    set class ::B\n    oo::define $class method bar {} { return bar }\n}\n",
            "tcl8.6",
        );
        let synthetic: Vec<_> = r
            .all_classes
            .iter()
            .filter(|(k, _)| k.contains("@dynclass@"))
            .collect();
        assert_eq!(
            synthetic.len(),
            2,
            "each dynamic oo::define call site gets its own ClassDef: {:?}",
            r.all_classes.keys().collect::<Vec<_>>(),
        );
        let has_foo_only = synthetic
            .iter()
            .any(|(_, c)| c.methods.contains_key("foo") && !c.methods.contains_key("bar"));
        let has_bar_only = synthetic
            .iter()
            .any(|(_, c)| c.methods.contains_key("bar") && !c.methods.contains_key("foo"));
        assert!(
            has_foo_only && has_bar_only,
            "neither call site's method may leak into the other's ClassDef: {:?}",
            synthetic
                .iter()
                .map(|(k, c)| (k, c.methods.keys().collect::<Vec<_>>()))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn dynamic_oo_define_target_does_not_touch_a_same_named_literal_class() {
        // FP guard — a literal class that happens to share source text with
        // an unrelated dynamic target (`$Widget` vs a real class literally
        // named `Widget`) must never be found/extended by the dynamic call:
        // the synthetic key can never collide with a real qualified name.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Widget {\n    method real {} {}\n}\n\
             proc addDynamic {} {\n    set class Widget\n    oo::define $class method dynamic {} {}\n}\n",
            "tcl8.6",
        );
        let widget = &r.all_classes["::Widget"];
        assert!(widget.methods.contains_key("real"), "{widget:?}");
        assert!(
            !widget.methods.contains_key("dynamic"),
            "the dynamic call must not extend the literal same-named class: {widget:?}",
        );
    }

    // handle_oo_define_command

    #[test]
    fn handle_oo_define_recognises_canonical_form() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_define_command(
            "oo::define",
            &["MyClass".to_string(), "method m {} {}".to_string()],
            &[],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_oo_define_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_define_command("oo::define", &[], &[], &[]);
        assert!(!handled);
    }

    // handle_incr_command

    #[test]
    fn handle_incr_defines_var() {
        let mut a = Analyser::new();
        a.handle_incr_command(&["counter".to_string()], &[esc_tok(span(0, 7))], &[]);
        assert!(a.result.global_scope.variables.contains_key("counter"));
        // incr-defined vars warn_if_unused = true (so a `set`-only
        // var pattern still fires; an `incr`-only-no-read does too).
        assert!(a.result.global_scope.variables["counter"].warn_if_unused);
    }

    #[test]
    fn handle_incr_with_amount() {
        let mut a = Analyser::new();
        a.handle_incr_command(
            &["counter".to_string(), "5".to_string()],
            &[esc_tok(span(0, 7)), esc_tok(span(8, 9))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("counter"));
    }

    #[test]
    fn handle_incr_no_args_no_op() {
        let mut a = Analyser::new();
        a.handle_incr_command(&[], &[], &[]);
        assert!(a.result.global_scope.variables.is_empty());
    }

    // ClassDef extended fields + UnknownProcInfo

    #[test]
    fn analyse_oo_class_records_metaclass_from_command_name() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::class create C {}", "tcl");
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.metaclass, "oo::class");
    }

    #[test]
    fn analyse_oo_class_body_records_constructors_and_destructor() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { constructor args { puts ctor }\ndestructor { puts dtor } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.constructors.len(), 1);
        assert_eq!(cls.constructors[0].kind, "constructor");
        assert!(cls.destructor.is_some());
        assert_eq!(cls.destructor.as_ref().unwrap().kind, "destructor");
    }

    #[test]
    fn analyse_oo_class_body_records_variables_filters_exports() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { variable x y\nfilter log\nexport foo bar\nunexport hidden }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.variables, vec!["x", "y"]);
        assert_eq!(cls.filters, vec!["log"]);
        assert!(cls.exports.contains("foo"));
        assert!(cls.exports.contains("bar"));
        assert!(cls.unexports.contains("hidden"));
    }

    #[test]
    fn analyse_oo_class_body_records_property_def() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { property colour -kind readwrite -get { return red } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        let pd = cls.properties.get("colour").expect("colour recorded");
        assert_eq!(pd.kind, "readwrite");
        assert!(pd.has_getter);
        assert!(!pd.has_setter);
    }

    #[test]
    fn analyse_unknown_proc_records_dispatch_targets_end_to_end() {
        // End-to-end: a ``proc unknown {cmd args} {...}`` with
        // an exact-match switch should populate
        // ``result.unknown_proc_info`` with the arm labels as
        // dispatch targets.  This is what gates W123.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } bar { return 2 } } }",
            "tcl",
        );
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(!info.empty_stub);
        assert!(info.dispatch_targets.contains("foo"));
        assert!(info.dispatch_targets.contains("bar"));
    }

    #[test]
    fn analyse_without_unknown_proc_leaves_unknown_proc_info_none() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc foo {} { return 1 }", "tcl");
        assert!(r.unknown_proc_info.is_none());
    }

    #[test]
    fn analyse_unknown_proc_with_empty_body_marks_empty_stub() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc unknown {cmd args} {}", "tcl");
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(info.empty_stub);
    }

    #[test]
    fn analyse_qualified_unknown_proc_also_populates_info() {
        // ``::tcl::unknown`` (the canonical fully-qualified
        // name) should trigger detection too.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc ::tcl::unknown {cmd args} { exec $cmd }", "tcl");
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(info.has_exec);
    }

    // stray-close-bracket recovery

    #[test]
    fn analyse_top_level_repairs_stray_close_bracket() {
        // ``set x string]`` is a typo for ``set x [string ...]``.
        // The recovery should rewrite the third argv entry into
        // a virtual ``CMD`` token before dispatch so the var
        // record is registered with the recovered shape.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("set x string]", "tcl");
        // ``x`` ends up in scope as a single-arg ``set`` (a var
        // read), not as a two-arg ``set`` with the broken text
        // — recovery yields the synthetic ``[string]`` command
        // word so dispatch sees the intended shape.
        assert!(r.global_scope.variables.contains_key("x"));
    }

    // unknown_proc_info / package require

    #[test]
    fn analyse_records_package_require() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require Tcl 8.6", "tcl");
        assert_eq!(r.package_requires.len(), 1);
        let p = &r.package_requires[0];
        assert_eq!(p.name, "Tcl");
        assert_eq!(p.version.as_deref(), Some("8.6"));
        assert!(!p.conditional);
    }

    #[test]
    fn analyse_records_package_require_exact_flag() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require -exact Tcl 8.6", "tcl");
        let p = &r.package_requires[0];
        assert_eq!(p.name, "Tcl");
        assert_eq!(p.version.as_deref(), Some("8.6"));
    }

    #[test]
    fn analyse_w123_suppressed_when_package_require_seen() {
        // W123 is suppressed when any package require is on
        // file — package may load arbitrary commands.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require Foo\nbogus_command arg", "tcl");
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    #[test]
    fn analyse_w123_suppressed_when_unknown_proc_chains_original() {
        // ``proc unknown`` that chains the original handler is
        // a *dynamic* shape — W123 is suppressed entirely
        // because runtime can resolve any command name.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { _original_unknown $cmd {*}$args }\nbogus_command arg",
            "tcl",
        );
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    #[test]
    fn analyse_w123_suppressed_when_unknown_proc_calls_exec() {
        // ``exec $cmd`` inside ``unknown`` is a dynamic shape;
        // any command may be a real binary on PATH.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { exec $cmd {*}$args }\nbogus_command arg",
            "tcl",
        );
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    #[test]
    fn analyse_w123_still_fires_outside_explicit_dispatch_targets() {
        // ``proc unknown`` with ONLY explicit dispatch targets
        // (no exec / auto_load / chain / pattern / case-fold)
        // is *not* dynamic — W123 should still fire for
        // commands not in the explicit target set.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nbogus_command arg",
            "tcl",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "W123 expected for ``bogus_command`` outside explicit dispatch targets; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_suppressed_for_explicit_dispatch_target() {
        // ``foo`` is in the explicit dispatch_targets — even
        // for the non-dynamic shape, the per-invocation loop
        // suppresses W123 for it.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nfoo arg",
            "tcl",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123 && d.message.contains("'foo'")),
            "W123 should not fire for command listed in dispatch_targets; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_still_fires_for_empty_unknown_stub() {
        // An empty ``unknown`` stub resolves nothing — W123
        // should still emit.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc unknown {cmd args} {}\nbogus_command arg", "tcl");
        // ``bogus_command`` should be flagged.
        assert!(r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    // deep_param_traits plumbing
    //
    // These tests pin the contract that flipping
    // `Analyser::deep_param_traits` actually changes the
    // `ProcDef.param_traits` surface: the shallow pass alone
    // misses traits hidden inside braced bodies, the deep pass
    // catches them, and the union of both is what the analyser
    // exposes.  The call-graph / symbol-graph / dataflow-graph /
    // semantic-graph builders flip this on.

    #[test]
    fn analyse_with_deep_param_traits_surfaces_nested_eval() {
        // `$body` is buried inside a `foreach` body — the
        // shallow pass walks only top-level commands and misses
        // it.  Flipping `deep_param_traits` on must surface
        // `Eval`.
        let source = "proc f {items body} {\n  foreach item $items {\n    uplevel 1 $body\n  }\n}";
        let mut shallow = crate::analyser::Analyser::new();
        let shallow_r = shallow.analyse(source, "tcl");
        let shallow_proc = shallow_r.all_procs.get("::f").expect("::f proc registered");
        let shallow_body_traits = shallow_proc.param_traits.get("body");
        assert!(
            !shallow_body_traits.is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Eval)),
            "shallow pass should miss nested Eval, got {shallow_body_traits:?}",
        );

        let mut deep = crate::analyser::Analyser::new();
        deep.deep_param_traits = true;
        let deep_r = deep.analyse(source, "tcl");
        let deep_proc = deep_r.all_procs.get("::f").expect("::f proc registered");
        let deep_body_traits = deep_proc
            .param_traits
            .get("body")
            .expect("body traits present with deep_param_traits on");
        assert!(
            deep_body_traits.contains(&crate::analyser::ProcArgTrait::Eval),
            "deep pass should surface nested Eval, got {deep_body_traits:?}",
        );
    }

    #[test]
    fn analyse_with_deep_param_traits_off_matches_shallow() {
        // For procs without nested-body usage, deep + shallow
        // produce the same trait map.  This pins that the deep
        // pass doesn't accidentally lose any shallow trait.
        let source = "proc g {body} { uplevel 1 $body }";
        let mut shallow = crate::analyser::Analyser::new();
        let shallow_r = shallow.analyse(source, "tcl");

        let mut deep = crate::analyser::Analyser::new();
        deep.deep_param_traits = true;
        let deep_r = deep.analyse(source, "tcl");

        let shallow_traits = shallow_r
            .all_procs
            .get("::g")
            .expect("::g registered")
            .param_traits
            .clone();
        let deep_traits = deep_r
            .all_procs
            .get("::g")
            .expect("::g registered")
            .param_traits
            .clone();
        assert_eq!(
            shallow_traits, deep_traits,
            "deep + shallow should match for top-level-only bodies",
        );
    }

    #[test]
    fn uplevel_nonzero_isolates_body_vars_from_proc() {
        // `uplevel 1 {set x 5}` runs in the caller's frame, not the enclosing
        // proc's — its `x` must not merge into the proc's locals (they are
        // different variables), and it lands in an isolated `Uplevel` child
        // scope tagged with the level word.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc f {} {\n    uplevel 1 {set x 5}\n    set y 1\n}\n",
            "tcl8.6",
        );
        let f_scope = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "f")
            .expect("f proc scope");
        assert!(f_scope.variables.contains_key("y"), "y is a proc local");
        assert!(
            !f_scope.variables.contains_key("x"),
            "uplevel body's `x` must not merge into the proc scope",
        );
        let up = f_scope
            .children
            .iter()
            .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
            .expect("isolated uplevel scope");
        assert_eq!(up.name, "1", "the level word tags the frame");
        assert!(up.variables.contains_key("x"));
    }

    #[test]
    fn nested_def_in_qualified_encloser_does_not_overwrite_global() {
        // `proc a::outer { proc helper ... }`: the nested `helper` homes to the
        // encloser's *defining* namespace (`::a::helper`), not the lexical
        // global — so the real global `proc helper` is preserved rather than
        // overwritten in `all_procs`.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc helper {} { puts global }\nproc a::outer {} {\n    proc helper {} { puts nested }\n}\n",
            "tcl8.6",
        );
        assert!(
            r.all_procs.contains_key("::a::helper"),
            "nested helper must home to ::a::helper: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
        let global = r
            .all_procs
            .get("::helper")
            .expect("global ::helper preserved");
        assert_eq!(
            global.name_span.start(),
            5,
            "::helper must remain the global declaration (line 0), not the nested one",
        );
        // The same for a nested class.
        let mut b = crate::analyser::Analyser::new();
        let rc = b.analyse(
            "oo::class create Widget {}\nproc a::outer {} {\n    oo::class create Widget {}\n}\n",
            "tcl8.6",
        );
        assert!(
            rc.all_classes.contains_key("::a::Widget"),
            "nested class homes to ::a::Widget: {:?}",
            rc.all_classes.keys().collect::<Vec<_>>(),
        );
        assert!(
            rc.all_classes.contains_key("::Widget"),
            "global ::Widget preserved",
        );
    }

    #[test]
    fn variable_global_upvar_skip_dynamic_names() {
        // A `variable` / `global` / `upvar` / `namespace upvar` whose name word
        // is computed (`$dyn` / `[f]`) is not a static declaration — the literal
        // substitution text must not be recorded as a variable.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "namespace eval ns {\n    variable $dyn\n    variable realvar 1\n}\nproc p {} {\n    global $g\n    upvar 1 other $loc\n}\n",
            "tcl8.6",
        );
        let ns = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "ns")
            .expect("ns scope");
        assert!(
            ns.variables.contains_key("realvar"),
            "a static `variable` name is still recorded",
        );
        assert!(
            !ns.variables.contains_key("dyn"),
            "`variable $dyn` must not record `dyn`: {:?}",
            ns.variables.keys().collect::<Vec<_>>(),
        );
        let p = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "p")
            .expect("p scope");
        assert!(
            !p.variables.contains_key("g") && !p.variables.contains_key("loc"),
            "dynamic `global`/`upvar` names must not be recorded: {:?}",
            p.variables.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn uplevel_zero_still_resets_to_global_frame() {
        // `uplevel #0` keeps the global-frame tag so variable resolution
        // resolves outward to the global namespace (not the caller-frame
        // abstention a non-`#0` level uses).
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc g {} {\n    uplevel #0 {set z 5}\n}\n", "tcl8.6");
        let g_scope = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "g")
            .expect("g proc scope");
        let up = g_scope
            .children
            .iter()
            .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
            .expect("uplevel scope");
        assert_eq!(up.name, "#0");
        assert!(up.variables.contains_key("z"));
    }

    // stub-overlay end-to-end
    //
    // These tests pin the contract that the stub overlay built
    // from `# tcl-lsp: stub` directives during `analyse()`
    // propagates into the per-proc `param_traits` map.

    #[test]
    fn analyse_with_stub_overlay_propagates_role_to_param_traits() {
        // The source declares a `# tcl-lsp: stub my_eval
        // {script:body}` directive, then defines a proc that
        // invokes `my_eval $body`.  The body arg's role flows
        // from the stub overlay → `param_traits["body"]
        // .contains(Body)`.
        let source = "\
# tcl-lsp: stubs-begin\n\
# tcl-lsp: stub my_eval {script:body}\n\
# tcl-lsp: stubs-end\n\
proc runs {body} {\n\
    my_eval $body\n\
}\n";
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(source, "tcl");
        let proc = r.all_procs.get("::runs").expect("::runs registered");
        let body_traits = proc
            .param_traits
            .get("body")
            .expect("body param has traits");
        assert!(
            body_traits.contains(&crate::analyser::ProcArgTrait::Body),
            "expected Body trait via stub overlay, got {body_traits:?}",
        );
    }

    #[test]
    fn analyse_without_stub_directive_leaves_body_untyped() {
        // Same proc, no stub directive — without the overlay
        // entry, `my_eval` isn't a known command, so the body
        // arg has no recorded role and `param_traits` is empty
        // for `body`.  This pins that the stub directive (not
        // some background heuristic) is what gives the
        // parameter its `Body` trait.
        let source = "proc runs {body} { my_eval $body }";
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(source, "tcl");
        let proc = r.all_procs.get("::runs").expect("::runs registered");
        assert!(
            !proc
                .param_traits
                .get("body")
                .is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Body)),
            "expected NO Body trait without stub directive, got {:?}",
            proc.param_traits.get("body"),
        );
    }

    // -- issue #1001 follow-up: safe-interp visibility survives `analyse_per_item` --

    /// A **separate** gap found while investigating issue #1001, now fixed
    /// alongside it: `analyse_per_item`'s shell/body-pass split (`per_item.rs`)
    /// defers *every* proc/method body — including one nested inside a
    /// tracked `interp eval` safe-interpreter body — to an isolated second
    /// pass (`DeferredBody` / `analyse_proc_body_isolated`). Before this fix
    /// that pass carried no `safe_interp_stack` snapshot at all, so W129
    /// never fired for a hidden call inside *any* proc body nested in a safe
    /// interpreter when analysed incrementally (the live LSP server's
    /// diagnostics path always uses `analyse_per_item`) — even a
    /// directly-written call, with no bracket-substitution indirection
    /// whatsoever. `DeferredBody::safe_interp_ctx` (a flattened snapshot of
    /// the stack's top entry, captured when the body is deferred and
    /// restored in `analyse_proc_body_isolated`) closes this.
    #[test]
    fn safe_interp_w129_reaches_deferred_proc_body_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\ninterp eval s { proc f {} { source foo }; f }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a hidden call inside a deferred proc body warns under \
             incremental (per-item) analysis, matching `Analyser::analyse`: {:?}",
            r.diagnostics
        );
    }

    /// The same fix for a directly-written `apply` lambda body (also
    /// deferred by the shell pass) rather than a `proc` body.
    #[test]
    fn safe_interp_w129_reaches_deferred_apply_body_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\ninterp eval s { apply {{} { source foo }} }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a hidden call inside a deferred apply-lambda body warns under \
             incremental (per-item) analysis: {:?}",
            r.diagnostics
        );
    }

    /// The same fix for a `TclOO` method body (also deferred by the shell pass
    /// via a distinct push site in `oo.rs`).
    #[test]
    fn safe_interp_w129_reaches_deferred_method_body_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\n\
             interp eval s {\n\
                 oo::class create C { method m {} { source foo } }\n\
                 [C new] m\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a hidden call inside a deferred TclOO method body warns under \
             incremental (per-item) analysis: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: a deferred proc body outside any safe interpreter must not
    /// warn — the new `safe_interp_ctx` snapshot must stay `None` and inert
    /// for the overwhelming common case.
    #[test]
    fn deferred_proc_body_outside_any_safe_interp_is_untouched_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item("proc f {} { source foo }\nf\n", "tcl8.6");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "no safe interpreter is involved, so no W129 can ever fire: {:?}",
            r.diagnostics
        );
    }

    /// A narrower, separate limitation the `safe_interp_ctx` fix above does
    /// **not** cover, found while adding it: a *nested* local redefinition —
    /// `proc source {…}` written **inside** a proc body that is itself
    /// deferred, redefining a name a *later statement in the same body* then
    /// calls — does not suppress W129 the way an identical redefinition at
    /// the top level of a tracked `interp eval` body already does (see
    /// `safe_interp_w129_redefined_command_not_flagged_through_indirection_1001`,
    /// which is unaffected — it never defers a body at all). Root cause:
    /// `mark_locally_defined_in_enclosing_interp` also requires
    /// `self.interp_path_stack` / `self.interpreters` to recognise the
    /// current interpreter, and `safe_interp_ctx` only snapshots
    /// `safe_interp_stack` (sufficient for the gate check itself, per that
    /// field's doc) — not those two, which `analyse_proc_body_isolated`'s
    /// fresh `Analyser` never seeds. Fixing this fully would mean threading
    /// the interpreter identity (not just its visibility snapshot) through
    /// `DeferredBody` too; out of scope here since the primary miss (a
    /// hidden call inside a deferred body not warning *at all*) is what this
    /// fix targets, and this narrower shadowing case is `SAFE_INTERP_HIDDEN`-
    /// specific low-severity — it only means an occasional cosmetic false
    /// positive on a rare pattern (redefining a hidden builtin's name
    /// *inside* the very body that also calls it), never a missed real
    /// violation. Pinned so a future contributor extending
    /// `DeferredBody` has a red test.
    #[test]
    fn safe_interp_w129_nested_redefinition_inside_deferred_body_still_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\n\
             interp eval s {\n\
                 proc f {} { proc source {} { return ok }; source }\n\
                 f\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "known narrower gap: a redefinition nested inside a deferred \
             body doesn't suppress W129 for a later call in the same body, \
             under per-item analysis specifically; flip this assertion if a \
             future fix threads interpreter identity through `DeferredBody`: {:?}",
            r.diagnostics
        );
    }

    /// Pins issue #1001's own second reported repro case verbatim:
    /// `{*}[list apply {...} $x]` combines *two* indirection mechanisms at
    /// once — `{*}`-expansion of this command's own effective head
    /// (`check_indirect_hiding`'s `{*}[list HEAD ...]` resolution) *and*
    /// the resolved head being `apply` (triggering the lambda-body
    /// recursion into `handle_apply_command`), so the hidden `source`
    /// nested inside the lambda body must still draw W129.
    #[test]
    fn safe_interp_w129_expand_list_quoted_apply_lambda_body_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { {*}[list apply {dir {source $dir/evil.tcl}} $env(HOME)] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `source` nested inside a `{{*}}[list apply ...]`-invoked \
             lambda body warns: {:?}",
            r.diagnostics
        );
    }

    /// Pins issue #1001's third reported repro case verbatim: the
    /// `package ifneeded` deferred script followed by the actual
    /// `package require` that triggers it — confirms the fix holds when the
    /// deferred script is later invoked, not just when it is merely
    /// declared.
    #[test]
    fn safe_interp_w129_list_quoted_apply_package_ifneeded_then_require_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded evil 1.0 [list apply {dir {source $dir/evil.tcl}} $env(HOME)]\n\
                 package require evil\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "the deferred script's hidden `source` warns once `package \
             require` is present too: {:?}",
            r.diagnostics
        );
    }

    // -- issue #1001 follow-up: namespace ensemble -map redirection --------

    /// TP: `namespace ensemble create -command myens -map {go source}` then
    /// `myens go ...` — the ensemble redirects `go` to the hidden `source`,
    /// so the call must warn exactly like a direct `source` call would, even
    /// though `myens` itself is never a registry name.
    #[test]
    fn safe_interp_w129_ensemble_create_map_redirect_to_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble -map redirect to a hidden command warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the same redirect declared via `namespace ensemble configure NAME
    /// -map {...}` (previously silently ignored entirely — only `create`
    /// was handled) rather than at `create` time.
    #[test]
    fn safe_interp_w129_ensemble_configure_map_redirect_to_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens\n\
                 namespace ensemble configure myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble -map redirect declared via `configure` warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the ensemble's default naming (no `-command`, so the command is
    /// the enclosing namespace's own name) also resolves the redirect.
    #[test]
    fn safe_interp_w129_ensemble_default_name_map_redirect_to_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace eval myns { namespace ensemble create -map {go source} }\n\
                 myns go pkg.tcl\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble redirect under its default (namespace) name warns: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: an ensemble `-map` redirect to a *safe* command must not
    /// warn — this fix widens W129's recall, it must not start flagging
    /// ordinary ensemble dispatch.
    #[test]
    fn safe_interp_w129_ensemble_map_redirect_to_safe_command_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go puts}\n\
                 myens go hi\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble redirect to a safe command must not warn: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: the exact same ensemble-redirect shape outside any safe
    /// interpreter must not warn (and, since the whole mechanism is gated on
    /// a non-empty `safe_interp_stack`, must not do anything at all).
    #[test]
    fn ensemble_map_redirect_outside_any_safe_interp_is_untouched_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace ensemble create -command myens -map {go source}\n\
             myens go pkg.tcl\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "no safe interpreter is involved, so no W129 can ever fire: {:?}",
            r.diagnostics
        );
    }
}
