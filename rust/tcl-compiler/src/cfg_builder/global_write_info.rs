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

//! Per-proc summary of global/namespace variable writes.
//!
//! Mirrors [`super::upvar_info`]'s shape and role in the pipeline, for the
//! complementary correctness gap: `upvar_info` tells a caller which of
//! *its own* frame variables a callee will alias and write; this module
//! tells a caller which *global/namespace* variables a callee will alias
//! (via `global` / `variable` / `upvar #0`) and write, so that
//! [`super::CfgBuilder::apply_upvar_invalidation`] can widen a call's
//! `defs` for those too. Without this, SCCP treats a bare global/namespace
//! name as an ordinary private local across an opaque call — unsound
//! (`proc mutate {} { global x; set x 99 }; set x 5; mutate; expr {$x+1}`
//! must not fold to `6`; tclsh yields `100`).
//!
//! ## Flow-insensitive, sound over-approximation
//!
//! Real Tcl style declares `global`/`variable` before using the name, but
//! this scan does not depend on that ordering: it first unions every
//! `global` / `variable` / `upvar #0` declaration in the whole proc body
//! (via the shared [`crate::var_observability::stmt_gen_with_metadata_context`] recognition
//! logic — reused, not re-derived) into one flag state, then checks every
//! write-target name found anywhere in the body against that *complete*
//! state. A write on a conditional branch that never executes together
//! with the declaring branch is still counted — the goal is soundness
//! (never *miss* a global write), not maximal precision.
//!
//! Dynamic command-binding transitions consume the registry's typed widening
//! facts and make the summary opaque; literal transitions retain a per-name
//! may-binding lattice. Dynamic `global`/`variable`/`upvar` variable targets
//! remain governed by the shared variable-observability and caller-frame
//! barriers rather than a second command-specific parser here.

use std::collections::{BTreeSet, HashMap};

use crate::command_binding::ModuleCommandBindings;
use crate::ir::{Module, Script, Statement};
use crate::ir_helpers::{ExecutionNamespace, nested_execution_bodies};
use crate::registry_invocation::InvocationMetadataContext;
use crate::var_observability::{State, stmt_gen_with_metadata_context};

/// Exact borrowed input for conditional global/caller-frame source summaries.
struct GlobalWriteSemantics<'a> {
    registry: &'a tcl_registry::CommandRegistry,
    aliases: &'a ModuleCommandBindings,
    metadata: InvocationMetadataContext<'a>,
    config: tcl_lexer::LexerConfig,
    input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
}

impl<'a> GlobalWriteSemantics<'a> {
    fn from_module(
        module: &'a Module,
        registry: &'a tcl_registry::CommandRegistry,
        aliases: &'a ModuleCommandBindings,
    ) -> Option<Self> {
        let metadata = InvocationMetadataContext::for_module(registry, module)?;
        let input = metadata.source_analysis_input()?;
        Some(Self {
            registry,
            aliases,
            metadata,
            config: module.lexer_config,
            input: Some(input),
        })
    }
}

fn opaque_procedure_summaries(module: &Module) -> HashMap<String, GlobalWriteInfo> {
    module
        .procedures
        .keys()
        .flat_map(|name| registered_keys(name))
        .map(|name| {
            (
                name,
                GlobalWriteInfo {
                    opaque_global_frame: true,
                    ..GlobalWriteInfo::default()
                },
            )
        })
        .collect()
}

fn literal_root(name: &str) -> &str {
    tcl_syntax::naming::split_element_ref(name).map_or(name, |(root, _)| root)
}

/// Per-proc summary: the outer-scope (global/namespace) variable names a
/// proc's body writes while aliased via `global` / `variable` / `upvar #0`,
/// or through a script it runs **at the global frame** (`uplevel #0 …`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct GlobalWriteInfo {
    /// Literal outer-scope names this proc's body writes.
    pub names: BTreeSet<String>,
    /// The proc runs a script at the **global frame** that this analysis
    /// cannot read — `uplevel #0 $body`, `uplevel #0 [list set $v 1]` with
    /// an unresolvable target, or a global-frame script whose write target
    /// is not a plain literal.  Any global/namespace name may be written
    /// *or read* there, so a call site must widen to an opaque barrier
    /// instead of trusting [`Self::names`] — otherwise O102 forwards a stale
    /// global constant straight across the call.
    pub opaque_global_frame: bool,
}

impl GlobalWriteInfo {
    /// True when the proc's body writes no outer-scope name.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && !self.opaque_global_frame
    }

    fn union_from(&mut self, other: &Self) -> bool {
        let old_len = self.names.len();
        let old_opaque = self.opaque_global_frame;
        self.names.extend(other.names.iter().cloned());
        self.opaque_global_frame |= other.opaque_global_frame;
        self.names.len() != old_len || self.opaque_global_frame != old_opaque
    }
}

/// Detect, for every proc in `module`, which outer-scope (global/
/// namespace) variable names its body writes — see the module doc for the
/// two-pass, flow-insensitive algorithm. Both the qualified and short
/// forms are registered for every proc, matching
/// [`super::detect_upvar_procs`]'s convention (the caller-side lookup at
/// [`super::CfgBuilder::apply_upvar_invalidation`] resolves a `Statement::
/// Call::command` by whichever spelling appears in source).
///
/// Then closes the per-proc summaries transitively over the direct-call
/// graph (a proc that calls a global-writing proc also "writes" those
/// names, recursively) via a bounded fixpoint — monotonic set-union, so
/// recursive/mutually-recursive cycles terminate safely.
#[must_use]
pub fn detect_global_write_procs(module: &Module) -> HashMap<String, GlobalWriteInfo> {
    let registry = module.resolved_registry();
    detect_global_write_procs_with_registry(module, registry)
}

