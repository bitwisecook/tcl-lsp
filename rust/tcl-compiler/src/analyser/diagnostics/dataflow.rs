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

//! Control- and data-flow diagnostics derived from the per-function CFG and
//! SSA form.
//!
//! These checks run from the per-function dispatcher over the compilation
//! unit: dead stores (W220), unused variables (W211) and parameters (W214),
//! read-before-set (W210) including the phi-from-undef and provably-unset
//! variants, unset-on-possibly-undefined (W213), a paste-error fingerprint
//! (H300), constant branches and switch arms (I230/I231), channel-argument
//! validation (W126), divide-by-zero (W233), interval-bounds violations, an
//! invalid IP-address literal at its def site (W124), the racy `static::`
//! cross-event flow (IRULE4005), and the flow-sensitive renamed-command
//! check (W128).

use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;

use rustc_hash::{FxHashMap, FxHashSet};

use super::helpers::{
    UndefSuppression, block_dominated_by, build_phi_undef_index, collect_existence_guards,
    find_dotted_quads, is_ident_continue, is_word_byte, phi_can_undef, source_slice,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::analyser::utils::param_name_spans;
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{ExprNode, UnaryOp};

/// The read-only name/guard/suppression context for the `return`-value
/// phi-from-undef W210 pass ([`Analyser::emit_return_phi_undef_w210`]):
/// the proc parameters, dominating existence guards, scope aliases, the
/// caller-supplied known-defined / defined-var sets, the executable block
/// set, and the undef-suppression model.  Bundled to keep the emitter under
/// the argument limit.
/// The phi-from-undef indices ([`build_phi_undef_index`] output) borrowed for
/// one W210 pass, so the per-read check can be split into its own helper.
struct PhiUndefIndex<'a> {
    phi_def: &'a super::helpers::PhiDefMap,
    phi_block: &'a super::helpers::PhiBlockMap,
    killed: &'a FxHashSet<(String, crate::ssa::Version)>,
}

/// The read-only scope and suppression facts for the version-0 / statement
/// W210 pass. Keeping this alongside [`ReturnUndefCtx`] prevents the two W210
/// emitters from acquiring separate, subtly divergent entry-scope contracts.
pub(super) struct ReadBeforeSetCtx<'a> {
    pub initial_global: bool,
    pub global_aliases: &'a HashSet<String>,
    pub defined_vars: &'a HashSet<String>,
    pub scope_aliases: &'a HashSet<String>,
    pub extra_known_defined: &'a HashSet<String>,
    pub supp: &'a UndefSuppression,
}

pub(super) struct ReturnUndefCtx<'a> {
    pub initial_global: bool,
    pub global_aliases: &'a HashSet<String>,
    pub dialect: Option<SurfaceQuery<'a>>,
    pub params: &'a HashSet<&'a str>,
    pub exists_guards: &'a [(String, crate::cfg::BlockId)],
    pub scope_aliases: &'a HashSet<String>,
    pub extra_known_defined: &'a HashSet<String>,
    pub defined_vars: &'a HashSet<String>,
    pub considered: &'a HashSet<crate::cfg::BlockId>,
    pub supp: &'a UndefSuppression,
}

/// Registry-owned startup lifecycle facts for one SSA variable version.
#[derive(Clone, Copy)]
struct StartupReadFacts {
    readable: bool,
    initially_bound: bool,
    lazy_read: bool,
}

fn startup_read_facts(
    name: &str,
    version: crate::ssa::Version,
    killed: bool,
    initial_global: bool,
    global_aliases: &HashSet<String>,
    dialect: Option<SurfaceQuery<'_>>,
) -> StartupReadFacts {
    let global_binding =
        super::helpers::has_global_startup_binding(name, initial_global, global_aliases);
    let startup_name = super::helpers::startup_var_name(name);
    StartupReadFacts {
        readable: global_binding
            && version == 0
            && !killed
            && tcl_registry::special_vars::is_readable_at_startup(startup_name, dialect),
        initially_bound: global_binding
            && version == 0
            && tcl_registry::special_vars::is_initially_bound(startup_name, dialect),
        lazy_read: global_binding
            && tcl_registry::special_vars::is_lazily_readable(startup_name, dialect),
    }
}

/// Facts used while recording the read sites of one undef def-use chain.
struct W210ChainCtx<'a> {
    exists_guards: &'a [(String, crate::cfg::BlockId)],
    supp: &'a UndefSuppression,
    startup: StartupReadFacts,
}

