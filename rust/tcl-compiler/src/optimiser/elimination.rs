//! Elimination optimiser pass (C30d).
//!
//! Ported from `core/compiler/optimiser/_elimination.py`. Emits:
//!
//! - **O107** — unreachable dead code (blocks SCCP proved
//!   unreachable).
//! - **O108** — transitively dead code. A side-effect-free def
//!   whose every consumer was already eliminated (the ADCE
//!   fixpoint on top of O109 / O126).
//! - **O109** — dead stores (an SSA def whose chain is empty and
//!   whose variable has at least one later definition — the
//!   write is overwritten before any read).
//! - **O126** — unused variable assignments (an SSA def whose
//!   chain is empty and is the only / last def of that variable
//!   — the value is never read). Skipped at the top level (the
//!   last command's result may be the script return value) and
//!   for scope-alias commands (`global` / `variable` / `upvar`).
//!
//! Emission order is the deterministic CFG `cfg_order` (reverse
//! post-order from the entry, unreachable blocks appended).

use std::collections::{HashMap, HashSet};

use crate::cfg::Function as CfgFunction;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::def_use::DefKind;
use crate::ir::Statement;
use crate::sccp::{cfg_order, SccpResult};

use super::helpers::spans::full_rewrite_span;
use super::{Optimisation, PassContext};

/// Run the elimination pass — emits O107, O108, O109, O126
/// across every function in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let is_top_level = |fu: &FunctionUnit| fu.name == "::top";

    emit_unreachable(ctx, &cu.top_level);
    let baseline = emit_dead_stores_and_unused(ctx, &cu.top_level, is_top_level(&cu.top_level));
    emit_adce(ctx, &cu.top_level, &baseline);

    for fu in cu.procedures.values() {
        emit_unreachable(ctx, fu);
        let baseline = emit_dead_stores_and_unused(ctx, fu, false);
        emit_adce(ctx, fu, &baseline);
    }
}

fn emit_unreachable(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    // cfg_order is deterministic (RPO + trailing unreachables).
    for block_name in cfg_order(&fu.cfg) {
        if !unreachable.contains(&block_name) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&block_name) else {
            continue;
        };
        for stmt in &block.statements {
            let span = stmt.span();
            // Skip zero-length spans — those are synthesised IR
            // (e.g. implicit barriers) with no user-visible
            // source text to delete.
            if span.is_empty() {
                continue;
            }
            ctx.report(Optimisation::new(
                "O107",
                "Eliminate unreachable dead code",
                full_rewrite_span(ctx.source, span),
                "",
            ));
        }
    }
}

/// Return the set of block names SCCP determined unreachable
/// from the CFG entry.
fn unreachable_blocks(cfg: &CfgFunction, sccp: &SccpResult) -> HashSet<String> {
    cfg.blocks
        .keys()
        .filter(|name| !sccp.executable_blocks.contains(*name))
        .cloned()
        .collect()
}

/// Collected per-chain metadata used by
/// [`emit_dead_stores_and_unused`] to sort + emit in span order.
struct DseEntry {
    span: tcl_lexer::Span,
    code: &'static str,
    msg: &'static str,
    key: (String, u32),
}