/// Detect global/namespace writes against the exact registry that lowered the
/// module.
///
/// Besides release-specific core descriptors, this preserves custom and pack
/// command argument roles used by constructed or embedded scripts.  The
/// default wrapper above is retained for standalone structural callers; the
/// whole-module CFG path always enters here.
#[must_use]
pub fn detect_global_write_procs_with_registry(
    module: &Module,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<String, GlobalWriteInfo> {
    let bindings = ModuleCommandBindings::analyse(module, registry);
    detect_global_write_procs_with_bindings(module, registry, &bindings)
}

/// Detect global/namespace writes using the module binding summary already
/// prepared by the whole-module CFG pipeline.
pub(crate) fn detect_global_write_procs_with_bindings(
    module: &Module,
    registry: &tcl_registry::CommandRegistry,
    aliases: &ModuleCommandBindings,
) -> HashMap<String, GlobalWriteInfo> {
    let Some(semantics) = GlobalWriteSemantics::from_module(module, registry, aliases) else {
        return opaque_procedure_summaries(module);
    };
    let semantics = &semantics;
    let mut own: HashMap<String, GlobalWriteInfo> = HashMap::new();
    let mut direct_calls: HashMap<String, BTreeSet<String>> = HashMap::new();
    // Qualified ordering keeps shared short-name unions and the complete
    // per-procedure memo input independent of HashMap traversal order.
    let mut entries: Vec<(&String, &crate::ir::Procedure)> = module.procedures.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (qname, proc) in &entries {
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
        let namespace = if holder.is_empty() { "::" } else { holder };
        let mut info = own_body_global_writes(&proc.body, semantics, namespace);
        let (calls, calls_opaque) = direct_call_targets(&proc.body, semantics, namespace);
        info.opaque_global_frame |= calls_opaque;
        for key in registered_keys(qname) {
            own.entry(key.clone()).or_default().union_from(&info);
            direct_calls.entry(key).or_default().extend(calls.clone());
        }
    }

    // Multiple live generations currently share one qualified-name summary
    // key while only one body is retained in Module::procedures. Preserve
    // soundness until generation identities are carried end to end.
    for qname in &module.redefined_procedures {
        let opaque = GlobalWriteInfo {
            names: BTreeSet::new(),
            opaque_global_frame: true,
        };
        for key in registered_keys(qname) {
            own.entry(key).or_default().union_from(&opaque);
        }
    }

    // Transitive closure: monotonic union over direct-callee summaries.
    // Bounded by the number of registered names, so an adversarial call
    // graph still terminates.
    //
    // Walk the callers in sorted order too: the union itself is order-independent,
    // but the `guard` bound can cut the fixpoint short on an adversarial graph, and
    // a truncated result must at least be the *same* truncated result every run.
    let mut callers: Vec<String> = direct_calls.keys().cloned().collect();
    callers.sort();
    let mut changed = true;
    let mut guard = 0usize;
    while changed && guard <= own.len() {
        changed = false;
        guard += 1;
        let snapshot = own.clone();
        for caller in &callers {
            let Some(callee_names) = direct_calls.get(caller) else {
                continue;
            };
            let Some(target_summary) = own.get_mut(caller) else {
                continue;
            };
            for callee in callee_names {
                let Some(source_summary) = snapshot.get(callee) else {
                    // A terminal recovered procedure can be present in the
                    // binding lattice without a retained Module::procedures
                    // body. It is not an effect-free call.
                    target_summary.opaque_global_frame = true;
                    continue;
                };
                // An opaque global-frame script is transitive the same way
                // the names are: a caller of `setter` clobbers whatever
                // `setter`'s `uplevel #0 $body` clobbers.
                changed |= target_summary.union_from(source_summary);
            }
        }
    }

    // Publish every statically enumerable source spelling into the same
    // summary map the CFG builder consumes.  A procedure body can have been
    // lowered before a later `interp alias`/`rename` establishes the spelling,
    // so its `Statement::Call` may not carry a lower-time canonical target.
    // Keeping this projection here makes direct and embedded call sites share
    // the module-wide may-binding result instead of re-deriving alias facts.
    let closed = own.clone();
    for binding_key in aliases.source_spellings() {
        let mut projected = own.get(&binding_key).cloned().unwrap_or_default();
        for target in aliases.targets(&binding_key, "::") {
            if let Some(summary) = closed.get(&target.command) {
                projected.union_from(summary);
            } else if !target.registry_backed {
                projected.opaque_global_frame = true;
            }
        }
        projected.opaque_global_frame |=
            aliases.target_resolution_may_be_unknown(&binding_key, "::");
        if projected.is_empty() {
            continue;
        }
        for key in registered_keys(&binding_key) {
            own.entry(key).or_default().union_from(&projected);
        }
    }

    own
}

/// Every call-site spelling of `qname`, from the one helper
/// [`super::detect_upvar_procs`] and [`super::prepare_cfg_context`] also
/// use — the three maps are looked up by the same key at the same call
/// site, so they must agree on what keys exist.
fn registered_keys(qname: &str) -> Vec<String> {
    super::qualified_lookup_keys(qname)
}

/// Pass 1: union every `global` / `variable` declaration in `body` into one
/// flag state (order-independent), *and* separately collect the
/// (local-alias-name → real-outer-name) map for `upvar #0` / `upvar 0` /
/// `namespace upvar` declarations — those alias a *different* local name to
/// the outer one, so a write to the local name must be attributed to the
/// outer name, not recorded under the local spelling.  `global`/`variable`
/// are identity aliases (the declared name *is* the outer name), so they
/// stay in the flag-state path.  Pass 2: walk `body` again collecting every
/// write-target name, resolving each through the alias map first and
/// falling back to the flag-state identity check.
fn own_body_global_writes(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
) -> GlobalWriteInfo {
    let registry = semantics.registry;
    let aliases = semantics.aliases;
    let mut state = State::default();
    let mut renamed_aliases = OuterAliasProjection::default();
    let execution_namespace = ExecutionNamespace::exact(namespace);
    accumulate_state(
        body,
        &mut state,
        &mut renamed_aliases,
        semantics,
        &execution_namespace,
    );

    let mut info = GlobalWriteInfo::default();
    // A literal binding with an unknown implementation, or an unbounded
    // transition executed by this body itself, can reach an arbitrary
    // procedure that writes the global frame. Unrelated module-wide opacity
    // is runtime-provenance state, not a per-call variable effect.
    if aliases.script_has_opaque_binding_effect(body, registry, namespace)
        || script_invokes_unknown_binding(body, aliases, namespace)
    {
        info.opaque_global_frame = true;
    }
    collect_write_targets(
        body,
        &state,
        &renamed_aliases,
        semantics,
        &execution_namespace,
        &mut info,
    );
    collect_global_frame_effects(body, semantics, namespace, &mut info);
    info
}

fn script_invokes_unknown_binding(
    script: &Script,
    aliases: &ModuleCommandBindings,
    namespace: &str,
) -> bool {
    fn walk(
        script: &Script,
        aliases: &ModuleCommandBindings,
        namespace: &ExecutionNamespace,
        depth: u32,
    ) -> bool {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            return true;
        }
        script.statements.iter().any(|stmt| {
            let own = match stmt {
                Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
                    let Some(command_namespace) = namespace.for_head_context(command) else {
                        return true;
                    };
                    aliases.target_resolution_may_be_unknown(command, command_namespace.as_ref())
                }
                _ => false,
            };
            own || nested_execution_bodies(stmt, namespace)
                .into_iter()
                .any(|(body, body_namespace)| walk(body, aliases, &body_namespace, depth + 1))
        })
    }

    walk(script, aliases, &ExecutionNamespace::exact(namespace), 0)
}

/// Pass 3: the writes a proc performs by running a script **at
/// the global frame** — `uplevel #0 {…}` (lowered to [`Statement::UpFrame`]
/// with `absolute` set and shift `0`) and `uplevel #0 [list CMD …]` /
/// `uplevel #0 $body` (still a plain call/barrier statement).  These need no
/// `global`/`variable` declaration at all, so the flag-state passes above
/// cannot see them — before this pass, `proc setter {} {uplevel #0 {set x
/// 99}}` summarised as writing nothing and O102 forwarded the caller's stale
/// `x` straight across `setter` (tclsh 9.0.3/9.0.4: prints `99`, the
/// optimised program printed `5`).
///
/// A literal write target goes into [`GlobalWriteInfo::names`]; anything
/// unreadable (a dynamic script, an unresolvable constructed target, a
/// non-literal name) widens [`GlobalWriteInfo::opaque_global_frame`], which
/// the call site turns into an opaque barrier.  Frame targeting mirrors
/// [`super::upvar_info`]'s table exactly: only the **global** frame lands
/// here — caller-relative levels are the caller-frame summary's business,
/// and `uplevel 0` stays in the callee's own frame.
fn collect_global_frame_effects(
    script: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    info: &mut GlobalWriteInfo,
) {
    collect_global_frame_effects_in_frame(script, semantics, namespace, false, info);
}

