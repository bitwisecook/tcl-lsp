//! Bridge from the IR/CFG to the unified [`Place`] model (Phase 8 /
//! SYNC-MAY31-1b, stage 3).
//!
//! Resolves each IR statement's *def* and *read* targets to canonical
//! [`Place`]s on demand, so the dataflow consumers (dead-store / unused /
//! read-before-set) can switch from name-string equality to the sound
//! [`overlap`](crate::place::overlap) relation — without re-stamping every
//! construction site in lowering.  The per-function [`ResolveContext`] is built
//! once by scanning the CFG for `global` / `variable` / `upvar` / `trace`
//! declarations (reusing the [`crate::var_scoping`] grammar).
//!
//! Faithful port of `compiler/place_bridge.py` on `main`.  No consumer is wired
//! yet (that is stage 4) — this stage is output-equivalent.

use tcl_registry::{ArgRole, CommandRegistry};

use crate::cfg::{Function, Terminator};
use crate::ir::Statement;
use crate::naming::{normalise_qualified_name, normalise_var_name};
use crate::place::{self, overlap, places_read_to_form, Place, PlaceKind};
use crate::segmenter::segment_commands;
use crate::ssa::structural_body_indices;
use crate::var_refs::{command_subst_texts, scan_var_ref_forms, vars_in_expr};
use crate::var_resolve::{resolve_place, ResolveContext};
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};

// Commands whose VAR_READ-role argument names the *whole* array, not a scalar
// (so a write to any element is observed).  Conservative — extra members only
// widen overlap (sound).
const WHOLE_ARRAY_COMMANDS: &[&str] = &["::array", "::parray"];
const WHOLE_ARRAY_BARE: &[&str] = &["array", "parray"];

/// Scan *cfg* for the in-scope variable declarations and build the
/// [`ResolveContext`] used to resolve places within the function.
///
/// `fn_qname` is the function's fully-qualified name (`::ns::p`); the resolve
/// namespace is its parent (`::ns`), so a `variable v` resolves to `::ns::v`.
#[must_use]
pub fn build_resolve_context(cfg: &Function, fn_qname: &str) -> ResolveContext {
    let parent = fn_qname.rsplit_once("::").map_or("", |(head, _)| head);
    let namespace = if parent.is_empty() {
        "::".to_owned()
    } else {
        parent.to_owned()
    };

    let mut ctx = ResolveContext {
        namespace: namespace.clone(),
        ..ResolveContext::default()
    };

    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            let (command, canonical, args) = match stmt {
                Statement::Call {
                    command,
                    canonical_command,
                    args,
                    ..
                }
                | Statement::Barrier {
                    command,
                    canonical_command,
                    args,
                    ..
                } => (command.as_str(), canonical_command.as_deref(), args),
                _ => continue,
            };
            // Match against the canonical (`::`-qualified) command name.
            let canon = canonical.map_or_else(|| normalise_qualified_name(command), str::to_owned);
            match canon.as_str() {
                "::global" => {
                    for i in global_declaration_indices(args) {
                        if let Some(a) = args.get(i) {
                            ctx.globals.insert(normalise_var_name(a).to_owned());
                        }
                    }
                }
                "::variable" => {
                    for i in variable_declaration_indices(args) {
                        if let Some(a) = args.get(i) {
                            ctx.ns_vars.insert(normalise_var_name(a).to_owned());
                        }
                    }
                }
                "::trace" => {
                    if let Some(t) = trace_target(args) {
                        ctx.traced.insert(normalise_var_name(t).to_owned());
                    }
                }
                _ => {}
            }
            // `upvar` / `namespace upvar` — recognised structurally by the
            // shared grammar (the IR command for `namespace upvar` is
            // `namespace`), so gate on the surface command, not the canonical.
            collect_upvar_aliases(command, args, &mut ctx.upvar_aliases, &namespace);
        }
    }

    ctx
}