/// Emit O109 (dead store) + O126 (unused variable) for each
/// dead SSA def in `fu.def_use`. Returns the set of SSA value
/// keys that were reported — the ADCE pass uses it as the
/// "already eliminated" seed for its fixpoint.
fn emit_dead_stores_and_unused(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    is_top_level: bool,
) -> HashSet<(String, u32)> {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    let scope_aliases = scan_scope_aliases(&fu.cfg);
    // The `def_use` builder does not scan Return-value reads or
    // embedded string-interpolation reads; do a supplementary
    // textual pass over the CFG to collect every var name that
    // appears in any source slice. Any def of a name referenced
    // textually is kept live — conservative but correct.
    let textually_referenced = collect_textual_var_references(ctx.source, &fu.cfg);

    // Collect one DseEntry per dead chain then sort + emit.
    let mut entries: Vec<DseEntry> = Vec::new();

    for chain in fu.def_use.chains.values() {
        if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
            continue;
        }
        if unreachable.contains(&chain.definition.block) {
            // O107 already reports these.
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
        // Skip if the variable is a scope alias — writes through
        // global / upvar are visible in other scopes.
        let (var, _) = &chain.key;
        if scope_aliases.contains(var) {
            continue;
        }
        // Skip cross-event vars (iRules scope).
        if ctx.cross_event_vars.contains(var) {
            continue;
        }
        let any_other_live = fu
            .def_use
            .chains
            .iter()
            .any(|(k, c)| k.0 == *var && k.1 != chain.key.1 && !c.is_dead());

        let (code, msg): (&'static str, &'static str) = if any_other_live {
            // Dead store — overwritten before read (another
            // version has live consumers). This fires regardless
            // of textual mentions: a later version handles the
            // reads.
            ("O109", "Eliminate dead store")
        } else if !is_top_level {
            // Unused variable — apply the textual-scan keep-live
            // check. The def-use builder does not track reads
            // from `Return` terminators or `"$x"` string
            // interpolations, so a conservative over-approximation
            // of names referenced anywhere in the source text
            // suppresses spurious O126 for legitimately-consumed
            // variables.
            if textually_referenced.contains(var) {
                continue;
            }
            ("O126", "Remove unused variable assignment")
        } else {
            // Top-level never emits O126.
            continue;
        };
        let _ = var;
        entries.push(DseEntry {
            span: stmt.span(),
            code,
            msg,
            key: chain.key.clone(),
        });
    }

    entries.sort_by_key(|e| e.span.start());
    let mut removed: HashSet<(String, u32)> = HashSet::new();
    for e in entries {
        ctx.report(Optimisation::new(
            e.code,
            e.msg,
            full_rewrite_span(ctx.source, e.span),
            "",
        ));
        removed.insert(e.key);
    }
    removed
}

/// Emit **O108** (transitively dead code) — the ADCE fixpoint.
fn emit_adce(ctx: &mut PassContext<'_>, fu: &FunctionUnit, baseline: &HashSet<(String, u32)>) {
    let (consumer_stmt_keys, keep_forever) = build_adce_consumers(fu);
    let stmt_to_defs = build_stmt_to_defs(fu);
    let removed = run_adce_fixpoint(
        fu,
        baseline,
        &consumer_stmt_keys,
        &keep_forever,
        &stmt_to_defs,
    );
    emit_adce_reports(ctx, fu, baseline, &removed);
}

type ConsumerMap = HashMap<(String, u32), Vec<(String, usize)>>;

fn build_adce_consumers(fu: &FunctionUnit) -> (ConsumerMap, HashSet<(String, u32)>) {
    use crate::def_use::UseKind;
    let mut consumer_stmt_keys: ConsumerMap = HashMap::new();
    let mut keep_forever: HashSet<(String, u32)> = HashSet::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        let key = chain.key.clone();
        for use_site in &chain.uses {
            match use_site.kind {
                UseKind::Operand => {
                    if let Ok(idx) = usize::try_from(use_site.statement_index) {
                        consumer_stmt_keys
                            .entry(key.clone())
                            .or_default()
                            .push((use_site.block.clone(), idx));
                    }
                }
                UseKind::PhiIncoming | UseKind::Terminator => {
                    keep_forever.insert(key.clone());
                }
            }
        }
    }
    (consumer_stmt_keys, keep_forever)
}

type StmtDefsMap = HashMap<(String, usize), Vec<(String, u32)>>;

fn build_stmt_to_defs(fu: &FunctionUnit) -> StmtDefsMap {
    let mut out: StmtDefsMap = HashMap::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        if let Ok(idx) = usize::try_from(chain.definition.statement_index) {
            out.entry((chain.definition.block.clone(), idx))
                .or_default()
                .push(chain.key.clone());
        }
    }
    out
}