/// Continue the global-frame pass under a known selected frame.  A nested
/// `uplevel 0` inherits its enclosing selected frame: inside `uplevel #0`, it
/// still writes globals, whereas inside `uplevel 1` it still writes the
/// caller.  Keeping that transform here prevents a nested [`Statement::UpFrame`]
/// from being silently treated as a body in the defining procedure's frame.
fn collect_global_frame_effects_in_frame(
    script: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    in_global_frame: bool,
    info: &mut GlobalWriteInfo,
) {
    for stmt in &script.statements {
        if !stmt.is_executable_invocation() {
            continue;
        }
        match stmt {
            Statement::UpFrame {
                absolute: true,
                frame_shift: 0,
                body,
                ..
            } => {
                record_global_upframe_body(body, semantics, info);
                collect_global_frame_effects_in_frame(body, semantics, "::", true, info);
                continue;
            }
            // Relative level zero preserves the selected frame.  It is a
            // global-frame script only when its enclosing literal body had
            // already selected `#0`.
            Statement::UpFrame {
                absolute: false,
                frame_shift: 0,
                body,
                ..
            } if in_global_frame => {
                record_global_upframe_body(body, semantics, info);
                collect_global_frame_effects_in_frame(body, semantics, "::", true, info);
                continue;
            }
            Statement::UpFrame { body, .. } => {
                // A different selected frame is not globally enumerable, but
                // an absolute `#0` nested inside it still is; keep walking to
                // find that explicit reset without attributing ordinary body
                // writes to the global frame.
                collect_global_frame_effects_in_frame(body, semantics, namespace, false, info);
                continue;
            }
            Statement::Call { .. } | Statement::Barrier { .. } => {
                record_selected_global_body(stmt, semantics, namespace, in_global_frame, info)
            }
            _ => {}
        }
        let execution_namespace = crate::ir_helpers::ExecutionNamespace::exact(namespace);
        for (body, body_namespace) in
            crate::ir_helpers::nested_execution_bodies(stmt, &execution_namespace)
        {
            let crate::ir_helpers::ExecutionNamespace::Exact(body_namespace) = body_namespace
            else {
                info.opaque_global_frame = true;
                continue;
            };
            collect_global_frame_effects_in_frame(
                body,
                semantics,
                &body_namespace,
                in_global_frame,
                info,
            );
        }
    }
}

fn record_selected_global_body(
    stmt: &Statement,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    in_global_frame: bool,
    info: &mut GlobalWriteInfo,
) {
    use tcl_registry::frame_effect::FrameLevel;
    let registry = semantics.registry;
    let aliases = semantics.aliases;
    let execution_namespace = ExecutionNamespace::exact(namespace);
    for invocation in aliases.resolve_statement_with_metadata_context(
        stmt,
        registry,
        Some(semantics.metadata),
        namespace,
    ) {
        use crate::command_binding::{ResolvedFrameBody, ResolvedFrameBodySelection};
        let body = invocation.resolved_frame_body_with_metadata_context(
            registry,
            aliases,
            &execution_namespace,
            Some(semantics.metadata),
        );
        let selected_global = |selection| match selection {
            ResolvedFrameBodySelection::Current => in_global_frame,
            ResolvedFrameBodySelection::Selected(FrameLevel::Absolute(0)) => true,
            ResolvedFrameBodySelection::Selected(FrameLevel::Relative(0)) => in_global_frame,
            ResolvedFrameBodySelection::Selected(
                FrameLevel::Relative(_) | FrameLevel::Absolute(_) | FrameLevel::Dynamic,
            ) => false,
        };
        match body {
            ResolvedFrameBody::Readable { source, selection } if selected_global(selection) => {
                if !record_literal_global_body(&source, semantics, info) {
                    info.opaque_global_frame = true;
                }
            }
            ResolvedFrameBody::Opaque { selection } if selected_global(selection) => {
                info.opaque_global_frame = true;
            }
            ResolvedFrameBody::NotApplicable
            | ResolvedFrameBody::KnownError
            | ResolvedFrameBody::Readable { .. }
            | ResolvedFrameBody::Opaque { .. } => {}
        }
    }
}

fn record_global_upframe_body(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    info: &mut GlobalWriteInfo,
) {
    // The literal body was lowered before all later module command-binding
    // transitions were known. Re-project its writes through the closed binding
    // lattice so an alias prefix such as `interp alias {} put {} set x`
    // contributes `x` even when the source call's lower-time `defs` is empty.
    // The body runs at #0, so bare command heads resolve in the global namespace.
    let projection = project_script_writes(body, semantics, "::");
    for name in projection.literal_names {
        record_global_frame_write(&name, info);
    }
    info.opaque_global_frame |= projection.opaque_variable_frame;
}

fn record_literal_global_body(
    source: &str,
    semantics: &GlobalWriteSemantics<'_>,
    info: &mut GlobalWriteInfo,
) -> bool {
    let registry = semantics.registry;
    let Some(input) = semantics.input else {
        return false;
    };
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, semantics.config)
        .with_dialect(Some(input.unit_profile()))
        .with_resolved_analysis_input(input.clone());
    if let Some(entry) = crate::command_binding::SourceAnalysisEntry::for_logical_source(input) {
        lowerer.set_source_analysis_options(entry.options());
    }
    let module = crate::lowering::lower_to_ir_with(lowerer, source);
    // An alias-baked `uplevel #0` reaches this path as an ordinary call, so
    // its body did not participate in the original module lowering. Project
    // the freshly lowered body through the original module's closed bindings:
    // a later `interp alias {} put {} set x` must be just as visible here as
    // it is for a directly lowered `Statement::UpFrame`.
    let projection = project_script_writes(&module.top_level, semantics, "::");
    for name in projection.literal_names {
        record_global_frame_write(&name, info);
    }
    info.opaque_global_frame |= projection.opaque_variable_frame;
    true
}

/// Whether any statement in `script` (recursively) assigns through a
/// **dynamic** write target (`set $n 1`, `set ${tok}(k) 1`) — a write
/// [`crate::ssa::defs_of`] cannot name and therefore silently drops, which
/// a frame-effect summary must widen on instead.  Shared with
/// [`super::upvar_info`]'s inlined-`uplevel`-body scan, which has the
/// identical blind spot one frame nearer.
pub(super) fn script_has_dynamic_write_target(script: &Script) -> bool {
    script.statements.iter().any(|stmt| {
        let own = match stmt {
            Statement::AssignConst {
                name, name_braced, ..
            }
            | Statement::AssignExpr {
                name, name_braced, ..
            }
            | Statement::AssignValue {
                name, name_braced, ..
            }
            | Statement::Incr {
                name, name_braced, ..
            } => crate::ssa::is_dynamic_write_target(name, *name_braced),
            _ => false,
        };
        own || crate::ir_helpers::nested_bodies(stmt)
            .into_iter()
            .any(script_has_dynamic_write_target)
    })
}

/// Record one global-frame write target: a plain literal name is
/// enumerable, anything else widens.
fn record_global_frame_write(name: &str, info: &mut GlobalWriteInfo) {
    let name = literal_root(name);
    if !name.is_empty() {
        info.names.insert(name.to_owned());
    } else {
        info.opaque_global_frame = true;
    }
}

fn accumulate_state(
    script: &Script,
    state: &mut State,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
) {
    let registry = semantics.registry;
    let aliases = semantics.aliases;
    for stmt in &script.statements {
        if let Some(command_namespace) = statement_command_namespace(stmt, namespace) {
            stmt_gen_with_metadata_context(stmt, state, registry, Some(semantics.metadata));
            collect_renamed_outer_alias(
                stmt,
                renamed_aliases,
                semantics,
                command_namespace.as_ref(),
            );
        }
        let embedded = source_substitutions(stmt, semantics, namespace);
        for words in embedded.all_commands() {
            let Some(head) = words
                .first()
                .and_then(crate::ir_helpers::CommandWord::literal)
            else {
                continue;
            };
            let Some(command_namespace) = namespace.for_head_context(head) else {
                continue;
            };
            for facts in aliases.resolve_command_words_with_metadata_context(
                words,
                registry,
                Some(semantics.metadata),
                command_namespace.as_ref(),
            ) {
                collect_renamed_outer_alias_facts(&facts, renamed_aliases, semantics);
            }
        }
        for (body, body_namespace) in nested_execution_bodies(stmt, namespace) {
            accumulate_state(body, state, renamed_aliases, semantics, &body_namespace);
        }
    }
}

fn source_substitutions(
    stmt: &Statement,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
) -> crate::ir_helpers::EvaluatedCommandSubstitutions {
    let resolve = |head: &str| semantics.aliases.resolved_embedded_head(head, namespace);
    crate::ir_helpers::evaluated_command_substitutions_with_heads_and_metadata_context(
        stmt,
        semantics.registry,
        Some(&resolve),
        Some(semantics.metadata),
        semantics.config,
    )
}

