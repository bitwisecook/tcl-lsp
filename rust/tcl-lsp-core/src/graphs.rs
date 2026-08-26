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

//! Analysis graph JSON builders — the call graph, symbol graph, and
//! dataflow/taint graph, each rendered to a `serde_json::Value`.
//!
//! This is the shared, consumer-agnostic home for the graph shapes: the
//! `tcl` CLI (`callgraph` / `symbolgraph` / `dataflow` verbs), the LSP
//! server, and the `tcl_lsp_py` `PyO3` facades all build the *same* graphs
//! from this one implementation. Callers supply a resolved
//! [`CommandRegistry`] and the dialect string; every position is 0-based
//! and UTF-16 counted, matching the LSP wire convention.

use serde_json::{Map, Value, json};
use tcl_compiler::analyser::{Analyser, AnalysisResult, ProcDef, Scope, ScopeKind, VarDef};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::dataflow_graph::{FunctionInputs, extract_dataflow_graph};
use tcl_compiler::ir::Module as IrModule;
use tcl_compiler::path_concat::find_path_concat_warnings;
use tcl_compiler::side_effects::EffectRegion;
use tcl_compiler::taint::{
    find_destructive_file_warnings, find_setter_constraint_warnings,
    find_taint_warnings_for_function, is_irules_dialect,
};
use tcl_compiler::uri_split::find_uri_split_suggestions;
use tcl_lexer::{LineIndex, Span};
use tcl_registry::CommandRegistry;

/// The synthetic caller node for calls made outside any proc body.
const TOP_LEVEL: &str = "<top-level>";

// Shared position helpers

/// 0-based source line of a span-start offset (`range.start.line`).
fn line0(line_index: &LineIndex, offset: u32) -> u32 {
    line_index.position_at(offset).line
}

/// `{"line": L, "character": C}` for a span-start offset, 0-based and counted
/// in UTF-16 code units, over the analyser `Range`.
fn pos_value(line_index: &LineIndex, source: &str, offset: u32) -> Value {
    let pos = line_index.position_at_utf16(offset, source);
    json!({ "line": pos.line, "character": pos.character.get() })
}

// symbol graph

/// Call sites of a proc, deduplicated by span.
fn find_proc_call_sites(name: &str, qualified_name: &str, analysis: &AnalysisResult) -> Vec<Span> {
    let no_prefix = qualified_name.strip_prefix("::").unwrap_or(qualified_name);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for inv in &analysis.command_invocations {
        let matches = match &inv.resolved_qualified_name {
            Some(resolved) => resolved == qualified_name,
            None => inv.name == name || inv.name == qualified_name || inv.name == no_prefix,
        };
        if matches && seen.insert((inv.range.start(), inv.range.end())) {
            out.push(inv.range);
        }
    }
    out
}

/// Serialise one scope (global / namespace) to the wire dict.
///
/// Procs first (source order), then this scope's
/// variables, then children — namespace children recurse with the full shape,
/// proc children contribute a `{kind, name, variables, children:[]}` node only
/// when they carry recorded variables. `HashMap` iteration is made
/// deterministic by sorting on the defining token's offset.
fn scope_to_value(
    scope: &Scope,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    source: &str,
    depth: u32,
) -> Value {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return json!({ "kind": scope.kind.as_str(), "name": scope.name });
    }
    let mut procs: Vec<&ProcDef> = scope.procs.values().collect();
    procs.sort_by_key(|p| p.name_span.start());
    let proc_values: Vec<Value> = procs
        .iter()
        .map(|proc| {
            let ref_count = analysis
                .command_invocations
                .iter()
                .filter(|inv| {
                    inv.resolved_qualified_name.as_deref() == Some(proc.qualified_name.as_str())
                        || inv.name == proc.name
                })
                .count();
            json!({
                "name": proc.name,
                "qualified_name": proc.qualified_name,
                "params": proc.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
                "line": line0(line_index, proc.name_span.start()),
                "ref_count": ref_count,
            })
        })
        .collect();

    // Namespace/global scopes carry no parameters, so an empty param order is
    // correct here.
    let variable_values = collect_var_values(scope, &[], line_index, source);

    let mut children = Vec::new();
    for child in &scope.children {
        match child.kind {
            ScopeKind::Namespace => {
                children.push(scope_to_value(
                    child,
                    analysis,
                    line_index,
                    source,
                    depth + 1,
                ));
            }
            ScopeKind::Proc => {
                // The proc's parameters all share the proc-name token's span,
                // so the scope alone can't order them; recover declaration
                // order from the owning `ProcDef` (matched by body span).
                let params: Vec<String> = scope
                    .procs
                    .values()
                    .find(|p| Some(p.body_span) == child.body_span)
                    .map(|p| p.params.iter().map(|x| x.name.clone()).collect())
                    .unwrap_or_default();
                let proc_vars = collect_var_values(child, &params, line_index, source);
                if !proc_vars.is_empty() {
                    children.push(json!({
                        "kind": "proc",
                        "name": child.name,
                        "variables": proc_vars,
                        "children": [],
                    }));
                }
            }
            _ => {}
        }
    }

    let mut map = serde_json::Map::new();
    map.insert("kind".to_string(), json!(scope.kind.as_str()));
    map.insert("name".to_string(), json!(scope.name));
    if !proc_values.is_empty() {
        map.insert("procs".to_string(), json!(proc_values));
    }
    if !variable_values.is_empty() {
        map.insert("variables".to_string(), json!(variable_values));
    }
    if !children.is_empty() {
        map.insert("children".to_string(), json!(children));
    }
    Value::Object(map)
}

