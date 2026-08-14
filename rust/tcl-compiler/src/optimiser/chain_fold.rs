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

//! O104 / O130 — write-only build-chain folding.
//!
//! Collapses a run of consecutive static writes to one variable into a
//! single `set`:
//!
//! ```tcl
//! set s ""        ;# →  set s "foobar"
//! append s foo
//! append s bar
//! ```
//! ```tcl
//! set l {}        ;# →  set l {a b c}
//! lappend l a
//! lappend l b c
//! ```
//!
//! `O104` folds string-concat (`append`) chains; `O130` folds list
//! (`lappend`) chains. The fold emits a `set` rewrite over the *last*
//! write and a paired deletion over each earlier one, all sharing one
//! group so they apply atomically.
//!
//! ## Soundness gates
//!
//! - The writes must be **strictly consecutive** — no statement runs
//!   between them, so no intermediate value can be observed (the
//!   `var_observability` flow-sensitive read check is
//!   subsumed: a read between writes would be a non-write statement and
//!   ends the run).
//! - Every value word must be a static literal (`Esc`/`Str` single-token
//!   word); a `$var` / `[cmd]` operand ends the run.
//! - The variable must not **escape** (be aliased via
//!   `global`/`upvar`/`variable` or be under a `trace`) and must not be a
//!   cross-event iRules state variable — folding would drop a trace
//!   callback or a value a later scope / event observes.
//!
//! These gates make the fold conservative (it can miss a chain a
//! flow-sensitive pass would fold) but never unsound.

use std::collections::HashSet;
use tcl_core_types::DiagCode;

use tcl_lexer::TokenType;
use tcl_registry::CommandRegistry;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{Script, Statement};
use crate::naming::normalise_var_name;
use crate::var_observability::analyse_var_observability;

use super::helpers::literals::render_static_string_word;
use super::helpers::spans::{full_rewrite_span, statement_delete_rewrite_range};
use super::{Optimisation, PassContext};

/// Run the chain-fold pass over the whole compilation unit.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    // A dynamic variable-trace target (`trace add variable $n …`) means
    // *every* name is potentially traced, so no intermediate write is
    // provably unobserved anywhere in the module (issue #1377).
    if cu.ir_module.has_dynamic_variable_trace {
        return;
    }
    let mut cross = ctx.cross_event_vars.clone();
    // The whole-module trace fact stores the canonical (`::`-stripped)
    // spelling, so it also protects a chain whose target is spelled
    // unqualified while the trace names `::var` (issue #1377) — the same
    // fact SCCP and O102 already consult.
    cross.extend(cu.ir_module.traced_variables.iter().cloned());
    // `ctx.registry` is always set by the `optimise*` entry points; a bare
    // hand-built `PassContext` (some pass-level unit tests) leaves it
    // `None`, so fall back to a default-dialect registry rather than
    // panic — `trace`'s `ESTABLISHES_VARIABLE_TRACE` grammar is core Tcl,
    // present in every dialect's registry.
    let registry: &CommandRegistry = ctx
        .registry
        .unwrap_or_else(|| tcl_registry::cache::default_registry());
    if !cu.top_level.dynamic_barrier_blocks_value_motion() {
        let top_protected = protected_vars(&cu.top_level, &cross, registry);
        fold_script(ctx, &cu.ir_module.top_level, &top_protected, 0);
    }
    for (qname, proc) in &cu.ir_module.procedures {
        let fu = cu.procedures.get(qname);
        // A computed variable name (`set $name …`) can write the accumulator
        // mid-chain under a spelling `classify_write` cannot see, so the
        // whole function abstains (issue #1374).
        if fu.is_some_and(FunctionUnit::dynamic_barrier_blocks_value_motion) {
            continue;
        }
        let protected = fu.map_or_else(|| cross.clone(), |fu| protected_vars(fu, &cross, registry));
        fold_script(ctx, &proc.body, &protected, 0);
    }
}