/// Resolve the command namespace relevant to one statement in a selected
/// execution frame. Structural statements use the selected frame directly;
/// absolute call heads remain resolvable even when a caller-selected frame is
/// otherwise unknown.
fn statement_command_namespace<'a>(
    stmt: &Statement,
    namespace: &'a ExecutionNamespace,
) -> Option<std::borrow::Cow<'a, crate::command_binding::SourceNamespaceKey>> {
    match stmt {
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
            namespace.for_head_context(command)
        }
        _ => namespace.for_head_context(""),
    }
}

/// Record the literal local-to-outer mapping declared by registry-owned
/// variable-cell alias transitions.  This includes renamed command spellings:
/// lowering retains their canonical target, and the descriptor — not the
/// source head — decides whether the invocation establishes an alias.
#[derive(Default)]
struct OuterAliasProjection {
    /// Every exact outer cell a literal local may denote on some path.
    targets: HashMap<String, BTreeSet<String>>,
    /// Literal locals whose outer cell cannot be enumerated.
    opaque_locals: BTreeSet<String>,
    /// Exact outer cells installed under a runtime-selected local name. Any
    /// concrete local write may therefore reach each of these cells.
    dynamic_local_targets: BTreeSet<String>,
    /// A runtime-selected local name aliases a non-enumerable outer cell.
    dynamic_local_opaque: bool,
}

enum OuterAliasTarget {
    Exact(String),
    Opaque,
}

fn collect_renamed_outer_alias(
    stmt: &Statement,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
) {
    let registry = semantics.registry;
    let aliases = semantics.aliases;
    for invocation in aliases.resolve_statement_with_metadata_context(
        stmt,
        registry,
        Some(semantics.metadata),
        namespace,
    ) {
        collect_renamed_outer_alias_facts(&invocation.facts, renamed_aliases, semantics);
    }
}

fn selected_outer_alias_target(
    target: &tcl_registry::VariableAliasTarget,
    semantics: &GlobalWriteSemantics<'_>,
) -> Option<OuterAliasTarget> {
    let literal = |variable: &tcl_registry::TransitionSubject| {
        variable.literal().map_or(OuterAliasTarget::Opaque, |name| {
            OuterAliasTarget::Exact(literal_root(name).to_owned())
        })
    };
    match target {
        tcl_registry::VariableAliasTarget::Global { variable }
        | tcl_registry::VariableAliasTarget::CurrentNamespace { variable } => {
            Some(literal(variable))
        }
        tcl_registry::VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => Some(if namespace.literal().is_some() {
            literal(variable)
        } else {
            OuterAliasTarget::Opaque
        }),
        tcl_registry::VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            let tcl_registry::CallerFrameSelection::Explicit(level) = frame else {
                return None;
            };
            let Some(level) = level.literal() else {
                return Some(OuterAliasTarget::Opaque);
            };
            let Some(dialect) = semantics
                .input
                .map(|input| tcl_registry::InvocationDialect::of_profile(input.unit_profile()))
                .or_else(|| {
                    semantics
                        .metadata
                        .context()
                        .environment
                        .point()
                        .map(tcl_registry::InvocationDialect::of_point)
                })
            else {
                return Some(OuterAliasTarget::Opaque);
            };
            match tcl_registry::frame_effect::FrameLevel::parse_native_bytes(
                level.as_bytes(),
                dialect,
            ) {
                Some(Some(level)) if level.is_global_frame() => Some(literal(variable)),
                // Invalid literal levels install no alias; other known levels
                // belong to the caller-frame summary instead of this one.
                Some(_) => None,
                // Counted NUL and unavailable numeric recipes are unresolved,
                // not evidence that an alias cannot have been installed.
                None => Some(OuterAliasTarget::Opaque),
            }
        }
    }
}

fn collect_renamed_outer_alias_facts(
    facts: &tcl_registry::InvocationFacts,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
) {
    let Some(transitions) = facts.state_transitions.declared() else {
        return;
    };
    for fact in transitions.facts() {
        let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        let Some(outer) = selected_outer_alias_target(&alias.target, semantics) else {
            continue;
        };
        match (alias.local.literal(), outer) {
            (Some(local), OuterAliasTarget::Exact(outer)) => {
                let local = literal_root(local);
                if !local.is_empty() && !outer.is_empty() {
                    renamed_aliases
                        .targets
                        .entry(local.to_owned())
                        .or_default()
                        .insert(outer);
                }
            }
            (Some(local), OuterAliasTarget::Opaque) => {
                let local = literal_root(local);
                if !local.is_empty() {
                    renamed_aliases.opaque_locals.insert(local.to_owned());
                }
            }
            (None, OuterAliasTarget::Exact(outer)) => {
                if !outer.is_empty() {
                    renamed_aliases.dynamic_local_targets.insert(outer);
                }
            }
            (None, OuterAliasTarget::Opaque) => renamed_aliases.dynamic_local_opaque = true,
        }
    }
}

fn collect_write_targets(
    script: &Script,
    state: &State,
    renamed_aliases: &OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
    info: &mut GlobalWriteInfo,
) {
    for stmt in &script.statements {
        if statement_command_namespace(stmt, namespace).is_some() {
            let (targets, opaque) = own_write_targets(stmt, semantics, namespace);
            info.opaque_global_frame |= opaque;
            for target in targets {
                let target = literal_root(&target);
                if let Some(outers) = renamed_aliases.targets.get(target) {
                    info.names.extend(outers.iter().cloned());
                }
                if renamed_aliases.opaque_locals.contains(target) {
                    info.opaque_global_frame = true;
                }
                info.names
                    .extend(renamed_aliases.dynamic_local_targets.iter().cloned());
                info.opaque_global_frame |= renamed_aliases.dynamic_local_opaque;
                if !renamed_aliases.targets.contains_key(target)
                    && !renamed_aliases.opaque_locals.contains(target)
                    && state.get(target).is_some_and(|f| f.writes_outer_scope())
                {
                    info.names.insert(target.to_owned());
                }
            }
        } else if matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
            // A relative command in a runtime-selected caller frame may reach
            // a procedure with arbitrary global effects.
            info.opaque_global_frame = true;
        }
        for (body, body_namespace) in nested_execution_bodies(stmt, namespace) {
            collect_write_targets(
                body,
                state,
                renamed_aliases,
                semantics,
                &body_namespace,
                info,
            );
        }
    }
}

/// Possible value destinations of the structured statement and its evaluated
/// substitutions. Generic calls use the current selected roles and original
/// alias prefix, rather than stale lower-time Call definitions. The Registry
/// distinguishes cell aliasing and trace installation from value writes.
fn own_write_targets(
    stmt: &Statement,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
) -> (Vec<String>, bool) {
    let registry = semantics.registry;
    let aliases = semantics.aliases;
    let structural = crate::ir_helpers::structural_variable_write_projection(stmt);
    let mut out = structural.literal_names;
    let Some(command_namespace) = statement_command_namespace(stmt, namespace) else {
        return (out, true);
    };
    let embedded = source_substitutions(stmt, semantics, namespace);
    let direct = aliases.variable_write_projection_with_metadata_context(
        stmt,
        registry,
        command_namespace.as_ref(),
        Some(semantics.metadata),
    );
    // Variable writes count from an in-frame expression word too: `set y
    // [expr {[incr ::hits]}]` writes the global exactly as `[incr ::hits]` in
    // a bare word does.
    let mut opaque =
        structural.opaque_variable_frame || direct.opaque_variable_frame || embedded.opaque;
    out.extend(direct.literal_names);
    for words in embedded.all_commands() {
        let Some(holder) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
            .and_then(|head| namespace.for_head_context(head))
        else {
            opaque = true;
            continue;
        };
        let projection = aliases.variable_write_projection_for_command_words_with_metadata_context(
            words,
            registry,
            Some(semantics.metadata),
            holder.as_ref(),
        );
        out.extend(projection.literal_names);
        opaque |= projection.opaque_variable_frame;
    }
    (out, opaque)
}

