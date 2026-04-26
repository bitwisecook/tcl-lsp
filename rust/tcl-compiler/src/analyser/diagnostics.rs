//! Diagnostic-emission orchestrator — Rust port of
//! `core/analysis/_analyser/_diagnostics.py`.
//!
//! Three top-level methods, mirroring the Python file 1:1:
//!
//! - [`Analyser::emit_variable_usage_diagnostics`] — kept as a
//!   no-op hook for future scope-tree consumers (Python's W211
//!   moved to the SSA-based pass; same here).
//! - [`Analyser::emit_cfg_ssa_diagnostics`] — main entry; builds
//!   a [`CompilationUnit`] on demand, walks the top-level
//!   function and every procedure, dispatches per-function
//!   diagnostics, and runs the cross-function post-passes
//!   (var-as-command, interpolated-command resolution).
//! - [`Analyser::emit_cfg_ssa_diagnostics_for_function`] —
//!   per-function dispatcher; calls each landed emitter in
//!   declaration order.
//!
//! Two utility passes round out the Python file:
//!
//! - [`Analyser::dedupe_diagnostics`] — drop exact duplicates
//!   plus the line-based pairs (E002 swallowed by E101 on the
//!   same line; W122 swallowed by W124 on the same line).
//! - [`Analyser::apply_disabled_diagnostics`] — filter out
//!   codes the caller asked to silence.
//!
//! **Strip-by-strip status.**
//!
//! - **C41d1** — orchestrator scaffold + dedupe + disabled-
//!   codes filter.  ✅ landed.
//! - **C41d2** — `_diag_var_lifecycle.py`.  ✅ landed:
//!   W220 (dead store), W211 (unused variable), W214
//!   (unused parameter), W210 (read-before-set), W213
//!   (unset on possibly-undef), and H300 (paste error).
//!   W210 / W213 are gated on procs only — top-level RBS
//!   needs the ``globals_written_by_procs`` filter Python
//!   uses, deferred until interproc analysis is wired in.
//! - **C41d3** — `_diag_var_command.py`.  ✅ landed:
//!   ``var_command_sites`` / ``cmd_command_sites`` recorded
//!   during the walk dispatch; **W307** (non-literal command
//!   name) and **W308** (unknown method on object) both emit
//!   via the cross-function post-pass.  W308 uses the C41e0
//!   ``ClassHierarchy::method_target`` for MRO-aware method
//!   resolution, with all the Python suppression paths
//!   wired (inherited ``unknown`` handler, external
//!   superclass, ``oo::objdefine`` per-instance methods).
//!   The ``[cmd] method`` return-type suppression for W307
//!   on ``cmd_command_sites`` remains deferred — it needs
//!   IR-level type-lattice plumbing extended into the
//!   analyser, which is a separate strip.
//! - **C41d4** — `_diag_commands.py`.  ✅ partial: W123
//!   (unknown command) is wired via the cross-function post-
//!   pass.  ``command_invocations`` are now recorded for every
//!   command head during the walk dispatch.  Deferred:
//!   ``_resolve_interpolated_commands`` (CONSTSET-driven W123
//!   suppression for ``$``-bearing names),
//!   ``_globals_written_by_procs`` (used by the C41d2 W210
//!   top-level RBS filter), ``suggest_similar`` "did you
//!   mean…?" suggestions, and the
//!   ``unknown_proc_info`` / ``has_dynamic_providers``
//!   early-returns.
//! - **C41d5** — `_diag_branches.py` + `_diag_channel.py`.
//!   ✅ landed: I230 / I231 (constant branch / switch-arm) and
//!   W126 (channel argument validation) all wired through the
//!   per-function dispatcher.  Severity-Info Python diagnostics
//!   map to ``Severity::Hint`` here (no Info variant on the
//!   Rust side).
//! - **C41d6** — `_diag_ip.py`.  ✅ landed: W124 (invalid IP
//!   address literal) — IPv4 octet validation (over-255 →
//!   Error, leading-zero → Warning) and IPv6 parsing via
//!   ``std::net::Ipv6Addr``.  Anchors at the SSA def site;
//!   seen-offsets dedup avoids duplicates across SSA versions.
//! - **C41d7** — `_diag_racy.py`.  ⏸ deferred: IRULE4005
//!   (racy ``static::`` cross-event flow) needs the
//!   connection-scope / cross-event analysis that the Rust
//!   pipeline doesn't yet have (Python's
//!   ``cu.connection_scope.racy_static_defs``).  Once
//!   ``ConnectionScope`` lands on the Rust side, the emitter
//!   wires up in a single call to ``emit_racy_static_diagnostics``.

use std::collections::HashSet;

use tcl_lexer::SourceMap;

use super::state::Analyser;
use super::types::Severity;

/// Find a case-insensitive match for `variable` in `defined_vars`.
///
/// Mirrors `_find_case_mismatch` in
/// `core/analysis/_analyser/_diag_var_lifecycle.py:135-148`.
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