impl Analyser {
    /// **W128.** Flag a call to a command that was
    /// renamed or deleted earlier in the same file — it falls through to
    /// the `unknown` handler.
    ///
    /// Backed by the flow-sensitive command-binding lattice
    /// ([`crate::command_binding`]).  The lattice is seeded with every
    /// module procedure (canonically qualified) as `Proc` so a proc
    /// defined inside a `namespace eval` block — whose top-level CFG never
    /// sees the full qname — is still known, matching the optimiser's
    /// gating view.  A call fires W128 only when its resolved binding is
    /// `Opaque` *and* its name was actually perturbed somewhere in this
    /// file (`rebound_names`); a merely-undefined external command (always
    /// opaque, never rebound) does not.  A dynamic mutation collapses the
    /// lattice to the wildcard ⊤, under which every binding resolves to
    /// `Unknown` (not `Opaque`), so W128 conservatively goes quiet.
    pub(super) fn emit_w128_renamed_command(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::command_binding::{Binding, BindingKind, analyse_command_binding};
        use crate::ir::Statement;
        use crate::naming::normalise_qualified_name as nqn;

        let cfg = &cu.top_level.cfg;
        let seed: Vec<(String, Binding)> = cu
            .ir_module
            .procedures
            .keys()
            .map(|q| {
                (
                    q.clone(),
                    Binding {
                        kind: BindingKind::Proc,
                        target: Some(q.clone()),
                    },
                )
            })
            .collect();
        let binding = analyse_command_binding(cfg, registry, &seed);
        let rebound = binding.rebound_names();
        if rebound.is_empty() {
            return;
        }
        // Reverse-postorder for deterministic diagnostic ordering.
        for block_id in cfg.reverse_postorder() {
            let Some(block) = cfg.blocks.get(&block_id) else {
                continue;
            };
            for (idx, stmt) in block.statements.iter().enumerate() {
                let Statement::Call { command, span, .. } = stmt else {
                    continue;
                };
                // The mutation commits after this invocation's successful
                // completion, so the point-wise binding query naturally sees
                // the command's incoming binding here.  No mutator-name
                // exclusion is required.
                if command.is_empty() {
                    continue;
                }
                if binding.binding_at(block_id, idx, command).kind != BindingKind::Opaque {
                    continue;
                }
                if !rebound.contains(&nqn(command)) {
                    continue; // never bound here → an ordinary external command
                }
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W128,
                        *span,
                        format!(
                            "Command '{command}' was renamed or deleted earlier in this \
file; this call falls through to the 'unknown' handler."
                        ),
                        Severity::Warning,
                    ));
            }
        }
    }

    /// Statements whose dead-store **W220** hint should be **suppressed**
    /// because their array-element / dict-path def place is observed by some
    /// read in the function.
    ///
    /// Name-level SSA folds `a(k)` / `a(j)` / `$a` to the base name `a`, so a
    /// later `set a(j) 2` looks like it overwrites `set a(k) 1` before any read
    /// — a false dead store when `a(k)` is in fact read.  Delegates to the
    /// shared [`crate::place_bridge::element_writes_observed_by_reads`] (also
    /// used by the optimiser's O109), which resolves each element write to a
    /// [`Place`](crate::place::Place) and consults the over-approximating
    /// [`overlap`](crate::place::overlap).  Scalars keep the precise name-level
    /// verdict (they don't fold), so a genuine `set x 1; set x 2; puts $x` dead
    /// store still fires.  Empty when no registry is bound (e.g. the bare
    /// `emit_cfg_ssa_diagnostics` test path).
    fn place_suppressed_dead_stores(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) -> std::collections::HashSet<(String, i32)> {
        self.registry
            .as_deref()
            .map_or_else(Default::default, |reg| {
                crate::place_bridge::element_writes_observed_by_reads(&fu.cfg, &fu.name, reg)
            })
    }

    /// Variable names read inside positions the version-precise SSA `used`
    /// set can't see — `[…]` command substitutions in command arguments,
    /// `expr` values, and `if`/`while`/`for` branch conditions. A write to
    /// such a name is not a dead store even when its SSA version looks
    /// unused.
    fn substitution_hidden_reads(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) -> FxHashSet<String> {
        self.registry
            .as_deref()
            .as_ref()
            .map_or_else(FxHashSet::default, |reg| {
                Self::substitution_hidden_reads_of(fu, reg)
            })
    }

    /// `self`-free core of [`Self::substitution_hidden_reads`] so the explorer's
    /// liveness dead-store pass (which has no `Analyser`) can reuse it.
    pub(crate) fn substitution_hidden_reads_of(
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
    ) -> FxHashSet<String> {
        use crate::var_refs::{VarReferenceScanner, VarScanOptions};
        let mut out = FxHashSet::default();
        // Command-argument + AssignValue substitutions (deep RMW scan minus
        // shallow), already factored out for the optimiser's elimination pass.
        out.extend(crate::optimiser::elimination::collect_rmw_hidden_reads(
            fu, registry,
        ));
        // Branch conditions and expr-valued statements carry their `[…]` in an
        // `ExprNode`, not a word. Walk the AST for `Command` nodes (bracketed
        // substitution text) and scan each — their inner reads are invisible to
        // the version-precise `used` set, so they keep every write alive. A
        // bare `$x` in `if {$x}` is already a version-precise condition use, so
        // it is not collected here.
        let mut deep = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: true,
            recurse_cmd_substitutions: true,
            include_reads_before_write: true,
            element_qualified: false,
        });
        let mut cmd_texts: Vec<String> = Vec::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                if let crate::ir::Statement::AssignExpr { expr, .. }
                | crate::ir::Statement::ExprEval { expr, .. } = stmt
                {
                    collect_expr_command_texts(expr, &mut cmd_texts);
                }
            }
            if let Some(crate::cfg::Terminator::Branch { condition, .. }) = &block.terminator {
                collect_expr_command_texts(condition, &mut cmd_texts);
            }
        }
        for text in &cmd_texts {
            out.extend(deep.scan_word(text, registry));
        }
        out
    }

    /// W220 — dead-store hint.
    ///
    /// A *dead store* is an
    /// assignment whose value is overwritten before being read —
    /// some other SSA version of the same variable is live, so
    /// this version's value never reaches a user.
    ///
    /// Walks every dead [`Statement`](crate::ir::Statement) chain
    /// in `fu.def_use`, checks that another version of the same
    /// variable has live uses, and emits a Hint at the dead
    /// statement's span.  When the variable's name has a
    /// case-insensitive twin among `defined_vars`, the message
    /// includes a "did you mean…?" suggestion.
    ///
    /// Filters applied:
    ///
    /// 1. **SCCP-unreachable blocks** — definitions in blocks
    ///    SCCP proved unreachable are reported as O107 by the
    ///    optimiser and intentionally suppressed here so we
    ///    don't double-up on dead-code calls.
    /// 2. **Scope aliases** (`global` / `upvar`) — writes are
    ///    visible in another scope; the local "no use" verdict
    ///    is unsafe.
    /// 3. **Cross-event vars** — for `pkgIndex.tcl` `$dir` and
    ///    iRules `::when::*` cross-event defs/imports, a write
    ///    in one event may be read in another at runtime.
    /// 4. **Globals (`::`-prefixed)** — externally consumed.
    /// 5. **Side-effecting stores** — only `AssignConst`,
    ///    `AssignValue` without `[`, and `AssignExpr` without a
    ///    command call are considered.  `Call.defs`, `Incr`, and
    ///    other side-effecting writes shouldn't be flagged
    ///    because removing the assignment would also drop the
    ///    side effect.
    pub(super) fn emit_dead_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use crate::ir::Statement;
        use crate::ir_helpers::expr_has_command;
        use std::fmt::Write as _;
        // A dynamic read (`[set $name]`, `subst $tmpl`) can observe *any*
        // store, so "this assignment is never read" is unprovable anywhere in
        // the function (issue #923 audit idx 2/64).  Abstain toward silence.
        if fu.dynamic_names.reads {
            return;
        }
        let hidden_reads = self.substitution_hidden_reads(fu);
        // Array-element / dict-path writes the
        // name-level SSA mis-folds but that a read actually observes.
        let place_suppressed = self.place_suppressed_dead_stores(fu);
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, _version) = &chain.key;
            // A name read inside a command substitution / expr / branch
            // condition the version-precise `used` set can't see keeps every
            // write of it alive (`set i 0` before `[incr i $j]`). Suppress at
            // name level.
            if hidden_reads.contains(var) {
                continue;
            }
            // Globals (``::``-prefixed) are externally consumed.
            if var.starts_with("::") {
                continue;
            }
            // Interpreter-provided special variables (``auto_path``, ``env``,
            // ``tcl_precision``, …) are read by the runtime / auto-loader even
            // when the script never reads them back, so ``set auto_path …`` is
            // not a dead store.  Dialect-aware: the iRules set differs (issue
            // #831).
            if tcl_registry::special_vars::is_externally_read(
                crate::naming::normalise_var_name(var),
                Some(self.analysis_context().context().authoring_query()),
            ) {
                continue;
            }
            // A synthetic may-def (base refresh / element fan) is not a
            // write the user made — never a reportable dead store.
            if fu.ssa.is_synthetic_def(
                &chain.definition.block,
                chain.definition.statement_index,
                var,
            ) {
                continue;
            }
            // The *direct* base def of a dynamic-key element write
            // (`set a($k) 9` defs base `a` directly): its liveness is
            // carried by the fanned element chains, which exact-name
            // liveness can't see — never report the base.
            if !var.contains('(') && def_is_element_write(fu, &chain.definition) {
                continue;
            }
            // Scope-aliased vars (introduced via ``global`` or
            // ``upvar``) write through to a different scope — the
            // local "no use" verdict is unsafe. Policy sets hold *base*
            // names, so an element symbol (`a(k)`) checks its base too.
            let var_base = crate::naming::normalise_var_name(var);
            if scope_aliases.contains(var) || scope_aliases.contains(var_base) {
                continue;
            }
            // Cross-event vars (iRules ``::when::*`` defs/imports
            // or ``pkgIndex.tcl`` ``$dir``) may be read in
            // another event/scope at runtime.
            if cross_event_vars.contains(var) || cross_event_vars.contains(var_base) {
                continue;
            }
            // Suppress dead stores in SCCP-unreachable blocks —
            // O107 already reports the whole block as dead, and
            // re-flagging individual stores inside it adds noise.
            if !fu
                .cfg
                .block_id(&chain.definition.block)
                .is_some_and(|id| fu.sccp.executable_blocks.contains(&id))
            {
                continue;
            }
            // A dead assignment is W220 whether or not the variable is also
            // unused overall: the assignment-level dead store (W220) and,
            // when the variable is never read at all, the variable-level
            // unused hint (W211) are distinct diagnostics with distinct
            // fixes (drop this assignment vs. drop the variable).  Fires
            // on any dead store regardless of other live versions.
            let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            // IR-statement type filter.
            // Only pure assignments are reportable; side-effecting
            // writes (``Call``, ``Incr``, command-substitution
            // values, expressions invoking commands) are skipped
            // because dropping them would also drop the side
            // effect.
            match stmt {
                Statement::AssignConst { .. } => {}
                Statement::AssignValue { value, .. } => {
                    if value.contains('[') {
                        continue;
                    }
                }
                Statement::AssignExpr { expr, .. } => {
                    if expr_has_command(expr) {
                        continue;
                    }
                }
                _ => continue,
            }
            // Suppress when this element write is observed by a read the
            // name-level SSA can't see (place-model overlap).
            if place_suppressed.contains(&(
                chain.definition.block.clone(),
                chain.definition.statement_index,
            )) {
                continue;
            }
            let cmd_span = fu.abs_span(stmt.span());
            if cmd_span.is_empty() {
                continue;
            }
            // Anchor at the variable name (the assignment target), not the
            // command-start column.
            let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
            let mut message = format!("Assignment to '{var}' is never read");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W220,
                    span,
                    message,
                    Severity::Hint,
                ));
        }
    }

    /// Narrow a whole-command span to its assignment-target token (the
    /// second word, `argv[1]`), returning that token's absolute span — or
    /// `None` when it can't be located, so callers fall back to the command
    /// span.  W211 / W220 anchor at the variable-name column, not the command
    /// start.  Re-lexes the command's own source slice (token-based) and
    /// takes the first non-separator word after the command name.
    fn narrow_to_assigned_name(&self, stmt_span: tcl_lexer::Span) -> Option<tcl_lexer::Span> {
        let base = stmt_span.start();
        let slice = source_slice(&self.source, stmt_span)?;
        let toks = tcl_lexer::Lexer::with_source_map(
            tcl_lexer::SourceMap::new(&slice),
            self.lexer_config(),
        )
        .tokenise_all()
        .ok()?;
        let name = toks
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    tcl_lexer::TokenType::Sep
                        | tcl_lexer::TokenType::Eol
                        | tcl_lexer::TokenType::Comment
                )
            })
            .nth(1)?;
        Some(tcl_lexer::Span::new(
            name.span.start() + base,
            name.span.end() + base,
        ))
    }

    /// Narrow a whole-command span to the `$var` read token for *var*,
    /// returning that token's absolute span — or `None` when no matching
    /// `Var` token is found anywhere in the statement (the caller falls
    /// back to the command span).  W210 anchors at the variable read, not
    /// the command-start column.
    ///
    /// The read is frequently nested inside a command substitution
    /// (`set x [cmd $var]`, `if {[llength $var]} …`), so the scan descends
    /// into `Cmd` tokens' inner scripts rather than stopping at the
    /// top-level word walk.  Braced (`Str`) words are literal text — a
    /// `$var` inside one is not a read — so they are never descended.
    fn narrow_to_read_var(&self, stmt_span: tcl_lexer::Span, var: &str) -> Option<tcl_lexer::Span> {
        // De-sigil + drop any array-index suffix so `$a(k)` / `${a}` / `$a`
        // all compare equal to the chain's scalar/element base name.
        fn base(text: &str) -> &str {
            let inner = text.strip_prefix("${").map_or_else(
                || text.strip_prefix('$').unwrap_or(text),
                |i| i.strip_suffix('}').unwrap_or(i),
            );
            inner.split('(').next().unwrap_or(inner)
        }
        /// First `$target` `Var` token within `slice` (whose absolute start
        /// is `abs_base`), descending into command-substitution contents.
        /// `depth` bounds pathological nesting.
        fn find_var(
            slice: &str,
            abs_base: u32,
            target: &str,
            config: tcl_lexer::LexerConfig,
            depth: u8,
        ) -> Option<tcl_lexer::Span> {
            if depth > 4 {
                return None;
            }
            let sm = tcl_lexer::SourceMap::new(slice);
            let toks = tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(slice), config)
                .tokenise_all()
                .ok()?;
            for t in &toks {
                match t.kind {
                    tcl_lexer::TokenType::Var if base(sm.token_text(*t)) == target => {
                        return Some(tcl_lexer::Span::new(
                            t.span.start() + abs_base,
                            t.span.end() + abs_base,
                        ));
                    }
                    // A `[…]` word: recurse into its inner script (the span
                    // covers `[inner` and excludes the closing `]`; the
                    // content starts past the opening bracket).
                    tcl_lexer::TokenType::Cmd => {
                        let content_start = t.span.start() as usize + usize::from(t.content_offset);
                        let content_end = t.span.end() as usize;
                        if let Some(inner) = slice.get(content_start..content_end)
                            && inner.contains('$')
                            && let Some(found) = find_var(
                                inner,
                                abs_base + t.span.start() + u32::from(t.content_offset),
                                target,
                                config,
                                depth + 1,
                            )
                        {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        let target = base(var);
        let slice = source_slice(&self.source, stmt_span)?;
        find_var(&slice, stmt_span.start(), target, self.lexer_config(), 0)
    }

    /// W211 — unused-variable hint.
    ///
    /// Fires when an
    /// assignment's variable has no live uses **and** no other
    /// SSA version is live (so the variable is entirely unused
    /// — distinct from W220's overwritten-before-read case).
    ///
    /// Three filters apply:
    ///
    /// 1. **Scope aliases** (``global`` / ``upvar``) — writes
    ///    are visible in the aliased scope, so a "no local use"
    ///    verdict is unsafe.
    /// 2. **Textual references** — variable names that appear
    ///    inside a ``"$x"`` string interpolation or a
    ///    ``Return`` value are kept live; the def-use builder
    ///    doesn't track those reads.
    /// 3. **Empty spans** — synthetic IR statements with no
    ///    user-visible source text.
    ///
    /// "Did you mean…?" suggestions use case-insensitive
    /// matching against the function's defined-variable set.
    pub(super) fn emit_unused_variable_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        textually_referenced: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;
        // A dynamic read (`foreach v [info locals] {… [set $v] …}`) reaches
        // every local by a name no literal `$x` token spells, so "set but
        // never used" is unprovable (issue #923 audit idx 2).  Abstain toward
        // silence.
        if fu.dynamic_names.reads {
            return;
        }
        // W211 is a per-variable verdict ("the variable is set but never
        // used"), not per-assignment: a variable set several times and never
        // read fires once, at its earliest definition. Collect the earliest
        // reportable span per variable, then emit a single W211 per unused
        // variable.
        let mut earliest: std::collections::HashMap<String, tcl_lexer::Span> =
            std::collections::HashMap::new();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            // A `::`-qualified write (`set ::ns::cfg 1`) is visible to every
            // other scope/file — single-unit dataflow cannot see its readers.
            if var.contains("::") {
                continue;
            }
            if scope_aliases.contains(var) {
                continue;
            }
            // A fully-qualified name (`::cfg`, `::ns::cfg`) explicitly targets
            // the global / a named namespace scope, whose reader may live in
            // another proc, another namespace, or another file that single-unit
            // dataflow can't see. The W220 dead-store and W210 read-before-set
            // passes already exempt `::`-qualified names as externally
            // consumable; W211 does the same so `set ::ns::cfg 1` at the top
            // level isn't flagged unused merely because this unit holds no
            // reader.
            if var.contains("::") {
                continue;
            }
            if textually_referenced.contains(var) {
                continue;
            }
            // A synthetic may-def (base refresh / element fan) is not a
            // write the user made — the element's own chain reports.
            if fu.ssa.is_synthetic_def(
                &chain.definition.block,
                chain.definition.statement_index,
                var,
            ) {
                continue;
            }
            // Interpreter-provided special variables (``auto_path``, ``env``,
            // …) are consumed by the runtime even when the script never reads
            // them, so a bare ``set auto_path …`` is not an unused variable.
            // Dialect-aware via the special-variable registry (issue #831).
            if tcl_registry::special_vars::is_externally_read(
                crate::naming::normalise_var_name(var),
                Some(self.analysis_context().context().authoring_query()),
            ) {
                continue;
            }
            // Only emit when no other SSA version of this var is
            // live — the W220 path handles overwritten cases.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *var && k.1 != *version && !c.is_dead());
            if any_other_live {
                continue;
            }
            let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            // Only pure assignments are reportable as "set but never used".
            // A variable written by a command (`scan` / `binary scan` /
            // `regexp -> capture`, etc.) or a barrier is a command output the
            // user may legitimately ignore; `Statement::Call` /
            // `Statement::Barrier` defs are skipped.  This is deliberate
            // policy, not a gap: a destructuring writer's surplus output
            // (`binary scan $d H2H* type rest` with
            // `rest` unread) is how Tcl spells "ignore the remainder" — there
            // is no `_` placeholder — so flagging it would punish the idiom
            // (review-2 audit, S5).
            if matches!(
                stmt,
                crate::ir::Statement::Call { .. } | crate::ir::Statement::Barrier { .. }
            ) {
                continue;
            }
            // Approach B: CFG span is relative to the unit's `base_offset`.
            let cmd_span = fu.abs_span(stmt.span());
            if cmd_span.is_empty() {
                continue;
            }
            // Anchor at the variable name (the assignment target), not the
            // command-start column.
            let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
            earliest
                .entry(var.clone())
                .and_modify(|s| {
                    if span.start() < s.start() {
                        *s = span;
                    }
                })
                .or_insert(span);
        }
        let mut entries: Vec<(String, tcl_lexer::Span)> = earliest.into_iter().collect();
        entries.sort_by_key(|(_, span)| span.start());
        // A variable that is set-but-never-used gets a W211 at its assignment's
        // name token. The dead-store pass (W220), which ran first, already
        // anchored a "never read" hint at the *same* token for that single
        // assignment — a redundant double-emit. W211 ("never used at all") is
        // the more informative message, so drop the co-located W220. Keyed on
        // the exact span, so a genuinely distinct dead store of a
        // multiply-assigned variable is untouched.
        let w211_spans: std::collections::HashSet<tcl_lexer::Span> =
            entries.iter().map(|(_, span)| *span).collect();
        self.result
            .diagnostics
            .retain(|d| !(d.code == DiagCode::W220 && w211_spans.contains(&d.span)));
        for (var, span) in entries {
            let mut message = format!("Variable '{var}' is set but never used");
            if let Some(similar) = find_case_mismatch(&var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W211,
                    span,
                    message,
                    Severity::Hint,
                ));
        }
    }

    /// H300 — possible paste error (duplicate dead-store with
    /// identical literal).
    ///
    /// When two consecutive
    /// statements in the same block are both dead stores AND
    /// share the same paste-fingerprint
    /// (same variable name + same trimmed literal value), emit
    /// a Hint at the *second* statement's span — the duplicate
    /// is the one that's almost certainly a paste error.
    ///
    /// Variables whose names start with ``_`` are excluded from
    /// the heuristic on the assumption that the leading
    /// underscore signals the user has flagged them as
    /// intentional.
    pub(super) fn emit_possible_paste_error_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::def_use::DefKind;

        // Pre-compute, per block, the set of statement indices
        // that are dead stores.  Walk every dead Statement-kind
        // chain in def_use, bucket by block.
        let mut dead_idx: FxHashMap<&str, FxHashSet<usize>> = FxHashMap::default();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            dead_idx
                .entry(chain.definition.block.as_str())
                .or_default()
                .insert(idx);
        }

        for block in fu.cfg.blocks.values() {
            let Some(dead_indices) = dead_idx.get(block.name.as_str()) else {
                continue;
            };
            // Walk consecutive pairs (idx, idx + 1).  Only the
            // first must be dead — the second's
            // dead-status is irrelevant; what matters is whether
            // the value being assigned matches.
            for idx in 0..block.statements.len().saturating_sub(1) {
                if !dead_indices.contains(&idx) {
                    continue;
                }
                let Some(first) = super::utils::possible_paste_fingerprint(&block.statements[idx])
                else {
                    continue;
                };
                let Some(second) =
                    super::utils::possible_paste_fingerprint(&block.statements[idx + 1])
                else {
                    continue;
                };
                if first != second {
                    continue;
                }
                let (var_name, literal) = first;
                if var_name.starts_with('_') {
                    continue;
                }
                let span = fu.abs_span(block.statements[idx + 1].span());
                if span.is_empty() {
                    continue;
                }
                let pretty = super::utils::format_literal_for_message(&literal);
                let message = format!(
                    "Possible paste error: repeated assignment to '{var_name}' \
                     with static value '{pretty}'; \
                     did you mean to assign a different variable?"
                );
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::H300,
                        span,
                        message,
                        Severity::Hint,
                    ));
            }
        }
    }

    /// Absolute source spans of each formal parameter's *name*, in
    /// declaration order and index-aligned with `ir_proc.params`.
    ///
    /// The parameter-list word is the first word after the proc-name token
    /// (recovered from the recorded [`crate::analyser::types::ProcDef::name_span`]);
    /// its name spans are delegated to [`param_name_spans`] — the same helper
    /// go-to-definition/rename use, so W214's range matches them exactly.
    /// Returns an empty vec when the proc isn't in `all_procs` or the word
    /// can't be isolated, so the caller falls back to the whole-def span.
    fn param_name_spans_for(&self, ir_proc: &crate::ir::Procedure) -> Vec<tcl_lexer::Span> {
        let Some(pdef) = self.result.all_procs.get(&ir_proc.qualified_name) else {
            return Vec::new();
        };
        let bytes = self.source.as_bytes();
        // Skip whitespace between the proc name and the parameter-list word.
        let mut i = pdef.name_span.end() as usize;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        let end = if bytes.get(i) == Some(&b'{') {
            // Braced list — advance to the matching close brace.
            let mut level = 0u32;
            let mut j = i;
            while j < bytes.len() {
                match bytes[j] {
                    b'{' => level += 1,
                    b'}' => {
                        level -= 1;
                        if level == 0 {
                            j += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            j
        } else {
            // Bare single-word parameter list — to the next whitespace.
            let mut j = i;
            while j < bytes.len() && !bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            j
        };
        let Some(raw) = self.source.get(start..end) else {
            return Vec::new();
        };
        param_name_spans(raw, u32::try_from(start).unwrap_or(u32::MAX))
    }

    /// W214 — unused-parameter hint.
    ///
    /// For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the parameter's *name* span (falling back to the
    /// proc's span when the name can't be located), so each unused param
    /// gets its own tight squiggle instead of stacking on the whole proc.
    pub(super) fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
    ) {
        // Empty-body procs (``proc foo {a b} {}``) are signature
        // placeholders — stubs declaring an API whose implementation
        // lives elsewhere.  Every parameter is necessarily "unused"
        // since there is no body to use it, so flagging is pure noise.
        if ir_proc.body.statements.is_empty() {
            return;
        }
        // Per-parameter name spans (index-aligned with `ir_proc.params`); an
        // empty result means we couldn't isolate the list and fall back to the
        // whole-definition span below.
        let param_spans = self.param_name_spans_for(ir_proc);
        let mut unused: Vec<(usize, String)> = Vec::new();
        for (idx, param) in ir_proc.params.iter().enumerate() {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if param == "args" {
                continue;
            }
            // Positional keyword markers: a param *written* as a quoted
            // literal (snit-style ``{"as" ""}``) is a syntactic placeholder
            // consumed by being PRESENT in the call form, not read as a
            // variable.  Flagging it is noise.  The marker lives in the source
            // spelling, not the name: Tcl decodes the quotes away (tclsh 9.0
            // — ``proc p {"as" v} {}`` → ``info args p`` is ``as v``), so the
            // test is on the declaration text the name span covers.
            // Conservative: only a spelling that starts AND ends with ``"``.
            if param_spans
                .get(idx)
                .and_then(|span| self.source.get(span.as_range()))
                .is_some_and(|spelling| {
                    spelling.len() >= 2 && spelling.starts_with('"') && spelling.ends_with('"')
                })
            {
                continue;
            }
            let any_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *param && !c.is_dead());
            if any_live {
                continue;
            }
            // Fallback: the def-use builder doesn't track variable
            // references inside ``[expr {...}]`` command
            // substitutions or arbitrary nested ``[cmd ...]``
            // bodies that don't lower into a structured IR.
            // If the body source contains a ``$param`` /
            // ``${param}`` reference anywhere, treat the parameter
            // as used and skip W214.  Saves the W214 over-emit on
            // ``proc f {x} { return [expr {$x + 1}] }``-style bodies.
            if let Some(body_source) = ir_proc.body_source.as_deref()
                && body_references_param(body_source, param, self.lexer_config())
            {
                continue;
            }
            unused.push((idx, param.clone()));
        }
        if unused.is_empty() {
            return;
        }
        // Dispatch-protocol suppression: when ≥3 peer procs in this
        // namespace share this proc's leading-param signature AND an
        // arity-compatible variable-command dispatcher exists, the leading
        // params are an external contract, not genuinely unused.  Computed
        // only when there is something to report.
        let ns = namespace_of(&ir_proc.qualified_name);
        let leading: Vec<String> = ir_proc
            .params
            .iter()
            .take_while(|p| *p != "args")
            .cloned()
            .collect();
        let protocol_params: HashSet<String> = if !leading.is_empty()
            && self
                .dispatch_protocol_signatures()
                .contains(&(ns, leading.clone()))
        {
            leading.into_iter().collect()
        } else {
            HashSet::new()
        };
        for (idx, param) in unused {
            if protocol_params.contains(&param) {
                continue;
            }
            let message = format!(
                "Parameter '{param}' of proc '{name}' is unused",
                name = ir_proc.qualified_name,
            );
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W214,
                    param_spans.get(idx).copied().unwrap_or(ir_proc.span),
                    message,
                    Severity::Hint,
                ));
        }
    }

    /// Identify `(namespace, leading-param-list)` pairs that look like a
    /// **dispatch protocol** — ≥3 peer procs in the same namespace sharing a
    /// leading-param signature dictated by an arity-compatible
    /// variable-command dispatcher.
    fn dispatch_protocol_signatures(&self) -> HashSet<(String, Vec<String>)> {
        // Group user procs by (namespace, leading-param-tuple stopping at `args`).
        let mut groups: FxHashMap<(String, Vec<String>), usize> = FxHashMap::default();
        for (qname, pdef) in &self.result.all_procs {
            let leading: Vec<String> = pdef
                .params
                .iter()
                .take_while(|p| p.name != "args")
                .map(|p| p.name.clone())
                .collect();
            if leading.is_empty() {
                continue;
            }
            *groups.entry((namespace_of(qname), leading)).or_insert(0) += 1;
        }
        let peer_protos: HashSet<(String, Vec<String>)> = groups
            .into_iter()
            .filter(|(_, n)| *n >= 3)
            .map(|(k, _)| k)
            .collect();
        if peer_protos.is_empty() {
            return HashSet::new();
        }
        // Dispatcher evidence: map each dispatcher namespace → the argument
        // counts observed at its variable-command sites.
        let mut dispatcher_ns_argc: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        for site in &self.var_command_sites {
            let off = site.cmd_span.start();
            let dns = self
                .result
                .all_procs
                .iter()
                .find(|(_, p)| p.body_span.start() <= off && off <= p.body_span.end())
                .map_or_else(|| "::".to_string(), |(q, _)| namespace_of(q));
            dispatcher_ns_argc.entry(dns).or_default().insert(site.argc);
        }
        peer_protos
            .into_iter()
            .filter(|(ns_key, params)| {
                let min_argc = params.len();
                dispatcher_ns_argc.iter().any(|(dns, argcs)| {
                    (dns == ns_key || dns.starts_with(&format!("{ns_key}::")))
                        && argcs.iter().any(|&a| a >= min_argc)
                })
            })
            .collect()
    }

    /// W210 + W213 — read-before-set / unset on possibly-undefined.
    ///
    /// Walks every
    /// version-0 chain (`DefKind::Parameter`) in `fu.def_use`
    /// — those are the synthetic defs the def-use builder
    /// emits when a variable is used without a preceding def.
    ///
    /// Distinguishes real proc parameters from synthetic RBS
    /// reads via `ir_proc.params`.  Only emits inside procedures
    /// (i.e. when `ir_proc` is `Some`) — top-level RBS needs the
    /// `globals_written_by_procs` filter.
    ///
    /// Per use site:
    ///
    /// - **Phi-incoming uses** are skipped — they sit at block
    ///   boundaries and don't anchor on a real statement.
    /// - **`unset` without `-nocomplain`** emits W213 (the more
    ///   specific code) instead of W210.  W213 message tells
    ///   the user to add `-nocomplain` rather than initialise
    ///   the variable.
    /// - **`safe_on_uninit` calls** that initialise the variable
    ///   themselves (it's in their `defs`) are skipped —
    ///   commands like `lappend` / `incr` / `dict set` safely
    ///   initialise an uninitialised variable.
    /// - Everything else emits W210 with the canonical
    ///   "read before set" message + optional "did you mean…?"
    ///   suggestion.
    pub(super) fn emit_read_before_set_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
        ctx: &ReadBeforeSetCtx<'_>,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;

        // A dynamic write (`set $name value`) defines a variable this pass
        // cannot name, so *no* local can still be proved unset (issue #923
        // audit idx 1 — tclsh 9.0.4 / 8.6.14: `proc g {n} {set $n 1; puts
        // $foo}; g foo` prints `1`).  Abstain toward silence for the whole
        // function.
        if fu.dynamic_names.writes {
            return;
        }

        // Top-level RBS uses the ``extra_known_defined`` set
        // (computed from ``globals_written_by_procs``) to suppress
        // W210 on globals that helper procs write.  Inside procs the
        // set is empty.
        let params_owned: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        let params = &params_owned;

        // Collect `[info exists X]` / `[array exists X]`
        // guards: `(var, guard_block)` where reads of `var` in any
        // block dominated by `guard_block` are guarded (X is known to
        // exist there).  Positive guards the true arm; `![info exists
        // X]` guards the false arm.
        let exists_guards = collect_existence_guards(fu);

        // W210 fires **once per variable**, at the earliest read-before-set.
        // The def-use walk below
        // visits *every* version-0 use, so record the earliest passing span
        // per variable here and emit after the walk (W213, a distinct code,
        // stays inline).
        let mut w210_min: std::collections::HashMap<String, tcl_lexer::Span> =
            std::collections::HashMap::new();

        for chain in fu.def_use.chains.values() {
            // Version-0 synthetic defs are the undef origin; an
            // `unset`-killed real version, and a phi version that can reach
            // an undef origin (one-branch `set` / try-handler merge), are
            // undef at their reads too — all flow through the same
            // suppression + emission logic below.
            if chain.definition.kind != DefKind::Parameter
                && !ctx.supp.killed.contains(&chain.key)
                && !ctx.supp.can_undef.contains(&chain.key)
            {
                continue;
            }
            let (var, version) = &chain.key;
            if params.contains(var.as_str()) {
                continue;
            }
            // Tcl 8.x's registry-declared `tcl_precision` read trace
            // recreates its value after `unset`; an eager startup binding
            // (for example argv) deliberately does not get this exemption.
            let startup = startup_read_facts(
                var,
                *version,
                ctx.supp.killed.contains(&chain.key),
                ctx.initial_global,
                ctx.global_aliases,
                Some(self.analysis_context().context().authoring_query()),
            );
            if startup.lazy_read && ctx.supp.killed.contains(&chain.key) {
                continue;
            }
            // `SPECIAL_VARS` distinguishes recognised runtime-sensitive names
            // from the subset the default host actually makes readable before
            // user code. That entry fact belongs only to this document's
            // initial global frame and version zero: local shadows and a
            // version killed by `unset` are still genuine W210 reads.
            // An element read (`$arr(a)`) of an array whose *base* is
            // defined, aliased, or a parameter anywhere in the function
            // stays silent: which elements a dynamic write / whole-array
            // command created is not statically knowable, so only a read
            // of a wholly-unwritten, unaliased array reports. Policy sets
            // are base-keyed, so the base is checked for those too.
            if let Some(open) = var.find('(') {
                let base = &var[..open];
                let base_defined = fu.ssa.var_symbol(base).is_some_and(|sym| {
                    fu.def_use.chains.keys().any(|(n, v)| n == base && *v > 0)
                        || fu
                            .ssa
                            .blocks
                            .values()
                            .any(|b| b.statements.iter().any(|st| st.defs.contains_key(&sym)))
                });
                if base_defined
                    || params.contains(base)
                    || (ctx.scope_aliases.contains(base) && !ctx.supp.killed.contains(&chain.key))
                    || ctx.extra_known_defined.contains(base)
                    || ctx.supp.suppresses(base)
                {
                    continue;
                }
            }
            // A fully-qualified read (`$::myVar`, `$ns::var`) explicitly
            // targets the global / a named namespace scope, whose definition
            // may live in another proc, another namespace, or — for a
            // multi-file project — another file entirely.  Single-unit
            // dataflow cannot see those writers, so an otherwise-unresolved
            // qualified read is conservatively exempt. A same-unit `unset`
            // records a killed chain, however, so it must still be reported.
            if var.contains("::") && !ctx.supp.killed.contains(&chain.key) {
                continue;
            }
            // A scope-aliased local (`global` / `variable` / `upvar` /
            // `namespace upvar` — literal *or* dynamic target) is bound to a
            // variable in another scope, so reading it is not read-before-set.
            // `upvar 1 $name local` and `upvar 1 outer local` are semantically
            // identical: both raise `can't read` only when the *caller*
            // variable is missing — a runtime condition, not a static one — so
            // the dynamic target is suppressed exactly like the literal one
            // (matching C Tcl and the "assume the may-run path does run" stance
            // the loop / cross-event passes already take). A known `unset` of
            // the alias wins over that conservative assumption; a genuinely
            // unrelated local is absent from `scope_aliases` and still fires.
            if ctx.scope_aliases.contains(var) && !ctx.supp.killed.contains(&chain.key) {
                continue;
            }
            if ctx.extra_known_defined.contains(var) {
                continue;
            }
            // `dict with`/`dict update` unpacking + qualified-`variable`
            // alias tails suppress version-0 reads of the unpacked / aliased
            // names (the `puts $a` inside `dict with d {…}` is not RBS).
            // Interproc constant propagation resolves an empty caller dict to
            // CONST("") (keys = ∅, not unknown), so the blanket variant fires
            // on a genuine missing-key read while still suppressing an
            // unknown-shape (mixed-caller / no-caller) dict.
            if ctx.supp.suppresses(var) {
                continue;
            }
            self.record_chain_w210_uses(
                fu,
                chain,
                &W210ChainCtx {
                    exists_guards: &exists_guards,
                    supp: ctx.supp,
                    startup,
                },
                &mut w210_min,
            );
        }

        let mut entries: Vec<(String, tcl_lexer::Span)> = w210_min.into_iter().collect();
        entries.sort_by_key(|(_, s)| s.start());
        for (var, span) in entries {
            let mut message = format!("Variable '{var}' is read before it is set");
            if let Some(similar) = undefined_var_suggestion(&var, ctx.defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W210,
                    span,
                    message,
                    Severity::Warning,
                ));
        }
    }

    /// Record the earliest read-before-set span for one undef def-use chain
    /// (and emit any W213 `unset`-without-`-nocomplain`).  Walks the chain's
    /// uses, skipping phi-incoming pseudo-uses, auto-creating read-modify-write
    /// targets, existence-guarded reads, and use sites that safely initialise
    /// the variable; survivors update `w210_min` with the earliest read span.
    fn record_chain_w210_uses(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        chain: &crate::def_use::DefUseChain,
        ctx: &W210ChainCtx<'_>,
        w210_min: &mut std::collections::HashMap<String, tcl_lexer::Span>,
    ) {
        use crate::def_use::UseKind;
        use crate::ir::Statement;

        let (var, _version) = &chain.key;
        for use_site in &chain.uses {
            if matches!(use_site.kind, UseKind::PhiIncoming) {
                continue;
            }
            // A `UseClass::Quoted` use is carried only by a brace-quoted word
            // the statement does not substitute (`puts {$y}` prints `$y` and
            // reads nothing). The use exists so liveness stays conservative
            // about a word that may be evaluated later; it is not a read
            // here, so it can never be read-*before*-set (issues #1142,
            // #1237).
            if use_site.class == crate::ssa::UseClass::Quoted {
                continue;
            }
            // An after-loop read of a variable the loop body defines on every
            // iteration is not read-before-set (see
            // `UndefSuppression::loop_entry_only_undef`): we assume a may-run
            // loop runs, matching C Tcl. A read *inside* the loop body still
            // fires.
            if ctx.supp.after_loop_defined(&chain.key, &use_site.block) {
                continue;
            }
            let Some(block) = fu.cfg.block_by_name(&use_site.block) else {
                continue;
            };
            let (span, stmt_opt): (tcl_lexer::Span, Option<&Statement>) =
                if use_site.statement_index == -1 {
                    let Some(span) = block
                        .terminator
                        .as_ref()
                        .and_then(crate::cfg::Terminator::span)
                    else {
                        continue;
                    };
                    (fu.abs_span(span), None)
                } else {
                    let Ok(idx) = usize::try_from(use_site.statement_index) else {
                        continue;
                    };
                    let Some(stmt) = block.statements.get(idx) else {
                        continue;
                    };
                    (fu.abs_span(stmt.span()), Some(stmt))
                };
            if span.is_empty() {
                continue;
            }
            // A `$var` read inside an opaque body-role script that also
            // defines `var` earlier in the same script reads that script's
            // *own* local, not the outer variable — so it is not a read of an
            // undefined outer name. `interp eval PATH { set x 1; expr {$x + 1}
            // }` runs its body in a child interpreter whose statements are
            // never flattened into this function's CFG, leaving the whole
            // script scanned as one `Statement::Barrier` value: the `$x` read
            // and the body-local `set x` collapse onto that single statement,
            // so the version-0 chain shows a read with no visible def and
            // W210 false-fired (issue #923). This is the only place the
            // body-local write is visible.
            if barrier_body_locally_sets(stmt_opt, var, self.registry.as_deref()) {
                continue;
            }
            // Skip the existence-query word itself and
            // reads narrowed by an enclosing `[info exists X]` guard.
            if existence_exempt(stmt_opt, var, ctx.exists_guards, &fu.ssa, &use_site.block) {
                continue;
            }
            // ``unset`` without ``-nocomplain`` → W213.
            if let Some(Statement::Call {
                command,
                args,
                tokens,
                ..
            }) = stmt_opt
                && command == "unset"
                && !args.iter().any(|a| a == "-nocomplain")
            {
                // Eager startup bindings (`argv`, `tcl_version`, …) already
                // exist when their first `unset` runs. A lazy read trace is
                // different: its first `unset` still errors until an earlier
                // actual read has materialised its value in this block.
                if ctx.startup.initially_bound
                    || (ctx.startup.lazy_read
                        && chain.uses.iter().any(|prior| {
                            prior.block == use_site.block
                                && prior.statement_index >= 0
                                && prior.statement_index < use_site.statement_index
                                && prior.class != crate::ssa::UseClass::Quoted
                        }))
                {
                    continue;
                }
                let message = format!(
                    "Variable '{var}' may not exist; \
                         use 'unset -nocomplain' to suppress the error",
                );
                // Narrow the squiggle to the offending variable word (so
                // `unset a b c` flags only the missing name), and attach a
                // quick fix that inserts `-nocomplain` right after `unset` —
                // the same fix the LSP layer synthesises, now carried on the
                // diagnostic itself so every editor surfaces it uniformly.
                let (diag_span, fixes) = w213_span_and_fix(fu, tokens.as_ref(), var, span);
                self.result.diagnostics.push(
                    crate::analyser::types::Diagnostic::new(
                        DiagCode::W213,
                        diag_span,
                        message,
                        Severity::Warning,
                    )
                    .with_fixes(fixes),
                );
                continue;
            }
            // A use site that itself safely initialises the variable
            // (`safe_on_uninit` calls like `lappend`/`dict set`, or an
            // `incr` of its own target) is not read-before-set.
            if use_site_safe_initialises(stmt_opt, var) {
                continue;
            }
            // This is an ordinary initial read, not the destructive `unset`
            // target handled above.  Its startup fact is registry-owned and
            // applies to the initial global frame, a qualified global, or a
            // registry-declared global alias — never a same-named local.
            if ctx.startup.readable {
                continue;
            }
            // Anchor at the `$var` read token; fall back to the command
            // span when the read is nested inside a quoted/compound word.
            let read_span = self.narrow_to_read_var(span, var).unwrap_or(span);
            w210_min
                .entry(var.clone())
                .and_modify(|s| {
                    if read_span.start() < s.start() {
                        *s = read_span;
                    }
                })
                .or_insert(read_span);
        }
    }

    /// W210 on `return $v` reads where `v`'s reaching version can be
    /// undefined on some executable path (phi-from-undef / `unset`-killed).
    /// Companion to [`Self::emit_read_before_set_diagnostics`]; see its
    /// trailing call site for why the def-use-chain pass cannot catch
    /// these (return values are terminator reads, not recorded uses).
    pub(super) fn emit_return_phi_undef_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReturnUndefCtx<'_>,
    ) {
        use crate::var_refs::{VarReferenceScanner, VarScanOptions};
        use std::fmt::Write as _;

        // `defined_vars` / `considered` are used directly here; the remaining
        // sets are consulted by `return_read_fires_w210` through `ctx`.
        let defined_vars = ctx.defined_vars;
        let considered = ctx.considered;

        let Some(registry) = self.registry.as_deref() else {
            return;
        };

        let (phi_def, phi_block, killed) = build_phi_undef_index(&fu.ssa, considered);
        let phi_idx = PhiUndefIndex {
            phi_def: &phi_def,
            phi_block: &phi_block,
            killed: &killed,
        };

        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: false,
        });

        let mut reported: FxHashSet<String> = FxHashSet::default();
        // Deterministic block order for stable diagnostics (by BlockId =
        // creation order; the analyser re-sorts diagnostics by span/code).
        let mut block_ids: Vec<crate::cfg::BlockId> = considered.iter().copied().collect();
        block_ids.sort_unstable();

        for bn in block_ids {
            let Some(cfg_block) = fu.cfg.blocks.get(&bn) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Return {
                value,
                expr,
                braced,
                ..
            }) = &cfg_block.terminator
            else {
                continue;
            };
            let Some(span) = cfg_block
                .terminator
                .as_ref()
                .and_then(crate::cfg::Terminator::span)
                .map(|s| fu.abs_span(s))
            else {
                continue;
            };
            if span.is_empty() {
                continue;
            }
            let Some(ssa_block) = fu.ssa.blocks.get(&bn) else {
                continue;
            };

            // Collect the variable names read by the return value (word
            // substitutions + nested `[...]`) and any parsed expr. The
            // return value is a single already-extracted word, not a
            // script, so it scans in value-body mode.
            // A **braced** value is literal — `return {$y}` returns the two
            // characters `$y` and reads nothing — so it contributes no reads
            // at all (`UseClass::Quoted`; issue #1237).
            let mut reads: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            if let Some(v) = value.as_ref().filter(|_| !*braced) {
                reads.extend(scanner.scan_word(v, registry));
            }
            if let Some(e) = expr {
                reads.extend(crate::var_refs::vars_in_expr(e, self.grammar()));
            }

            for name in reads {
                if reported.contains(&name) {
                    continue;
                }
                let ver = fu
                    .ssa
                    .var_symbol(&name)
                    .and_then(|s| ssa_block.exit_versions.get(&s))
                    .copied()
                    .unwrap_or(0);
                if !Self::return_read_fires_w210(fu, &name, ver, bn, &phi_idx, ctx) {
                    continue;
                }
                reported.insert(name.clone());
                let mut message = format!("Variable '{name}' is read before it is set");
                if let Some(similar) = undefined_var_suggestion(&name, defined_vars) {
                    let _ = write!(message, "; did you mean '{similar}'?");
                }
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W210,
                        span,
                        message,
                        Severity::Warning,
                    ));
            }
        }
    }

    /// Decide whether a single `return`-value read of `(name, ver)` in block
    /// `bn` is a W210 phi-from-undef read: its reaching version must be able to
    /// reach an undef origin, and it must not be a parameter / scope alias /
    /// known-defined / qualified / suppressed name or be proven
    /// defined by a dominating existence guard.  Version-0 reads are handled by
    /// the def-use `DefKind::Parameter` emitter, so they never fire here.
    fn return_read_fires_w210(
        fu: &crate::compilation_unit::FunctionUnit,
        name: &str,
        ver: crate::ssa::Version,
        bn: crate::cfg::BlockId,
        phi_idx: &PhiUndefIndex<'_>,
        ctx: &ReturnUndefCtx<'_>,
    ) -> bool {
        // Version-0 return reads are now recorded in def_use, so the version-0
        // (`DefKind::Parameter`) emitter handles them with the full suppression
        // set — this pass only covers the phi-from-undef / `unset`-killed
        // (version > 0) cases, which def-use can't express.  Skipping ver 0
        // avoids double-firing.
        if ver == 0 {
            return false;
        }
        let mut seen = FxHashSet::default();
        let undef_ctx = super::helpers::PhiUndefCtx {
            phi_def: phi_idx.phi_def,
            phi_block: phi_idx.phi_block,
            killed: phi_idx.killed,
            considered: ctx.considered,
            executable_edges: &fu.sccp.executable_edges,
            exists_guards: ctx.exists_guards,
            initial_global: ctx.initial_global,
            global_aliases: ctx.global_aliases,
            dialect: ctx.dialect,
            ssa: &fu.ssa,
        };
        if !phi_can_undef(name, ver, &undef_ctx, &mut seen) {
            return false;
        }
        // A killed SSA version is concrete same-unit evidence that overrides
        // the conservative external-scope assumptions for `global`/`upvar`
        // aliases and qualified names.
        let known_killed = phi_idx.killed.contains(&(name.to_owned(), ver));
        if ctx.params.contains(name)
            || (ctx.scope_aliases.contains(name) && !known_killed)
            || ctx.extra_known_defined.contains(name)
            || (name.contains("::") && !known_killed)
            || ctx.supp.suppresses(name)
        {
            return false;
        }
        // An after-loop `return` of a variable the loop body defines on every
        // iteration is not read-before-set (we assume a may-run loop runs,
        // matching C Tcl); the return block sits outside the loop body.
        if ctx
            .supp
            .after_loop_defined(&(name.to_string(), ver), fu.cfg.block_name(bn))
        {
            return false;
        }
        // A dominating existence guard proves the var exists here.
        if ctx
            .exists_guards
            .iter()
            .any(|(gv, gblk)| gv == name && block_dominated_by(&fu.ssa, bn, *gblk))
        {
            return false;
        }
        true
    }

    /// **W210 (provably-unset regexp / scan output).** A `regexp` / `scan`
    /// with literal pattern + input that can be statically proven not to
    /// match leaves its output variables unset, so a later read of one is a
    /// real read-before-set.  Handles both the top-level call form and the
    /// call embedded in an `if` / `while` condition (firing only on the
    /// no-match branch).
    pub(super) fn emit_provably_unset_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        considered: &HashSet<crate::cfg::BlockId>,
        defined_vars: &HashSet<String>,
    ) {
        use crate::ir::Statement;
        use std::fmt::Write as _;

        let config = self.lexer_config();
        // var name -> (def_block, def_stmt_idx); idx == -1 means "from the
        // start of the block" (the embedded-condition no-match target).
        let mut provably_unset: std::collections::HashMap<String, (crate::cfg::BlockId, i32)> =
            std::collections::HashMap::new();

        for &bn in considered {
            let Some(block) = fu.cfg.blocks.get(&bn) else {
                continue;
            };
            // Top-level regexp / scan calls.
            for (idx, stmt) in block.statements.iter().enumerate() {
                let Statement::Call {
                    command,
                    canonical_command,
                    args,
                    defs,
                    ..
                } = stmt
                else {
                    continue;
                };
                let canon = canonical_command.as_deref().unwrap_or(command);
                // Name-guarded on purpose (not `pattern_type == Regex`): this
                // check statically evaluates `regexp`'s no-match result from
                // its exact positional form (pattern / input after the
                // options, trailing out-vars), paired with `scan` — per-form
                // value semantics the registry does not model.
                let is_regexp = canon == "::regexp" || command == "regexp";
                let is_scan = canon == "::scan" || command == "scan";
                if (!is_regexp && !is_scan) || defs.is_empty() {
                    continue;
                }
                if let Some(no_match) = regexp_scan_no_match(is_regexp, args)
                    && no_match
                {
                    for d in defs {
                        provably_unset
                            .entry(d.clone())
                            .or_insert_with(|| (bn, i32::try_from(idx).unwrap_or(i32::MAX)));
                    }
                }
            }
            // regexp / scan embedded in the branch condition.
            if let Some(crate::cfg::Terminator::Branch {
                condition,
                true_target,
                false_target,
                ..
            }) = &block.terminator
            {
                Self::collect_embedded_provably_unset(
                    condition,
                    *true_target,
                    *false_target,
                    &mut provably_unset,
                    config,
                );
            }
        }

        if provably_unset.is_empty() {
            return;
        }

        // Fire on every executable use after the def (same block) or in a
        // block dominated by the def block.
        let mut reported: FxHashSet<String> = FxHashSet::default();
        let mut block_ids: Vec<crate::cfg::BlockId> = considered.iter().copied().collect();
        block_ids.sort_unstable();
        for bn in block_ids {
            let Some(ssa_block) = fu.ssa.blocks.get(&bn) else {
                continue;
            };
            for (idx, s) in ssa_block.statements.iter().enumerate() {
                for &sym in s.uses.keys() {
                    // A quoted (unevaluated brace-word) mention is not a read
                    // here — see `emit_read_before_set_diagnostics`.
                    if s.quoted_uses.contains(&sym) {
                        continue;
                    }
                    let name = fu.ssa.var_name(sym);
                    if reported.contains(name) {
                        continue;
                    }
                    let Some((def_block, def_idx)) = provably_unset.get(name) else {
                        continue;
                    };
                    let in_def_block_after =
                        bn == *def_block && i32::try_from(idx).unwrap_or(i32::MAX) > *def_idx;
                    let dominated = bn != *def_block && block_dominated_by(&fu.ssa, bn, *def_block);
                    if !(in_def_block_after || dominated) {
                        continue;
                    }
                    let span = match fu.cfg.blocks.get(&bn).and_then(|b| b.statements.get(idx)) {
                        Some(st) if !st.span().is_empty() => fu.abs_span(st.span()),
                        _ => continue,
                    };
                    reported.insert(name.to_owned());
                    let mut message = format!("Variable '{name}' is read before it is set");
                    if let Some(similar) = undefined_var_suggestion(name, defined_vars) {
                        let _ = write!(message, "; did you mean '{similar}'?");
                    }
                    self.result
                        .diagnostics
                        .push(crate::analyser::types::Diagnostic::new(
                            DiagCode::W210,
                            span,
                            message,
                            Severity::Warning,
                        ));
                }
            }
        }
    }

    /// Walk a branch `condition` for an embedded `[regexp …]` / `[scan …]`
    /// command substitution that provably can't match, recording its output
    /// variables as provably-unset on the no-match branch target (only when
    /// the condition is exactly `[cmd]` → false target, or `![cmd]` → true
    /// target; more complex shapes are skipped).
    fn collect_embedded_provably_unset(
        condition: &ExprNode,
        true_target: crate::cfg::BlockId,
        false_target: crate::cfg::BlockId,
        provably_unset: &mut std::collections::HashMap<String, (crate::cfg::BlockId, i32)>,
        config: tcl_lexer::LexerConfig,
    ) {
        let (cmd_node, no_match_target) = match condition {
            ExprNode::Command { .. } => (condition, false_target),
            ExprNode::Unary {
                op: UnaryOp::Not | UnaryOp::WordNot,
                operand,
            } if matches!(operand.as_ref(), ExprNode::Command { .. }) => {
                (operand.as_ref(), true_target)
            }
            _ => return,
        };
        let ExprNode::Command { text, .. } = cmd_node else {
            return;
        };
        // Strip the surrounding `[` … `]` and segment the interior.
        let inner = text
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(text);
        let segs = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
        let Some(seg) = segs.first() else {
            return;
        };
        let Some(cmd) = seg.texts.first() else {
            return;
        };
        let bare = cmd
            .trim_start_matches(':')
            .rsplit("::")
            .next()
            .unwrap_or(cmd);
        // Same name-guard rationale as `emit_provably_unset_w210`: exact
        // `regexp` / `scan` form semantics, not a generic regex-pattern query.
        let is_regexp = bare == "regexp";
        let is_scan = bare == "scan";
        if !is_regexp && !is_scan {
            return;
        }
        let args: Vec<String> = seg.texts[1..].to_vec();
        let pos = skip_options(&args, if is_regexp { &["-start"] } else { &[] });
        if pos + 2 > args.len() {
            return;
        }
        let out_vars = &args[(pos + 2).min(args.len())..];
        if out_vars.is_empty() {
            return;
        }
        if regexp_scan_no_match(is_regexp, &args) != Some(true) {
            return;
        }
        for v in out_vars {
            let name = crate::naming::normalise_var_name(v);
            if !name.is_empty() {
                provably_unset
                    .entry(name.to_string())
                    .or_insert((no_match_target, -1));
            }
        }
    }

    /// I230 / I231 — constant branch / switch-arm condition.
    ///
    /// For every
    /// branch SCCP folded to a constant, when the *not-taken*
    /// target is also unreachable (i.e. SCCP confirmed only one
    /// path is feasible), emit an Info-level diagnostic so the
    /// LSP can highlight the dead arm.
    ///
    /// Code selection:
    /// - Block name starts with ``switch_`` → I231 (switch-arm).
    /// - Block name starts with ``if_`` → I230 (constant if).
    /// - Otherwise → I230 with the generic
    ///   ``"Branch condition '...' is constant"`` message.
    ///
    /// Severity is mapped to ``Hint`` because the
    /// [`Severity`] enum has no ``Info`` variant — ``Hint`` is
    /// the closest non-actionable level.
    pub(super) fn emit_constant_branch_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        for branch in &fu.sccp.constant_branches {
            // A branch is dead when the not-taken target is
            // unreachable.  SCCP exposes
            // ``executable_blocks`` (the complement); a block
            // is unreachable iff it's in ``cfg.blocks`` but
            // NOT in ``executable_blocks``.
            if fu
                .cfg
                .block_id(&branch.not_taken_target)
                .is_some_and(|id| fu.sccp.executable_blocks.contains(&id))
            {
                continue;
            }
            // Locate the branch's terminator span.
            let Some(block) = fu.cfg.block_by_name(&branch.block) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Branch {
                span: Some(span), ..
            }) = &block.terminator
            else {
                continue;
            };
            let span = fu.abs_span(*span);

            let names = [
                branch.block.as_str(),
                branch.taken_target.as_str(),
                branch.not_taken_target.as_str(),
            ];
            let is_switch = names.iter().any(|n| n.starts_with("switch_"));
            let is_if = names.iter().any(|n| n.starts_with("if_"));
            let is_loop = names.iter().any(|n| {
                n.starts_with("while_") || n.starts_with("for_") || n.starts_with("foreach_")
            });
            // Suppress the idiomatic infinite loop `while 1 { … }`:
            // a constant-TRUE loop condition is intentional, not a bug (a
            // constant-FALSE loop still flags its unreachable body).
            if is_loop && branch.value {
                continue;
            }

            let (code, message) = if is_switch {
                let code = DiagCode::I231;
                let msg = if branch.value {
                    format!(
                        "Switch condition '{}' is always true here; \
                         subsequent switch arms are unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Switch arm condition '{}' is always false; \
                         this arm is unreachable",
                        branch.condition,
                    )
                };
                (code, msg)
            } else if is_if {
                let msg = if branch.value {
                    format!(
                        "Condition '{}' is always true; \
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Condition '{}' is always false; \
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                };
                (DiagCode::I230, msg)
            } else {
                let msg = format!(
                    "Branch condition '{}' is constant; one branch is unreachable",
                    branch.condition,
                );
                (DiagCode::I230, msg)
            };

            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    code,
                    span,
                    message,
                    // I230/I231 are observational (LSP `Information`);
                    // they previously collapsed to `Hint`.
                    Severity::Info,
                ));
        }
    }

    /// I230 — fold `[info exists X]` / `[array exists X]` conditions.
    ///
    /// SCCP can't fold these (the predicate lowers to an
    /// opaque `ExprNode::Command`, and SCCP has no parameter/existence
    /// facts), so the fold is computed by
    /// [`crate::sccp::existence_constant_branches`] using the frame's formal
    /// parameters — the same helper whose result
    /// `FunctionUnit::build` appends to `sccp.constant_branches` for the
    /// optimiser's O101 fold / DCE.  Emitting the I230 here (rather than
    /// via [`Self::emit_constant_branch_diagnostics`]) is deliberate:
    /// that emitter gates on the not-taken arm being unreachable in
    /// `executable_blocks`, which these post-pass folds don't update, so
    /// it skips them and there is no double emission.
    ///
    /// `frame` supplies the typed entry facts for whichever kind of body
    /// this is (issue #1129): a procedure contributes its parameters, a
    /// `TclOO` method body contributes its parameters **and** its class's
    /// instance variables, on which the fold must abstain.  Both halves come
    /// from the same IR the optimiser's copy of the fold reads, so the two
    /// consumers cannot drift.
    pub(super) fn emit_existence_constant_branch_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        frame: crate::sccp::ExistenceFrame<'_>,
    ) {
        // The fold consults the registry's scope-alias roles to skip
        // out-of-frame-linked locals; a registry-less analyser falls back to
        // the cached default registry (the same convention as
        // `command_takes_regex_pattern` — direct handler calls in unit
        // tests), so the alias skip stays sound there too.
        let branches = {
            // Scoped borrow: `self.registry.as_deref()` must release before the
            // `&mut self` diagnostic pushes below.
            let registry = self.registry.as_deref().map_or_else(
                || {
                    tcl_registry::model::ingress::static_context_for("tcl8.6")
                        .commands()
                        .as_ref()
                },
                |r| r,
            );
            crate::sccp::existence_constant_branches(&fu.cfg, frame, registry, fu.dynamic_names)
        };
        for cb in branches {
            let Some(span) = cb.span.map(|s| fu.abs_span(s)) else {
                continue;
            };
            let message = if cb.value {
                format!(
                    "Condition '{}' is always true; the alternate branch is unreachable",
                    cb.condition,
                )
            } else {
                format!(
                    "Condition '{}' is always false; the alternate branch is unreachable",
                    cb.condition,
                )
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::I230,
                    span,
                    message,
                    // I230 is observational (LSP `Information`).
                    Severity::Info,
                ));
        }
    }

    /// W126 — channel-argument validation.
    ///
    /// Walks every
    /// SSA-annotated `Call` statement for commands that declare
    /// `ArgRole::Channel` arguments; for each channel-position
    /// argument, checks the SSA type lattice to determine whether
    /// the value is genuinely a channel.  Two failure modes:
    ///
    /// - **`$var` reference** with `TypeKind::Known` and a non-
    ///   `TclType::Channel` type — emits "passed as channel … has
    ///   type X, not CHANNEL".
    /// - **String literal** that isn't `stdin` / `stdout` /
    ///   `stderr` and contains no substitutions — emits
    ///   "String literal 'X' used as channel argument".
    ///
    /// The standard channels (`stdin`, `stdout`, `stderr`) are
    /// always accepted.  Unknown / overdefined types skip the
    /// check (could be anything).
    pub(super) fn emit_channel_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::ir::Statement;
        use tcl_registry::ArgRole;

        const STANDARD_CHANNELS: &[&str] = &["stdout", "stderr", "stdin"];

        for block in fu.ssa.blocks.values() {
            for ssa_stmt in &block.statements {
                let Statement::Call {
                    command,
                    args,
                    span,
                    tokens,
                    ..
                } = &ssa_stmt.statement
                else {
                    continue;
                };
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                let channel_indices =
                    registry.arg_indices_for_role(command, &arg_refs, ArgRole::Channel);
                if channel_indices.is_empty() {
                    continue;
                }
                for idx in channel_indices {
                    if idx >= args.len() {
                        continue;
                    }
                    let arg_text = &args[idx];
                    // Tight range: the channel argument word (`argv[0]` is the
                    // command name, so `args[idx]` is `argv[idx + 1]`), not the
                    // whole command. Falls back to the command span when the
                    // per-word tokens are unavailable.
                    let arg_span = tokens
                        .as_ref()
                        .and_then(|t| t.argv.get(idx + 1))
                        .map_or_else(|| fu.abs_span(*span), |&s| fu.abs_span(s));
                    // Extract bare var name from ``$var`` / ``${var}``.
                    let var_name: Option<&str> =
                        if arg_text.starts_with("${") && arg_text.ends_with('}') {
                            Some(&arg_text[2..arg_text.len() - 1])
                        } else if let Some(rest) = arg_text.strip_prefix('$') {
                            Some(rest)
                        } else {
                            None
                        };

                    if let Some(name) = var_name {
                        let Some(sym) = fu.ssa.var_symbol(name) else {
                            continue;
                        };
                        let Some(&version) = ssa_stmt.uses.get(&sym) else {
                            continue;
                        };
                        let key: crate::ssa::ValueKey = (sym, version);
                        let Some(var_type) = fu.types.get(&key) else {
                            continue;
                        };
                        let Some(type_label) = non_channel_union_label(var_type) else {
                            continue;
                        };
                        let message = format!(
                            "Variable '${name}' passed as channel to '{command}' \
                             has type {type_label}, not CHANNEL.",
                        );
                        self.result
                            .diagnostics
                            .push(crate::analyser::types::Diagnostic::new(
                                DiagCode::W126,
                                arg_span,
                                message,
                                Severity::Warning,
                            ));
                    } else {
                        // Literal — strip surrounding braces / quotes.
                        let literal = arg_text
                            .trim_matches('"')
                            .trim_start_matches('{')
                            .trim_end_matches('}');
                        if STANDARD_CHANNELS.contains(&literal) {
                            continue;
                        }
                        // Only warn for clearly-not-substituted literals.
                        if arg_text.contains('$') || arg_text.contains('[') {
                            continue;
                        }
                        let message = format!(
                            "String literal '{literal}' used as channel argument to \
                             '{command}' — expected a channel from open/socket/chan create.",
                        );
                        self.result
                            .diagnostics
                            .push(crate::analyser::types::Diagnostic::new(
                                DiagCode::W126,
                                arg_span,
                                message,
                                Severity::Warning,
                            ));
                    }
                }
            }
        }
    }

    /// W124 — invalid IP address literal.
    ///
    /// Walks every
    /// SSA-tracked constant string in the function's SCCP
    /// values; regex-searches for IPv4 dotted-quad and IPv6
    /// candidates and validates each.
    ///
    /// **Validation:**
    /// - **IPv4** — each octet must be 0..255; leading-zero
    ///   octets emit a Warning (interpreted as octal in some
    ///   contexts); over-255 octets emit an Error.  Patterns
    ///   preceded by ``/`` (CIDR / version-number context) are
    ///   skipped.
    /// - **IPv6** — parsed via [`std::net::Ipv6Addr`]; failure
    ///   emits an Error.
    ///
    /// Diagnostic anchors at the SSA def site (the assignment
    /// statement's span); seen-offsets dedup avoids duplicate
    /// emissions when multiple SSA versions share a def.
    /// **W233.** Division / modulo by a provably-zero divisor — raises
    /// "divide by zero" at runtime.  Delegates to the canonical
    /// interval-bounds analysis [`crate::interval_bounds::find_divide_by_zero`]
    /// (the single source of truth, shared with the interval-bounds index
    /// checks): a `/` or `%` whose divisor's interval — guard-narrowed at the
    /// use site and seeded from the SCCP lattice — is exactly `[0, 0]`, on the
    /// always-evaluated spine of an executable expression.
    ///
    /// (Verified against tclsh 8.4–9.0: integer `1/0` and `5%0` raise "divide
    /// by zero"; float division such as `1.0/0` yields `Inf` and does not
    /// error. The interval domain is integer, matching that boundary for the
    /// common cases.)
    pub(super) fn emit_w233_divide_by_zero(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        // The block set SCCP proved reachable; fall back to every SSA block
        // when SCCP produced nothing (e.g. a trivial function) so the check
        // still runs — matching the previous emitter's reachability fallback.
        let executable: HashSet<crate::cfg::BlockId> = if fu.sccp.executable_blocks.is_empty() {
            fu.ssa.blocks.keys().copied().collect()
        } else {
            fu.sccp.executable_blocks.clone()
        };
        for finding in crate::interval_bounds::find_divide_by_zero_with(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            &executable,
            // The document's own numeral grammar: a divisor literal means what
            // this dialect says it means (`0755` is 493 up to 8.6, 755 from
            // 9.0), and this process analyses documents of several dialects.
            crate::intervals::numbers_for_dialect(Some(self.profile)),
            self.grammar(),
        ) {
            let span = fu.abs_span(finding.span);
            if span.is_empty() {
                continue;
            }
            let verb = if finding.op == "/" {
                "Division"
            } else {
                "Modulo"
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W233,
                    span,
                    format!(
                        "{verb} by a provably-zero divisor — raises 'divide by zero' at runtime."
                    ),
                    Severity::Warning,
                ));
        }
    }

    /// **W230 / W231 / W232 (dynamic).** Interval-driven out-of-range index
    /// detection for a `$var` index whose [`crate::intervals`] range — guard-
    /// narrowed at the use site — proves the access is wholly out of range
    /// against a statically-established container length.  Complements the
    /// syntactic bounds checks (literal index + literal container only); the
    /// two never double-fire because the syntactic checks back off on any
    /// `$var` index.  Restricted to SCCP-reachable blocks so a dynamic index
    /// in dead code does not warn.
    pub(super) fn emit_interval_bounds_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        let executable: HashSet<crate::cfg::BlockId> = if fu.sccp.executable_blocks.is_empty() {
            fu.ssa.blocks.keys().copied().collect()
        } else {
            fu.sccp.executable_blocks.iter().copied().collect()
        };
        let findings = crate::interval_bounds::find_interval_bounds_with(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            &executable,
            self.profile.character_model(),
            // The document's own numeral grammar, alongside the character model
            // — both dialect-derived facts, both threaded rather than ambient.
            crate::intervals::numbers_for_dialect(Some(self.profile)),
            self.grammar(),
        );
        for f in findings {
            if f.span.is_empty() {
                continue;
            }
            let bound = if f.reason == "negative" {
                "below 0".to_string()
            } else {
                format!("past the end ({})", f.length)
            };
            let rng = if f.reason == "negative" {
                "negative".to_string()
            } else if f.index_interval.lo == f.index_interval.hi {
                format!("is {}", f.index_interval.lo.unwrap_or(0))
            } else {
                let lo = f
                    .index_interval
                    .lo
                    .map_or("-inf".to_string(), |l| l.to_string());
                let hi = f
                    .index_interval
                    .hi
                    .map_or("+inf".to_string(), |h| h.to_string());
                format!("is in [{lo}, {hi}]")
            };
            let outcome = if f.code == DiagCode::W231 {
                "raises 'index out of range' at runtime"
            } else {
                "silently returns the empty string"
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    f.code,
                    fu.abs_span(f.span),
                    format!(
                        "{}: index ${} {rng}, {bound} \u{2014} {outcome}.",
                        f.command, f.index_var
                    ),
                    Severity::Warning,
                ));
        }
    }

    pub(super) fn emit_invalid_ip_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::net::Ipv6Addr;
        use std::str::FromStr;

        let mut seen_offsets: FxHashSet<u32> = FxHashSet::default();
        for (key, lv) in &fu.sccp.values {
            let Some(text) = (match lv {
                LatticeValue::Const(ConstValue::String(s)) => Some(s.as_str()),
                _ => None,
            }) else {
                continue;
            };

            // ---- IPv4 candidates ----
            for quad in find_dotted_quads(text, 4) {
                let bytes = text.as_bytes();
                if quad.start > 0 && bytes[quad.start - 1] == b'/' {
                    continue;
                }
                // Skip OID-like patterns: the matched quad is a slice of a
                // longer dotted-digit chain (LDAP/SNMP OIDs like
                // ``1.3.6.1.4.1.4203.1.11.3``).  Detect a ``digit.<quad>``
                // before or a ``<quad>.digit`` after.
                let before_dot_digit = quad.start >= 2
                    && bytes[quad.start - 1] == b'.'
                    && bytes[quad.start - 2].is_ascii_digit();
                let after_dot_digit = quad.end + 1 < bytes.len()
                    && bytes[quad.end] == b'.'
                    && bytes[quad.end + 1].is_ascii_digit();
                if before_dot_digit || after_dot_digit {
                    continue;
                }
                let octets = quad.octets;
                let mut diag: Option<(String, Severity)> = None;
                for (i, octet) in octets.iter().enumerate() {
                    let v: u32 = octet.parse().unwrap_or(0);
                    if v > 255 {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) exceeds 255 — this is not a valid IP address.",
                                i + 1,
                                octet,
                            ),
                            Severity::Error,
                        ));
                        break;
                    }
                    if octet.len() > 1
                        && octet.starts_with('0')
                        && octet.bytes().all(|b| (b'0'..=b'7').contains(&b))
                    {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) has a leading zero — may be interpreted as octal in some contexts.",
                                i + 1,
                                octet,
                            ),
                            Severity::Warning,
                        ));
                        break;
                    }
                }
                if let Some((msg, sev)) = diag {
                    let literal = &text[quad.start..quad.end];
                    self.emit_ip_diag_at_def(fu, *key, &msg, sev, Some(literal), &mut seen_offsets);
                    break;
                }
            }

            // ---- IPv6 candidates ----
            for candidate in find_ipv6_candidates(text) {
                if Ipv6Addr::from_str(candidate).is_err() {
                    let msg = format!("Invalid IPv6 address '{candidate}'.");
                    self.emit_ip_diag_at_def(
                        fu,
                        *key,
                        &msg,
                        Severity::Error,
                        Some(candidate),
                        &mut seen_offsets,
                    );
                    break;
                }
            }
        }
    }

    /// Helper for [`Self::emit_invalid_ip_diagnostics`].
    ///
    /// Anchors on the offending IP `literal` within the def statement when
    /// it appears verbatim in the source (with non-address bytes on both
    /// sides), falling back to the whole statement span when the constant
    /// was folded from parts the source never spells out.
    fn emit_ip_diag_at_def(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        key: crate::ssa::ValueKey,
        message: &str,
        severity: Severity,
        literal: Option<&str>,
        seen_offsets: &mut FxHashSet<u32>,
    ) {
        let (sym, version) = key;
        let var_name = fu.ssa.var_name(sym);
        let Some(chain) = fu.def_use.chain_for(var_name, version) else {
            return;
        };
        let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
            return;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            return;
        };
        let Some(stmt) = block.statements.get(idx) else {
            return;
        };
        let stmt_span = fu.abs_span(stmt.span());
        if stmt_span.is_empty() {
            return;
        }
        if !seen_offsets.insert(stmt_span.start()) {
            return;
        }
        // Tight anchor: the literal's own bytes inside the statement, when
        // the source spells it out directly.
        let span = literal
            .and_then(|lit| {
                let slice = source_slice(&self.source, stmt_span)?;
                let is_addr_byte = |b: u8| b.is_ascii_hexdigit() || b == b'.' || b == b':';
                let mut from = 0;
                while let Some(off) = slice[from..].find(lit) {
                    let start = from + off;
                    let end = start + lit.len();
                    let left_ok = start == 0 || !is_addr_byte(slice.as_bytes()[start - 1]);
                    let right_ok = end >= slice.len() || !is_addr_byte(slice.as_bytes()[end]);
                    if left_ok && right_ok {
                        return Some(tcl_lexer::Span::new(
                            stmt_span.start() + u32::try_from(start).ok()?,
                            stmt_span.start() + u32::try_from(end).ok()?,
                        ));
                    }
                    from = start + 1;
                }
                None
            })
            .unwrap_or(stmt_span);
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W124,
                span,
                message.to_string(),
                severity,
            ));
    }

    /// IRULE4005 — racy ``static::`` cross-event flow.
    ///
    /// Walks every
    /// SSA statement in `fu` and emits IRULE4005 for any
    /// non-``unset`` def of a name in `racy_vars`.
    /// `racy_vars` comes from
    /// [`crate::connection_scope::ConnectionScope::racy_static_defs`]
    /// — built once per `CompilationUnit` and shared by every
    /// ``::when::*`` proc except `RULE_INIT`.
    pub(super) fn emit_racy_static_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        racy_vars: &HashSet<String>,
    ) {
        if self.disabled_diagnostics.contains("IRULE4005") {
            return;
        }
        let mut emitted_spans: FxHashSet<u32> = FxHashSet::default();
        for block in fu.ssa.blocks.values() {
            for stmt in &block.statements {
                // Skip unset — not a real write.
                if let crate::ir::Statement::Call { command, .. } = &stmt.statement
                    && command == "unset"
                {
                    continue;
                }
                for &sym in stmt.defs.keys() {
                    let name = fu.ssa.var_name(sym);
                    if !racy_vars.contains(name) {
                        continue;
                    }
                    let span = fu.abs_span(stmt.statement.span());
                    if span.is_empty() || !emitted_spans.insert(span.start()) {
                        continue;
                    }
                    let message = format!(
                        "Potential race: '{name}' is written outside RULE_INIT and read in \
                         another event. static:: variables persist across all connections on \
                         the same virtual server; concurrent writes can produce unpredictable \
                         results."
                    );
                    self.result
                        .diagnostics
                        .push(crate::analyser::types::Diagnostic::new(
                            DiagCode::Irule4005,
                            span,
                            message,
                            Severity::Warning,
                        ));
                }
            }
        }
    }
}