/// Variables that must never have a write-chain folded: those that escape
/// the frame (aliased / traced) plus the iRules cross-event state set and
/// the whole-module traced names the caller folded into `cross_event`.
fn protected_vars(
    fu: &FunctionUnit,
    cross_event: &HashSet<String>,
    registry: &CommandRegistry,
) -> HashSet<String> {
    let mut set = analyse_var_observability(&fu.cfg, registry).escaping_var_names();
    set.extend(cross_event.iter().cloned());
    set
}

/// Fold chains in `script`, then recurse into control-flow bodies (a
/// chain never crosses a control-flow boundary, so each body is folded
/// independently). `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn fold_script(
    ctx: &mut PassContext<'_>,
    script: &Script,
    protected: &HashSet<String>,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    let stmts = &script.statements;
    let mut i = 0;
    while i < stmts.len() {
        if let Some(consumed) = try_fold_chain_at(ctx, stmts, i, protected) {
            i += consumed;
        } else {
            i += 1;
        }
    }
    for stmt in stmts {
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    fold_script(ctx, &c.body, protected, depth + 1);
                }
                if let Some(b) = else_body {
                    fold_script(ctx, b, protected, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                fold_script(ctx, init, protected, depth + 1);
                fold_script(ctx, next, protected, depth + 1);
                fold_script(ctx, body, protected, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => fold_script(ctx, body, protected, depth + 1),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                fold_script(ctx, body, protected, depth + 1);
                for h in handlers {
                    fold_script(ctx, &h.body, protected, depth + 1);
                }
                if let Some(fb) = finally_body {
                    fold_script(ctx, fb, protected, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        fold_script(ctx, b, protected, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    fold_script(ctx, b, protected, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// A single static write to a variable.
enum Write {
    /// `set var <static>` — establishes the chain's initial value.
    Set { var: String, value: String },
    /// `append var <static>…` — string-concat extension.
    Append {
        var: String,
        word: String,
        pieces: Vec<String>,
    },
    /// `lappend var <static>…` — list extension.
    Lappend {
        var: String,
        word: String,
        elements: Vec<String>,
    },
}

/// The (normalised) target variable name of a classified write.
fn write_var(w: &Write) -> &str {
    match w {
        Write::Set { var, .. } | Write::Append { var, .. } | Write::Lappend { var, .. } => var,
    }
}

/// Classify `stmt` as a static write, or `None` for anything else
/// (dynamic operand, other command, control flow).
fn classify_write(stmt: &Statement) -> Option<Write> {
    match stmt {
        Statement::AssignConst { name, value, .. } => Some(Write::Set {
            var: normalise_var_name(name).to_owned(),
            value: value.clone(),
        }),
        // `set s ""` / `set s foo` lower to `AssignValue`; only a static
        // single-token literal value (no command/var substitution) anchors
        // a foldable chain.
        Statement::AssignValue {
            name,
            value,
            value_needs_backsubst,
            tokens,
            ..
        } => {
            if *value_needs_backsubst {
                return None;
            }
            let tokens = tokens.as_ref()?;
            let kind = tokens.argv_kinds.get(2)?;
            let single = tokens.single_token_word.get(2).copied()?;
            if !single || !matches!(kind, TokenType::Esc | TokenType::Str) {
                return None;
            }
            Some(Write::Set {
                var: normalise_var_name(name).to_owned(),
                value: value.clone(),
            })
        }
        // No membership guard here: the fold's per-command semantics below
        // (`set` resets the chain, `append` extends the string, `lappend`
        // extends the list) ARE the dispatch — any other command falls out
        // of the final match.
        Statement::Call {
            command,
            args,
            tokens,
            ..
        } => {
            let tokens = tokens.as_ref()?;
            // Value words are argv index `vararg + 1 ..` (argv[0] is the
            // command, argv[1] is the variable).
            let var_word = args.first()?.clone();
            let var = normalise_var_name(&var_word).to_owned();
            let value_words = &args[1..];
            let mut values = Vec::with_capacity(value_words.len());
            for (j, val) in value_words.iter().enumerate() {
                let argv_idx = j + 2; // skip command + variable
                let kind = tokens.argv_kinds.get(argv_idx)?;
                let single = tokens.single_token_word.get(argv_idx).copied()?;
                if !single || !matches!(kind, TokenType::Esc | TokenType::Str) {
                    return None;
                }
                values.push(val.clone());
            }
            match command.as_str() {
                "set" if values.len() == 1 => Some(Write::Set {
                    var,
                    value: values.into_iter().next().unwrap(),
                }),
                "append" if !values.is_empty() => Some(Write::Append {
                    var,
                    word: var_word,
                    pieces: values,
                }),
                "lappend" if !values.is_empty() => Some(Write::Lappend {
                    var,
                    word: var_word,
                    elements: values,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Attempt to fold a write-chain starting at `stmts[start]`. Returns the
/// number of statements consumed (the run length) when a fold fires, else
/// `None`.
fn try_fold_chain_at(
    ctx: &mut PassContext<'_>,
    stmts: &[Statement],
    start: usize,
    protected: &HashSet<String>,
) -> Option<usize> {
    let Write::Set { var, value } = classify_write(&stmts[start])? else {
        return None;
    };

    let mut chain_value = value;
    let mut elements: Option<Vec<String>> = None;
    let mut writes = vec![start];
    let mut last_word: Option<String> = None;

    let mut j = start + 1;
    while j < stmts.len() {
        match classify_write(&stmts[j]) {
            Some(Write::Append {
                var: v,
                word,
                pieces,
            }) if v == var && elements.is_none() => {
                for p in pieces {
                    chain_value.push_str(&p);
                }
                last_word = Some(word);
                writes.push(j);
                j += 1;
            }
            Some(Write::Lappend {
                var: v,
                word,
                elements: els,
            }) if v == var => {
                if elements.is_none() {
                    // First lappend after the set — reinterpret the current
                    // string value as a list (bail if it is not one).
                    let Ok(base) = tcl_syntax::list::split_list(&chain_value) else {
                        break;
                    };
                    elements = Some(base.into_iter().map(std::borrow::Cow::into_owned).collect());
                }
                if let Some(list) = elements.as_mut() {
                    list.extend(els);
                }
                last_word = Some(word);
                writes.push(j);
                j += 1;
            }
            // Precise-flow (O104/O130): a *static-literal* write to a
            // **different** variable cannot read or write the accumulator,
            // has no side effect, and is not a barrier (`classify_write`
            // only matches single-token `Esc`/`Str` value words with no
            // substitution), so the chain continues past it — the
            // interleaved statement stays in place and is not folded.
            Some(other) if write_var(&other) != var => {
                j += 1;
            }
            _ => break,
        }
    }

    // The whole-module traced-variable fact is stored `::`-stripped (see
    // `populate_variable_trace_facts`), so a `::`-qualified chain target is
    // checked under that canonical spelling too.
    if writes.len() < 2
        || protected.contains(&var)
        || protected.contains(var.trim_start_matches("::"))
    {
        return None;
    }
    // `last_word` is always set: a run of ≥2 writes has at least one
    // append/lappend after the anchoring set.
    let var_word = last_word?;

    let (code, fold_msg, dead_msg, rendered) = if let Some(els) = &elements {
        (
            DiagCode::O130,
            "Fold write-only list build chain",
            "Remove dead intermediate list write",
            render_list_word(els),
        )
    } else {
        (
            DiagCode::O104,
            "Fold write-only string build chain",
            "Remove dead intermediate string write",
            render_static_string_word(&chain_value)?,
        )
    };

    let source = ctx.source;
    let group = ctx.alloc_group();

    let last = *writes.last().unwrap();
    let last_span = full_rewrite_span(source, stmts[last].span());
    let mut fold = Optimisation::new(
        code,
        fold_msg,
        last_span,
        format!("set {var_word} {rendered}"),
    );
    fold.group = Some(group);
    ctx.report(fold);

    for &w in &writes[..writes.len() - 1] {
        let full = full_rewrite_span(source, stmts[w].span());
        let next_start = stmts.get(w + 1).map(|s| s.span().start() as usize);
        let del_span = statement_delete_rewrite_range(source, full, next_start);
        let mut del = Optimisation::new(code, dead_msg, del_span, "");
        del.group = Some(group);
        ctx.report(del);
    }

    Some(writes.len())
}

/// Render `elements` as the single `set` value-word that recreates the
/// list — join into a canonical Tcl list, then quote that as one element.
/// The joined string never begins with a bare `#` (the join already quotes
/// a leading `#`), so `list_element`'s first-element rule is equivalent here.
fn render_list_word(elements: &[String]) -> String {
    tcl_syntax::list::list_element(&tcl_syntax::list::join_list(elements))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interprocedural::InterproceduralAnalysis;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// Apply every grouped O104/O130 rewrite to `source` (reverse offset
    /// order so earlier edits don't shift later spans) and return the
    /// rewritten text.
    fn apply(source: &str) -> String {
        let mut opts: Vec<Optimisation> = run_pass(source)
            .into_iter()
            .filter(|o| o.code == DiagCode::O104 || o.code == DiagCode::O130)
            .collect();
        opts.sort_by_key(|o| std::cmp::Reverse(o.span.start()));
        let mut out = source.to_owned();
        for o in opts {
            out.replace_range(
                o.span.start() as usize..o.span.end() as usize,
                &o.replacement,
            );
        }
        out
    }

    /// Regression coverage for issue #996: `fold_script` recurses once per
    /// nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch` body,
    /// with no depth cap of its own before this fix. Transitively bounded
    /// to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today, so this
    /// is defence-in-depth / consistency with every other full-tree walker
    /// in this crate, not a currently-reproducible crash. 1000 levels of
    /// source nesting is comfortably past this new cap; the assertion is
    /// that `run_pass` returns at all, not what it returns. Spawns its own
    /// big-stack thread since the lexer/CST/segmenter stages upstream of
    /// the lowering cap still walk the full un-truncated source nesting
    /// before that cap trims it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_fold_script() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set s \"\"\nappend s foo\nappend s bar\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = run_pass(&src);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn string_chain_folds_to_single_set() {
        let opts = run_pass("set s \"\"\nappend s foo\nappend s bar");
        let fold = opts
            .iter()
            .find(|o| o.code == DiagCode::O104 && o.replacement.starts_with("set"))
            .expect("expected an O104 fold");
        assert_eq!(fold.replacement, "set s foobar");
        // One fold + two deletions, all in one group.
        let o104: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O104).collect();
        assert_eq!(o104.len(), 3);
        let groups: HashSet<_> = o104.iter().filter_map(|o| o.group).collect();
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn string_chain_rewrite_applies_cleanly() {
        assert_eq!(
            apply("set s \"\"\nappend s foo\nappend s bar"),
            "set s foobar"
        );
        assert_eq!(
            apply("set s start\nappend s _mid\nappend s _end"),
            "set s start_mid_end",
        );
    }

    #[test]
    fn chain_continues_past_interleaved_literal_set() {
        // `set t 1` writes a *different* variable with a static literal, so
        // the build chain folds across it (precise-flow); the interleaved
        // statement stays in place.
        assert_eq!(
            apply("set s \"\"\nset t 1\nappend s foo\nappend s bar"),
            "set t 1\nset s foobar",
        );
    }

    #[test]
    fn chain_breaks_on_interleaved_dynamic_statement() {
        // A `puts $s` reads the accumulator — `classify_write` returns
        // None for it, so the chain must NOT fold across it (only the
        // trailing two appends, which is a fresh sub-chain of length < 2).
        let opts = run_pass("set s \"\"\nputs $s\nappend s foo\nappend s bar");
        let fold = opts
            .iter()
            .find(|o| o.code == DiagCode::O104 && o.replacement.starts_with("set s"));
        // The `set s ""; puts $s` prefix breaks; the two trailing appends
        // have no anchoring `set`, so no fold fires.
        assert!(
            fold.is_none(),
            "must not fold across a reader, got {opts:?}"
        );
    }

    #[test]
    fn list_chain_folds_with_lappend() {
        assert_eq!(
            apply("set l {}\nlappend l a\nlappend l b c"),
            "set l {a b c}"
        );
    }

    #[test]
    fn list_chain_quotes_spacey_elements() {
        // An element containing a space must be re-quoted as a list word.
        assert_eq!(
            apply("set l {}\nlappend l {a b}\nlappend l c"),
            "set l {{a b} c}"
        );
    }

    #[test]
    fn single_write_does_not_fold() {
        let opts = run_pass("set s \"\"\nappend s foo");
        // set + one append = 2 writes → folds (the chain needs >= 2 writes).
        assert!(opts.iter().any(|o| o.code == DiagCode::O104));
        // But a lone set is not a chain.
        let opts = run_pass("set s foo");
        assert!(opts.iter().all(|o| o.code != DiagCode::O104));
    }

    #[test]
    fn dynamic_value_ends_the_run() {
        // `append s $x` is dynamic — the chain stops before it, and the
        // `set; append foo` prefix (2 writes) still folds.
        let opts = run_pass("set s \"\"\nappend s foo\nappend s $x");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O104 && o.replacement == "set s foo")
        );
    }

    #[test]
    fn intervening_read_ends_the_run_but_folds_prefix() {
        // `puts $s` reads the accumulator, ending the run after `append s
        // foo`. The consecutive prefix still folds to `set s foo` (the same
        // value `puts` observes); the trailing `append s bar` is not folded
        // into it. Matches `finish_chain`-on-read behaviour.
        let opts = run_pass("set s \"\"\nappend s foo\nputs $s\nappend s bar");
        let folds: Vec<&str> = opts
            .iter()
            .filter(|o| o.code == DiagCode::O104 && o.replacement.starts_with("set"))
            .map(|o| o.replacement.as_str())
            .collect();
        assert_eq!(folds, ["set s foo"], "got {opts:?}");
    }

    #[test]
    fn escaping_global_var_not_folded() {
        // `s` is global — every write is visible to other scopes.
        let opts = run_pass("proc ::f {} { global s\nset s \"\"\nappend s foo\nappend s bar }");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O104),
            "global var must not fold, got {opts:?}",
        );
    }

    #[test]
    fn cross_event_var_not_folded() {
        let src = "set s \"\"\nappend s foo\nappend s bar";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.cross_event_vars.insert("s".to_owned());
        run(&mut ctx, &cu);
        assert!(ctx.optimisations.iter().all(|o| o.code != DiagCode::O104));
    }

    #[test]
    fn folds_inside_proc_body() {
        assert_eq!(
            apply("proc ::f {} {\n    set s \"\"\n    append s a\n    append s b\n}"),
            "proc ::f {} {\n    set s ab\n}",
        );
    }

    /// Issue #1374 — a computed variable name between the writes can hit the
    /// accumulator under a spelling `classify_write` cannot see (`f acc`
    /// returns `zzz b` in tclsh; the fold's `a b` would be a miscompile), so
    /// the whole proc abstains from O104 / O130.
    #[test]
    fn dynamic_name_write_blocks_chain_fold() {
        let src = "proc ::f {name} { set acc {}; lappend acc a; set $name zzz; lappend acc b; return $acc }";
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| o.code != DiagCode::O130 && o.code != DiagCode::O104),
            "dynamic-name proc must not chain-fold, got {opts:?}",
        );
    }

    /// Issue #1377 — a write trace observes every intermediate store. The
    /// module fact records the trace target `::acc` canonically as `acc`, so
    /// the unqualified chain over `acc` must be protected too.
    #[test]
    fn traced_variable_blocks_chain_fold() {
        let src = "proc onw {a b c} { puts trace }\ntrace add variable ::acc write ::onw\nset acc {}\nlappend acc a\nlappend acc b";
        let opts = run_pass(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O130),
            "traced accumulator must not chain-fold, got {opts:?}",
        );
    }

    /// Issue #1377 — a dynamic trace target (`trace add variable $n …`)
    /// makes every name potentially traced, so no chain folds at all.
    #[test]
    fn dynamic_trace_target_blocks_chain_fold() {
        let src = "proc onw {a b c} { puts trace }\ntrace add variable $n write ::onw\nset acc {}\nlappend acc a\nlappend acc b";
        let opts = run_pass(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O130),
            "dynamic trace target must block every chain fold, got {opts:?}",
        );
    }
}