fn qualify_namespace(ns_arg: &str, current: &str) -> String {
    if ns_arg.starts_with("::") {
        return ns_arg.to_owned();
    }
    if current.is_empty() || current == "::" {
        format!("::{ns_arg}")
    } else {
        format!("{current}::{ns_arg}")
    }
}

fn collect_upvar_aliases(
    command: &str,
    args: &[String],
    out: &mut std::collections::HashMap<String, String>,
    namespace: &str,
) {
    // The namespace argument of a `namespace upvar` form (None for plain upvar).
    let ns_arg: Option<&str> =
        if command == "namespace" && args.first().map(String::as_str) == Some("upvar") {
            args.get(1).map(String::as_str)
        } else if command == "namespace upvar" {
            args.first().map(String::as_str)
        } else {
            None
        };

    for local_i in upvar_local_declaration_indices(command, args) {
        let Some(alias_raw) = args.get(local_i) else {
            continue;
        };
        let alias = normalise_var_name(alias_raw).to_owned();
        if alias.is_empty() {
            continue;
        }
        let target_arg = local_i
            .checked_sub(1)
            .and_then(|j| args.get(j))
            .map_or("", String::as_str);
        // A target with any embedded substitution names a runtime-computed
        // caller var, so it is unresolvable: record it as dynamic ("").
        let dynamic = target_arg.is_empty() || target_arg.contains('$') || target_arg.contains('[');
        let target = if dynamic {
            String::new()
        } else if let Some(ns) = ns_arg {
            if ns.is_empty() || ns.contains('$') || ns.contains('[') {
                String::new()
            } else {
                let ns_fq = qualify_namespace(ns, namespace);
                format!(
                    "{}::{}",
                    ns_fq.trim_end_matches(':'),
                    target_arg.trim_start_matches(':')
                )
            }
        } else {
            target_arg.to_owned()
        };
        out.insert(alias, target);
    }
}

fn trace_target(args: &[String]) -> Option<&str> {
    if args.len() >= 3 && args[0] == "add" && args[1] == "variable" {
        return Some(&args[2]);
    }
    if args.len() >= 2 && args[0] == "variable" {
        return Some(&args[1]);
    }
    None
}

/// The places *stmt* writes (element-granular).
#[must_use]
pub fn def_places(
    stmt: &Statement,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => {
            vec![resolve_place(name, ctx, false, registry)]
        }
        Statement::Call { defs, .. } => defs
            .iter()
            .map(|d| resolve_place(d, ctx, false, registry))
            .collect(),
        _ => Vec::new(),
    }
}

/// Resolve the reads of every command in `script_text` — VAR_READ-role name
/// args (`array get arr` reads whole array `arr`; `info exists x` reads `x`)
/// plus a recursive descent of each arg word.
fn command_reads(
    script_text: &str,
    ctx: &ResolveContext,
    out: &mut Vec<Place>,
    registry: &CommandRegistry,
) {
    for cmd in segment_commands(script_text) {
        let name = cmd.name();
        if name.is_empty() {
            continue;
        }
        let args = cmd.args();
        let whole = WHOLE_ARRAY_BARE.contains(&name)
            || WHOLE_ARRAY_COMMANDS.contains(&normalise_qualified_name(name).as_str());
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        for i in registry.arg_indices_for_role(name, &arg_strs, ArgRole::VarRead) {
            if let Some(a) = args.get(i) {
                if !a.starts_with('$') && !a.starts_with('[') {
                    out.push(resolve_place(a, ctx, whole, registry));
                } else {
                    // Dynamic VAR_READ name (`array get $arr_name`): the read
                    // target is computed → UNKNOWN (overlaps everything) so a
                    // write to any array is not falsely flagged dead.
                    out.push(place::unknown_top());
                }
            }
        }
        word_reads(name, ctx, out, registry);
        for arg in args {
            word_reads(arg, ctx, out, registry);
        }
    }
}