/// Collect the bracketed text of every `[…]` command-substitution node in
/// an `expr` AST (recursing operands but stopping at the substitution
/// boundary). Used to recover
/// variable reads hidden inside `if`/`while` conditions and `expr` values.
fn collect_expr_command_texts(node: &ExprNode, out: &mut Vec<String>) {
    // Entry point: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`collect_expr_command_texts_at`]).
    collect_expr_command_texts_at(node, out, 0);
}

fn collect_expr_command_texts_at(node: &ExprNode, out: &mut Vec<String>, depth: u32) {
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the command texts gathered so far is the safe fallback
    // (substitutions buried deeper than the cap are not collected; never a
    // crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Command { text, .. } => out.push(text.clone()),
        ExprNode::Binary { left, right, .. } => {
            collect_expr_command_texts_at(left, out, depth + 1);
            collect_expr_command_texts_at(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => collect_expr_command_texts_at(operand, out, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_command_texts_at(condition, out, depth + 1);
            collect_expr_command_texts_at(true_branch, out, depth + 1);
            collect_expr_command_texts_at(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_command_texts_at(arg, out, depth + 1);
            }
        }
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => {}
    }
}

/// Find every IPv6 *candidate* substring — `\b[hex]{1,4}(:[hex]{0,4}){2,7}\b`
/// — in `text` (the caller validates each via `Ipv6Addr::from_str`).
/// Replaces the regex; each candidate begins at a word boundary, has a
/// 1-4 hex-digit first group, 2-7 following `:`-groups (each 0-4 hex),
/// and ends on a hex digit at a trailing word boundary.
pub(super) fn find_ipv6_candidates(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before
            && bytes[i].is_ascii_hexdigit()
            && let Some(end) = match_ipv6_candidate(bytes, i)
        {
            out.push(&text[i..end]);
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Read up to `max` contiguous hex-digit bytes from `start`, returning
/// the count.
fn hex_run_len(bytes: &[u8], start: usize, max: usize) -> usize {
    let mut k = 0;
    while k < max && start + k < bytes.len() && bytes[start + k].is_ascii_hexdigit() {
        k += 1;
    }
    k
}

/// Match an IPv6 candidate starting at `start`, returning the end offset
/// of the longest `hex(:hex?){2,7}` run that ends on a hex digit and is
/// followed by a word boundary, or `None`.
fn match_ipv6_candidate(bytes: &[u8], start: usize) -> Option<usize> {
    let first = hex_run_len(bytes, start, 4);
    if first == 0 {
        return None;
    }
    let mut pos = start + first;
    let mut groups = 0usize;
    let mut best: Option<usize> = None;
    while groups < 7 && bytes.get(pos) == Some(&b':') {
        let after_colon = pos + 1;
        let h = hex_run_len(bytes, after_colon, 4);
        pos = after_colon + h;
        groups += 1;
        // A valid `\b`-terminated end: ≥2 groups, ends on a hex digit,
        // and is followed by a non-word byte (or end of input).
        if groups >= 2 && h >= 1 && (pos >= bytes.len() || !is_word_byte(bytes[pos])) {
            best = Some(pos);
        }
    }
    best
}

/// The suggestion name for an undefined-variable "; did you mean 'X'?"
/// suffix (W210): a case-insensitive twin among `defined_vars` wins at
/// any edit distance ([`find_case_mismatch`] — the established W210/W211/
/// W220 behaviour), otherwise the closest *other* defined name within
/// the length-scaled edit budget ([`crate::text::scaled_max_distance`],
/// so a short typo can't fish an unrelated short name). `None` when
/// nothing is close — the message then stays suffix-free.
fn undefined_var_suggestion<'a>(
    variable: &str,
    defined_vars: &'a HashSet<String>,
) -> Option<&'a str> {
    if let Some(similar) = find_case_mismatch(variable, defined_vars) {
        return Some(similar);
    }
    // The read variable can itself appear in `defined_vars` when it is
    // assigned later in the function (`puts $x; set x 1`) — never
    // suggest the typo as its own correction.
    crate::text::suggest_similar(
        variable,
        defined_vars
            .iter()
            .map(String::as_str)
            .filter(|name| *name != variable),
        1,
        crate::text::scaled_max_distance_strict(variable),
    )
    .first()
    .copied()
}

/// Find a defined variable that differs from `variable` only in case.
/// Returns the lexicographically smallest other-cased variant —
/// deterministic across runs.
fn find_case_mismatch<'a>(variable: &str, defined_vars: &'a HashSet<String>) -> Option<&'a str> {
    let lower = variable.to_lowercase();
    let mut matches: Vec<&str> = defined_vars
        .iter()
        .filter(|n| n.as_str() != variable && n.to_lowercase() == lower)
        .map(String::as_str)
        .collect();
    matches.sort_unstable();
    matches.into_iter().next()
}