/// `[{name, line, references:[…]}]` for a scope's variables in insertion
/// order: parameters first (declaration order, since they share the
/// proc-name span and can't be span-ordered), then body locals by definition
/// offset. `params` is empty for namespace/global scopes.
fn collect_var_values(
    scope: &Scope,
    params: &[String],
    line_index: &LineIndex,
    source: &str,
) -> Vec<Value> {
    let param_pos: std::collections::HashMap<&str, usize> = params
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut vars: Vec<&VarDef> = scope.variables.values().collect();
    vars.sort_by(|a, b| {
        match (
            param_pos.get(a.name.as_str()),
            param_pos.get(b.name.as_str()),
        ) {
            (Some(ia), Some(ib)) => ia.cmp(ib),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.definition_span.start().cmp(&b.definition_span.start()),
        }
    });
    vars.iter()
        .map(|var| {
            let refs: Vec<Value> = var
                .references
                .iter()
                .map(|r| pos_value(line_index, source, r.start()))
                .collect();
            json!({
                "name": var.name,
                "line": line0(line_index, var.definition_span.start()),
                "references": refs,
            })
        })
        .collect()
}

/// Count variables with a definition site across the whole scope tree.
fn count_variables(scope: &Scope, depth: u32) -> usize {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return 0;
    }
    scope.variables.len()
        + scope
            .children
            .iter()
            .map(|c| count_variables(c, depth + 1))
            .sum::<usize>()
}

/// Count namespace scopes across the whole scope tree.
fn count_namespaces(scope: &Scope, depth: u32) -> usize {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return 0;
    }
    scope
        .children
        .iter()
        .map(|c| usize::from(c.kind == ScopeKind::Namespace) + count_namespaces(c, depth + 1))
        .sum()
}