/// Collect every variable name defined anywhere in `cfg`.
///
/// Mirrors `_collect_defined_vars` in
/// `_diag_var_lifecycle.py:123-133`.  Walks every block and pulls
/// the `defs` field off each [`crate::ir::Statement`] that has
/// one (assignments, ``incr``, ``Call`` statements with explicit
/// defs).  Used for the "did you mean…?" case-mismatch
/// suggestion in W210 / W211 / W220 messages.
fn collect_defined_vars(cfg: &crate::cfg::Function) -> HashSet<String> {
    use crate::ir::Statement;
    let mut names: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let normalised = crate::naming::normalise_var_name(name);
                    if !normalised.is_empty() {
                        names.insert(normalised.to_string());
                    }
                }
                Statement::Call { defs, .. } => {
                    for def in defs {
                        names.insert(def.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Compute the set of global variable names that any procedure
/// in `cu` writes.
///
/// Mirrors `_globals_written_by_procs` in
/// `core/analysis/_analyser/_diag_commands.py:264-296`.
///
/// A global write happens when a proc either:
///
/// 1. assigns to a fully-qualified name (``::var``), or
/// 2. declares ``global var`` and then assigns to ``var`` in the
///    same proc body.
///
/// The result is the union of (1) and the intersection of
/// global aliases × locally-written names (case (2)).  Used at
/// top-level to suppress W210 for globals a helper proc may
/// populate before the top-level read.
///
/// **Simplification vs. Python.** The Rust port doesn't yet
/// have ``CommandRegistry::is_destroys_variable`` so commands
/// like ``unset`` aren't filtered out of the "writes" set.
/// That makes the suppression slightly more permissive (more
/// vars marked "written-by-procs" → more W210 suppressions).
/// Safe-on-correctness — the alternative is false positives
/// on real RBS sites.  When the registry gains
/// ``destroys_variable``, add the filter here for parity.
fn globals_written_by_procs(cu: &crate::compilation_unit::CompilationUnit) -> HashSet<String> {
    use crate::ir::Statement;
    let mut result: HashSet<String> = HashSet::new();
    for fu in cu.procedures.values() {
        let mut global_aliases: HashSet<String> = HashSet::new();
        let mut written: HashSet<String> = HashSet::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let names: Vec<&String> = match stmt {
                    Statement::Call { command, defs, .. } => {
                        if command == "global" {
                            for d in defs {
                                global_aliases.insert(d.clone());
                            }
                            continue;
                        }
                        if matches!(command.as_str(), "variable" | "upvar") {
                            continue;
                        }
                        defs.iter().collect()
                    }
                    Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => vec![name],
                    _ => continue,
                };
                for name in names {
                    if let Some(bare) = name.strip_prefix("::") {
                        let bare = bare.trim_start_matches(':');
                        if !bare.is_empty() {
                            result.insert(bare.to_string());
                        }
                    } else {
                        written.insert(name.clone());
                    }
                }
            }
        }
        for n in global_aliases.intersection(&written) {
            result.insert(n.clone());
        }
    }
    result
}

impl Analyser {
    /// Scope-tree-driven variable diagnostic emitter.
    ///
    /// Mirrors `_emit_variable_usage_diagnostics` in
    /// `_diagnostics.py:111-116`.  Python keeps this method as
    /// an empty hook because W211 (unused-variable) moved to the
    /// SSA-based pass in `_emit_cfg_ssa_diagnostics_for_function`.
    /// The Rust port preserves the hook so future scope-tree-
    /// driven emitters (none currently planned) have a target.
    pub fn emit_variable_usage_diagnostics(&mut self) {
        // Intentionally empty — see module docstring.
    }

    /// CFG/SSA-backed diagnostic orchestrator.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics` in
    /// `_diagnostics.py:118-181`.  Builds a
    /// [`crate::compilation_unit::CompilationUnit`] for `source`,
    /// then walks the top-level + every procedure, dispatching
    /// per-function emitters.
    ///
    /// **C41d2 lands** the full ``_diag_var_lifecycle.py``
    /// emitter set (W220, W211, W214, W210, W213, H300).
    /// **C41d3 lands** the var-as-command post-pass (W307); W308
    /// awaits the class-hierarchy port.  W242 (interpolated-
    /// command resolution) lands in **C41d4**.
    pub fn emit_cfg_ssa_diagnostics(&mut self, source: &str) {
        use tcl_registry::prelude::DialectSet;
        use tcl_registry::CommandRegistry;

        let mut registry = CommandRegistry::build_default();
        if let Some(d) = DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);

        // **C41e3 follow-up.** Compute the set of globals any
        // proc in this module writes to.  Top-level RBS (W210)
        // is suppressed for these variables — a helper proc may
        // populate them before the top-level read fires.
        // Mirrors `_globals_written_by_procs` in
        // `_diag_commands.py:264-296`.
        let globals_written = globals_written_by_procs(&cu);

        // Top-level first, then procedures in insertion order —
        // matches the iteration order of
        // ``CompilationUnit::functions``.
        // Iterate top-level explicitly so we can pass the IR
        // module through.
        self.emit_cfg_ssa_diagnostics_for_function_with_extra(
            &cu.top_level,
            &cu.ir_module,
            &globals_written,
        );
        self.emit_channel_diagnostics(&cu.top_level, &registry);
        for (qname, fu) in &cu.procedures {
            self.emit_cfg_ssa_diagnostics_for_function(fu, &cu.ir_module);
            self.emit_channel_diagnostics(fu, &registry);
            // **C41d7.** IRULE4005 — racy ``static::``
            // cross-event flow.  Only fires for non-RULE_INIT
            // ``when`` procs when ``ConnectionScope::racy_static_defs``
            // is non-empty.  Mirrors Python's
            // ``_emit_racy_static_diagnostics`` call site in
            // ``_diagnostics.py:171-175``.
            if let Some(scope) = cu.connection_scope.as_ref() {
                if qname.starts_with("::when::") && !scope.racy_static_defs.is_empty() {
                    let event = crate::ir::when_event_name(qname);
                    if event != "RULE_INIT" {
                        self.emit_racy_static_diagnostics(fu, &scope.racy_static_defs);
                    }
                }
            }
        }

        // Cross-function post-pass: resolve $var-as-command sites
        // collected during the walk.  Mirrors
        // ``_emit_var_command_diagnostics`` in
        // ``_diag_var_command.py``.
        self.emit_var_command_diagnostics(&cu, &registry);
    }

    /// Per-function diagnostic dispatcher.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics_for_function` in
    /// `_diagnostics.py:183-209`.  Called once for the top-level
    /// script and once per procedure.  Each per-emitter call is
    /// gated on its own predicate inside the helper.
    ///
    /// **C41d2 wires** all six ``_diag_var_lifecycle.py``
    /// emitters.  Each future C41d strip adds another emitter
    /// call here.
    pub fn emit_cfg_ssa_diagnostics_for_function(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
    ) {
        self.emit_cfg_ssa_diagnostics_for_function_with_extra(
            function_unit,
            ir_module,
            &HashSet::new(),
        );
    }

    /// Per-function diagnostic dispatcher with an extra
    /// "known-defined" set passed through to RBS suppression.
    ///
    /// Same as [`Self::emit_cfg_ssa_diagnostics_for_function`]
    /// but accepts an additional set of variable names that
    /// should be treated as already-defined for the W210
    /// (read-before-set) emitter.  Used at the top-level to
    /// suppress RBS for variables that any proc in the module
    /// writes — matches the
    /// ``extra_known_defined_vars=self._globals_written_by_procs(cu)``
    /// argument in `_diagnostics.py:154`.
    pub fn emit_cfg_ssa_diagnostics_for_function_with_extra(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
        extra_known_defined: &HashSet<String>,
    ) {
        let defined = collect_defined_vars(&function_unit.cfg);
        let scope_aliases = crate::optimiser::elimination::scan_scope_aliases(&function_unit.cfg);
        let textually_referenced = crate::optimiser::elimination::collect_textual_var_references(
            &self.source,
            &function_unit.cfg,
        );
        let ir_proc = ir_module.procedures.get(&function_unit.name);
        self.emit_dead_store_diagnostics(function_unit, &defined, &scope_aliases);
        self.emit_unused_variable_diagnostics(
            function_unit,
            &defined,
            &scope_aliases,
            &textually_referenced,
        );
        self.emit_possible_paste_error_diagnostics(function_unit);
        self.emit_read_before_set_diagnostics(
            function_unit,
            ir_proc,
            &defined,
            &scope_aliases,
            extra_known_defined,
        );
        self.emit_constant_branch_diagnostics(function_unit);
        self.emit_invalid_ip_diagnostics(function_unit);
        if let Some(ir_proc) = ir_proc {
            self.emit_unused_param_diagnostics(function_unit, ir_proc);
        }
    }

    /// W220 — dead-store hint.
    ///
    /// Mirrors `_emit_dead_store_diagnostics` in
    /// `_diag_var_lifecycle.py:29-72`.  A *dead store* is an
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
    /// **Known limitations** (deferred to a follow-up): no
    /// scope-alias filter (vars introduced via ``global`` /
    /// ``upvar`` are visible elsewhere and may be falsely
    /// flagged); no textual-reference filter (vars only ever
    /// referenced inside ``"$x"`` interpolations or inside a
    /// ``Return`` value miss the def-use scan).
    fn emit_dead_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            // Scope-aliased vars (introduced via ``global`` or
            // ``upvar``) write through to a different scope — the
            // local "no use" verdict is unsafe.
            if scope_aliases.contains(var) {
                continue;
            }
            // ``any_other_live`` — another SSA version of this
            // variable has live uses, so this assignment is
            // overwritten.  When no other version is live, the
            // variable is truly unused — that's W211, handled
            // separately.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *var && k.1 != *version && !c.is_dead());
            if !any_other_live {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            let span = stmt.span();
            if span.is_empty() {
                continue;
            }
            let mut message = format!("Assignment to '{var}' is never read");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W220".to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// W211 — unused-variable hint.
    ///
    /// Mirrors `_emit_unused_variable_diagnostics` in
    /// `_diag_var_lifecycle.py:226-258`.  Fires when an
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
    fn emit_unused_variable_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        textually_referenced: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            if scope_aliases.contains(var) {
                continue;
            }
            if textually_referenced.contains(var) {
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
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            let span = stmt.span();
            if span.is_empty() {
                continue;
            }
            let mut message = format!("Variable '{var}' is set but never used");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W211".to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// H300 — possible paste error (duplicate dead-store with
    /// identical literal).
    ///
    /// Mirrors `_emit_possible_paste_error_diagnostics` in
    /// `_diag_var_lifecycle.py:74-121`.  When two consecutive
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
    fn emit_possible_paste_error_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::def_use::DefKind;
        use std::collections::HashMap;

        // Pre-compute, per block, the set of statement indices
        // that are dead stores.  Walk every dead Statement-kind
        // chain in def_use, bucket by block.
        let mut dead_idx: HashMap<&str, HashSet<usize>> = HashMap::new();
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

        for (block_name, block) in &fu.cfg.blocks {
            let Some(dead_indices) = dead_idx.get(block_name.as_str()) else {
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
                let span = block.statements[idx + 1].span();
                if span.is_empty() {
                    continue;
                }
                let pretty = super::utils::format_literal_for_message(&literal);
                let message = format!(
                    "Possible paste error: repeated assignment to '{var_name}' \
                     with static value '{pretty}'; \
                     did you mean to assign a different variable?"
                );
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "H300".to_string(),
                    span,
                    message,
                    severity: Severity::Hint,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// W214 — unused-parameter hint.
    ///
    /// Mirrors `_emit_unused_param_diagnostics` in
    /// `_diag_var_lifecycle.py:260-274`.  For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the proc's span.
    ///
    /// Diverges slightly from Python's ``analysis.unused_params``:
    /// Python pre-computes the unused-params list during
    /// ``analyse_ir_module``; the Rust port inlines the same
    /// def-use scan here because the Rust ``FunctionAnalysis``
    /// builder hasn't been ported yet.  The check is equivalent —
    /// a parameter is unused iff no SSA version of its name has
    /// live uses.
    fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
    ) {
        for param in &ir_proc.params {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if param == "args" {
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
            let message = format!(
                "Parameter '{param}' of proc '{name}' is unused",
                name = ir_proc.qualified_name,
            );
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W214".to_string(),
                span: ir_proc.span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// W210 + W213 — read-before-set / unset on possibly-undefined.
    ///
    /// Mirrors `_emit_read_before_set_diagnostics` in
    /// `_diag_var_lifecycle.py:159-224`.  Walks every
    /// version-0 chain (`DefKind::Parameter`) in `fu.def_use`
    /// — those are the synthetic defs the def-use builder
    /// emits when a variable is used without a preceding def.
    ///
    /// Distinguishes real proc parameters from synthetic RBS
    /// reads via `ir_proc.params`.  Only emits inside procedures
    /// (i.e. when `ir_proc` is `Some`) — top-level RBS would
    /// need the `globals_written_by_procs` filter Python uses
    /// (deferred to a later strip).
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
    fn emit_read_before_set_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        extra_known_defined: &HashSet<String>,
    ) {
        use crate::def_use::{DefKind, UseKind};
        use crate::ir::Statement;
        use std::fmt::Write as _;

        // **C41e3 follow-up.** Top-level RBS now uses the
        // ``extra_known_defined`` set (computed from
        // ``globals_written_by_procs``) to suppress W210 on
        // globals that helper procs write.  Inside procs the
        // set is empty, matching Python's per-call argument.
        let params_owned: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        let params = &params_owned;

        for chain in fu.def_use.chains.values() {
            if chain.definition.kind != DefKind::Parameter {
                continue;
            }
            let (var, _version) = &chain.key;
            if params.contains(var.as_str()) {
                continue;
            }
            if scope_aliases.contains(var) {
                continue;
            }
            if extra_known_defined.contains(var) {
                continue;
            }
            for use_site in &chain.uses {
                if matches!(use_site.kind, UseKind::PhiIncoming) {
                    continue;
                }
                let Some(block) = fu.cfg.blocks.get(&use_site.block) else {
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
                        (span, None)
                    } else {
                        let Ok(idx) = usize::try_from(use_site.statement_index) else {
                            continue;
                        };
                        let Some(stmt) = block.statements.get(idx) else {
                            continue;
                        };
                        (stmt.span(), Some(stmt))
                    };
                if span.is_empty() {
                    continue;
                }
                // ``unset`` without ``-nocomplain`` → W213.
                if let Some(Statement::Call { command, args, .. }) = stmt_opt {
                    if command == "unset" && !args.iter().any(|a| a == "-nocomplain") {
                        let message = format!(
                            "Variable '{var}' may not exist; \
                             use 'unset -nocomplain' to suppress the error",
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W213".to_string(),
                            span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                        continue;
                    }
                }
                // ``safe_on_uninit`` calls that initialise the
                // variable themselves are not RBS — they handle
                // the uninitialised case.
                if let Some(Statement::Call {
                    safe_on_uninit,
                    defs,
                    ..
                }) = stmt_opt
                {
                    if *safe_on_uninit && defs.contains(var) {
                        continue;
                    }
                }
                let mut message = format!("Variable '{var}' is read before it is set");
                if let Some(similar) = find_case_mismatch(var, defined_vars) {
                    let _ = write!(message, "; did you mean '{similar}'?");
                }
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W210".to_string(),
                    span,
                    message,
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// I230 / I231 — constant branch / switch-arm condition.
    ///
    /// Mirrors `_emit_constant_branch_diagnostics` in
    /// `core/analysis/_analyser/_diag_branches.py`.  For every
    /// branch SCCP folded to a constant, when the *not-taken*
    /// target is also unreachable (i.e. SCCP confirmed only one
    /// path is feasible), emit an Info-level diagnostic so the
    /// LSP can highlight the dead arm.
    ///
    /// Code selection follows the Python rules:
    /// - Block name starts with ``switch_`` → I231 (switch-arm).
    /// - Block name starts with ``if_`` → I230 (constant if).
    /// - Otherwise → I230 with the generic
    ///   ``"Branch condition '...' is constant"`` message.
    ///
    /// Severity is mapped to ``Hint`` because the Rust
    /// [`Severity`] enum has no ``Info`` variant — ``Hint`` is
    /// the closest non-actionable level.
    fn emit_constant_branch_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        for branch in &fu.sccp.constant_branches {
            // The Python check is "not_taken_target in
            // unreachable_blocks".  Rust SCCP exposes
            // ``executable_blocks`` (the complement); a block
            // is unreachable iff it's in ``cfg.blocks`` but
            // NOT in ``executable_blocks``.
            if fu.sccp.executable_blocks.contains(&branch.not_taken_target) {
                continue;
            }
            // Locate the branch's terminator span.
            let Some(block) = fu.cfg.blocks.get(&branch.block) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Branch {
                span: Some(span), ..
            }) = &block.terminator
            else {
                continue;
            };
            let span = *span;

            let names = [
                branch.block.as_str(),
                branch.taken_target.as_str(),
                branch.not_taken_target.as_str(),
            ];
            let is_switch = names.iter().any(|n| n.starts_with("switch_"));
            let is_if = names.iter().any(|n| n.starts_with("if_"));

            let (code, message) = if is_switch {
                let code = "I231";
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
                ("I230", msg)
            } else {
                let msg = format!(
                    "Branch condition '{}' is constant; one branch is unreachable",
                    branch.condition,
                );
                ("I230", msg)
            };

            self.result.diagnostics.push(super::types::Diagnostic {
                code: code.to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// W126 — channel-argument validation.
    ///
    /// Mirrors `_emit_channel_diagnostics` in
    /// `core/analysis/_analyser/_diag_channel.py`.  Walks every
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
    fn emit_channel_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::ir::Statement;
        use crate::types::TypeKind;
        use tcl_registry::ArgRole;

        const STANDARD_CHANNELS: &[&str] = &["stdout", "stderr", "stdin"];

        for block in fu.ssa.blocks.values() {
            for ssa_stmt in &block.statements {
                let Statement::Call {
                    command,
                    args,
                    span,
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
                        let Some(&version) = ssa_stmt.uses.get(name) else {
                            continue;
                        };
                        let key: crate::ssa::ValueKey = (name.to_string(), version);
                        let Some(var_type) = fu.types.get(&key) else {
                            continue;
                        };
                        if var_type.kind != TypeKind::Known {
                            continue;
                        }
                        let Some(tcl_type) = var_type.tcl_type else {
                            continue;
                        };
                        if matches!(tcl_type, tcl_registry::TclType::Channel) {
                            continue;
                        }
                        let type_label = format!("{tcl_type:?}").to_uppercase();
                        let message = format!(
                            "Variable '${name}' passed as channel to '{command}' \
                             has type {type_label}, not CHANNEL.",
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W126".to_string(),
                            span: *span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
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
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W126".to_string(),
                            span: *span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    /// W124 — invalid IP address literal.
    ///
    /// Mirrors `_emit_invalid_ip_diagnostics` in
    /// `core/analysis/_analyser/_diag_ip.py`.  Walks every
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
    fn emit_invalid_ip_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::net::Ipv6Addr;
        use std::str::FromStr;

        let dotted_quad =
            regex::Regex::new(r"\b(\d{1,4})\.(\d{1,4})\.(\d{1,4})\.(\d{1,4})\b").expect("regex");
        let ipv6_candidate =
            regex::Regex::new(r"\b([0-9A-Fa-f]{1,4}(?::[0-9A-Fa-f]{0,4}){2,7})\b").expect("regex");

        let mut seen_offsets: HashSet<u32> = HashSet::new();
        for (key, lv) in &fu.sccp.values {
            let Some(text) = (match lv {
                LatticeValue::Const(ConstValue::String(s)) => Some(s.as_str()),
                _ => None,
            }) else {
                continue;
            };

            // ---- IPv4 candidates ----
            for caps in dotted_quad.captures_iter(text) {
                let m = caps.get(0).unwrap();
                if m.start() > 0 && text.as_bytes()[m.start() - 1] == b'/' {
                    continue;
                }
                let octets: Vec<&str> = (1..=4).map(|i| caps.get(i).unwrap().as_str()).collect();
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
                    self.emit_ip_diag_at_def(fu, key, &msg, sev, &mut seen_offsets);
                    break;
                }
            }

            // ---- IPv6 candidates ----
            for caps in ipv6_candidate.captures_iter(text) {
                let candidate = caps.get(1).unwrap().as_str();
                if Ipv6Addr::from_str(candidate).is_err() {
                    let msg = format!("Invalid IPv6 address '{candidate}'.");
                    self.emit_ip_diag_at_def(fu, key, &msg, Severity::Error, &mut seen_offsets);
                    break;
                }
            }
        }
    }

    /// Helper for [`Self::emit_invalid_ip_diagnostics`].
    fn emit_ip_diag_at_def(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        key: &crate::ssa::ValueKey,
        message: &str,
        severity: Severity,
        seen_offsets: &mut HashSet<u32>,
    ) {
        let (var_name, version) = key;
        let Some(chain) = fu.def_use.chain_for(var_name, *version) else {
            return;
        };
        let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
            return;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            return;
        };
        let Some(stmt) = block.statements.get(idx) else {
            return;
        };
        let span = stmt.span();
        if span.is_empty() {
            return;
        }
        if !seen_offsets.insert(span.start()) {
            return;
        }
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W124".to_string(),
            span,
            message: message.to_string(),
            severity,
            fixes: Vec::new(),
        });
    }

    /// W123 — unknown / unresolved command head.
    ///
    /// Mirrors `_emit_unresolved_command_diagnostics` in
    /// `core/analysis/_analyser/_diag_commands.py:39-186`.
    /// Walks every command invocation recorded during the
    /// analyser walk and emits W123 ("Unknown command 'X'")
    /// when no matching definition is in scope.
    ///
    /// Resolution paths checked in order — first match
    /// suppresses W123:
    ///
    /// - `cmd_name in registry_names` (built-in command).
    /// - `cmd_name` contains `::` (qualified — defer to
    ///   per-namespace logic, conservative skip).
    /// - `cmd_name` starts with `$` / `[` (interpolated /
    ///   substituted head — handled by W307 / W308).
    /// - User-defined proc tail or absolute name.
    /// - User-defined class tail or absolute name.
    /// - Command alias tail.
    /// - Ensemble namespace tail.
    ///
    /// Idempotency: ``self.unresolved_commands_emitted`` guards
    /// against double-emission when ``analyse`` is called twice
    /// or the chunked entry runs both passes.
    ///
    /// **Deferred** (Python parity gaps documented in the
    /// commit body): ``has_dynamic_providers`` early-return;
    /// ``stub_commands`` candidate set; ``suggest_similar``
    /// "did you mean…?" suggestions and the ``CodeFix``
    /// payload; the CONSTSET-driven interpolation suppression
    /// for ``$``-bearing command names.
    pub fn emit_unresolved_command_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.unresolved_commands_emitted {
            return;
        }
        self.unresolved_commands_emitted = true;
        if self.disabled_diagnostics.contains("W123") {
            return;
        }

        // Conservative gate: if any ``package require`` was seen,
        // suppress W123 entirely.  The package may load arbitrary
        // commands at runtime that the analyser can't see.
        if !self.result.package_requires.is_empty() {
            return;
        }

        // **C41e3 follow-up.** When the document defines a
        // user-level ``unknown`` proc, the handler may resolve
        // any command name at runtime — the analyser can't
        // statically prove a command is unresolved.  Match
        // Python's
        // ``_diag_commands.py:_emit_unresolved_command_diagnostics``
        // early-return.  An empty-stub ``unknown`` (``proc
        // unknown {cmd args} {}``) intentionally resolves
        // nothing, so don't suppress for that shape.
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            if !info.empty_stub {
                return;
            }
        }

        let registry_names: HashSet<String> =
            registry.command_names().map(str::to_string).collect();
        let proc_tail_names: HashSet<String> = self
            .result
            .all_procs
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let class_tail_names: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let alias_names: HashSet<String> = self
            .result
            .command_aliases
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let ensemble_cmds: HashSet<String> = self
            .ensemble_namespaces
            .iter()
            .filter_map(|ns| ns.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        // Drain so the iteration loop can mutate
        // ``self.result.diagnostics`` freely; restore at the end
        // (matches the snapshot/restore round-trip contract).
        let invocations = std::mem::take(&mut self.result.command_invocations);
        for inv in &invocations {
            let name = &inv.name;
            if registry_names.contains(name) {
                continue;
            }
            if name.contains("::") {
                continue;
            }
            if name.starts_with('$') || name.starts_with('[') {
                continue;
            }
            if proc_tail_names.contains(name) {
                continue;
            }
            if class_tail_names.contains(name) {
                continue;
            }
            if alias_names.contains(name) {
                continue;
            }
            if ensemble_cmds.contains(name) {
                continue;
            }
            // Absolute-form fallback — ``cmd`` may be defined as
            // ``::cmd`` in the global namespace.
            if self.result.all_procs.contains_key(&format!("::{name}")) {
                continue;
            }
            if self.result.all_classes.contains_key(&format!("::{name}")) {
                continue;
            }

            let message = format!("Unknown command '{name}'");
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W123".to_string(),
                span: inv.range,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
        self.result.command_invocations = invocations;
    }

    /// W307 — non-literal command name (variable / command-sub
    /// used as command head).
    ///
    /// Mirrors the W307 half of `_emit_var_command_diagnostics`
    /// in `core/analysis/_analyser/_diag_var_command.py:22-294`.
    /// Walks every recorded site in [`Self::var_command_sites`]
    /// and emits W307 unless the variable's value is statically
    /// resolvable to a finite set of known command names.
    ///
    /// **Resolution paths** (mirrors Python; first match
    /// suppresses W307):
    ///
    /// - Aggregate every CONSTSET / CONST entry in `cu`'s SCCP
    ///   results for the variable name; if every value in the
    ///   set is a known command, proc, class, or class-tail name,
    ///   the command head is statically resolvable — suppress.
    ///
    /// **Known limitations.**  W308 (unknown method on object)
    /// is deferred to a follow-up — it needs the
    /// `class_hierarchy` / MRO port (the C41e0 architectural
    /// decision still pending).  Likewise the
    /// `_cmd_command_sites` (``[cmd] method``) suppression via
    /// return-type analysis is deferred — that path needs the
    /// IR-level type-lattice plumbing extended into the
    /// analyser, which is a larger change than fits this strip.
    /// In-method W307 suppression and dict-with /
    /// dict-update barrier-range suppression also defer.
    #[allow(clippy::too_many_lines)]
    fn emit_var_command_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
        use crate::types::TypeKind;
        use std::collections::HashMap;

        if self.var_command_sites.is_empty() {
            return;
        }
        // Aggregate type-lattice knowledge per variable name
        // across every FunctionUnit.  For each var with a
        // ``TclType::Object`` lattice entry that has a
        // ``class_name``, record the class qualified name so
        // W308 can validate the method against the class
        // hierarchy.  Mirrors the ``all_typed_vars`` /
        // ``all_types`` aggregation in
        // ``_diag_var_command.py:49-67``.
        let mut all_object_types: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_object_types =
            |types: &HashMap<crate::ssa::ValueKey, crate::types::TypeLattice>,
             out: &mut HashMap<String, HashSet<String>>| {
                for ((var_name, _ver), tl) in types {
                    if tl.kind != TypeKind::Known {
                        continue;
                    }
                    if !matches!(tl.tcl_type, Some(tcl_registry::TclType::Object)) {
                        continue;
                    }
                    let Some(class_name) = &tl.class_name else {
                        continue;
                    };
                    out.entry(var_name.clone())
                        .or_default()
                        .insert(class_name.clone());
                }
            };
        collect_object_types(&cu.top_level.types, &mut all_object_types);
        for fu in cu.procedures.values() {
            collect_object_types(&fu.types, &mut all_object_types);
        }

        // Build the class hierarchy once for W308 method
        // resolution (uses the C41e0 ``ClassHierarchy``).
        let hierarchy = if self.result.all_classes.is_empty() {
            None
        } else {
            Some(super::class_hierarchy::build_class_hierarchy(
                self.result.all_classes.clone(),
            ))
        };

        // Aggregate constant-string knowledge per variable name
        // across every function in the CompilationUnit.  Python
        // uses ``_lattice_to_set`` which expands CONST and
        // CONSTSET into a flat set of values; we replicate that
        // shape here.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |sccp: &crate::sccp::SccpResult,
                            out: &mut HashMap<String, HashSet<String>>| {
            for (key, lv) in &sccp.values {
                let (var_name, _ver) = key;
                let values: Option<Vec<String>> = match lv {
                    LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
                    LatticeValue::ConstSet(set) => set
                        .iter()
                        .map(|cv| match cv {
                            ConstValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>(),
                    _ => None,
                };
                let Some(values) = values else { continue };
                let entry = out.entry(var_name.clone()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
        collect_from(&cu.top_level.sccp, &mut all_constsets);
        for fu in cu.procedures.values() {
            collect_from(&fu.sccp, &mut all_constsets);
        }

        // Build the "known commands" universe — registry +
        // user-defined procs + class tail names.
        let known_cmds: HashSet<String> = registry.command_names().map(str::to_string).collect();
        let known_procs: HashSet<String> = self.result.all_procs.keys().cloned().collect();
        let known_proc_bare: HashSet<String> = known_procs
            .iter()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let known_class_tails: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        let is_known_command = |v: &str| {
            known_cmds.contains(v)
                || known_procs.contains(v)
                || known_proc_bare.contains(v)
                || known_procs.contains(&format!("::{v}"))
                || known_class_tails.contains(v)
                || self.result.all_classes.contains_key(&format!("::{v}"))
        };

        // Drain sites so we can borrow self.result mutably below.
        let sites = std::mem::take(&mut self.var_command_sites);
        let objdefined_vars = self.objdefined_vars.clone();
        for site in &sites {
            // **W308 path.**  Variable known to hold an Object
            // — validate the method name against the class
            // hierarchy.  When the method isn't found and the
            // class doesn't have an external superclass that
            // could carry it, emit W308.
            if let Some(class_names) = all_object_types.get(&site.var_name) {
                if let (Some(method_name), Some(hierarchy)) = (&site.method_name, &hierarchy) {
                    let mut found = false;
                    let mut has_local_class = false;
                    for cls in class_names {
                        if hierarchy.method_target(cls, method_name).is_some() {
                            found = true;
                            break;
                        }
                        if let Some(cd) = self.result.all_classes.get(cls) {
                            has_local_class = true;
                            if cd.methods.contains_key(method_name)
                                || cd.class_methods.contains_key(method_name)
                                || matches!(
                                    method_name.as_str(),
                                    "new" | "create" | "destroy" | "configure" | "cget"
                                )
                                || cd.methods.contains_key("unknown")
                            {
                                found = true;
                                break;
                            }
                        }
                    }
                    // Inherited ``unknown`` handler via MRO.
                    if !found && has_local_class {
                        for cls in class_names {
                            if hierarchy.method_target(cls, "unknown").is_some() {
                                found = true;
                                break;
                            }
                        }
                    }
                    // External superclass: a method might come
                    // from a class outside the current index.
                    if !found && has_local_class {
                        const OO_BASE: &[&str] = &["oo::object", "oo::class"];
                        'cls_loop: for cls in class_names {
                            if let Some(cd) = self.result.all_classes.get(cls) {
                                for s in &cd.superclasses {
                                    if !self.result.all_classes.contains_key(s)
                                        && !OO_BASE.contains(&s.as_str())
                                    {
                                        found = true;
                                        break 'cls_loop;
                                    }
                                }
                            }
                        }
                    }
                    // ``oo::objdefine`` adds per-instance
                    // methods we can't see at the class level.
                    if !found && objdefined_vars.contains(&site.var_name) {
                        found = true;
                    }
                    if !found && has_local_class && !self.disabled_diagnostics.contains("W308") {
                        let mut classes_sorted: Vec<&str> =
                            class_names.iter().map(String::as_str).collect();
                        classes_sorted.sort_unstable();
                        let cls_display = classes_sorted.join(", ");
                        let message =
                            format!("Unknown method '{method_name}' on class '{cls_display}'",);
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W308".to_string(),
                            span: site.cmd_span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
                // W307 path doesn't fire when the var is a
                // known Object — the method-name check is the
                // load-bearing piece.
                continue;
            }

            // **W307 path.**  Variable not a known Object.
            // ``in_method`` short-circuits W307 because OO
            // methods routinely use ``$obj method`` patterns.
            // The Rust analyser doesn't track method context
            // yet (lands in C41e — pending a Method scope kind),
            // so this filter currently matches Python's
            // ``in_method=False`` always-fall-through behaviour.
            if site.in_method {
                continue;
            }
            if let Some(values) = all_constsets.get(&site.var_name) {
                if !values.is_empty() && values.iter().all(|v| is_known_command(v)) {
                    continue;
                }
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W307".to_string(),
                span: site.cmd_span,
                message: "Non-literal command name — cannot statically analyze".to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        // Restore the sites list — snapshot/restore expects it
        // to round-trip independently of emission.
        self.var_command_sites = sites;
    }

    /// Drop exact-duplicate diagnostics + line-based suppression
    /// pairs.
    ///
    /// Mirrors `_dedupe_diagnostics` in
    /// `_diagnostics.py` (lives in `_core.py:595-630` — the
    /// orchestrator file imports it through the mixin
    /// hierarchy).  Two passes:
    ///
    /// 1. Compute the set of source lines on which `E101`
    ///    (missing-open-brace) and `W124` (SSA-based IP check)
    ///    fired.  These are sentinels for the related
    ///    redundant-message codes.
    /// 2. Walk diagnostics in source order, deduplicating by
    ///    `(code, span, message, severity)` and dropping:
    ///    - `E002` on a line where `E101` fired (the recovered
    ///      switch makes the arity message a false positive).
    ///    - `W122` on a line where `W124` fired (the SSA check
    ///      is more precise).
    ///
    /// Lines come from the [`SourceMap`] over `self.source`.
    pub fn dedupe_diagnostics(&mut self) {
        let sm = SourceMap::new(&self.source);
        let mut e101_lines: HashSet<u32> = HashSet::new();
        let mut w124_lines: HashSet<u32> = HashSet::new();
        for d in &self.result.diagnostics {
            let line = sm.range_positions(d.span).0.line;
            match d.code.as_str() {
                "E101" => {
                    e101_lines.insert(line);
                }
                "W124" => {
                    w124_lines.insert(line);
                }
                _ => {}
            }
        }

        let mut seen: HashSet<(String, u32, u32, String, Severity)> = HashSet::new();
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut deduped = Vec::with_capacity(drained.len());
        for d in drained {
            let key = (
                d.code.clone(),
                d.span.start(),
                d.span.end(),
                d.message.clone(),
                d.severity,
            );
            if seen.contains(&key) {
                continue;
            }
            let line = sm.range_positions(d.span).0.line;
            if d.code == "E002" && e101_lines.contains(&line) {
                continue;
            }
            if d.code == "W122" && w124_lines.contains(&line) {
                continue;
            }
            seen.insert(key);
            deduped.push(d);
        }
        self.result.diagnostics = deduped;
    }

    /// Filter out diagnostics whose codes are in
    /// [`Self::disabled_diagnostics`].
    ///
    /// Mirrors the per-emitter `if "Wxxx" in
    /// self._disabled_diagnostics:` early-returns in Python's
    /// emitter files.  Centralising the filter on the orchestrator
    /// side keeps the per-emitter code (in C41d2 / C41d3 / etc.)
    /// from having to thread the check at every emit site —
    /// emitters can push freely and the orchestrator drops the
    /// silenced codes at the end.
    ///
    /// Idempotent on an empty filter set (no allocations).
    pub fn apply_disabled_diagnostics(&mut self) {
        if self.disabled_diagnostics.is_empty() {
            return;
        }
        // Borrow-checker dance: `retain` closure can't capture
        // `&self.disabled_diagnostics` while ``self.result`` is
        // mut-borrowed; clone the set into a local first.  The
        // disabled set is small (LSP-config-scale) so the clone
        // cost is negligible vs. the rest of the diagnostics
        // pipeline.
        let disabled = self.disabled_diagnostics.clone();
        self.result
            .diagnostics
            .retain(|d| !disabled.contains(&d.code));
    }

    /// IRULE4005 — racy ``static::`` cross-event flow.
    ///
    /// Mirrors `_emit_racy_static_diagnostics` in
    /// `core/analysis/_analyser/_diag_racy.py`.  Walks every
    /// SSA statement in `fu` and emits IRULE4005 for any
    /// non-``unset`` def of a name in `racy_vars`.
    /// `racy_vars` comes from
    /// [`crate::connection_scope::ConnectionScope::racy_static_defs`]
    /// — built once per `CompilationUnit` and shared by every
    /// ``::when::*`` proc except `RULE_INIT`.
    fn emit_racy_static_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        racy_vars: &HashSet<String>,
    ) {
        if self.disabled_diagnostics.contains("IRULE4005") {
            return;
        }
        let mut emitted_spans: HashSet<u32> = HashSet::new();
        for block in fu.ssa.blocks.values() {
            for stmt in &block.statements {
                // Skip unset — not a real write.  Mirrors the
                // Python guard.
                if let crate::ir::Statement::Call { command, .. } = &stmt.statement {
                    if command == "unset" {
                        continue;
                    }
                }
                for name in stmt.defs.keys() {
                    if !racy_vars.contains(name) {
                        continue;
                    }
                    let span = stmt.statement.span();
                    if span.is_empty() || !emitted_spans.insert(span.start()) {
                        continue;
                    }
                    let message = format!(
                        "Potential race: '{name}' is written outside RULE_INIT and read in \
                         another event. static:: variables persist across all connections on \
                         the same virtual server; concurrent writes can produce unpredictable \
                         results."
                    );
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "IRULE4005".to_string(),
                        span,
                        message,
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::types::Diagnostic;
    use tcl_lexer::Span;

    fn diag(code: &str, span: Span, msg: &str) -> Diagnostic {
        Diagnostic {
            code: code.to_string(),
            span,
            message: msg.to_string(),
            severity: Severity::Warning,
            fixes: Vec::new(),
        }
    }

    #[test]
    fn emit_variable_usage_diagnostics_is_a_noop() {
        // Hook is intentionally empty — running it must leave
        // the diagnostics list untouched.
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.emit_variable_usage_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_runs_without_panicking_on_empty_source() {
        // Smoke test — the orchestrator handles empty input
        // gracefully (an empty CompilationUnit yields no
        // diagnostics).
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("");
        assert!(a.result.diagnostics.is_empty());
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_no_w220_on_simple_assignment() {
        // ``set x 1`` — single assignment, no overwrite, no
        // W220.  Smoke test that pipeline runs without
        // emitting spurious W codes for clean code.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W220"),
            "W220 must not fire on a single assignment; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w220_dead_store_overwritten() {
        // ``set x 1\nset x 2\nputs $x`` — the first ``set x 1``
        // is overwritten before being read.  W220 should fire
        // at the first assignment.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            !w220s.is_empty(),
            "W220 expected for overwritten ``set x 1``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w220s.iter().any(|d| d.message.contains("'x'")));
        assert_eq!(w220s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_unused_param() {
        // ``proc foo {x y} { puts $x }`` — parameter ``y`` is
        // declared but never read in the body.  W214 should
        // fire on it.  Parameter ``x`` is read, so no W214.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x y} { puts $x }");
        let w214s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W214")
            .collect();
        assert_eq!(
            w214s.len(),
            1,
            "expected exactly one W214 for unused param ``y``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w214s[0].message.contains("'y'"));
        assert!(w214s[0].message.contains("'::foo'"));
        assert_eq!(w214s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_unused_variable() {
        // ``proc foo {} { set y 1 }`` — y is set, never read,
        // and there's no other version → W211 fires.
        // Top-level test would be subject to global-scope
        // assumptions, so use a proc body where the local-only
        // verdict is safe.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set y 1 }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211")
            .collect();
        assert!(
            !w211s.is_empty(),
            "W211 expected for unused var ``y`` in proc foo; got {:?}",
            a.result.diagnostics,
        );
        assert!(w211s[0].message.contains("'y'"));
        assert!(w211s[0].message.contains("set but never used"));
        assert_eq!(w211s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_skipped_for_textually_referenced() {
        // ``proc foo {} { set msg hello; puts "got $msg" }`` —
        // ``msg`` is referenced inside a quoted string; the
        // textual-reference filter should suppress W211 because
        // the def-use builder doesn't track ``"$msg"`` reads.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set msg hello; puts \"got $msg\" }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211" && d.message.contains("'msg'"))
            .collect();
        assert!(
            w211s.is_empty(),
            "W211 must not fire on var referenced via $-interpolation; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_skipped_for_global_aliased() {
        // ``proc foo {} { global config; set config 1 }`` —
        // ``config`` is global-aliased; the write goes to the
        // outer scope, so the local "no use" verdict is unsafe.
        // W211 must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { global config; set config 1 }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211" && d.message.contains("'config'"))
            .collect();
        assert!(
            w211s.is_empty(),
            "W211 must not fire on global-aliased var; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_repeated_assignment() {
        // ``proc foo {} { set x 1; set x 1 }`` — same var,
        // same literal value, consecutive statements.  The
        // first is a dead store; H300 fires on the second.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 1 }");
        let h300s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "H300")
            .collect();
        assert!(
            !h300s.is_empty(),
            "H300 expected for repeated ``set x 1``; got {:?}",
            a.result.diagnostics,
        );
        assert!(h300s[0].message.contains("'x'"));
        assert!(h300s[0].message.contains("Possible paste error"));
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_skips_underscore_vars() {
        // Vars starting with ``_`` are excluded (the convention
        // for "intentionally unused").
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set _x 1\nset _x 1 }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "H300"),
            "H300 must not fire on underscore-prefixed vars",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_skips_distinct_values() {
        // Same var, different literal → not a paste error.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 2 }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "H300"),
            "H300 must not fire when literal values differ",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_read_before_set() {
        // ``proc foo {} { puts $undef }`` — undef is not a
        // parameter and not in scope; W210 fires at the use.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { puts $undef }");
        let w210s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W210" && d.message.contains("'undef'"))
            .collect();
        assert!(
            !w210s.is_empty(),
            "W210 expected for read of undef ``$undef``; got {:?}",
            a.result.diagnostics,
        );
        assert_eq!(w210s[0].severity, Severity::Warning);
        assert!(w210s[0].message.contains("read before it is set"));
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_skipped_for_real_param() {
        // ``proc foo {x} { puts $x }`` — x IS a real parameter,
        // so W210 must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x} { puts $x }");
        let w210s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W210" && d.message.contains("'x'"))
            .collect();
        assert!(
            w210s.is_empty(),
            "W210 must not fire on real param ``x``; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w213_unset_on_possibly_undef() {
        // ``proc foo {} { unset xs }`` — ``xs`` may not exist;
        // ``unset`` without ``-nocomplain`` would error at
        // runtime.  W213 fires (instead of W210) at the unset
        // statement.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { unset xs }");
        let w213s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W213")
            .collect();
        assert!(
            !w213s.is_empty(),
            "W213 expected for ``unset xs`` on possibly-undef var; got {:?}",
            a.result.diagnostics,
        );
        assert!(w213s[0].message.contains("'xs'"));
        assert!(w213s[0].message.contains("unset -nocomplain"));
        assert_eq!(w213s[0].severity, Severity::Warning);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w213_skipped_with_nocomplain() {
        // ``unset -nocomplain xs`` is the safe form — W213
        // must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { unset -nocomplain xs }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W213"),
            "W213 must not fire when ``-nocomplain`` is present; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_fires_at_top_level() {
        // **C41e3 follow-up.** Top-level RBS now fires when no
        // proc writes the variable.  ``puts $undef`` reads
        // ``undef`` without any preceding write.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("puts $undef");
        assert!(
            a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must fire at top-level when no proc writes the var; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_suppressed_when_proc_writes_global() {
        // A helper proc ``init`` writes ``::counter`` via ``set``,
        // so the top-level read should not flag W210 — the proc
        // may run before the read.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc init {} { set ::counter 0 }\nputs $counter");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must be suppressed for globals written by procs; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_suppressed_via_global_alias() {
        // ``proc init {} { global counter; set counter 0 }`` — the
        // ``global`` declaration aliases the proc-local ``counter``
        // to the global.  Top-level read should not flag W210.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc init {} { global counter; set counter 0 }\nputs $counter");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must be suppressed via global-alias case; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn analyse_irule4005_racy_static_emitted_for_per_request_writes() {
        // ``static::counter`` written in HTTP_REQUEST and read
        // in HTTP_RESPONSE — both per-request events; the
        // cross-event flow is racy ⇒ IRULE4005 fires.
        let mut a = Analyser::new();
        let r = a.analyse(
            "when HTTP_REQUEST { incr static::counter }\n\
             when HTTP_RESPONSE { log local0. \"$static::counter\" }",
            "f5-irules",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "IRULE4005"),
            "IRULE4005 expected for racy static cross-event flow; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_irule4005_no_emit_for_rule_init_writes() {
        // ``static::config`` written in RULE_INIT is racy-safe
        // (RULE_INIT runs once at iRule load) — IRULE4005 must
        // not fire.
        let mut a = Analyser::new();
        let r = a.analyse(
            "when RULE_INIT { set static::config 1 }\n\
             when HTTP_REQUEST { log local0. \"$static::config\" }",
            "f5-irules",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "IRULE4005"),
            "IRULE4005 must not fire for RULE_INIT writes; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w124_ipv4_octet_overflow() {
        // ``proc foo {} { set ip 192.168.1.999 }`` — 999 > 255,
        // not a valid IP.  W124 fires at the assignment.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.1.999 }", "tcl");
        let w124s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W124").collect();
        assert!(
            !w124s.is_empty(),
            "W124 expected for IPv4 octet > 255; got {:?}",
            r.diagnostics,
        );
        assert!(w124s[0].message.contains("999"));
        assert!(w124s[0].message.contains("exceeds 255"));
        assert_eq!(w124s[0].severity, Severity::Error);
    }

    #[test]
    fn analyse_no_w124_for_valid_ipv4() {
        // ``proc foo {} { set ip 192.168.1.1 }`` — valid IP.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.1.1 }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W124"),
            "W124 must not fire on valid IPv4; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w124_ipv4_leading_zero_warning() {
        // ``proc foo {} { set ip 192.168.01.1 }`` — leading
        // zero on octet 3; might be octal in some contexts.
        // Severity is Warning.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.01.1 }", "tcl");
        let w124s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W124").collect();
        assert!(
            !w124s.is_empty(),
            "W124 expected for IPv4 leading-zero octet; got {:?}",
            r.diagnostics,
        );
        assert_eq!(w124s[0].severity, Severity::Warning);
        assert!(w124s[0].message.contains("leading zero"));
    }

    #[test]
    fn analyse_i230_constant_if_branch() {
        // ``proc foo {} { if {1} { puts hi } }`` — the ``if 1``
        // condition is constant, the false branch is unreachable.
        // I230 should fire.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { if {1} { puts hi } }", "tcl");
        let i230s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "I230").collect();
        assert!(
            !i230s.is_empty(),
            "I230 expected for constant ``if 1``; got {:?}",
            r.diagnostics,
        );
        assert!(i230s[0].message.contains("always true"));
    }

    #[test]
    fn analyse_no_i230_for_dynamic_condition() {
        // ``proc foo {x} { if {$x > 0} {} }`` — ``$x > 0`` is
        // not constant; no I230.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {x} { if {$x > 0} { puts hi } }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "I230"),
            "I230 must not fire on dynamic condition; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_unknown_command() {
        // ``no_such_cmd hello`` — bare name that's not a
        // built-in / proc / class / alias.  W123 fires.
        let mut a = Analyser::new();
        let r = a.analyse("no_such_cmd hello", "tcl");
        let w123s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W123").collect();
        assert!(
            !w123s.is_empty(),
            "W123 expected for unknown command; got {:?}",
            r.diagnostics,
        );
        assert!(w123s[0].message.contains("'no_such_cmd'"));
        assert_eq!(w123s[0].severity, Severity::Hint);
    }

    #[test]
    fn analyse_no_w123_for_builtin_command() {
        // ``puts hello`` — ``puts`` is a built-in; no W123.
        let mut a = Analyser::new();
        let r = a.analyse("puts hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire on built-in command; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_no_w123_for_user_proc() {
        // User-defined proc, then call it.  Both go through
        // the analyser walk; the call site must NOT trip W123.
        let mut a = Analyser::new();
        let r = a.analyse("proc greet {} { puts hi }\ngreet", "tcl");
        let w123s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W123").collect();
        assert!(
            w123s.is_empty(),
            "W123 must not fire on user-defined proc call; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_no_w123_for_qualified_command_name() {
        // Qualified names (``a::b``) skip W123 — defer to
        // per-namespace logic.
        let mut a = Analyser::new();
        let r = a.analyse("ns::cmd hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire on qualified command name; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_package_require_gate_suppresses_when_recorded() {
        // The ``package_requires`` gate suppresses W123 entirely
        // when any package require has been recorded.  The
        // analyser walk doesn't yet record ``package require``
        // (deferred — handler not landed), so we exercise the
        // gate by pre-populating ``result.package_requires``
        // and re-running the post-pass directly.
        use crate::signature_scan::types::SignaturePackageRequire;
        use tcl_lexer::Span;
        let mut a = Analyser::new();
        a.result.package_requires.push(SignaturePackageRequire {
            name: "Tcl".to_string(),
            version: Some("8.6".to_string()),
            range: Span::new(0, 24),
            conditional: false,
        });
        // Seed an invocation that would otherwise trip W123.
        a.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                name: "random_cmd".to_string(),
                range: Span::new(25, 35),
            },
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        a.emit_unresolved_command_diagnostics(&registry);
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must be fully suppressed when package_requires is non-empty; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_filtered_by_disabled_diagnostics() {
        // ``# tcl-lsp: disable=W123`` at top of file silences
        // the diagnostic via the existing disable filter.
        let mut a = Analyser::new();
        let r = a.analyse("# tcl-lsp: disable=W123\nno_such_cmd hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must be silenced by file-suppression directive; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_var_as_command() {
        // ``proc foo {x} { $x arg1 }`` — ``$x`` used as command
        // head; we have no static knowledge of what it holds, so
        // W307 fires.  Must go through ``analyse`` (not raw
        // ``emit_cfg_ssa_diagnostics``) because ``var_command_sites``
        // is populated by the analyser's walk dispatch, not the
        // emitter pipeline.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {x} { $x arg1 }", "tcl");
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            !w307s.is_empty(),
            "W307 expected for ``$x arg1``; got {:?}",
            r.diagnostics,
        );
        assert_eq!(w307s[0].severity, Severity::Warning);
        assert!(w307s[0].message.contains("Non-literal command name"));
    }

    #[test]
    fn analyse_no_w307_for_static_known_command() {
        // ``proc foo {} { set cmd puts; $cmd hello }`` — ``cmd``
        // has constant value "puts" which IS a known command, so
        // W307 must be suppressed.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set cmd puts\n$cmd hello }", "tcl");
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            w307s.is_empty(),
            "W307 must be suppressed when var holds known command name; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_var_command_sites_recorded_during_walk() {
        // Smoke: confirm the recording infrastructure populates
        // ``var_command_sites`` for ``$var`` heads.  Run analyse
        // (not just emit) so the apply_disabled_diagnostics +
        // dedupe don't matter — we inspect post-analyse state.
        let mut a = Analyser::new();
        let _ = a.analyse("proc foo {x} { $x arg }", "tcl");
        // After analyse, var_command_sites is consumed by the
        // post-pass but restored at the end (snapshot/restore
        // contract).
        assert!(
            a.var_command_sites.iter().any(|s| s.var_name == "x"),
            "var_command_sites should record ``$x`` head; got {:?}",
            a.var_command_sites,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_cmd_command_sites_recorded_during_walk() {
        // ``[cmd] arg`` records to ``cmd_command_sites`` even
        // though no W307 emitter consumes it yet.
        let mut a = Analyser::new();
        let _ = a.analyse("proc foo {} { [puts hi] arg }", "tcl");
        assert!(
            !a.cmd_command_sites.is_empty(),
            "cmd_command_sites should be populated for ``[cmd] arg``; got {:?}",
            a.cmd_command_sites,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_skips_args_param() {
        // The variadic ``args`` is conventional and frequently
        // declared without use; W214 must not fire on it.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x args} { puts $x }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W214"),
            "W214 should not fire on ``args``; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn dedupe_drops_exact_duplicates() {
        // Same code + span + message + severity → kept once.
        let mut a = Analyser::new();
        a.source = "set x 1".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn dedupe_keeps_distinct_diagnostics_at_different_spans() {
        let mut a = Analyser::new();
        a.source = "set x 1\nset y 2".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(12, 13), "y"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 2);
    }

    #[test]
    fn dedupe_drops_e002_on_e101_line() {
        // E101 fires on a line; any E002 on the same line is
        // a false positive (arity check confused by the
        // recovered switch) and gets dropped.
        let mut a = Analyser::new();
        a.source = "switch $x { foo {puts foo}".to_string();
        let switch_span = Span::new(0, 6);
        a.result
            .diagnostics
            .push(diag("E101", switch_span, "missing open brace"));
        a.result
            .diagnostics
            .push(diag("E002", switch_span, "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E101"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn dedupe_drops_w122_on_w124_line() {
        // W124 (SSA-based IP check) on a line → W122 (regex IP
        // check) on the same line is redundant.
        let mut a = Analyser::new();
        a.source = "if {[IP::addr $ip]} {}".to_string();
        let ip_span = Span::new(15, 18);
        a.result
            .diagnostics
            .push(diag("W124", ip_span, "invalid IP"));
        a.result
            .diagnostics
            .push(diag("W122", ip_span, "regex IP check"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W124"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W122"));
    }

    #[test]
    fn dedupe_keeps_e002_on_unrelated_line() {
        // E101 on line 0, E002 on line 1 — different lines, so
        // the suppression rule doesn't fire.
        let mut a = Analyser::new();
        a.source = "switch $x {\nset y 1".to_string();
        a.result
            .diagnostics
            .push(diag("E101", Span::new(0, 6), "missing brace"));
        a.result
            .diagnostics
            .push(diag("E002", Span::new(12, 15), "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn apply_disabled_diagnostics_removes_listed_codes() {
        let mut a = Analyser::with_disabled_diagnostics(
            ["W113"].iter().map(|s| (*s).to_string()).collect(),
        );
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "shadows"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(0, 3), "unset"));
        a.apply_disabled_diagnostics();
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W113"));
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W210"));
    }

    #[test]
    fn apply_disabled_diagnostics_no_op_when_empty() {
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.apply_disabled_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }
}
