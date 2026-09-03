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

//! Bridge from the IR/CFG to the unified [`Place`] model.
//!
//! Resolves each IR statement's *def* and *read* targets to canonical
//! [`Place`]s on demand, so the dataflow consumers (dead-store / unused /
//! read-before-set) can switch from name-string equality to the sound
//! [`overlap`] relation — without re-stamping every
//! construction site in lowering.  The per-function [`ResolveContext`] is built
//! once by scanning the CFG for `global` / `variable` / `upvar` / `trace`
//! declarations (reusing the [`crate::var_scoping`] grammar).
//!
//! No consumer is wired yet.

use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::cfg::{Function, Terminator};
use crate::ir::{CommandTokens, Statement};
use crate::naming::{normalise_qualified_name, normalise_var_name};
use crate::place::{self, Place, PlaceKind, overlap, places_read_to_form};
use crate::segmenter::segment_commands_with_offset_and_config;
use crate::ssa::structural_body_indices;
use crate::var_refs::{
    command_subst_texts_with_config, scan_var_ref_forms_with_config, vars_in_expr,
};
use crate::var_resolve::{ResolveContext, resolve_place};
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};

/// True when `name`'s `VarRead`-role argument names the *whole* array,
/// not a scalar (so a write to any element is observed) — the registry's
/// [`Traits::WHOLE_ARRAY_ARG`] (`array` / `parray`). `registry.get`
/// resolves the bare and `::`-qualified forms alike.
fn is_whole_array_command(registry: &CommandRegistry, name: &str) -> bool {
    registry
        .get(name)
        .is_some_and(|s| s.traits.contains(Traits::WHOLE_ARRAY_ARG))
}

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
    // Tolerate a fully-qualified head (`::upvar`, `::namespace upvar`): the
    // shared `upvar_local_declaration_indices` grammar matches the bare forms,
    // so a `::`-qualified call would otherwise leave `upvar_aliases` incomplete.
    let command = command.strip_prefix("::").unwrap_or(command);
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