/// Build the full symbol-graph payload: scope hierarchy with proc/variable
/// references, proc call-site index, package requirements, and a summary.
#[must_use]
pub fn symbol_graph(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> Value {
    let result = Analyser::new().analyse(source, dialect.name);
    let line_index = LineIndex::new(source);

    let scopes = vec![scope_to_value(
        &result.global_scope,
        &result,
        &line_index,
        source,
        0,
    )];

    // proc_references: every proc's deduped call sites, source-ordered keys.
    let mut all_procs: Vec<&ProcDef> = result.all_procs.values().collect();
    all_procs.sort_by_key(|p| p.name_span.start());
    let mut proc_references = serde_json::Map::new();
    for proc in all_procs {
        let sites = find_proc_call_sites(&proc.name, &proc.qualified_name, &result);
        if !sites.is_empty() {
            let positions: Vec<Value> = sites
                .iter()
                .map(|s| pos_value(&line_index, source, s.start()))
                .collect();
            proc_references.insert(proc.qualified_name.clone(), json!(positions));
        }
    }

    let package_requires: Vec<Value> = result
        .package_requires
        .iter()
        .map(|pr| {
            let mut map = serde_json::Map::new();
            map.insert("name".to_string(), json!(pr.name));
            map.insert(
                "line".to_string(),
                json!(line0(&line_index, pr.range.start())),
            );
            if let Some(version) = &pr.version {
                map.insert("version".to_string(), json!(version));
            }
            Value::Object(map)
        })
        .collect();

    let total_procs = result.all_procs.len();
    let total_variables = count_variables(&result.global_scope, 0);
    let total_namespaces = count_namespaces(&result.global_scope, 0);

    json!({
        "scopes": scopes,
        "proc_references": Value::Object(proc_references),
        "package_requires": package_requires,
        "summary": {
            "total_procs": total_procs,
            "total_variables": total_variables,
            "total_namespaces": total_namespaces,
        },
    })
}

// call graph

/// Human-readable string for an [`EffectRegion`].
/// Empty → `"NONE"`; otherwise the contained
/// single-bit member names joined with `|` in definition order (which, for
/// a single member, yields just that name).
fn effect_region_str(er: EffectRegion) -> String {
    const MEMBERS: &[(EffectRegion, &str)] = &[
        (EffectRegion::HTTP_STATE, "HTTP_STATE"),
        (EffectRegion::RESPONSE_LIFECYCLE, "RESPONSE_LIFECYCLE"),
        (EffectRegion::GLOBAL_STATE, "GLOBAL_STATE"),
        (EffectRegion::UNKNOWN_STATE, "UNKNOWN_STATE"),
    ];
    if er.is_empty() {
        return "NONE".to_owned();
    }
    MEMBERS
        .iter()
        .filter(|(flag, _)| er.contains(*flag))
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join("|")
}

/// 0-based start/end line pair for a proc's definition span.
fn proc_line_span(ir_module: &IrModule, qname: &str, line_index: &LineIndex) -> Option<(u32, u32)> {
    ir_module.procedures.get(qname).map(|p| {
        (
            line0(line_index, p.span.start()),
            line0(line_index, p.span.end()),
        )
    })
}

/// Whether a span lies wholly within some proc's definition.
fn is_inside_proc(range: Span, ir_module: &IrModule, line_index: &LineIndex) -> bool {
    let start = line0(line_index, range.start());
    let end = line0(line_index, range.end());
    ir_module.procedures.values().any(|p| {
        let ps = line0(line_index, p.span.start());
        let pe = line0(line_index, p.span.end());
        start >= ps && end <= pe
    })
}

/// Call-site positions of `callee_qname` within `containing_proc`'s body.
fn find_call_sites_in_scope(
    analysis: &AnalysisResult,
    callee_qname: &str,
    containing_proc: &str,
    ir_module: &IrModule,
    line_index: &LineIndex,
    source: &str,
) -> Vec<Value> {
    let Some((proc_start, proc_end)) = proc_line_span(ir_module, containing_proc, line_index)
    else {
        return Vec::new();
    };
    let short = callee_qname.trim_start_matches(':');
    let mut callee_forms: std::collections::HashSet<&str> =
        [callee_qname, short].into_iter().collect();
    if let Some(stripped) = callee_qname.strip_prefix("::") {
        callee_forms.insert(stripped);
    }

    let mut sites = Vec::new();
    for inv in &analysis.command_invocations {
        if line0(line_index, inv.range.start()) < proc_start {
            continue;
        }
        if line0(line_index, inv.range.end()) > proc_end {
            continue;
        }
        match &inv.resolved_qualified_name {
            Some(resolved) => {
                if resolved != callee_qname {
                    continue;
                }
            }
            None => {
                if !callee_forms.contains(inv.name.as_str()) {
                    continue;
                }
            }
        }
        sites.push(pos_value(line_index, source, inv.range.start()));
    }
    sites
}

/// Resolve a top-level invocation to a known proc qname. Proc names are
/// iterated in sorted order
/// for a deterministic pick on the (rare) ambiguous short-name fallback.
fn resolve_invocation_target(
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    proc_names_sorted: &[String],
) -> Option<String> {
    if let Some(resolved) = &inv.resolved_qualified_name
        && proc_names_sorted.iter().any(|n| n == resolved)
    {
        return Some(resolved.clone());
    }
    for qname in proc_names_sorted {
        let short = qname.strip_prefix("::").unwrap_or(qname);
        if inv.name == *qname || inv.name == short || inv.name == qname.trim_start_matches(':') {
            return Some(qname.clone());
        }
    }
    None
}

/// The `nodes` list — one entry per proc (sorted by qname), carrying
/// params, 0-based definition line, purity, and the effect-region string.
fn build_nodes(
    proc_names: &[String],
    interproc: &tcl_compiler::interprocedural::InterproceduralAnalysis,
    ir_module: &IrModule,
    line_index: &LineIndex,
) -> Vec<Value> {
    proc_names
        .iter()
        .map(|qname| {
            let summary = &interproc.procedures[qname];
            let line = ir_module
                .procedures
                .get(qname)
                .map(|p| line0(line_index, p.span.start()));
            let effects = effect_region_str(summary.effect_reads | summary.effect_writes);
            json!({
                "name": qname,
                "params": summary.params,
                "line": line,
                "pure": summary.pure,
                "effects": effects,
            })
        })
        .collect()
}

/// Build the full call-graph payload.
#[must_use]
pub fn call_graph(
    source: &str,
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Value {
    // Build the full compilation unit (via `ensure_compilation_unit`) so the
    // interprocedural pass sees the same lowered IR — raw `lower_to_ir` alone
    // does not surface nested `[cmd …]` call sites to the call scanner.
    let profile = dialect;
    let cu = CompilationUnit::build_for_profile(source, registry, false, profile)
        .with_interprocedural(registry, Some(profile));
    let ir_module = &cu.ir_module;
    let interproc = cu
        .interproc
        .as_ref()
        .expect("with_interprocedural populates the summary");
    let analysis = Analyser::new().analyse(source, dialect.name);
    let line_index = LineIndex::new(source);

    // Proc qnames in sorted order (iterates the summary dict, which is
    // keyed and surfaced in sorted qualified-name order).
    let mut proc_names: Vec<String> = interproc.procedures.keys().cloned().collect();
    proc_names.sort();
    let proc_set: std::collections::HashSet<&str> = proc_names.iter().map(String::as_str).collect();

    let nodes = build_nodes(&proc_names, interproc, ir_module, &line_index);

    // Edges + bookkeeping.
    let mut edges: Vec<Value> = Vec::new();
    let mut called_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut has_outgoing: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for qname in &proc_names {
        let summary = &interproc.procedures[qname];
        for callee in &summary.direct_calls {
            if !proc_set.contains(callee.as_str()) {
                continue;
            }
            has_outgoing.insert(qname.as_str());
            called_set.insert(callee.clone());
            let sites =
                find_call_sites_in_scope(&analysis, callee, qname, ir_module, &line_index, source);
            edges.push(json!({
                "caller": qname,
                "callee": callee,
                "call_sites": sites,
            }));
        }
    }

    // Top-level calls (outside any proc body), in first-seen order.
    let mut top_level: Map<String, Value> = Map::new();
    for inv in &analysis.command_invocations {
        if is_inside_proc(inv.range, ir_module, &line_index) {
            continue;
        }
        if let Some(target) = resolve_invocation_target(inv, &proc_names) {
            let pos = pos_value(&line_index, source, inv.range.start());
            top_level
                .entry(target)
                .or_insert_with(|| Value::Array(Vec::new()))
                .as_array_mut()
                .expect("call-site list is an array")
                .push(pos);
        }
    }
    let has_top_level_calls = !top_level.is_empty();
    for (callee, sites) in &top_level {
        called_set.insert(callee.clone());
        edges.push(json!({
            "caller": TOP_LEVEL,
            "callee": callee,
            "call_sites": sites,
        }));
    }

    // Roots = uncalled procs, sorted; `<top-level>` prepended when any
    // invocation sits outside a proc body.
    let mut roots: Vec<String> = proc_names
        .iter()
        .filter(|qn| !called_set.contains(*qn))
        .cloned()
        .collect();
    roots.sort();
    let any_top_level_inv = analysis
        .command_invocations
        .iter()
        .any(|inv| !is_inside_proc(inv.range, ir_module, &line_index));
    if has_top_level_calls || any_top_level_inv {
        roots.insert(0, TOP_LEVEL.to_owned());
    }

    // Leaves = procs with no outgoing proc calls, sorted.
    let mut leaf_procs: Vec<String> = proc_names
        .iter()
        .filter(|qn| !has_outgoing.contains(qn.as_str()))
        .cloned()
        .collect();
    leaf_procs.sort();

    json!({
        "nodes": nodes,
        "edges": edges,
        "roots": roots,
        "leaf_procs": leaf_procs,
    })
}

// dataflow graph

/// Shared, per-module context for [`collect_taint_warnings`] — identical for
/// every function unit in the graph, so it is built once and threaded as a
/// single argument (keeping the collector under clippy's argument ceiling).
#[derive(Clone, Copy)]
struct TaintWarnCtx<'a> {
    /// The analysis dialect's command registry.
    registry: &'a CommandRegistry,
    /// The dialect string (drives iRules-only warning families + octal reads).
    dialect: &'static tcl_dialect::DialectProfile,
    /// Namespace-scoped proc names that shadow builtins for their namespace.
    shadow_proc_qnames: &'a std::collections::HashSet<&'a str>,
    /// Whole-module variable-trace facts.
    module_traces: tcl_compiler::compilation_unit::ModuleTraceFacts<'a>,
    /// Byte-offset → line/column index for the source.
    line_index: &'a LineIndex,
}

/// Aggregate every taint warning kind for one function unit into the
/// dataflow JSON shape. Runs five families in a
/// fixed order: sink injection, setter-constraint,
/// uri-split, path-concat, then destructive-file — the same order
/// and dialect gating as `compiler_checks::run_all_checks`. Every kind
/// yields a `variable` + `sink_command` (the path-concat warning is a
/// `TaintWarning` with `sink_command="set"`; the Rust `PathConcatWarning`
/// carries no command field, so we supply the same constant).
#[allow(clippy::too_many_lines)] // fixed diagnostic-family ordering mirrors compiler_checks.
fn collect_taint_warnings(
    fu: &FunctionUnit,
    taints: &std::collections::HashMap<
        tcl_compiler::ssa::ValueKey,
        tcl_compiler::taint::TaintLattice,
    >,
    ctx: TaintWarnCtx<'_>,
    out: &mut Vec<Value>,
) {
    let TaintWarnCtx {
        registry,
        dialect,
        shadow_proc_qnames,
        module_traces,
        line_index,
    } = ctx;
    let profile = dialect;
    let mut push = |code: &str, span: Span, message: &str, variable: &str, sink: &str| {
        out.push(json!({
            "code": code,
            "line": line0(line_index, span.start()),
            "message": message,
            "variable": variable,
            "sink_command": sink,
        }));
    };

    // A namespace-scoped proc of the same name shadows a builtin for every
    // call made from that namespace — see `taint::shadowed_builtin_names`.
    let namespace = tcl_compiler::optimiser::helpers::naming::namespace_from_qualified(&fu.name);
    let shadowed = tcl_compiler::taint::shadowed_builtin_names(
        &namespace,
        shadow_proc_qnames.iter().copied(),
        registry,
    );
    // `trace add variable` / `trace variable` lets a runtime callback rewrite
    // a variable in ways static analysis can't see — a traced name is
    // conservatively treated as tainted everywhere in the module — see
    // `taint::apply_module_variable_traces`.
    let taints =
        tcl_compiler::taint::apply_module_variable_traces(taints.clone(), &fu.ssa, module_traces);
    let taints = &taints;

    // 1. Sink injection (T100 / T101 / T102 families).
    for w in find_taint_warnings_for_function(
        fu,
        taints,
        &fu.sccp.executable_blocks,
        registry,
        Some(profile),
        &shadowed,
        module_traces,
    ) {
        push(
            w.code.as_str(),
            w.span,
            &w.message,
            &w.variable,
            &w.sink_command,
        );
    }

    // 2 + 3. Setter-constraint + uri-split are iRules-only. The helpers
    // gate internally too; skipping the walk under a non-iRules dialect
    // matches `compiler_checks` and avoids needless work.
    if is_irules_dialect(Some(profile)) {
        for w in find_setter_constraint_warnings(
            registry,
            &fu.cfg,
            &fu.ssa,
            taints,
            &fu.sccp.executable_blocks,
            Some(profile),
        ) {
            push(
                w.code.as_str(),
                w.span,
                &w.message,
                &w.variable,
                &w.sink_command,
            );
        }
        for w in find_uri_split_suggestions(
            &fu.cfg,
            &fu.ssa,
            Some(&fu.sccp.values),
            &fu.sccp.executable_blocks,
            registry,
            Some(profile),
        ) {
            push(
                w.code.as_str(),
                w.span,
                &w.message,
                &w.variable,
                &w.sink_command,
            );
        }
    }

    // 4. Path-concat (`W201`). Reports `sink_command="set"`.
    for w in find_path_concat_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.rendered_props,
        taints,
        &fu.sccp.executable_blocks,
    ) {
        push(w.code.as_str(), w.span, &w.message, &w.variable, "set");
    }

    // 5. Destructive-file (`W313`).
    for w in find_destructive_file_warnings(
        &fu.cfg,
        &fu.ssa,
        taints,
        &fu.sccp.executable_blocks,
        registry,
        Some(profile),
    ) {
        push(
            w.code.as_str(),
            w.span,
            &w.message,
            &w.variable,
            &w.sink_command,
        );
    }
}