/// Conditional source write footprint under exact availability and grammar.
/// Missing or foreign metadata stays opaque; this grants no frame entry/write.
pub(super) fn script_value_write_projection_with_metadata_context(
    script: &Script,
    registry: &tcl_registry::CommandRegistry,
    aliases: &ModuleCommandBindings,
    namespace: &str,
    metadata: Option<InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
) -> tcl_registry::VariableWriteProjection {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return tcl_registry::VariableWriteProjection {
            opaque_variable_frame: true,
            ..tcl_registry::VariableWriteProjection::default()
        };
    };
    project_script_writes(
        script,
        &GlobalWriteSemantics {
            registry,
            aliases,
            metadata,
            config,
            input: metadata.source_analysis_input(),
        },
        namespace,
    )
}

fn project_script_writes(
    script: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
) -> tcl_registry::VariableWriteProjection {
    fn walk(
        script: &Script,
        semantics: &GlobalWriteSemantics<'_>,
        namespace: &str,
        names: &mut BTreeSet<String>,
        opaque: &mut bool,
        depth: u32,
    ) {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            *opaque = true;
            return;
        }
        for stmt in &script.statements {
            let (targets, stmt_opaque) =
                own_write_targets(stmt, semantics, &ExecutionNamespace::exact(namespace));
            names.extend(targets);
            *opaque |= stmt_opaque;
            // A nested UpFrame explicitly selects a different variable
            // frame, so its body is not a write of the script's own frame.
            // The enclosing global/caller-frame collectors classify that
            // shifted effect separately.
            if matches!(stmt, Statement::UpFrame { .. }) {
                continue;
            }
            let execution_namespace = crate::ir_helpers::ExecutionNamespace::exact(namespace);
            for (body, body_namespace) in
                crate::ir_helpers::nested_execution_bodies(stmt, &execution_namespace)
            {
                let crate::ir_helpers::ExecutionNamespace::Exact(body_namespace) = body_namespace
                else {
                    *opaque = true;
                    continue;
                };
                walk(body, semantics, &body_namespace, names, opaque, depth + 1);
            }
        }
    }

    let mut names = BTreeSet::new();
    let mut opaque = false;
    walk(script, semantics, namespace, &mut names, &mut opaque, 0);
    tcl_registry::VariableWriteProjection {
        literal_names: names.into_iter().collect(),
        read_before_write_names: Vec::new(),
        opaque_variable_frame: opaque,
    }
}

/// Direct-call targets in `body` — the command name of every
/// direct statement and evaluated `[…]` substitution, recursed into every
/// nested body. Used to build the call graph for the transitive-closure step;
/// resolution against `module.procedures` (qualified vs. short name) happens
/// at the lookup site in [`detect_global_write_procs`].
fn direct_call_targets(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
) -> (BTreeSet<String>, bool) {
    let mut calls = BTreeSet::new();
    let mut opaque = false;
    collect_direct_calls(
        body,
        semantics,
        &crate::ir_helpers::ExecutionNamespace::exact(namespace),
        &mut calls,
        &mut opaque,
        0,
    );
    (calls, opaque)
}