/// True when `stmt` is a `Statement::Barrier` whose body-role argument (an
/// opaque script run in a separate context — `interp eval PATH { ... }`)
/// contains a top-level `set VAR ...` for `var`.
///
/// Such a body is never flattened into this function's CFG (its target
/// interpreter is unknowable to static analysis), so its whole script text is
/// scanned as one statement's value: a `$var` read and the body's own `set
/// var` collapse onto the same `Statement::Barrier`, and the version-0
/// def-use chain then shows a read with no visible definition. Recovering the
/// body's own top-level assignments here is the only place that write is
/// visible, so a plain write-then-read *inside* the body doesn't false-fire
/// W210 (issue #923). Deliberately conservative — it suppresses whenever the
/// body sets the name, a false-negative direction (a genuine read-before-set
/// entirely within the opaque body is unreported either way, and the outer
/// interpreter-handle vs. inner-local name clash drops that outer read too),
/// never a new false positive.
fn barrier_body_locally_sets(
    stmt: Option<&crate::ir::Statement>,
    var: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> bool {
    use crate::ir::Statement;
    let (Some(Statement::Barrier { command, args, .. }), Some(registry)) = (stmt, registry) else {
        return false;
    };
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    registry
        .arg_indices_for_role(command, &arg_strs, tcl_registry::ArgRole::Body)
        .into_iter()
        .filter_map(|idx| args.get(idx))
        .flat_map(|body_text| {
            crate::segmenter::segment_commands_with_offset_and_config(
                body_text,
                0,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
            )
        })
        .filter(|seg| seg.texts.first().map(String::as_str) == Some("set"))
        .filter_map(|seg| {
            seg.texts
                .get(1)
                .map(|w| crate::naming::normalise_var_name(w).to_owned())
        })
        .any(|name| name == var)
}

/// Variables this statement queries *only for
/// existence* (`info exists X` / `array exists X`, whether a bare call
/// or a `[...]` command substitution inside an assignment / argument).
/// Such a reference is not a value read, so it must not raise W210.
fn existence_query_vars(stmt: &crate::ir::Statement) -> Vec<String> {
    use crate::expr_ast::existence_query_in_text;
    use crate::ir::Statement;
    let mut out = Vec::new();
    // Bare-call form: `info exists X` / `array exists X`.
    if let Statement::Call { command, args, .. } = stmt
        && matches!(command.as_str(), "info" | "array")
        && args.first().map(String::as_str) == Some("exists")
        && let Some(v) = args.get(1)
    {
        out.push(v.clone());
    }
    // Command-substitution form: `set y [info exists X]`,
    // `puts [array exists X]`, etc.
    let texts: &[String] = match stmt {
        Statement::AssignValue { value, .. } => std::slice::from_ref(value),
        Statement::Call { args, .. } => args,
        _ => &[],
    };
    for t in texts {
        if let Some((v, _command)) = existence_query_in_text(t.trim()) {
            out.push(v);
        }
    }
    out
}

/// True when a read of `var` at `use_block` is exempt
/// from W210 because it is the existence-query word itself, or because
/// it sits in a region guarded by an enclosing `[info exists var]`.
fn existence_exempt(
    stmt_opt: Option<&crate::ir::Statement>,
    var: &str,
    exists_guards: &[(String, crate::cfg::BlockId)],
    ssa: &crate::ssa::SsaFunction,
    use_block: &str,
) -> bool {
    if let Some(stmt) = stmt_opt
        && existence_query_vars(stmt).iter().any(|q| q == var)
    {
        return true;
    }
    let Some(use_id) = ssa.block_id(use_block) else {
        return false;
    };
    exists_guards
        .iter()
        .any(|(gv, gblk)| gv == var && block_dominated_by(ssa, use_id, *gblk))
}

/// True when a read of `var` at this use-site statement is in fact a safe
/// self-initialisation, not a read-before-set: a `safe_on_uninit` call (e.g.
/// `lappend`/`dict set`/`append`) that defines `var`, or an `incr` of its own
/// target (which initialises an unset var to 0 in Tcl 8.5+).
fn use_site_safe_initialises(stmt: Option<&crate::ir::Statement>, var: &str) -> bool {
    use crate::ir::Statement;
    match stmt {
        Some(Statement::Call {
            safe_on_uninit,
            defs,
            ..
        }) => *safe_on_uninit && defs.iter().any(|d| d == var),
        Some(Statement::Incr {
            name,
            safe_on_uninit,
            ..
        }) => *safe_on_uninit && crate::naming::normalise_var_name(name) == var,
        _ => false,
    }
}

/// The namespace of a fully-qualified name: everything up to the last `::`,
/// or `::` for a top-level name.
fn namespace_of(qualified_name: &str) -> String {
    match qualified_name.rsplit_once("::") {
        Some((ns, _)) if !ns.is_empty() => ns.to_string(),
        _ => "::".to_string(),
    }
}

/// Compute W213's diagnostic span and quick fix for an `unset` of a
/// possibly-missing `var`.
///
/// The span narrows to the offending variable's own word (so `unset a b c`
/// squiggles just the missing name), falling back to the whole-command `span`
/// when the tokens aren't available or the name can't be located. The fix
/// inserts ` -nocomplain` immediately after the `unset` command word — a
/// zero-width insertion — turning `unset x` into `unset -nocomplain x`.
fn w213_span_and_fix(
    fu: &crate::compilation_unit::FunctionUnit,
    tokens: Option<&crate::ir::CommandTokens>,
    var: &str,
    span: tcl_lexer::Span,
) -> (tcl_lexer::Span, Vec<super::types::CodeFix>) {
    let Some(toks) = tokens else {
        return (span, Vec::new());
    };
    // Narrow to the argument word whose text is this variable (argv[0] is the
    // `unset` command word, so the names start at index 1).
    let diag_span = toks
        .argv_texts
        .iter()
        .zip(&toks.argv)
        .skip(1)
        .find(|(text, _)| text.as_str() == var)
        .map_or(span, |(_, &word)| fu.abs_span(word));
    // Insert ` -nocomplain` right after the `unset` word.
    let fixes = toks.argv.first().map_or_else(Vec::new, |&cmd_word| {
        let at = fu.abs_span(cmd_word).end();
        vec![super::types::CodeFix {
            span: tcl_lexer::Span::new(at, at),
            new_text: " -nocomplain".to_string(),
            description: "Add '-nocomplain' to unset".to_string(),
            // W213: `-nocomplain` stops `unset` raising on a missing variable.
            // Suppressing that error is the point of the fix, and a program
            // relying on it (a `catch`ed probe) observes the change.
            safety: crate::irules_checks::FixSafety::BehaviourHardening,
        }]
    });
    (diag_span, fixes)
}

/// Tcl ARE metacharacters: a pattern free of these reduces to a literal
/// substring search.
const TCL_REGEX_METACHARS: &str = r"\^$.|?*+()[]{}";

/// `regexp` switches that don't change match-vs-no-match for a pure-literal
/// pattern.
fn is_regexp_literal_safe_switch(opt: &str) -> bool {
    matches!(
        opt,
        "-indices" | "-inline" | "-all" | "-line" | "-lineanchor" | "-linestop" | "-start" | "--"
    )
    // `-expanded` is handled separately (whitespace/comment-gated) by the
    // caller, so it is intentionally not listed here.
}

/// True iff `regexp PATTERN INPUT` provably returns 0.  Sound only when
/// `pat` is a pure-literal pattern (no ARE metacharacters), reducing the
/// match to substring search.  Unknown / unsafe switches bail (return
/// `false` = cannot prove no-match).
fn regexp_literal_no_match(pat: &str, inp: &str, options: &[String]) -> bool {
    if pat.chars().any(|c| TCL_REGEX_METACHARS.contains(c)) {
        return false;
    }
    let mut nocase = false;
    let mut expanded = false;
    for opt in options {
        if !opt.starts_with('-') {
            continue; // an option value (e.g. after `-start`)
        }
        if opt == "-nocase" {
            nocase = true;
            continue;
        }
        if opt == "-expanded" {
            expanded = true;
            continue;
        }
        if is_regexp_literal_safe_switch(opt) {
            continue;
        }
        return false; // unknown / unsafe switch
    }
    // `-expanded` makes Tcl ignore unescaped whitespace and `#`-comments in
    // the pattern, so a pattern containing either is NOT a plain substring
    // (`regexp -expanded {a b} {ab}` matches).  Bail in that case so the
    // no-match proof stays sound — a whitespace/comment-free literal is
    // still safe.
    if expanded && pat.chars().any(|c| c.is_whitespace() || c == '#') {
        return false;
    }
    if nocase {
        !inp.to_lowercase().contains(&pat.to_lowercase())
    } else {
        !inp.contains(pat)
    }
}

/// `Some(true)` when a `regexp` / `scan` call (`is_regexp` selects the arg
/// order) with literal pattern + input provably can't match; `Some(false)`
/// when it might match; `None` when the args can't be statically resolved
/// (dynamic substitution, too few args).
fn regexp_scan_no_match(is_regexp: bool, args: &[String]) -> Option<bool> {
    let value_opts: &[&str] = if is_regexp { &["-start"] } else { &[] };
    let pos = skip_options(args, value_opts);
    if pos + 1 >= args.len() {
        return None;
    }
    let a = &args[pos];
    let b = &args[pos + 1];
    // `regexp ?opts? PATTERN STRING …`; `scan STRING FORMAT …`.
    let (pat, inp) = if is_regexp { (a, b) } else { (b, a) };
    // Dynamic substitution markers — runtime value unknown.
    if pat.contains(['$', '[']) || inp.contains(['$', '[']) {
        return None;
    }
    if is_regexp {
        let opts: Vec<String> = args[..pos].to_vec();
        Some(regexp_literal_no_match(pat, inp, &opts))
    } else {
        Some(crate::scan_predicate::scan_provably_no_match(pat, inp))
    }
}

/// Index of the first non-option argument in `args`, skipping `-option`
/// flags and the values of options in `value_opts`.
fn skip_options(args: &[String], value_opts: &[&str]) -> usize {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if value_opts.contains(&a.as_str()) && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    i
}

/// Return ``true`` when ``body`` contains a ``$param`` /
/// ``${param}`` substitution.  Used as a fallback by the W214
/// (unused-parameter) emitter to suppress the warning when the
/// parameter is read inside a ``[expr {...}]`` / ``[cmd ...]``
/// substitution that the IR lowerer doesn't track as a use.
///
/// Conservative — false negatives are fine (W214 still fires
/// when the param genuinely isn't referenced), but false
/// positives would cause the over-emit this guard exists to
/// prevent.  The bare-name match enforces a non-identifier
/// boundary on each side so ``$abc`` doesn't match ``$ab``,
/// and skips the variable when it follows a ``\\`` escape.
/// True when the proc body textually references the parameter `$param` /
/// `${param}`, scanning command-by-command so a `namespace eval` body — which
/// runs in the *namespace* frame, not the caller's — does **not** falsely
/// recover a read of the caller's parameter.  Other bodies (`eval`, `if`,
/// loops) run in the caller frame, so their `$param` reads still count.
pub(super) fn body_references_param(
    body: &str,
    param: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    if param.is_empty() {
        return false;
    }
    let cmds = crate::segmenter::segment_commands_with_offset_and_config(body, 0, config);
    for cmd in &cmds {
        // `namespace eval NS BODY` — the trailing body word evaluates in NS's
        // frame, so exclude it; the NS-name word (e.g. `namespace eval $x …`)
        // is still substituted in the caller frame and is scanned.
        let is_ns_eval = cmd.texts.first().map(String::as_str) == Some("namespace")
            && cmd.texts.get(1).map(String::as_str) == Some("eval");
        let skip_last = is_ns_eval && cmd.texts.len() >= 4;
        let last_idx = cmd.texts.len().saturating_sub(1);
        for (i, word) in cmd.texts.iter().enumerate() {
            if skip_last && i == last_idx {
                continue;
            }
            if word_references_param(word, param) {
                return true;
            }
        }
    }
    false
}

/// True when a single word textually references `$param` / `${param}`.  Flat
/// byte scan with identifier-boundary and `\$` escape handling.
fn word_references_param(body: &str, param: &str) -> bool {
    if param.is_empty() {
        return false;
    }
    let bytes = body.as_bytes();
    let plen = param.len();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c != b'$' {
            i += 1;
            continue;
        }
        // Skip backslash-escaped ``\$``.
        if i > 0 && bytes[i - 1] == b'\\' {
            i += 1;
            continue;
        }
        // ``${name}`` form.
        if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let start = i + 2;
            if start + plen <= bytes.len()
                && &bytes[start..start + plen] == param.as_bytes()
                && start + plen < bytes.len()
                && bytes[start + plen] == b'}'
            {
                return true;
            }
        } else {
            // ``$name`` form — bare identifier match.
            let start = i + 1;
            if start + plen <= bytes.len() && &bytes[start..start + plen] == param.as_bytes() {
                let after = start + plen;
                let next_ok = after >= bytes.len() || !is_ident_continue(bytes[after]);
                if next_ok {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Must-policy over a channel argument's type union: `Some(label)` when
/// EVERY member is a non-channel — whatever path ran, the value cannot be a
/// channel — with the members rendered `"INT | STRING"` for the message. A
/// union with any Channel member, or an Unknown / Overdefined node, returns
/// `None`: some path may be fine.
fn non_channel_union_label(var_type: &crate::types::TypeLattice) -> Option<String> {
    use crate::types::{TypeKind, TypeShape};
    if !matches!(var_type.kind(), TypeKind::Known | TypeKind::Shimmered) {
        return None;
    }
    let member_types: Vec<tcl_registry::TclType> =
        var_type.shapes().iter().map(TypeShape::coarse).collect();
    if member_types.is_empty()
        || member_types
            .iter()
            .any(|t| matches!(t, tcl_registry::TclType::Channel))
    {
        return None;
    }
    Some(
        member_types
            .iter()
            .map(|t| format!("{t:?}").to_uppercase())
            .collect::<Vec<_>>()
            .join(" | "),
    )
}

/// Whether the def site's statement is an array-element write
/// (`set a(k) …` / `set a($k) …` / `incr a(k)`) — the base def such a
/// write records carries no reportable liveness of its own.
fn def_is_element_write(
    fu: &crate::compilation_unit::FunctionUnit,
    def: &crate::def_use::DefSite,
) -> bool {
    use crate::ir::Statement;
    fu.cfg
        .block_by_name(&def.block)
        .and_then(|b| {
            usize::try_from(def.statement_index)
                .ok()
                .and_then(|i| b.statements.get(i))
        })
        .is_some_and(|stmt| {
            matches!(
                stmt,
                Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. }
                    if name.contains('(')
            )
        })
}

#[cfg(test)]
mod issue996_tests {
    use super::*;

    /// Regression coverage for issue #996: `collect_expr_command_texts`
    /// recurses once per `ExprNode` level with no depth cap before this fix.
    /// A tree built directly is unbounded (the Pratt parser caps its own
    /// output at 256) and empirically overflowed the native stack (SIGABRT)
    /// in the low thousands of levels on a 2 MiB thread. 3000 is past that
    /// crash range and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is
    /// that it returns at all.
    #[test]
    fn deeply_nested_collect_expr_command_texts_survives() {
        let mut node = ExprNode::Command {
            text: "[x]".into(),
            start: 0,
            end: 3,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let mut out = Vec::new();
        collect_expr_command_texts(&node, &mut out);
    }
}