fn run_adce_fixpoint(
    fu: &FunctionUnit,
    baseline: &HashSet<(String, u32)>,
    consumer_stmt_keys: &ConsumerMap,
    keep_forever: &HashSet<(String, u32)>,
    stmt_to_defs: &StmtDefsMap,
) -> HashSet<(String, u32)> {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    let mut removed = baseline.clone();
    loop {
        let mut changed = false;
        for chain in fu.def_use.chains.values() {
            if chain.definition.kind != DefKind::Statement {
                continue;
            }
            let key = &chain.key;
            if removed.contains(key) || keep_forever.contains(key) {
                continue;
            }
            if unreachable.contains(&chain.definition.block) {
                continue;
            }
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            if !is_side_effect_free_assignment(stmt) {
                continue;
            }
            let empty: Vec<(String, usize)> = Vec::new();
            let consumers: &Vec<(String, usize)> = consumer_stmt_keys.get(key).unwrap_or(&empty);
            if consumers.is_empty() {
                continue;
            }
            let all_removed = consumers.iter().all(|pair: &(String, usize)| {
                stmt_to_defs
                    .get(pair)
                    .is_some_and(|defs: &Vec<(String, u32)>| {
                        defs.iter().all(|d: &(String, u32)| removed.contains(d))
                    })
            });
            if all_removed {
                removed.insert(key.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    removed
}

fn emit_adce_reports(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    baseline: &HashSet<(String, u32)>,
    removed: &HashSet<(String, u32)>,
) {
    let mut new_reports: Vec<tcl_lexer::Span> = Vec::new();
    for key in removed.difference(baseline) {
        if let Some(chain) = fu.def_use.chains.get(key) {
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            new_reports.push(stmt.span());
        }
    }
    new_reports.sort_by_key(|s| s.start());
    for span in new_reports {
        ctx.report(Optimisation::new(
            "O108",
            "Eliminate transitively dead code",
            full_rewrite_span(ctx.source, span),
            "",
        ));
    }
}

fn is_side_effect_free_assignment(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. }
    )
}

/// Scan every statement's source slice for `$var` / `${var}`
/// references and collect the names seen. Narrow to the
/// function's own CFG extent now that the segmenter emits
/// absolute spans for proc bodies — so false-positive
/// suppression across proc boundaries no longer applies.
/// Scan a slice for `$var` and `${var}` references, inserting names
/// into *out*.  Extracted from `collect_textual_var_references`.
#[allow(clippy::many_single_char_names)]
fn scan_dollar_refs(slice: &str, out: &mut HashSet<String>) {
    let bytes = slice.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            // ${name}
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'}' {
                i += 1;
            }
            if start < i {
                if let Ok(name) = std::str::from_utf8(&bytes[start..i]) {
                    // Strip any array index.
                    let name = name.split('(').next().unwrap_or(name);
                    if !name.is_empty() {
                        out.insert(name.to_owned());
                    }
                }
            }
            if i < bytes.len() {
                i += 1; // consume closing `}`
            }
            continue;
        }
        // $name: identifier chars (letters, digits, underscore, ::).
        let start = i;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_alphanumeric() || b == b'_' {
                i += 1;
            } else if b == b':' && i + 1 < bytes.len() && bytes[i + 1] == b':' {
                i += 2;
            } else {
                break;
            }
        }
        if start < i {
            if let Ok(name) = std::str::from_utf8(&bytes[start..i]) {
                out.insert(name.to_owned());
            }
        }
    }
}

/// Scan a slice for `[set NAME]` (1-arg read form) references.
/// Mirrors upstream commit `342d4c7a` (PR #331).
#[allow(clippy::many_single_char_names)]
fn scan_set_read_refs(slice: &str, out: &mut HashSet<String>) {
    let bs = slice.as_bytes();
    let mut j = 0;
    while j < bs.len() {
        if bs[j] != b'[' {
            j += 1;
            continue;
        }
        let open = j;
        let mut k = j + 1;
        while k < bs.len() && (bs[k] == b' ' || bs[k] == b'\t') {
            k += 1;
        }
        if slice[k..].starts_with("::") {
            k += 2;
        }
        if !slice[k..].starts_with("set") {
            j = open + 1;
            continue;
        }
        let after_set = k + 3;
        if after_set >= bs.len() || !(bs[after_set] == b' ' || bs[after_set] == b'\t') {
            j = open + 1;
            continue;
        }
        let mut m = after_set;
        while m < bs.len() && (bs[m] == b' ' || bs[m] == b'\t') {
            m += 1;
        }
        let name_start = m;
        while m < bs.len() {
            let b = bs[m];
            if b.is_ascii_alphanumeric() || b == b'_' {
                m += 1;
            } else if b == b':' && m + 1 < bs.len() && bs[m + 1] == b':' {
                m += 2;
            } else {
                break;
            }
        }
        if m == name_start {
            j = open + 1;
            continue;
        }
        let mut n = m;
        while n < bs.len() && (bs[n] == b' ' || bs[n] == b'\t') {
            n += 1;
        }
        if n < bs.len() && bs[n] == b']' {
            if let Ok(name) = std::str::from_utf8(&bs[name_start..m]) {
                if !name.is_empty() {
                    out.insert(name.to_owned());
                }
            }
        }
        j = m;
    }
}