fn collect_direct_calls(
    script: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    calls: &mut BTreeSet<String>,
    opaque: &mut bool,
    depth: u32,
) {
    let aliases = semantics.aliases;
    fn collect_resolved_head(
        head: &str,
        command_namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
        aliases: &ModuleCommandBindings,
        calls: &mut BTreeSet<String>,
        opaque: &mut bool,
    ) {
        // Publish only terminal identities selected by the shared binding
        // lattice.  Retaining the raw short spelling as well can accidentally
        // attach a sibling namespace's same-tailed procedure summary after an
        // enclosing `uplevel` selected a different frame.
        calls.extend(
            aliases
                .targets(head, command_namespace)
                .into_iter()
                // Registry-backed effects are projected directly from their
                // descriptors. Only user procedures participate in the
                // procedure-summary call graph; treating every builtin as a
                // missing procedure body makes ordinary calls spuriously
                // opaque.
                .filter(|target| !target.registry_backed)
                .map(|target| target.command),
        );
        *opaque |= aliases.target_resolution_may_be_unknown(head, command_namespace);
    }

    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        *opaque = true;
        return;
    }
    for stmt in &script.statements {
        if let Statement::Call {
            command, tokens, ..
        }
        | Statement::Barrier {
            command, tokens, ..
        } = stmt
        {
            let command_namespace = namespace.for_head_context(command);
            match tokens.as_ref() {
                Some(tokens) if tokens.synthetic.is_some() => {}
                Some(tokens) => match tokens
                    .words()
                    .first()
                    .map(|head| crate::registry_invocation::invocation_word(head))
                {
                    Some(tcl_registry::InvocationWord::Literal(_)) | None => {
                        let Some(command_namespace) = command_namespace else {
                            *opaque = true;
                            continue;
                        };
                        collect_resolved_head(
                            command,
                            command_namespace.as_ref(),
                            aliases,
                            calls,
                            opaque,
                        );
                    }
                    Some(_) => *opaque = true,
                },
                None => {
                    let Some(command_namespace) = command_namespace else {
                        *opaque = true;
                        continue;
                    };
                    collect_resolved_head(
                        command,
                        command_namespace.as_ref(),
                        aliases,
                        calls,
                        opaque,
                    );
                }
            }
        }
        let embedded = source_substitutions(stmt, semantics, namespace);
        *opaque |= embedded.opaque;
        for words in embedded.all_commands() {
            let Some(head) = words.first() else {
                continue;
            };
            let Some(head_name) = head.literal() else {
                *opaque = true;
                continue;
            };
            let Some(command_namespace) = namespace.for_head_context(head_name) else {
                *opaque = true;
                continue;
            };
            collect_resolved_head(
                head_name,
                command_namespace.as_ref(),
                aliases,
                calls,
                opaque,
            );
        }
        for (body, body_namespace) in crate::ir_helpers::nested_execution_bodies(stmt, namespace) {
            collect_direct_calls(body, semantics, &body_namespace, calls, opaque, depth + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::{ArgRole, CommandRegistry, CommandSpec, StateTransitionDescriptor};

    fn module(src: &str) -> Module {
        module_with_registry(src, &CommandRegistry::build_default())
    }

    fn module_with_registry(src: &str, registry: &CommandRegistry) -> Module {
        let context = tcl_registry::model::ingress::static_context_for("tcl")
            .with_command_store(registry.snapshot().shared_registry());
        let profile = tcl_registry::model::ingress::static_context_for("tcl")
            .commands()
            .profile()
            .unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            config,
        );
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_dialect(Some(profile))
            .with_context_registry(input.context_registry());
        lowerer.set_source_analysis_options(
            crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        );
        crate::lowering::lower_to_ir_with(lowerer, src)
    }

    #[test]
    fn supplied_global_write_metadata_keeps_alias_arguments_and_literal_names() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source summaries establish no Native frame/link/current cell.
        let registry = CommandRegistry::build_default();
        for (source, expected) in [
            (
                "interp alias {} put {} set {$scalar(open}; proc p {} {global {$scalar(open}; put VALUE}",
                "$scalar(open",
            ),
            (
                "rename set moved; interp alias {} put {} moved {café}; proc p {} {global café; set result [put VALUE]}",
                "café",
            ),
            (
                "interp alias {} bind {} upvar #0 {$outer}; proc p {} {bind {$local}; set {$local} VALUE}",
                "$outer",
            ),
            (
                "proc p {} {uplevel #0 {set {scalar(open} VALUE}}",
                "scalar(open",
            ),
            (
                "interp alias {} at-root {} uplevel #0; proc p {} {at-root {set {café(open} VALUE}}",
                "café(open",
            ),
        ] {
            let module = module_with_registry(source, &registry);
            let information = detect_global_write_procs_with_registry(&module, &registry);
            assert!(
                information["::p"].names.contains(expected),
                "{source}: {information:?}"
            );
        }
        for source in [
            "proc set args {}; interp alias {} put {} set hidden; proc p {} {put VALUE}",
            "interp alias {} put {} set hidden; rename set moved; proc set args {}; proc p {} {put VALUE}",
        ] {
            let module = module_with_registry(source, &registry);
            assert!(
                !detect_global_write_procs_with_registry(&module, &registry)["::p"]
                    .names
                    .contains("hidden")
            );
        }
    }

    #[test]
    fn supplied_global_write_summary_refuses_missing_foreign_and_stale_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let registry = CommandRegistry::build_default();
        let module = module_with_registry("proc p {} {uplevel #0 {set g VALUE}}", &registry);
        assert!(
            detect_global_write_procs_with_registry(&module, &registry)["::p"]
                .names
                .contains("g")
        );
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            detect_global_write_procs_with_registry(&missing, &registry)["::p"].opaque_global_frame
        );
        let mut foreign = module.clone();
        let input = module.source_metadata_input.as_ref().unwrap();
        foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            input.lexer_config(),
        ));
        assert!(
            detect_global_write_procs_with_registry(&foreign, &registry)["::p"].opaque_global_frame
        );
        let mut stale = module;
        stale.lexer_config.expand_syntax = !stale.lexer_config.expand_syntax;
        assert!(
            detect_global_write_procs_with_registry(&stale, &registry)["::p"].opaque_global_frame
        );
    }

    #[test]
    fn supplied_script_write_projection_uses_current_roles_instead_of_call_defs() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "gated-write",
            surface: registry.get("dict").unwrap().surface,
            arg_roles: &[(0, ArgRole::VarWrite)],
            ..CommandSpec::DEFAULT
        });
        let module = module_with_registry("gated-write {$scalar(open} VALUE", &registry);
        let aliases = ModuleCommandBindings::analyse(&module, &registry);
        for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
            let context = tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(registry.snapshot().shared_registry());
            let projection = script_value_write_projection_with_metadata_context(
                &module.top_level,
                &registry,
                &aliases,
                "::",
                Some((&context).into()),
                module.lexer_config,
            );
            assert_eq!(
                projection
                    .literal_names
                    .contains(&"$scalar(open".to_owned()),
                available,
                "{environment}: {projection:?}"
            );
            if !available {
                assert!(projection.opaque_variable_frame);
            }
        }
        assert!(
            script_value_write_projection_with_metadata_context(
                &module.top_level,
                &registry,
                &aliases,
                "::",
                None,
                module.lexer_config
            )
            .opaque_variable_frame
        );
    }

    #[test]
    fn supplied_global_write_call_closure_includes_source_expression_targets() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let module = module(
            "proc mutate {} {global g; set g VALUE; return 0}; proc p {} {return [expr {[mutate]}]}",
        );
        assert!(
            detect_global_write_procs(&module)["::p"]
                .names
                .contains("g")
        );
    }

    #[test]
    fn empty_module_has_no_writes() {
        let m = module("");
        assert!(detect_global_write_procs(&m).is_empty());
    }

    /// Procedures sharing a short spelling both contribute to that spelling's
    /// may-summary. A call through `run` may resolve to either definition as
    /// namespaces and aliases change, so last-writer selection is unsound.
    #[test]
    fn short_name_collision_unions_every_qualified_summary() {
        let m = module(
            "proc ::b::run {} { global bbb; set bbb 1 }\n\
             proc ::a::run {} { global aaa; set aaa 1 }\n",
        );
        let info = detect_global_write_procs(&m);
        assert_eq!(
            info.get("run").expect("short key registered").names,
            BTreeSet::from(["aaa".to_owned(), "bbb".to_owned()]),
        );
        assert!(info["run"].names.contains("bbb"));
        assert!(info["run"].names.contains("aaa"));
    }

    #[test]
    fn proc_with_no_global_write_is_empty() {
        let m = module("proc ::p {} { set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().is_empty());
    }

    #[test]
    fn direct_global_write_recorded() {
        // The `mutate` repro: global x; set x 99.
        let m = module("proc ::mutate {} { global x; set x 99 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::mutate").unwrap().names.contains("x"));
        // Registered under both the qualified and short forms.
        assert!(info.get("mutate").unwrap().names.contains("x"));
    }

    #[test]
    fn variable_namespace_write_recorded() {
        let m = module("proc ::ns::p {} { variable v; set v 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::ns::p").unwrap().names.contains("v"));
    }

    #[test]
    fn upvar_hash_zero_counts_as_global() {
        let m = module("proc ::p {} { upvar #0 g local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("g"));
        // Not the local alias name — recording "local" would be a no-op at
        // the caller (no such caller-frame variable), missing the real "g".
        assert!(!info.get("::p").unwrap().names.contains("local"));
    }

    #[test]
    fn namespace_upvar_rename_resolves_to_outer_name() {
        let m = module("proc ::ns::p {} { namespace upvar ::ns nsvar localvar\nset localvar 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::ns::p").unwrap().names.contains("nsvar"));
        assert!(!info.get("::ns::p").unwrap().names.contains("localvar"));
    }

    #[test]
    fn branch_alternative_outer_aliases_are_unioned() {
        let m = module("proc ::p {c} { if {$c} { upvar #0 a x } else { upvar #0 b x }; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert_eq!(
            info["::p"].names,
            BTreeSet::from(["a".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn dynamic_outer_alias_target_widens_the_global_frame() {
        let m = module("proc ::p {name} { upvar #0 $name x; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info["::p"].opaque_global_frame, "got {:?}", info["::p"]);
    }

    #[test]
    fn dynamic_namespace_alias_target_widens_the_global_frame() {
        let m = module("proc ::p {ns} { namespace upvar $ns x local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info["::p"].opaque_global_frame, "got {:?}", info["::p"]);
    }

    #[test]
    fn caller_frame_upvar_is_not_a_global_write() {
        // `upvar 1` aliases the *caller* frame, not global/namespace —
        // must not be treated as an outer-scope write.
        let m = module("proc ::p {} { upvar 1 caller_x local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().is_empty());
    }

    #[test]
    fn read_only_global_is_not_a_write() {
        let m = module("proc ::p {} { global x; puts $x }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get("::p").unwrap().is_empty(),
            "read-only declaration produced effects: {:?}",
            info.get("::p").unwrap()
        );
    }

    #[test]
    fn renamed_global_descriptor_establishes_alias_without_counting_declaration_as_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "rename global declare_global\n\
             proc ::p {} { declare_global x; set x 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn renamed_trace_descriptor_is_not_mistaken_for_a_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "rename trace observe\n\
             proc ::p {} { variable x; observe add variable x write callback }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info["::p"].is_empty());
    }

    #[test]
    fn custom_replacement_under_builtin_spelling_uses_its_own_descriptor() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "global",
            arg_roles: &[(0, ArgRole::VarWrite)],
            state_transitions: Some(StateTransitionDescriptor::EMPTY),
            world_effects: Some(tcl_registry::WorldEffectDescriptor::VARIABLE_WRITE),
            native_compilation: Some(tcl_registry::native_compilation::NativeCompilationSpec {
                grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
                operation: tcl_registry::SemanticOperationId::Invoke,
                body: tcl_registry::native_compilation::NativeBodyCompilation::Direct,
            }),
            successful_handler: Some(
                tcl_registry::native_compilation::SuccessfulHandlerSpec::VariableOperands,
            ),
            ..CommandSpec::DEFAULT
        });
        let m = module_with_registry("proc ::p {} { variable x; global x value }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn variable_declaration_without_value_is_not_a_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry("proc ::p {} { variable x }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info["::p"].is_empty());
    }

    #[test]
    fn variable_value_form_records_the_namespace_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry("proc ::p {} { variable x 1 }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn variable_mixed_pairs_only_record_value_bearing_cells() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry("proc ::p {} { variable x 1 y }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_baked_level_is_part_of_exact_transition_resolution() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_defined_after_the_procedure_is_live_when_the_body_runs() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { g x local; set local 1 }\n\
             interp alias {} g {} upvar #0",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_established_inside_the_procedure_applies_to_later_calls() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { interp alias {} g {} upvar #0; g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_established_in_evaluated_substitution_applies_to_later_calls() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} {\
                 set ignored [interp alias {} g {} upvar #0]; \
                 g x local; \
                 set local 1\
             }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn late_namespaced_alias_resolves_inside_namespace_eval_block() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::n::p {} { eval { g x local; set local 1 } }\n\
             interp alias {} ::n::g {} upvar #0",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::n::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_established_by_another_callable_procedure_is_in_the_may_state() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::install {} { interp alias {} g {} upvar #0 }\n\
             proc ::p {} { install; g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_created_in_namespaced_proc_is_global_in_source_interpreter() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::n::install {} { interp alias {} g {} upvar #0 }\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn dynamic_command_binding_transition_widens_the_global_frame_summary() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {from to} { rename $from $to; unknown_target }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info["::p"].opaque_global_frame);
    }

    #[test]
    fn canonical_alias_target_participates_in_transitive_write_closure() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::mutate {} { global x; set x 99 }\n\
             interp alias {} call_mutate {} mutate\n\
             proc ::outer {} { call_mutate }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::outer"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn late_alias_summary_is_published_under_its_source_spelling() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::mutate {} { global x; set x 99 }\n\
             proc ::outer {} { call_mutate }\n\
             interp alias {} call_mutate {} mutate",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["call_mutate"].names, BTreeSet::from(["x".to_owned()]));
        assert_eq!(info["::outer"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_global_target_survives_same_tail_namespace_collision() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::mutate {} { global x; set x 99 }\n\
             proc ::z::mutate {} {}\n\
             interp alias {} call_mutate {} mutate\n\
             proc ::outer {} { call_mutate }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["call_mutate"].names, BTreeSet::from(["x".to_owned()]));
        assert_eq!(info["::outer"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn rename_preserves_every_module_live_alias_state_with_its_baked_arguments() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             rename g h\n\
             proc ::p {} { h x local; set local 1 }\n\
             proc ::old_name {} { g y local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
        // Summaries deliberately use the module-wide may-state: `g` was a live
        // binding at one executable point, so a stored body calling it remains
        // conservative even though this particular definition follows the
        // rename. Lifetime-sensitive procedure reachability is not encoded in
        // the summary key.
        assert_eq!(info["::old_name"].names, BTreeSet::from(["y".to_owned()]));
    }

    #[test]
    fn alias_chain_composes_every_baked_argument_in_order() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} bind_global {} upvar #0\n\
             interp alias {} bind_x {} bind_global x\n\
             interp alias {} bind_local {} bind_x\n\
             proc ::p {} { bind_local local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn embedded_global_alias_transition_maps_later_local_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { set _ [upvar #0 g local]; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["g".to_owned()]));
        assert!(!info["::p"].opaque_global_frame);
    }

    #[test]
    fn embedded_alias_prefix_preserves_global_alias_arguments() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} bind {} upvar #0 g\n\
             proc ::p {} { set _ [bind local]; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["g".to_owned()]));
        assert!(!info["::p"].opaque_global_frame);
    }

    #[test]
    fn embedded_namespace_alias_transition_maps_later_local_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { set _ [namespace upvar ::target x local]; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
        assert!(!info["::p"].opaque_global_frame);
    }

    #[test]
    fn alias_target_prefix_resolves_in_the_target_interpreters_global_namespace() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             proc ::n::upvar args {}\n\
             proc ::n::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::n::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn conditional_alias_override_joins_both_live_bindings() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             if {$replace} { interp alias {} g {} puts }\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn conditional_namespace_alias_keeps_global_fallback_live() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             proc ::n::install {replace} {\n\
                 if {$replace} { interp alias {} ::n::g {} puts }\n\
             }\n\
             proc ::n::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::n::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn non_mutating_alias_alternative_preserves_the_incoming_binding() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             interp alias {} mutate {} list\n\
             if {$replace} { interp alias {} mutate {} rename }\n\
             mutate g {}\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_baked_namespace_upvar_prefix_is_part_of_exact_transition_resolution() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} bind_ns {} namespace upvar ::target\n\
             proc ::p {} { bind_ns x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_baked_variable_name_still_records_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} initialise {} variable x\n\
             proc ::p {} { initialise 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_baked_set_target_is_a_global_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} put {} set x\n\
             proc ::p {} { global x; put 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn alias_baked_custom_varwrite_target_uses_its_registry_descriptor() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "write-cell",
            arg_roles: &[(0, ArgRole::VarWrite)],
            ..CommandSpec::DEFAULT
        });
        let m = module_with_registry(
            "interp alias {} write_x {} write-cell x\n\
             proc ::p {} { global x; write_x value }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn write_on_conditional_branch_still_counted() {
        // Sound over-approximation: flow-insensitive, so a write inside
        // an `if` still counts even though it may not always execute.
        let m = module("proc ::p {} { global x\nif {$c} { set x 1 } }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn declaration_after_write_still_counted() {
        // Flow-insensitive union: order within the body doesn't matter.
        let m = module("proc ::p {} { set x 1\nglobal x }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn embedded_incr_in_command_substitution_recorded() {
        let m = module("proc ::p {} { global x\nset y [incr x] }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn expanded_global_write_target_widens_the_global_frame() {
        let m = module("proc ::p {names} { global {*}$names; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info["::p"].opaque_global_frame);
    }

    #[test]
    fn embedded_proc_call_in_assign_value_propagates_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::outer {} { set result \"value=[mutate]\" }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info["::outer"].names.contains("x"));
    }

    #[test]
    fn recovered_procedure_without_a_published_summary_is_opaque() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "namespace eval ::n [list proc writer {} {global g; set g 1}]\n\
             proc ::caller {} {::n::writer}",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info["::caller"].opaque_global_frame);
    }

    #[test]
    fn redefined_procedure_generation_is_an_opaque_global_effect() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc p {} {}\n\
             rename p old\n\
             proc p {} {global g; set g 1}\n\
             proc caller {} {p}",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info["::p"].opaque_global_frame);
        assert!(info["::caller"].opaque_global_frame);
    }

    #[test]
    fn embedded_proc_call_in_return_propagates_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::outer {} { return [mutate] }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info["::outer"].names.contains("x"));
    }

    #[test]
    fn embedded_proc_calls_in_assign_expr_and_expr_eval_propagate_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::assign_expr {} { set result [expr {[mutate] + 1}] }\n\
             proc ::expr_eval {} { expr {[mutate] + 1} }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info["::assign_expr"].names.contains("x"));
        assert!(info["::expr_eval"].names.contains("x"));
    }

    #[test]
    fn embedded_proc_calls_in_if_and_while_conditions_propagate_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::conditional {} { if {[mutate]} { return } }\n\
             proc ::loop {} { while {[mutate]} { break } }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info["::conditional"].names.contains("x"));
        assert!(info["::loop"].names.contains("x"));
    }

    #[test]
    fn dynamic_eval_body_widens_later_command_binding_resolution() {
        // The body can install `late` before the following call executes:
        // `outer {interp alias {} late {} mutate}`. Its text is dynamic in
        // the stored procedure, so the registry's dynamic-body fact and the
        // lowered Barrier must conservatively widen the command table.
        let m = module(
            "proc ::mutate {} { global x; set x 99 }\n\
             proc ::outer {body} { eval $body; late }\n\
             outer {interp alias {} late {} mutate}",
        );
        let info = detect_global_write_procs(&m);
        assert!(info["::outer"].opaque_global_frame);
    }

    #[test]
    fn transitive_closure_through_direct_call() {
        // `outer` calls `mutate`, which writes global x — `outer`'s
        // summary must include `x` too (transitive).
        let m = module("proc ::mutate {} { global x; set x 99 }\nproc ::outer {} { mutate }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::outer").unwrap().names.contains("x"));
    }

    #[test]
    fn recursive_proc_terminates_and_includes_write() {
        let m = module(
            "proc ::p {n} { if {$n == 0} { return }\n global x\n incr x\n p [expr {$n-1}] }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn mutually_recursive_procs_share_the_write() {
        let m = module("proc ::a {} { global x; set x 1; b }\nproc ::b {} { a }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::a").unwrap().names.contains("x"));
        assert!(info.get("::b").unwrap().names.contains("x"));
    }

    #[test]
    fn uplevel_hash_zero_literal_body_records_global_writes() {
        // `uplevel #0 {set x 99}` needs no `global`
        // declaration at all (tclsh 9.0.3/9.0.4: the caller-visible `x`
        // really is 99 afterwards).
        let m = module("proc ::setter {} { uplevel #0 { set x 99 } }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::setter").unwrap().names.contains("x"));
        assert!(!info.get("::setter").unwrap().opaque_global_frame);
    }

    #[test]
    fn uplevel_hash_zero_literal_body_resolves_late_alias_prefix_write() {
        // The proc body is lowered before the later alias exists, so `put`
        // carries no lower-time def.  Tcl 9.0.4 runs the alias prefix as
        // `set x 99` in frame #0; the closed module binding projection must
        // therefore attribute the global `x` write to `setter`.
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} { uplevel #0 { put 99 } }\n\
             interp alias {} put {} set x",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::setter"].names, BTreeSet::from(["x".to_owned()]));
        assert!(!info["::setter"].opaque_global_frame);
    }

    #[test]
    fn alias_baked_global_literal_body_resolves_late_inner_alias() {
        // `g` is itself installed after the procedure body was lowered, so
        // this takes the plain-call literal-body path rather than UpFrame.
        // Its inner `put` alias is later still and must remain visible.
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} { g { put 99 } }\n\
             interp alias {} g {} uplevel #0\n\
             interp alias {} put {} set x",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::setter"].names, BTreeSet::from(["x".to_owned()]));
        assert!(!info["::setter"].opaque_global_frame);
    }

    #[test]
    fn nested_relative_zero_inside_global_upframe_stays_global() {
        // The inner `uplevel 0` inherits the outer `#0` frame rather than
        // returning to the defining procedure's frame.
        let m = module("proc ::setter {} { uplevel #0 { uplevel 0 { ::set g 1 } } }");
        let info = detect_global_write_procs(&m);
        assert!(info["::setter"].names.contains("g"), "got {info:?}");
        assert!(!info["::setter"].opaque_global_frame, "got {info:?}");
    }

    #[test]
    fn late_alias_to_relative_zero_inside_global_frame_stays_global() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} {uplevel #0 {::inner {::set g 1}}}\n\
             interp alias {} ::inner {} uplevel 0",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::setter"].names, BTreeSet::from(["g".to_owned()]));
        assert!(!info["::setter"].opaque_global_frame);
    }

    #[test]
    fn late_alias_to_eval_inside_global_frame_stays_global() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} {uplevel #0 {::inner {::set g 1}}}\n\
             interp alias {} ::inner {} eval",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::setter"].names, BTreeSet::from(["g".to_owned()]));
        assert!(!info["::setter"].opaque_global_frame);
    }

    #[test]
    fn transparent_eval_inside_global_upframe_uses_global_command_resolution() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::helper {} { global global_hit; set global_hit 1 }\n\
             namespace eval ::n {\n\
                 proc helper {} { global namespaced_hit; set namespaced_hit 1 }\n\
                 proc setter {} { uplevel #0 { eval { helper } } }\n\
             }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info["::n::setter"].names.contains("global_hit"));
        assert!(
            !info["::n::setter"].names.contains("namespaced_hit"),
            "a transparent block must not restore its lowering-time namespace: {info:?}"
        );
        assert!(!info["::n::setter"].opaque_global_frame);
    }

    #[test]
    fn transparent_eval_inside_global_upframe_resolves_a_late_global_alias() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "namespace eval ::n {\n\
                 proc put args {}\n\
                 proc setter {} { uplevel #0 { eval { put 2 } } }\n\
             }\n\
             interp alias {} put {} set x",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info["::n::setter"].names,
            BTreeSet::from(["x".to_owned()]),
            "a transparent Block inherits the selected global command namespace"
        );
        assert!(!info["::n::setter"].opaque_global_frame);
    }

    #[test]
    fn uplevel_hash_zero_constructed_list_records_global_writes() {
        // `uplevel #0 [list set k 77]` — tclsh 9.0.4: the global `k` is 77.
        let m = module("proc ::s3 {} { uplevel #0 [list set k 77] }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::s3").unwrap().names.contains("k"));
        assert!(!info.get("::s3").unwrap().opaque_global_frame);
    }

    #[test]
    fn constructed_global_body_with_dynamic_operand_is_opaque() {
        let m = module("proc ::p {name} { uplevel #0 [list set $name 1] }");
        let info = detect_global_write_procs(&m);
        assert!(info["::p"].opaque_global_frame);
        assert!(info["::p"].names.is_empty());
    }

    #[test]
    fn rebound_list_constructor_widens_global_frame_effects() {
        let m = module(
            "proc list args { return {set hidden 1} }\n\
             proc ::p {} { uplevel #0 [list set claimed 1] }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info["::p"].opaque_global_frame);
        assert!(info["::p"].names.is_empty());
    }

    #[test]
    fn alias_baked_global_uplevel_frame_is_resolved_from_invocation_facts() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} uplevel #0\n\
             proc ::p {} { g {set x 1} }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::p"].names, BTreeSet::from(["x".to_owned()]));
        assert!(!info["::p"].opaque_global_frame);
    }

    #[test]
    fn constructed_global_uplevel_resolves_command_in_global_namespace() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} set\n\
             interp alias {} ::n::g {} list\n\
             proc ::n::p {} { uplevel #0 [list g x 1] }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(info["::n::p"].names, BTreeSet::from(["x".to_owned()]));
    }

    #[test]
    fn uplevel_hash_zero_dynamic_script_is_opaque() {
        // `uplevel #0 $body` — any global may be written or read.
        let m = module("proc ::s2 {body} { uplevel #0 $body }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::s2").unwrap().opaque_global_frame);
        assert!(!info.get("::s2").unwrap().is_empty());
    }

    #[test]
    fn uplevel_hash_zero_opaqueness_is_transitive() {
        let m = module("proc ::s2 {body} { uplevel #0 $body }\nproc ::outer {body} { s2 $body }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::outer").unwrap().opaque_global_frame);
    }

    #[test]
    fn uplevel_hash_zero_non_writing_script_stays_empty() {
        // TN — a global-frame script that writes nothing (`uplevel #0
        // [list puts hi]`) contributes neither names nor opaqueness.
        let m = module("proc ::shout {} { uplevel #0 [list puts hi] }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::shout").unwrap().is_empty());
    }

    #[test]
    fn uplevel_caller_frame_is_not_a_global_write() {
        // An absolute value-write command proves the body only writes the
        // caller's frame, which is `upvar_info`'s business. A relative `set`
        // could be shadowed in the caller namespace by an arbitrary global-
        // writing procedure and must therefore widen this summary.
        let m = module("proc ::p {} { uplevel 1 { ::set n 1 } }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().is_empty());
    }

    #[test]
    fn uplevel_hash_zero_dynamic_write_target_in_literal_body_is_opaque() {
        // `uplevel #0 {set $n 1}` — the body is readable but the written
        // name is not.
        let m = module("proc ::p {n} { uplevel #0 \"set $n 1\" }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().opaque_global_frame);
    }

    #[test]
    fn unrelated_global_write_is_not_confused_with_other_procs() {
        let m =
            module("proc ::mutate_y {} { global y; set y 1 }\nproc ::p {} { global x; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
        assert!(!info.get("::p").unwrap().names.contains("y"));
    }
}