/// Tainted variable names in `fu` (deduplicated, sorted for a
/// deterministic order). The per-unit taint lattice map
/// is iterated in SSA insertion order; the taint map is a `HashMap`, so the
/// order is recovered by sorting.
///
/// Version-0 entries are skipped: a `(name, 0)` slot is the enclosing-
/// scope / pre-existing value, and the only way it becomes tainted is the
/// conservative cross-procedure global-write seeding in `propagate_taints`
/// (e.g. `::store` in `proc save {v} { set ::store $v }`). The raw per-unit
/// taint map does no such seeding, so it never surfaces a
/// version-0-tainted variable — filtering them here keeps the
/// `tainted_variables` output without disturbing the seeding the sink
/// warnings rely on.
fn tainted_var_names(fu: &FunctionUnit) -> Vec<&str> {
    // Definition-site offset of each `(name, version)` SSA value, so
    // tainted variables order by where they are defined — recovering
    // the SSA/source iteration order over the per-unit taint map rather
    // than an alphabetical sort. Each SSA version is defined once, so
    // `or_insert` records that single site.
    let mut def_offset: std::collections::HashMap<(&str, u32), u32> =
        std::collections::HashMap::new();
    for block in fu.ssa.blocks.values() {
        for st in &block.statements {
            let off = st.statement.span().start();
            for (name, &ver) in &st.defs {
                def_offset
                    .entry((fu.ssa.var_name(*name), ver))
                    .or_insert(off);
            }
        }
    }

    let mut entries: Vec<(&str, u32)> = fu
        .taints
        .iter()
        .filter(|((_, version), lattice)| *version != 0 && lattice.is_tainted())
        .map(|((name, version), _)| (fu.ssa.var_name(*name), *version))
        .collect();
    // (def offset, version, name) keeps the order deterministic and
    // source-ordered; values defined outside this unit (no def site) sort
    // last.
    entries.sort_by_key(|(name, ver)| {
        (
            def_offset.get(&(*name, *ver)).copied().unwrap_or(u32::MAX),
            *ver,
            *name,
        )
    });
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    entries
        .into_iter()
        .filter_map(|(name, _)| seen.insert(name).then_some(name))
        .collect()
}