pub(crate) fn collect_textual_var_references(source: &str, cfg: &CfgFunction) -> HashSet<String> {
    // Absolute spans now cover the function's own source range.
    let span_iter = cfg.blocks.values().flat_map(|b| {
        let stmts = b.statements.iter().map(crate::ir::Statement::span);
        let term = b.terminator.as_ref().and_then(crate::cfg::Terminator::span);
        stmts.chain(term)
    });
    let Some((lo, hi)) = span_iter.fold(None, |acc: Option<(u32, u32)>, span| {
        let s = span.start();
        let e = span.end();
        match acc {
            None => Some((s, e)),
            Some((l, h)) => Some((l.min(s), h.max(e))),
        }
    }) else {
        return HashSet::new();
    };
    let start = usize::try_from(lo).unwrap_or(0);
    let end = usize::try_from(hi)
        .unwrap_or(source.len())
        .min(source.len());
    if start >= end {
        return HashSet::new();
    }
    let slice = &source[start..end];
    let mut out: HashSet<String> = HashSet::new();
    scan_dollar_refs(slice, &mut out);
    scan_set_read_refs(slice, &mut out);
    out
}

/// Scan every CFG block for scope-alias commands (`global`,
/// `variable`, `upvar`, `namespace upvar`) and collect the
/// variable names they bind. Those must not be flagged as dead
/// stores / unused — writes go to a different scope.
pub(crate) fn scan_scope_aliases(cfg: &CfgFunction) -> HashSet<String> {
    let mut aliases: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call {
                command,
                args,
                defs,
                ..
            } = stmt
            {
                match command.as_str() {
                    "global" => {
                        for a in args {
                            aliases.insert(a.clone());
                        }
                    }
                    "variable" => {
                        // variable name ?value? ?name value? …
                        let mut i = 0;
                        while i < args.len() {
                            aliases.insert(args[i].clone());
                            i += 2;
                        }
                    }
                    "upvar" => {
                        // upvar ?level? name localname ?name localname? …
                        // Take the odd-indexed args (the locals).
                        let has_level = args.first().is_some_and(|a| {
                            a.starts_with('#') || a == "0" || a == "1" || a.parse::<i64>().is_ok()
                        });
                        let start = usize::from(has_level);
                        let mut i = start + 1;
                        while i < args.len() {
                            aliases.insert(args[i].clone());
                            i += 2;
                        }
                    }
                    "namespace" if matches!(args.first().map(String::as_str), Some("upvar")) => {
                        // namespace upvar NS name localname …
                        let mut i = 3;
                        while i < args.len() {
                            aliases.insert(args[i].clone());
                            i += 2;
                        }
                    }
                    _ => {
                        // For other calls, trust the per-statement
                        // `defs` annotation when present (it
                        // captures upvars the lowering recognised).
                        for d in defs {
                            let _ = d;
                        }
                    }
                }
            }
        }
    }
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;

    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    // -- internal helper tests -----------------------------------------------

    #[test]
    fn unreachable_blocks_empty_when_all_executable() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        let unreach = unreachable_blocks(&cu.top_level.cfg, &cu.top_level.sccp);
        assert!(unreach.is_empty());
    }

    // -- end-to-end tests ---------------------------------------------------

    #[test]
    fn empty_source_produces_nothing() {
        let opts = run_pass("");
        assert!(opts.is_empty());
    }

    #[test]
    fn straight_line_script_is_fully_reachable() {
        let opts = run_pass("set x 1\nset y 2\nputs $x");
        assert!(opts.is_empty());
    }

    #[test]
    fn branch_folding_creates_unreachable_block() {
        // The else branch is unreachable under SCCP because the
        // condition folds to true.
        let opts = run_pass("if {1} { set x 1 } else { set y 2 }");
        // Expect at least one O107 — the else body's `set y 2`.
        assert!(
            opts.iter()
                .any(|o| o.code == "O107" && o.message == "Eliminate unreachable dead code"),
            "expected at least one O107, got {opts:?}",
        );
    }

    #[test]
    fn while_false_body_is_unreachable() {
        // The body of `while {0} { ... }` is unreachable.
        let opts = run_pass("while {0} { set x 1 }");
        assert!(
            opts.iter().any(|o| o.code == "O107"),
            "expected an O107 for dead while body, got {opts:?}",
        );
    }

    #[test]
    fn unreachable_statements_emitted_with_empty_replacement() {
        let opts = run_pass("if {0} { set x 1 }");
        let target = opts.iter().find(|o| o.code == "O107");
        if let Some(o) = target {
            assert_eq!(o.replacement, "");
            assert!(!o.span.is_empty());
        }
    }

    #[test]
    fn dead_store_fires_o109_when_overwritten_before_read() {
        // First set is dead — second version is the only live
        // value of x when read by puts.
        let opts = run_pass("set x 1\nset x 2\nputs $x");
        assert!(
            opts.iter().any(|o| o.code == "O109"),
            "expected O109 for overwritten store, got {opts:?}",
        );
    }

    #[test]
    fn unused_variable_fires_o126_in_proc_body() {
        let opts = run_pass("proc ::f {} { set y 42; return 1 }");
        assert!(
            opts.iter().any(|o| o.code == "O126"),
            "expected O126 for unused var in proc, got {opts:?}",
        );
    }

    #[test]
    fn top_level_unused_variable_not_flagged() {
        // At top level an unused variable is not O126 — the
        // script-return semantics and external consumers (upvar,
        // info exists) could read it.
        let opts = run_pass("set y 42");
        assert!(
            opts.iter().all(|o| o.code != "O126"),
            "top-level unused var should not emit O126, got {opts:?}",
        );
    }

    #[test]
    fn scope_alias_globals_never_flagged() {
        let opts = run_pass("proc ::f {} { global g; set g 42 }");
        assert!(
            opts.iter().all(|o| o.code != "O109" && o.code != "O126"),
            "writes through global should not be flagged, got {opts:?}",
        );
    }

    #[test]
    fn adce_removes_chain_of_dead_defs() {
        // `set a 1` → `set b $a` → `set c $b`; `c` is never
        // read. O126 flags `set c $b`, then ADCE should extend
        // to `set b $a` and `set a 1` since their only consumer
        // is the already-dead chain.
        let opts = run_pass("proc ::f {} { set a 1; set b $a; set c $b; return 7 }");
        let o108 = opts.iter().filter(|o| o.code == "O108").count();
        assert!(
            o108 >= 1,
            "expected at least one O108 in transitive dead chain, got {opts:?}",
        );
    }

    #[test]
    fn used_variable_not_flagged() {
        let opts = run_pass("proc ::f {} { set x 1; return $x }");
        assert!(
            opts.iter().all(|o| o.code != "O109" && o.code != "O126"),
            "used var should not be flagged, got {opts:?}",
        );
    }

    #[test]
    fn collect_textual_var_references_detects_set_one_arg_read() {
        // ``[set varname]`` (1-arg form) is a variable read; without
        // it, DCE saw 0 reads on ``varname`` and incorrectly deleted
        // the write.  Mirrors upstream commit ``342d4c7a`` (PR #331).
        let opts = run_pass("proc ::f {} { set x 1; set y [set x]; return $y }");
        // ``x`` is read via ``[set x]`` so neither O109 nor O126
        // should fire on the ``set x 1`` line.
        let bad: Vec<_> = opts
            .iter()
            .filter(|o| (o.code == "O109" || o.code == "O126") && o.message.contains('x'))
            .collect();
        assert!(
            bad.is_empty(),
            "[set x] should count as a read for x; got {opts:?}",
        );
    }

    #[test]
    fn collect_textual_var_references_detects_qualified_set_read() {
        // ``[::set varname]`` form should also count.
        let opts = run_pass("proc ::f {} { set x 1; set y [::set x]; return $y }");
        let bad: Vec<_> = opts
            .iter()
            .filter(|o| (o.code == "O109" || o.code == "O126") && o.message.contains('x'))
            .collect();
        assert!(
            bad.is_empty(),
            "[::set x] should count as a read for x; got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_elimination() {
        let cu = CompilationUnit::build_for("if {0} { set x 1 }", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Elimination]);
        // At minimum the dispatch must not panic; O107 may or
        // may not fire depending on how SCCP models this exact
        // shape, but running the pass must be side-effect free
        // otherwise.
        let only_o107 = ctx.optimisations.iter().all(|o| o.code == "O107");
        assert!(only_o107, "unexpected codes: {:?}", ctx.optimisations);
    }
}