/// Resolve reads in a single word: `$`-refs (precise, with array index) at the
/// top level, and any `[...]` command substitution recursively.
fn word_reads(text: &str, ctx: &ResolveContext, out: &mut Vec<Place>, registry: &CommandRegistry) {
    if text.is_empty() {
        return;
    }
    if text.contains('$') {
        for r in scan_var_ref_forms(text) {
            out.push(resolve_place(&r, ctx, false, registry));
        }
    }
    for inner in command_subst_texts(text) {
        command_reads(&inner, ctx, out, registry);
    }
}

/// The places *stmt* reads (sound over-approximation).
///
/// Covers `$`-substitution reads (element-granular for arrays), VAR_READ-role
/// name args at every nesting level (`[array get arr]` reads the whole array),
/// and the name/index variables read to *form* a dynamic target/ref (`set $X`
/// reads `X`; `$arr($i)` reads `i`) via [`places_read_to_form`].
#[must_use]
pub fn read_places(
    stmt: &Statement,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();

    match stmt {
        Statement::AssignValue { value, .. } => word_reads(value, ctx, &mut out, registry),
        Statement::Incr { name, amount, .. } => {
            // `incr x` reads its own target; the amount may read more.
            out.push(resolve_place(name, ctx, false, registry));
            if let Some(a) = amount {
                word_reads(a, ctx, &mut out, registry);
            }
        }
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            for nm in vars_in_expr(expr) {
                out.push(resolve_place(&nm, ctx, false, registry));
            }
            for cmd_text in expr.command_texts() {
                word_reads(&cmd_text, ctx, &mut out, registry);
            }
        }
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                word_reads(v, ctx, &mut out, registry);
            }
            if let Some(e) = expr {
                for nm in vars_in_expr(e) {
                    out.push(resolve_place(&nm, ctx, false, registry));
                }
            }
        }
        Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            args,
            tokens,
            ..
        } => {
            let whole = canonical_command
                .as_deref()
                .is_some_and(|c| WHOLE_ARRAY_COMMANDS.contains(&c));
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            let read_idx: std::collections::HashSet<usize> = registry
                .arg_indices_for_role(command, &arg_strs, ArgRole::VarRead)
                .into_iter()
                .collect();
            // A structural body (proc/method/test/snit) runs in its own scope
            // and is analysed separately — its reads must not leak into this
            // scope.
            let structural = structural_body_indices(command, args, tokens.as_ref(), registry);
            for (i, arg) in args.iter().enumerate() {
                if structural.contains(&i) {
                    continue;
                }
                if read_idx.contains(&i) && !arg.starts_with('$') && !arg.starts_with('[') {
                    // A name-valued read arg (`info exists x`, `array get a`).
                    out.push(resolve_place(arg, ctx, whole, registry));
                } else {
                    if read_idx.contains(&i) {
                        // Dynamic VAR_READ name (`array get $arr_name`) → UNKNOWN.
                        out.push(place::unknown_top());
                    }
                    word_reads(arg, ctx, &mut out, registry);
                }
            }
            word_reads(command, ctx, &mut out, registry);
        }
        Statement::Block { body, .. } => {
            // A `Block` body (inlined passthrough / `eval {…}` brace-literal)
            // runs in the enclosing scope — recover its reads so a `$x` inside
            // `eval {puts $x}` isn't seen as dead.  (`namespace eval ns {…}`
            // lowers to a `Barrier`, not a `Block`, so it is not recursed.)
            for inner in &body.statements {
                out.extend(read_places(inner, ctx, registry));
            }
        }
        _ => {}
    }

    // The vars read to *form* any dynamic def target (`set $X` / `set ${tok}(k)`
    // / `set a($i)`) AND any dynamic read ref (`$arr($i)` reads `i`) — the
    // genuine name/index reads the dead-store/unused checks need.
    let mut formed: Vec<Place> = Vec::new();
    for dp in def_places(stmt, ctx, registry) {
        formed.extend(places_read_to_form(&dp));
    }
    for rp in &out {
        formed.extend(places_read_to_form(rp));
    }
    out.extend(formed);

    out
}

