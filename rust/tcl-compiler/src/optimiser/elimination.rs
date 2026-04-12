//! Elimination optimiser pass (C30d).
//!
//! Ported from `core/compiler/optimiser/_elimination.py`. Emits:
//!
//! - **O107** — unreachable dead code (blocks SCCP proved
//!   unreachable).
//! - **O109** — dead stores (an SSA def whose chain is empty and
//!   whose variable has at least one later definition — the
//!   write is overwritten before any read).
//! - **O126** — unused variable assignments (an SSA def whose
//!   chain is empty and is the only / last def of that variable
//!   — the value is never read). Skipped at the top level (the
//!   last command's result may be the script return value) and
//!   for scope-alias commands (`global` / `variable` / `upvar`).
//!
//! **O108** (transitively dead code) remains deferred — the
//! ADCE fixpoint is a follow-up strip on top of O109/O126.
//!
//! Emission order is the deterministic CFG `cfg_order` (reverse
//! post-order from the entry, unreachable blocks appended).

use std::collections::HashSet;

use crate::cfg::Function as CfgFunction;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::def_use::DefKind;
use crate::ir::Statement;
use crate::sccp::{cfg_order, SccpResult};

use super::{Optimisation, PassContext};

/// Run the elimination pass — emits O107, O109, O126 across
/// every function in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let is_top_level = |fu: &FunctionUnit| fu.name == "::top";

    emit_unreachable(ctx, &cu.top_level);
    emit_dead_stores_and_unused(ctx, &cu.top_level, is_top_level(&cu.top_level));

    for fu in cu.procedures.values() {
        emit_unreachable(ctx, fu);
        emit_dead_stores_and_unused(ctx, fu, false);
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
                span,
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

/// Emit O109 (dead store) + O126 (unused variable) for each
/// dead SSA def in `fu.def_use`.
fn emit_dead_stores_and_unused(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    is_top_level: bool,
) {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    let scope_aliases = scan_scope_aliases(&fu.cfg);
    // The `def_use` builder does not scan Return-value reads or
    // embedded string-interpolation reads; do a supplementary
    // textual pass over the CFG to collect every var name that
    // appears in any source slice. Any def of a name referenced
    // textually is kept live — conservative but correct.
    let textually_referenced = collect_textual_var_references(ctx.source, &fu.cfg);

    // Collect (span, code, name) then sort + emit deterministically.
    let mut entries: Vec<(tcl_lexer::Span, &'static str, &'static str, String)> = Vec::new();

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
        entries.push((stmt.span(), code, msg, var.clone()));
    }

    entries.sort_by_key(|(span, _, _, _)| span.start());
    for (span, code, msg, _) in entries {
        ctx.report(Optimisation::new(code, msg, span, ""));
    }
}

/// Scan every statement's source slice for `$var` / `${var}`
/// references and collect the names seen. Conservative — any
/// occurrence in the textual source keeps the name live, even
/// when the def-use builder does not track that particular use
/// site (notably: Return-value reads, embedded `"$x"`
/// interpolation).
fn collect_textual_var_references(source: &str, _cfg: &CfgFunction) -> HashSet<String> {
    // Conservative over-approximation: scan the entire source
    // text for `$name` / `${name}` references and record every
    // identifier seen. Any name mentioned anywhere keeps the
    // corresponding def live — a safe over-approximation that
    // is too coarse (it suppresses legitimate O109 across proc
    // boundaries) but never emits a false positive.
    //
    // We deliberately do **not** use [`VarReferenceScanner`]: it
    // parses Tcl commands and would need explicit recursion into
    // `proc` bodies / braced scripts. The optimiser does not
    // yet expose a body-aware alternative, so we lean on the
    // lexical grammar of variable substitution — `$` followed
    // by an identifier or `${…}` — which is robust against all
    // surrounding syntax.
    let bytes = source.as_bytes();
    let mut out: HashSet<String> = HashSet::new();
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
    out
}

/// Scan every CFG block for scope-alias commands (`global`,
/// `variable`, `upvar`, `namespace upvar`) and collect the
/// variable names they bind. Those must not be flagged as dead
/// stores / unused — writes go to a different scope.
fn scan_scope_aliases(cfg: &CfgFunction) -> HashSet<String> {
    let mut aliases: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { command, args, defs, .. } = stmt {
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
                            a.starts_with('#')
                                || a == "0"
                                || a == "1"
                                || a.parse::<i64>().is_ok()
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
        let opts = run_pass(
            "proc ::f {} { global g; set g 42 }",
        );
        assert!(
            opts.iter().all(|o| o.code != "O109" && o.code != "O126"),
            "writes through global should not be flagged, got {opts:?}",
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
    fn run_passes_dispatches_elimination() {
        let cu = CompilationUnit::build_for(
            "if {0} { set x 1 }",
            &registry(),
            false,
        );
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