/// True when arg `arg_index` of a command is a **braced literal** word
/// (`{…}`): Tcl performs no `$` / `[` substitution on it, so its de-braced IR
/// text must NOT be scanned for reads — `puts {$a(k)}` does *not* read `a(k)`.
///
/// Thin wrapper over the shared
/// [`CommandTokens::arg_is_braced_literal`] for the `Option<&CommandTokens>`
/// this module carries.
fn is_braced_literal(tokens: Option<&CommandTokens>, arg_index: usize) -> bool {
    tokens.is_some_and(|t| t.arg_is_braced_literal(arg_index))
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
    // The registry carries the environment's profile, so the embedded script
    // is segmented under the document's own grammar.
    for cmd in segment_commands_with_offset_and_config(
        script_text,
        0,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    ) {
        let name = cmd.name();
        if name.is_empty() {
            continue;
        }
        let args = cmd.args();
        let whole = is_whole_array_command(registry, name);
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
    // The registry carries the environment's profile, so the `$`/`[…]`
    // boundaries this word is read at are the document's own.
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    if text.contains('$') {
        for r in scan_var_ref_forms_with_config(text, config) {
            out.push(resolve_place(&r, ctx, false, registry));
        }
    }
    for inner in command_subst_texts_with_config(text, config) {
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
    let grammar = registry
        .profile()
        .map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar);

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
            for nm in vars_in_expr(expr, grammar) {
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
                for nm in vars_in_expr(e, grammar) {
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
                .is_some_and(|c| is_whole_array_command(registry, c));
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            let read_idx: std::collections::HashSet<usize> = registry
                .arg_indices_for_role(command, &arg_strs, ArgRole::VarRead)
                .into_iter()
                .collect();
            // A structural body (proc/method/test/snit) runs in its own scope
            // and is analysed separately — its reads must not leak into this
            // scope.
            let lookup = canonical_command.as_deref().unwrap_or(command);
            let structural = structural_body_indices(lookup, args, tokens.as_ref(), registry);
            // Body-role args (incl. non-structural ones like an iRules
            // `clientside {…}` script) DO run, so their `$`-refs are real reads
            // and must still be scanned even when braced.
            let body_idx: std::collections::HashSet<usize> = registry
                .arg_indices_for_role(command, &arg_strs, ArgRole::Body)
                .into_iter()
                .collect();
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
                    // A braced literal *data* word performs no substitution, so
                    // its de-braced IR text is not a read site (`puts {$a(k)}`
                    // must not look like a read of `a(k)`, or a real dead store
                    // to `a(k)` would be wrongly suppressed).
                    if body_idx.contains(&i) || !is_braced_literal(tokens.as_ref(), i) {
                        word_reads(arg, ctx, &mut out, registry);
                    }
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
    let grammar = registry
        .profile()
        .map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar);
    match term {
        Terminator::Branch { condition, .. } => {
            for nm in vars_in_expr(condition, grammar) {
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
                    for nm in vars_in_expr(e, grammar) {
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

/// True when two places denote the **exact same** literal-key array element or
/// dict path (same kind, namespace, base name, and statically-known index /
/// key path).  Dynamic indices never compare equal — they keep the
/// conservative behaviour.
fn same_literal_element(a: &Place, b: &Place) -> bool {
    use crate::place::IndexKind;
    if a.kind != b.kind || a.ns != b.ns || a.name != b.name {
        return false;
    }
    match a.kind {
        PlaceKind::ArrayElem => matches!(
            (&a.index, &b.index),
            (Some(ai), Some(bi))
                if ai.kind == IndexKind::Literal
                    && bi.kind == IndexKind::Literal
                    && ai.value == bi.value
        ),
        PlaceKind::DictPath => {
            !a.keys.is_empty()
                && a.keys.len() == b.keys.len()
                && a.keys.iter().zip(&b.keys).all(|(ai, bi)| {
                    ai.kind == IndexKind::Literal
                        && bi.kind == IndexKind::Literal
                        && ai.value == bi.value
                })
        }
        _ => false,
    }
}

/// True when a literal-key `ArrayElem` / `DictPath` write at
/// `block.statements[def_idx]` is overwritten by a *later* write in the same
/// block to the EXACT same place, with no intervening read of that specific
/// element.  Such a store is dead regardless of any later-version read of the
/// element — the must-alias kill overrides the over-approximating
/// element-observed suppression.
fn must_alias_killed_in_block(
    block: &crate::cfg::Block,
    def_idx: usize,
    def_place: &Place,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> bool {
    use crate::place::IndexKind;
    let def_is_literal = match def_place.kind {
        PlaceKind::ArrayElem => def_place
            .index
            .as_ref()
            .is_some_and(|i| i.kind == IndexKind::Literal),
        PlaceKind::DictPath => {
            !def_place.keys.is_empty()
                && def_place.keys.iter().all(|i| i.kind == IndexKind::Literal)
        }
        _ => false,
    };
    if !def_is_literal {
        return false;
    }
    for stmt in block.statements.iter().skip(def_idx + 1) {
        // An intervening read of the same element cancels the kill.
        if read_places(stmt, ctx, registry)
            .iter()
            .any(|rp| same_literal_element(rp, def_place))
        {
            return false;
        }
        // A later write to the same element is a must-alias kill.
        if def_places(stmt, ctx, registry)
            .iter()
            .any(|dp| same_literal_element(dp, def_place))
        {
            return true;
        }
    }
    false
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
        // `incr a(k)` also defines an array element and folds to the base `a`
        // under name-level SSA, so it participates in the same FP — and
        // `def_places` already resolves it.  (The analyser's W220 never flags
        // an `Incr`, but the optimiser's O109 reaches it, so include it.)
        matches!(
            stmt,
            Statement::AssignConst { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::Incr { name, .. }
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

    for block in cfg.blocks.values() {
        for (idx, stmt) in block.statements.iter().enumerate() {
            if !is_array_assign(stmt) {
                continue;
            }
            // An element write is suppressed only when a read *observes* it AND
            // it is not overwritten by a later same-element write first: a
            // must-alias kill (a later write to the exact same literal key with
            // no intervening read of it) makes this store dead regardless of any
            // later-version read.
            let suppress = def_places(stmt, &ctx, registry).iter().any(|d| {
                matches!(d.kind, PlaceKind::ArrayElem | PlaceKind::DictPath)
                    && reads.iter().any(|r| overlap(d, r))
                    && !must_alias_killed_in_block(block, idx, d, &ctx, registry)
            });
            if suppress && let Ok(i) = i32::try_from(idx) {
                out.insert((block.name.clone(), i));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use crate::place::{Index, LOCAL_NS, PlaceKind, array_elem, overlap, scalar};

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
        // Element-granular defs/reads: `set a(k) 1` defs a(k); `puts $a(k)`
        // reads a(k); `set a(j) 2` defs a(j), which NO read observes.  Dead-store
        // analysis suppresses the W220 on a(k) (a read overlaps it) but keeps a(j)'s.
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
    fn build_context_tolerates_qualified_upvar_head() {
        // A fully-qualified `::upvar` head must still register the alias —
        // `collect_upvar_aliases` strips the leading `::` so the bare-form
        // grammar matches (otherwise `upvar_aliases` would be incomplete).
        let r = registry();
        let cu = CompilationUnit::build_for("proc f {} { ::upvar 1 caller y; set x 1 }", &r, false);
        let fu = cu.function("::f").unwrap();
        let ctx = build_resolve_context(&fu.cfg, &fu.name);
        assert!(
            ctx.upvar_aliases.contains_key("y"),
            "::upvar alias should be registered; upvar_aliases={:?}",
            ctx.upvar_aliases
        );
        assert_eq!(
            resolve_place("y", &ctx, false, &r).kind,
            PlaceKind::UpvarAlias
        );
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