/// The places a block *terminator* reads — an `if` / `while` / `for` branch
/// condition or a `return` value/expr.  Covers the direct `$`-refs plus any
/// variables hidden in a command substitution within the condition.
#[must_use]
pub fn terminator_read_places(
    term: &Terminator,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();
    match term {
        Terminator::Branch { condition, .. } => {
            for nm in vars_in_expr(condition) {
                out.push(resolve_place(&nm, ctx, false, registry));
            }
            for cmd_text in condition.command_texts() {
                word_reads(&cmd_text, ctx, &mut out, registry);
            }
        }
        Terminator::Return {
            value,
            expr,
            braced,
            ..
        } => {
            // A braced `return {…}` is a literal — its `$`-refs are not reads.
            if !braced {
                if let Some(v) = value {
                    word_reads(v, ctx, &mut out, registry);
                }
                if let Some(e) = expr {
                    for nm in vars_in_expr(e) {
                        out.push(resolve_place(&nm, ctx, false, registry));
                    }
                    for cmd_text in e.command_texts() {
                        word_reads(&cmd_text, ctx, &mut out, registry);
                    }
                }
            }
        }
        Terminator::Goto { .. } => {}
    }
    out
}

/// `(block_name, statement_index)` of every array-element / dict-path
/// assignment in *cfg* whose def [`Place`] is **observed** by some read place
/// in the function — the element writes a *name-level* dead-store / unused
/// analysis mis-folds (SSA collapses `a(k)` / `a(j)` / `$a` to the base `a`)
/// but that a read in fact keeps live.
///
/// Shared by the analyser (W220) and the optimiser (O109): both flag an
/// overwritten/unused SSA def at name granularity, so both consult this to
/// suppress the array-element false positives.  The suppression is
/// element-granular only — scalar defs (which never fold) keep their precise
/// name-level verdict.  A whole-array / UNKNOWN / upvar-alias read overlaps
/// every element (so `array get a` keeps each `a(k)` write live); this
/// full-scan `overlap` matches `main`'s bucketed `_make_element_observed`,
/// where `UNKNOWN` / `UPVAR_ALIAS` reads are wildcards (here `overlap` returns
/// `true` on those edges).  Empty (no cost) when *cfg* has no array-element
/// assignment.
#[must_use]
pub fn element_writes_observed_by_reads(
    cfg: &Function,
    fn_qname: &str,
    registry: &CommandRegistry,
) -> std::collections::HashSet<(String, i32)> {
    use std::collections::HashSet;

    let is_array_assign = |stmt: &Statement| -> bool {
        matches!(
            stmt,
            Statement::AssignConst { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::AssignExpr { name, .. }
                if name.contains('(')
        )
    };

    let mut out = HashSet::new();
    // Cheap pre-check: only array-element assignments can be mis-folded.
    if !cfg
        .blocks
        .values()
        .any(|b| b.statements.iter().any(&is_array_assign))
    {
        return out;
    }

    let ctx = build_resolve_context(cfg, fn_qname);
    // All read places in the function, collected once.
    let mut reads: Vec<Place> = Vec::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            reads.extend(read_places(stmt, &ctx, registry));
        }
        if let Some(term) = &block.terminator {
            reads.extend(terminator_read_places(term, &ctx, registry));
        }
    }

    for (block_name, block) in &cfg.blocks {
        for (idx, stmt) in block.statements.iter().enumerate() {
            if !is_array_assign(stmt) {
                continue;
            }
            let observed = def_places(stmt, &ctx, registry).iter().any(|d| {
                matches!(d.kind, PlaceKind::ArrayElem | PlaceKind::DictPath)
                    && reads.iter().any(|r| overlap(d, r))
            });
            if observed {
                if let Ok(i) = i32::try_from(idx) {
                    out.insert((block_name.clone(), i));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use crate::place::{array_elem, overlap, scalar, Index, PlaceKind, LOCAL_NS};

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Collect every def-place and read-place across a proc's CFG.
    fn places_of(src: &str, qname: &str) -> (Vec<Place>, Vec<Place>) {
        let r = registry();
        let cu = CompilationUnit::build_for(src, &r, false);
        let fu = cu.function(qname).expect("proc not found");
        let ctx = build_resolve_context(&fu.cfg, &fu.name);
        let mut defs = Vec::new();
        let mut reads = Vec::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                defs.extend(def_places(stmt, &ctx, &r));
                reads.extend(read_places(stmt, &ctx, &r));
            }
        }
        (defs, reads)
    }

    #[test]
    fn array_element_def_and_read_are_element_granular() {
        // The end-to-end stage-3 point: `set a(k) 1` defs a(k); `puts $a(k)`
        // reads a(k); `set a(j) 2` defs a(j), which NO read observes.  Stage 4
        // will suppress the W220 on a(k) (a read overlaps it) but keep a(j)'s.
        let (defs, reads) = places_of("proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }", "::f");
        let ak = array_elem("a", Index::literal("k"), LOCAL_NS, false);
        let aj = array_elem("a", Index::literal("j"), LOCAL_NS, false);
        assert!(defs.contains(&ak), "expected a(k) def, got {defs:?}");
        assert!(defs.contains(&aj), "expected a(j) def, got {defs:?}");
        // a read observes a(k) …
        assert!(
            reads.iter().any(|r| overlap(r, &ak)),
            "a(k) should be read by `puts $a(k)`, reads={reads:?}",
        );
        // … but nothing observes a(j).
        assert!(
            !reads.iter().any(|r| overlap(r, &aj)),
            "a(j) must not be observed by any read, reads={reads:?}",
        );
    }

    #[test]
    fn scalar_def_and_read() {
        let (defs, reads) = places_of("proc f {} { set x 1; puts $x }", "::f");
        let x = scalar("x", LOCAL_NS, false);
        assert!(defs.contains(&x));
        assert!(reads.iter().any(|r| overlap(r, &x)));
    }

    #[test]
    fn var_read_role_arg_resolves_whole_array() {
        // `array get arr` reads the *whole* array arr → overlaps every element.
        let (_defs, reads) = places_of("proc f {} { set out [array get arr] }", "::f");
        let arr_k = array_elem("arr", Index::literal("k"), LOCAL_NS, false);
        assert!(
            reads.iter().any(|r| overlap(r, &arr_k)),
            "array get arr should read the whole array, reads={reads:?}",
        );
    }

    #[test]
    fn build_context_collects_declarations() {
        let r = registry();
        let cu = CompilationUnit::build_for(
            "proc f {} { global g; variable v; upvar 1 caller y; set x 1 }",
            &r,
            false,
        );
        let fu = cu.function("::f").unwrap();
        let ctx = build_resolve_context(&fu.cfg, &fu.name);
        assert!(ctx.globals.contains("g"), "globals={:?}", ctx.globals);
        assert!(ctx.ns_vars.contains("v"), "ns_vars={:?}", ctx.ns_vars);
        assert!(
            ctx.upvar_aliases.contains_key("y"),
            "upvar_aliases={:?}",
            ctx.upvar_aliases
        );
        // `y` resolves to an upvar alias, not a plain local scalar.
        let p = resolve_place("y", &ctx, false, &r);
        assert_eq!(p.kind, PlaceKind::UpvarAlias);
    }

    #[test]
    fn dynamic_index_read_records_the_index_var() {
        // `set a($i) 1` defs a($i) (dynamic) and reads `i` (to form the index).
        let (defs, reads) = places_of("proc f {} { set i 0; set a($i) 1 }", "::f");
        assert!(
            defs.iter()
                .any(|d| d.kind == PlaceKind::ArrayElem && d.name == "a"),
            "defs={defs:?}",
        );
        assert!(
            reads
                .iter()
                .any(|r| overlap(r, &scalar("i", LOCAL_NS, false))),
            "the dynamic index read of `i` must be recovered, reads={reads:?}",
        );
    }
}