/// Build the dataflow / taint graph payload.
#[must_use]
pub fn dataflow_graph(
    source: &str,
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Value {
    let profile = dialect;
    let cu = CompilationUnit::build_for_profile(source, registry, false, profile)
        .with_interprocedural(registry, Some(profile));
    let line_index = LineIndex::new(source);

    let mut proc_names: Vec<&String> = cu.procedures.keys().collect();
    proc_names.sort();

    // Interprocedural taint solve: colour-aware return summaries + parameter
    // entry taints. Its `top_taints` / `proc_taints` feed the warning
    // families (cross-proc entry-taint), feeding
    // `find_taint_warnings`. The `tainted_variables` listing below keeps the
    // per-unit `fu.taints` lattice.
    let solved =
        tcl_compiler::taint_interproc::solve_interprocedural_taints(&cu, registry, Some(profile));

    // Taint warnings: top level first, then each procedure.
    let shadow_proc_qnames: std::collections::HashSet<&str> =
        cu.procedures.keys().map(String::as_str).collect();
    let module_traces = tcl_compiler::compilation_unit::ModuleTraceFacts {
        traced_variables: &cu.ir_module.traced_variables,
        has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
    };
    let taint_ctx = TaintWarnCtx {
        registry,
        dialect,
        shadow_proc_qnames: &shadow_proc_qnames,
        module_traces,
        line_index: &line_index,
    };
    let mut taint_warnings: Vec<Value> = Vec::new();
    collect_taint_warnings(
        &cu.top_level,
        solved.taints_for(&cu.top_level.name, &cu.top_level.taints),
        taint_ctx,
        &mut taint_warnings,
    );
    for qname in &proc_names {
        let fu = &cu.procedures[*qname];
        collect_taint_warnings(
            fu,
            solved.taints_for(&fu.name, &fu.taints),
            taint_ctx,
            &mut taint_warnings,
        );
    }

    // Tainted variables per scope: top level first, then each procedure.
    let mut tainted_variables: Vec<Value> = Vec::new();
    for name in tainted_var_names(&cu.top_level) {
        tainted_variables.push(json!({ "scope": "<top-level>", "variable": name }));
    }
    for qname in &proc_names {
        for name in tainted_var_names(&cu.procedures[*qname]) {
            tainted_variables.push(json!({ "scope": qname, "variable": name }));
        }
    }

    // Per-proc side-effect classification (interproc summaries, by qname).
    let interproc = cu
        .interproc
        .as_ref()
        .expect("with_interprocedural populates the summary");
    let mut effect_names: Vec<&String> = interproc.procedures.keys().collect();
    effect_names.sort();
    let mut proc_effects: Vec<Value> = Vec::new();
    let mut pure_count = 0i64;
    let mut impure_count = 0i64;
    for qname in &effect_names {
        let summary = &interproc.procedures[*qname];
        if summary.pure {
            pure_count += 1;
        } else {
            impure_count += 1;
        }
        proc_effects.push(json!({
            "name": qname,
            "pure": summary.pure,
            "reads": effect_region_str(summary.effect_reads),
            "writes": effect_region_str(summary.effect_writes),
            "has_barrier": summary.has_barrier,
        }));
    }

    json!({
        "taint_warnings": taint_warnings,
        "tainted_variables": tainted_variables,
        "proc_effects": proc_effects,
        "summary": {
            "total_taint_warnings": taint_warnings.len(),
            "tainted_variable_count": tainted_variables.len(),
            "pure_proc_count": pure_count,
            "impure_proc_count": impure_count,
        },
    })
}

// def-use / SSA data-flow graph

/// One function unit's SSA def-use view, serialised to the camelCase wire dict.
fn function_dataflow_json(f: &tcl_compiler::dataflow_graph::FunctionDataFlowGraph) -> Value {
    let nodes: Vec<Value> = f
        .nodes
        .iter()
        .map(|n| {
            json!({
                "name": n.name,
                "version": n.version,
                "block": n.block,
                "defKind": n.def_kind,
                "statementIndex": n.statement_index,
                "lattice": n.lattice,
                "typeInfo": n.type_info,
                "isDead": n.is_dead,
                "useCount": n.use_count,
            })
        })
        .collect();
    let edges: Vec<Value> = f
        .edges
        .iter()
        .map(|e| {
            json!({
                "fromName": e.from_name,
                "fromVersion": e.from_version,
                "toBlock": e.to_block,
                "toStatementIndex": e.to_statement_index,
                "edgeKind": e.edge_kind.as_str(),
                "toName": e.to_name,
                "toVersion": e.to_version,
            })
        })
        .collect();
    let aliases: Vec<Value> = f
        .aliases
        .iter()
        .map(|a| {
            json!({
                "localName": a.local_name,
                "localKind": a.local_kind,
                "targetName": a.target_name,
                "targetKind": a.target_kind,
                "reason": a.reason,
            })
        })
        .collect();
    json!({
        "name": f.function_name,
        "nodes": nodes,
        "edges": edges,
        "aliases": aliases,
        "summary": {
            "totalDefs": f.total_defs,
            "totalUses": f.total_uses,
            "deadDefs": f.dead_defs,
            "aliasedVars": f.aliased_vars,
        },
    })
}

/// Build the SSA **def-use chain** graph — per-function def nodes (with use
/// counts, lattice values, types), def→use edges, and memory-SSA alias info.
/// The top-level scope is included first, then each procedure.
#[must_use]
pub fn def_use_graph(
    source: &str,
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Value {
    fn build_inputs(fu: &FunctionUnit) -> FunctionInputs<'_> {
        FunctionInputs {
            name: fu.name.as_str(),
            ssa: &fu.ssa,
            du: &fu.def_use,
            sccp: Some(&fu.sccp),
            mem: fu.memory_ssa.as_ref(),
            types: Some(&fu.types),
        }
    }

    let profile = dialect;
    let cu = CompilationUnit::build_for_profile(source, registry, false, profile)
        .with_interprocedural(registry, Some(profile));

    let mut proc_names: Vec<&String> = cu.procedures.keys().collect();
    proc_names.sort();

    let mut inputs: Vec<FunctionInputs<'_>> = vec![build_inputs(&cu.top_level)];
    inputs.extend(proc_names.iter().map(|q| build_inputs(&cu.procedures[*q])));

    let graph = extract_dataflow_graph(&inputs);
    let functions: Vec<Value> = graph.functions.iter().map(function_dataflow_json).collect();

    json!({
        "functions": functions,
        "summary": {
            "totalDefs": graph.total_defs(),
            "totalUses": graph.total_uses(),
            "totalAliases": graph.total_aliases(),
            "functionCount": graph.functions.len(),
        },
    })
}

// memory-SSA alias graph

/// A single memory location's display string — `name` (with a `:qualifier`
/// suffix when the location carries one).
fn memory_location_str(loc: &tcl_compiler::memory_ssa::MemoryLocation) -> String {
    if loc.qualifier.is_empty() {
        loc.name.clone()
    } else {
        format!("{}:{}", loc.name, loc.qualifier)
    }
}

/// Serialise one function's memory-SSA to the alias wire dict.
fn memory_function_json(
    name: &str,
    mem: Option<&tcl_compiler::memory_ssa::MemorySsaFunction>,
) -> Value {
    let Some(mem) = mem else {
        return json!({ "name": name, "alias_sets": [], "memory_ops": 0 });
    };
    let alias_sets: Vec<Value> = mem
        .alias_sets
        .iter()
        .map(|aset| {
            json!({
                "names": aset.names().into_iter().collect::<Vec<_>>(),
                "reason": aset.reason,
                "locations": aset.locations.iter().map(memory_location_str).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "name": name,
        "alias_sets": alias_sets,
        "memory_ops": mem.memory_ops.len(),
        "memory_defs": mem.count_defs,
        "memory_uses": mem.count_uses,
        "clobbers": mem.count_clobbers,
    })
}

/// Build the memory-SSA **alias** graph — per-function alias sets (variables the
/// analysis proved may refer to the same storage, via `upvar` / `global` /
/// `variable`) with the reason and locations, plus memory-op counts.
#[must_use]
pub fn memory_alias_graph(
    source: &str,
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Value {
    let profile = dialect;
    let cu = CompilationUnit::build_for_profile(source, registry, false, profile)
        .with_interprocedural(registry, Some(profile))
        .with_memory_ssa(
            registry,
            crate::document_context_for_profile(profile).authoring_mask(),
        );

    let mut functions: Vec<Value> = vec![memory_function_json(
        "::top",
        cu.top_level.memory_ssa.as_ref(),
    )];
    let mut proc_names: Vec<&String> = cu.procedures.keys().collect();
    proc_names.sort();
    for qname in &proc_names {
        functions.push(memory_function_json(
            qname,
            cu.procedures[*qname].memory_ssa.as_ref(),
        ));
    }

    let total_alias_sets: usize = functions
        .iter()
        .filter_map(|f| f.get("alias_sets").and_then(Value::as_array).map(Vec::len))
        .sum();
    let total_memory_ops: u64 = functions
        .iter()
        .filter_map(|f| f.get("memory_ops").and_then(Value::as_u64))
        .sum();

    json!({
        "functions": functions,
        "summary": {
            "total_alias_sets": total_alias_sets,
            "total_memory_ops": total_memory_ops,
            "function_count": functions.len(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    /// Regression coverage for issue #996: `scope_to_value`,
    /// `count_variables`, and `count_namespaces` recurse once per nested
    /// namespace scope, with no depth cap before this fix
    /// (`MAX_SCOPE_WALK_DEPTH`, `crate::lib`). 80 nested `namespace eval`
    /// levels is past the point (confirmed empirically: 100+) where
    /// unguarded namespace-scope recursion overflows `cargo test`'s bare
    /// ~2 MiB per-test default — namespace nesting costs meaningfully more
    /// native stack per level than `if`-nesting does. The assertion is
    /// that `symbol_graph` returns at all, not what it returns.
    #[test]
    fn deeply_nested_namespaces_survive_symbol_graph() {
        const DEPTH: usize = 80;
        let mut source = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(source, "namespace eval ns{i} {{");
        }
        source.push_str("proc leaf {} { return 1 }\n");
        for _ in 0..DEPTH {
            source.push_str("}\n");
        }
        let graph = symbol_graph(
            &source,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        assert!(graph.get("scopes").is_some());
    }
}
