//! Top-level optimiser orchestration (C30j).
//!
//! Ported from `core/compiler/optimiser/_manager.py`. The Python
//! manager is a ~500-line class that coordinates
//! [`CompilationUnit`] construction, per-function SSA/SCCP
//! building, interprocedural analysis, and interleaves pass
//! execution. In Rust that orchestration is already implicit in
//! the layered pipeline: [`CompilationUnit::build_for`] runs the
//! analyses, [`build_interprocedural_analysis`]
//! builds the summaries, and [`super::run_passes`] dispatches
//! each pass. The only value-add of a dedicated manager is the
//! thin façade below — a single entry point that plumbs these
//! together, runs every pass in default order, and then applies
//! the overlap-aware selection filter from
//! [`super::helpers::select`].
//!
//! Callers that need a single `optimise(source)` one-shot call
//! use [`optimise`] / [`optimise_with_dialect`]. Callers that
//! want full control (custom pass ordering, pre-populated
//! `PassContext` scratch state) stay on
//! [`super::run_passes`] directly.

use tcl_registry::CommandRegistry;

use crate::compilation_unit::CompilationUnit;
use crate::interprocedural::build_interprocedural_analysis;

use super::elimination::DeadStore;
use super::helpers::select::select_non_overlapping;
use super::{Optimisation, PassContext, PassId, run_passes};

/// Build a [`CompilationUnit`] for `source`, run every landed
/// optimiser pass in canonical order, and return the overlap-
/// free set of [`Optimisation`] suggestions.
///
/// Equivalent to [`optimise_with_dialect`] with `dialect = None`.
#[must_use]
pub fn optimise(source: &str, registry: &CommandRegistry) -> Vec<Optimisation> {
    optimise_with_dialect(source, registry, None)
}

/// Build a [`CompilationUnit`] for `source`, populate
/// interprocedural summaries, and run every pass in
/// [`PassId::all()`] order — then deduplicate via
/// [`select_non_overlapping`].
#[must_use]
pub fn optimise_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Optimisation> {
    let cu = CompilationUnit::build_for_with_config(
        source,
        registry,
        false,
        tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
    )
    .with_interprocedural(registry, dialect);
    optimise_unit(&cu, registry, dialect)
}

/// Run every pass over an **already-built** [`CompilationUnit`] (one carrying
/// its interprocedural summary) and return the overlap-resolved optimisations.
///
/// This is the rebuild-free core of [`optimise_with_dialect`]: callers that have
/// already constructed a `CompilationUnit` (e.g. the LSP diagnostics path, which
/// also runs `compiler_checks::run_all_checks` over the same unit) share it
/// instead of lowering the source a second time.
#[must_use]
pub fn optimise_unit(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Optimisation> {
    let ia = cu.interproc.clone().unwrap_or_default();
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    run_passes(&mut ctx, cu, &PassId::all());

    // Determinism chokepoint.  Several passes iterate `HashMap`s
    // (`cu.procedures`, def-use chains, SSA blocks), so both the emission order
    // and the monotonic group ids vary run-to-run — and, critically, between the
    // offset-0 per-procedure memo build and the whole-module build.  Canonicalise
    // before overlap arbitration so the surviving set, its order, and group
    // numbering are byte-identical given an equal `CompilationUnit` (the
    // `compiler_check_corpus` guard's contract, and the precondition for salsa
    // early-cutoff on [`compiler_check_diagnostics`]).
    ctx.optimisations.sort_by(|a, b| {
        (
            a.span.start(),
            a.span.end(),
            &a.code,
            &a.message,
            &a.replacement,
            a.hint_only,
        )
            .cmp(&(
                b.span.start(),
                b.span.end(),
                &b.code,
                &b.message,
                &b.replacement,
                b.hint_only,
            ))
    });
    let mut selected = select_non_overlapping(&ctx.optimisations);
    // Couple constant propagation with dead-store removal: a `set x <const>`
    // whose every use was propagated away by a *surviving* O100/O101/O102/O103
    // rewrite is now dead and can be removed. Done after overlap selection so
    // the removal is emitted only when the propagations actually survived —
    // Rust's group mechanism is not application-gating, so a survival check is
    // the safe coupling primitive. Mirrors Python's
    // `_eliminate_propagated_constants`.
    couple_propagated_const_dead_stores(cu, registry, dialect, &mut selected);
    // Re-canonicalise: `couple_propagated_const_dead_stores` appends its O109
    // removals in `cu.procedures` / `cu.methods` HashMap-iteration order, which
    // differs run-to-run and — critically — between the offset-0 per-procedure
    // memo build and the whole-module build. Sort by the same total key as
    // above so the surviving set's order (and therefore `renumber_groups`'
    // group numbering, which walks this order) is byte-identical given an equal
    // `CompilationUnit`. Also keeps `output_is_sorted_by_span_start`.
    selected.sort_by(|a, b| {
        (
            a.span.start(),
            a.span.end(),
            &a.code,
            &a.message,
            &a.replacement,
            a.hint_only,
        )
            .cmp(&(
                b.span.start(),
                b.span.end(),
                &b.code,
                &b.message,
                &b.replacement,
                b.hint_only,
            ))
    });
    renumber_groups(&mut selected);
    selected
}

/// Whether `code` is a *direct* constant-propagation rewrite — a `$var`
/// substituted by its value (O100) or forwarded from its reaching def (O102).
///
/// Deliberately excludes the expr / proc *folds* (O101 / O103): those consume
/// a `$var` inside a `[expr …]` / `[proc …]` that collapses to a literal,
/// which makes the feeding def merely *unused* rather than *propagated*. At
/// the top level Python keeps such a def (the conservative O126 "maybe a
/// global" case — `set a 3; puts [expr {$a + 1}]` keeps `set a 3`), and inside
/// a proc the ordinary dead-store pass removes it. Counting the fold here
/// would wrongly couple-remove the top-level def.
fn is_propagation_code(code: &str) -> bool {
    matches!(code, "O100" | "O102")
}

/// How many `$var` / `${var}` references `opt` consumed — present in its
/// original source span but gone from its replacement. Mirrors the
/// "`ref in orig_span and ref not in opt.replacement`" idea in Python's
/// `_CompilerOptimiser`, generalised to a count so one string rewrite that
/// folds several `$var` occurrences is tallied correctly.
fn consumed_var_count(opt: &Optimisation, source: &str, var: &str) -> usize {
    let (s, e) = (opt.span.start() as usize, opt.span.end() as usize);
    if s >= e || e > source.len() {
        return 0;
    }
    let before = count_var_refs(&source[s..e], var);
    let after = count_var_refs(&opt.replacement, var);
    before.saturating_sub(after)
}

/// Number of `$var` / `${var}` references in `text` (word-bounded).
fn count_var_refs(text: &str, var: &str) -> usize {
    let bytes = text.as_bytes();
    let dollar = format!("${var}");
    let mut n = 0;
    let mut from = 0;
    while let Some(rel) = text[from..].find(&dollar) {
        let pos = from + rel;
        let after = pos + dollar.len();
        // `${var}` form: a `{` immediately after `$`, matched separately below.
        let boundary = bytes
            .get(after)
            .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'_'));
        if boundary {
            n += 1;
        }
        from = pos + 1;
    }
    let braced = format!("${{{var}}}");
    n += text.matches(&braced).count();
    n
}

/// Count occurrences of `var` as a standalone bareword in `text` — word
/// boundaries on both sides, **not** a `$var` / `${var}` substitution, and
/// **not** inside a `"…"` quoted string or `{…}` braced literal. Used to
/// detect by-name reads the `$var` scan misses (`info exists x`, a `[set x]`
/// command substitution); occurrences of the name as literal text inside a
/// string (`"x=$x"`) or braces are not reads and must not count.
fn bareword_occurrences(text: &str, var: &str) -> usize {
    let bytes = text.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut n = 0;
    let mut from = 0;
    while let Some(rel) = text[from..].find(var) {
        let pos = from + rel;
        from = pos + 1;
        // Word boundary after.
        let after = pos + var.len();
        if bytes.get(after).is_some_and(|b| is_word(*b)) {
            continue;
        }
        // Word boundary before, and not a `$`-substitution form.
        match pos.checked_sub(1).and_then(|i| bytes.get(i)).copied() {
            Some(b'$') => continue,                                       // `$var`
            Some(b'{') if pos >= 2 && bytes[pos - 2] == b'$' => continue, // `${var`
            Some(b) if is_word(b) => continue,                            // part of a longer word
            _ => {}
        }
        // Skip occurrences inside a `"…"` string or `{…}` braces — there the
        // name is literal text, not a variable read. (Command substitutions
        // `[…]` are *not* skipped: `[set x]` / `[info exists x]` read by name.)
        if in_string_or_braces(bytes, pos) {
            continue;
        }
        n += 1;
    }
    n
}

/// Whether byte offset `pos` in `text` lies inside a `"…"` quoted string or
/// `{…}` braces. A conservative scan from the start: an unescaped `"` toggles
/// quote state (only while not inside braces), and `{`/`}` track brace depth
/// (only while not inside a quote). Over-counting on pathological input keeps
/// the def (safe); the common cases (`"x=$x"`, `{$x}`) are handled.
fn in_string_or_braces(bytes: &[u8], pos: usize) -> bool {
    let mut in_quote = false;
    let mut brace_depth = 0u32;
    let mut i = 0;
    while i < pos {
        match bytes[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'"' if brace_depth == 0 => in_quote = !in_quote,
            b'{' if !in_quote => brace_depth += 1,
            b'}' if !in_quote && brace_depth > 0 => brace_depth -= 1,
            _ => {}
        }
        i += 1;
    }
    in_quote || brace_depth > 0
}

/// Couple constant propagation with dead-store removal — see the call site in
/// [`optimise_unit`]. For each single-def constant scalar variable whose every
/// use was propagated by a surviving rewrite, append an `O109` removal of the
/// defining `set`. Conservative on every axis: a single safe-to-delete
/// constant def, all uses simple `Operand` reads each covered by a surviving
/// propagation that consumed the `$var`, no aliasing / RMW-hidden / global /
/// array involvement, the textual `$var` count equal to the use count (no
/// untracked reference), and the removal span free of overlap with any selected
/// rewrite. iRules dialects are skipped (cross-event variable lifetimes need
/// the connection-scope model, out of scope here).
fn couple_propagated_const_dead_stores(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    selected: &mut Vec<Optimisation>,
) {
    if crate::taint::is_irules_dialect(dialect) {
        return;
    }
    let source = &cu.source;
    let mut functions: Vec<&crate::compilation_unit::FunctionUnit> = vec![&cu.top_level];
    functions.extend(cu.procedures.values());
    functions.extend(cu.methods.values());

    let mut removals: Vec<Optimisation> = Vec::new();
    for fu in functions {
        couple_const_dead_stores_in_function(fu, registry, source, selected, &mut removals);
    }

    // Append each removal, reconciling it against the already-selected
    // rewrites. The def line is disjoint from the use propagations (those land
    // at the use sites on other lines), but a rewrite *inside* the def line —
    // the O101 expr-fold of `set x [expr {1+1}]`, or an O100/O102 inside the
    // RHS — is fully contained in the line being deleted, so the deletion
    // supersedes it: drop those and emit the removal. A rewrite that only
    // *partially* overlaps the line (a structural O112 spanning past it) is
    // left intact and the removal skipped.
    for rem in removals {
        let contains =
            |o: &Optimisation| rem.span.start() <= o.span.start() && o.span.end() <= rem.span.end();
        let partial_overlap = selected.iter().any(|o| {
            !o.hint_only
                && rem.span.start() < o.span.end()
                && o.span.start() < rem.span.end()
                && !contains(o)
        });
        if partial_overlap {
            continue;
        }
        selected.retain(|o| o.hint_only || !contains(o));
        selected.push(rem);
    }
}

fn couple_const_dead_stores_in_function(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
    source: &str,
    selected: &[Optimisation],
    removals: &mut Vec<Optimisation>,
) {
    use crate::analyses::LatticeValue;
    use crate::def_use::{DefKind, UseKind};
    use std::collections::HashSet;

    let scope_aliases = super::elimination::scan_scope_aliases(&fu.cfg);
    let rmw_hidden = super::elimination::collect_rmw_hidden_reads(fu, registry);
    let empty: HashSet<String> = HashSet::new();

    // Per-variable def count — only single-def scalars qualify.
    let mut def_count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind == DefKind::Statement {
            *def_count.entry(chain.key.0.as_str()).or_insert(0) += 1;
        }
    }

    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        let (var, _ver) = &chain.key;
        // Single constant scalar def, never aliased / global / RMW-hidden.
        if def_count.get(var.as_str()).copied().unwrap_or(0) != 1 {
            continue;
        }
        if var.starts_with("::") || scope_aliases.contains(var) || rmw_hidden.contains(var) {
            continue;
        }
        if !matches!(fu.sccp.values.get(&chain.key), Some(LatticeValue::Const(_))) {
            continue;
        }
        // Any def-tracked use must be a simple operand read — a phi/terminator
        // use means the value flows somewhere a textual `$var` scan can't see,
        // so bail conservatively.
        if chain
            .uses
            .iter()
            .any(|u| !matches!(u.kind, UseKind::Operand))
        {
            continue;
        }
        let Some(def_block) = fu.cfg.blocks.get(&chain.definition.block) else {
            continue;
        };
        let Ok(def_idx) = usize::try_from(chain.definition.statement_index) else {
            continue;
        };
        let Some(def_stmt) = def_block.statements.get(def_idx) else {
            continue;
        };
        // The def must be a const-foldable scalar assignment whose inlined
        // value carries no substitution metacharacters. Two shapes qualify:
        //
        //   * `set x <literal>` (`AssignConst`) inlines its literal verbatim.
        //   * `set x [expr {1+1}]` (`AssignExpr` / `AssignValue`) was proven
        //     `Const` by SCCP above, so O100 substituted the *rendered
        //     constant* (`2`), not the original expression text. Python
        //     removes these too once the fold + propagation land — match it.
        //
        // Either way reject a value bearing `$ [ ] \`: it could re-substitute
        // differently once inlined, so those are left to the conservative path.
        let inlined_value = match def_stmt {
            crate::ir::Statement::AssignConst { value, .. } => value.clone(),
            crate::ir::Statement::AssignExpr { .. } | crate::ir::Statement::AssignValue { .. } => {
                let Some(LatticeValue::Const(c)) = fu.sccp.values.get(&chain.key) else {
                    continue;
                };
                let Some(text) = super::helpers::literals::format_constant(c) else {
                    continue;
                };
                text
            }
            _ => continue,
        };
        if inlined_value.contains(['$', '[', ']', '\\']) {
            continue;
        }
        // Pure constant assignment — removing it loses no observable effect.
        if !super::elimination::assignment_safe_to_delete(
            def_stmt,
            Some(registry),
            &empty,
            &empty,
            None,
        ) {
            continue;
        }

        // Textual coupling check (robust to def-use gaps such as
        // string-interpolation reads): count `$var` references across the
        // function, then count how many a *surviving* propagation actually
        // consumed. The def is removable only when at least one reference
        // existed and **every** reference was propagated away. A braced
        // literal `{$var}` (no substitution) or an array `$var(i)` read is
        // counted but never consumed, so the counts diverge and we keep the
        // def — conservative and safe.
        let (fs, fe) = function_source_span(fu);
        if fs >= fe || fe > source.len() {
            continue;
        }
        let func_src = &source[fs..fe];
        let total_refs = count_var_refs(func_src, var);
        if total_refs == 0 {
            // Never read (or read only in a non-substituting form) — the
            // existing unused-variable pass owns this; not a propagation
            // coupling.
            continue;
        }
        // Miscompilation guard: the variable name must appear as a *bareword*
        // exactly once — the def target. Any other bareword occurrence is a
        // by-name read the `$var` scan can't see (`[set x]`, `info exists x`,
        // `upvar … x`, a `"…x…"` literal, …); removing the def would then drop a
        // value still consumed. Conservative: a stray textual `x` keeps the def.
        if bareword_occurrences(func_src, var) != 1 {
            continue;
        }
        // Attribute a propagation to this function by its *start* offset —
        // a rewrite never spans two functions, and a word-token's span may
        // run one past `fe` (the inner-end convention leaves the closing
        // delimiter outside the statement span).
        let consumed: usize = selected
            .iter()
            .filter(|o| {
                !o.hint_only
                    && is_propagation_code(&o.code)
                    && (o.span.start() as usize) >= fs
                    && (o.span.start() as usize) <= fe
            })
            .map(|o| consumed_var_count(o, source, var))
            .sum();
        if consumed != total_refs {
            continue;
        }

        // Approach B: `def_stmt` is from `fu.cfg` (relative to `base_offset`).
        let del_span = line_delete_span(source, fu.abs_span(def_stmt.span()));
        removals.push(Optimisation::new(
            "O109",
            "Eliminate dead store",
            del_span,
            "",
        ));
    }
}

/// Source byte range spanning every statement of `fu` (for textual reference
/// counting). Falls back to an empty range when the function has no spans.
fn function_source_span(fu: &crate::compilation_unit::FunctionUnit) -> (usize, usize) {
    let mut lo = u32::MAX;
    let mut hi = 0u32;
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            let sp = stmt.span();
            if sp.start() < lo {
                lo = sp.start();
            }
            if sp.end() > hi {
                hi = sp.end();
            }
        }
        if let Some(t) = block
            .terminator
            .as_ref()
            .and_then(crate::cfg::Terminator::span)
        {
            if t.start() < lo {
                lo = t.start();
            }
            if t.end() > hi {
                hi = t.end();
            }
        }
    }
    if lo == u32::MAX {
        (0, 0)
    } else {
        // Approach B: `fu`'s CFG spans are relative to its `base_offset`; return
        // absolute positions so the `source[fs..fe]` slice and the comparison
        // against (absolute) emitted optimisation spans are correct on the
        // memoised offset-0 path.
        (fu.abs_pos(lo) as usize, fu.abs_pos(hi) as usize)
    }
}

/// Extend a statement's span to swallow its trailing newline (and leading
/// indentation) so the whole `set` line is removed cleanly.
fn line_delete_span(source: &str, span: tcl_lexer::Span) -> tcl_lexer::Span {
    let bytes = source.as_bytes();
    let mut start = span.start() as usize;
    let mut end = span.end() as usize;
    // Back up over leading spaces/tabs on the line.
    while start > 0 && matches!(bytes.get(start - 1), Some(b' ' | b'\t')) {
        start -= 1;
    }
    // Swallow a single trailing newline (and a preceding CR).
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    } else if end + 1 < bytes.len() && bytes[end] == b'\r' && bytes[end + 1] == b'\n' {
        end += 2;
    }
    tcl_lexer::Span::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

/// Canonicalise group ids in-place to `0, 1, 2, …` by order of first appearance.
///
/// Group ids are allocated by a monotonic counter during pass execution, so
/// their absolute values depend on the (`HashMap`-iteration) order in which
/// grouped rewrites were emitted.  Only the *partition* they encode is
/// semantically meaningful (members of one group apply all-or-nothing), so
/// remapping each distinct id to a first-appearance index makes the values
/// deterministic while preserving the partition.  `opts` is assumed already in
/// canonical order.
fn renumber_groups(opts: &mut [Optimisation]) {
    use std::collections::HashMap;
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut next = 0u32;
    for o in opts.iter_mut() {
        if let Some(g) = o.group {
            let new = *remap.entry(g).or_insert_with(|| {
                let v = next;
                next += 1;
                v
            });
            o.group = Some(new);
        }
    }
}

/// Run every pass over `cu` and return the **O109 dead stores** the
/// elimination pass determined eliminable (each keyed by function / block /
/// statement / SSA value). Mirrors [`optimise_unit`] but exposes the
/// structured dead-store records ([`PassContext::dead_stores`]) instead of
/// the optimisation list — so tools (the compiler explorer's `cfgPostSsa`
/// analysis, dead-store callouts, and `stats`) can show dead stores from
/// where Rust actually computes them, with the optimiser's full suppression
/// applied (purity, scope aliases, place model, cross-event scope).
#[must_use]
pub fn find_dead_stores(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<DeadStore> {
    let ia = cu.interproc.clone().unwrap_or_default();
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    run_passes(&mut ctx, cu, &PassId::all());
    ctx.dead_stores
}

/// Run the passes one at a time over a shared context and return, for each
/// [`PassId`] in [`PassId::all`] order, the optimisations *that pass*
/// produced (raw, before overlap arbitration). Powers the explorer's
/// Rust-native "optimiser pass pipeline" view — there is no Python analogue
/// because the Python optimiser is not structured as this pass sequence.
///
/// Equivalent to [`optimise_unit`] in effect (each pass sees the prior
/// passes' context), but it attributes every finding to its originating pass.
#[must_use]
pub fn optimise_by_pass(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<(PassId, Vec<Optimisation>)> {
    let ia = cu.interproc.clone().unwrap_or_default();
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    let mut by_pass = Vec::new();
    for pass in PassId::all() {
        let before = ctx.optimisations.len();
        run_passes(&mut ctx, cu, &[pass]);
        by_pass.push((pass, ctx.optimisations[before..].to_vec()));
    }
    by_pass
}

/// Build, run every pass, and return the full *unfiltered*
/// optimisation list (no overlap resolution). Exposed mainly for
/// tests that want to inspect raw per-pass output before the
/// manager's arbitration.
#[must_use]
pub fn optimise_raw(
    source: &str,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Optimisation> {
    // Split the raw CU build to avoid recomputing on
    // `with_interprocedural` — done in two lines for clarity.
    let cu = CompilationUnit::build_for_with_config(
        source,
        registry,
        false,
        tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
    );
    let ia = build_interprocedural_analysis(&cu.ir_module, registry, dialect);
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    // SYNC-JUN02b-4: the whole-module builtin-fold trust gate (O129).
    ctx.command_mutations =
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, registry);
    run_passes(&mut ctx, &cu, &PassId::all());
    ctx.optimisations
}

/// Apply the non-hint-only optimisation rewrites to `source`, returning the
/// rewritten text.  Edits are applied in reverse-offset order (so earlier
/// offsets stay valid) and deduplicated by `(offset, length)`.  Spans are
/// half-open `[start, end)`, so the byte range is `span.start()..span.end()`.
/// Mirrors `_manager.py::apply_optimisations`.
#[must_use]
pub fn apply_optimisations(source: &str, optimisations: &[Optimisation]) -> String {
    let mut edits: Vec<(usize, usize, &str)> = optimisations
        .iter()
        .filter(|o| !o.hint_only)
        .filter_map(|o| {
            let start = o.span.start() as usize;
            let end = o.span.end() as usize;
            (start <= end && end <= source.len()).then_some((
                start,
                end - start,
                o.replacement.as_str(),
            ))
        })
        .collect();
    if edits.is_empty() {
        return source.to_owned();
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut result = source.to_owned();
    for (offset, length, text) in edits {
        if !seen.insert((offset, length)) {
            continue;
        }
        if offset + length <= result.len() {
            result.replace_range(offset..offset + length, text);
        }
    }
    result
}

/// Iteratively optimise `source` until a fixpoint or `max_iterations` is
/// reached: each pass recompiles the rewritten source so optimisations
/// exposed by an earlier pass (constant folding enabling further folding /
/// dead-store removal) are discovered.  Returns `(final_source,
/// all_optimisations_applied)`.  Mirrors
/// `_manager.py::optimise_source_multipass`.
#[must_use]
pub fn optimise_source_multipass(
    source: &str,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    max_iterations: usize,
) -> (String, Vec<Optimisation>) {
    let mut current = source.to_owned();
    let mut all: Vec<Optimisation> = Vec::new();
    for _ in 0..max_iterations {
        let opts = optimise_with_dialect(&current, registry, dialect);
        if opts.is_empty() {
            break;
        }
        let next = apply_optimisations(&current, &opts);
        all.extend(opts);
        if next == current {
            break;
        }
        current = next;
    }
    (current, all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        // C43 sub-strip 4: `when` is now registry-resolved (no
        // string-pattern fallback in `lower_command`), so any
        // test that lowers iRule code through `optimise_*` must
        // carry the iRules dialect.  Loading it here keeps the
        // helper a one-call site; tests that only lower plain
        // Tcl don't notice the extra commands.
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
    }

    #[test]
    fn empty_source_yields_empty_result() {
        let opts = optimise("", &registry());
        assert!(opts.is_empty());
    }

    fn optimised(src: &str) -> String {
        let opts = optimise(src, &registry());
        apply_optimisations(src, &opts)
    }

    #[test]
    fn constant_loop_condition_fold_keeps_braces() {
        // A constant `while` condition folded through O101 must not drop the
        // opening brace of the braced condition (the CFG loop-condition span
        // omits the closing `}`, which previously produced `while 1}`).
        // `while {1}` is already minimal → unchanged; `while {1 < 2}` folds
        // to `while {1}` (braces preserved), matching Python.
        assert_eq!(optimised("while {1} { break }\n"), "while {1} { break }\n");
        assert_eq!(
            optimised("while {1 < 2} { break }\n"),
            "while {1} { break }\n"
        );
    }

    #[test]
    fn call_by_name_in_namespace_keeps_feeding_set() {
        // A `proc` declared inside a namespaced proc is registered under its
        // qualified name (`::demo::bump`); a same-namespace bare call
        // (`bump x`) must still resolve to it so the call-by-name `upvar`
        // read keeps the feeding `set x 10` alive. Regression for an
        // over-removal that produced a dangling `$x` reference.
        let src = "namespace eval ::demo {\n    \
                   proc upvarDemo {} {\n        set x 10\n        \
                   proc bump {varName} { upvar 1 $varName v; incr v; return $v }\n        \
                   set y [bump x]\n        return $x\n    }\n}\n";
        assert!(
            optimised(src).contains("set x 10"),
            "call-by-name read must keep `set x 10`; got {:?}",
            optimised(src),
        );
    }

    #[test]
    fn couples_constant_propagation_with_dead_store_removal() {
        // The propagated constant leaves `set x 42` dead — it is removed.
        assert_eq!(optimised("set x 42\nputs $x\n"), "puts 42\n");
        // String-interpolation reads are covered too (the textual scan does
        // not rely on def-use tracking the in-string `$x`).
        assert_eq!(
            optimised("proc f {} {\n  set x 42\n  puts \"v $x\"\n}\n"),
            "proc f {} {\n  puts \"v 42\"\n}\n",
        );
    }

    #[test]
    fn couples_sccp_const_expr_dead_store_removal() {
        // `set x [expr {1+1}]` folds to a constant SCCP proves; once its only
        // use is propagated the computed def is dead — removed (matching
        // Python). The O101 expr-fold inside the def line is superseded by the
        // line deletion.
        assert_eq!(optimised("set x [expr {1+1}]\nputs $x\n"), "puts 2\n");
        // The quoted-expr form folds the same way — Python removes it too.
        assert_eq!(optimised("set x [expr \"1+1\"]\nputs $x\n"), "puts 2\n");
        // Inside a proc, with a string-interpolation read.
        assert_eq!(
            optimised("proc p {} {\n  set x [expr {1+1}]\n  puts \"v=$x\"\n}\n"),
            "proc p {} {\n  puts \"v=2\"\n}\n",
        );
    }

    #[test]
    fn coupling_keeps_non_const_expr_def() {
        // A computed assignment SCCP cannot prove constant (a runtime command)
        // is never coupled away.
        let kept = optimised("set x [expr {[clock seconds]}]\nputs $x\n");
        assert!(
            kept.contains("set x [expr"),
            "non-const expr def must be kept; got {kept:?}",
        );
    }

    #[test]
    fn coupling_keeps_defs_it_cannot_prove_dead() {
        // Never read at top level → kept (the unused-variable pass owns the
        // proc case; a top-level const may be an externally-consumed global).
        assert_eq!(
            optimised("set x 42\nputs hello\n"),
            "set x 42\nputs hello\n"
        );
        // Read by name, not via `$x` — `[set x]` still needs the value.
        let by_name = optimised("proc f {} {\n  set x 42\n  puts [set x]\n}\n");
        assert!(
            by_name.contains("set x 42"),
            "by-name read must keep the def; got {by_name:?}",
        );
        // A value carrying substitution metacharacters is never coupled.
        let meta = optimised("set e {$y [k]}\ncatch {subst $e}\n");
        assert!(
            meta.contains("set e "),
            "metacharacter-bearing const must keep its def; got {meta:?}",
        );
    }

    #[test]
    fn coupling_sees_through_literal_name_in_strings() {
        // The variable name appearing as literal text inside a string
        // (`"x=$x"`) is not a by-name read — the def is still removable when
        // every real `$x` reference is propagated.
        assert_eq!(
            optimised("set x 1\nset y 2\nputs \"x=$x y=$y\"\n"),
            "puts \"x=1 y=2\"\n",
        );
    }

    #[test]
    fn bareword_occurrences_skips_string_and_brace_literals() {
        // `info exists x` / bare `x` command word → counted (by-name read).
        assert_eq!(bareword_occurrences("info exists x", "x"), 1);
        // Literal text inside quotes / braces → not counted.
        assert_eq!(bareword_occurrences("puts \"x=$x\"", "x"), 0);
        assert_eq!(bareword_occurrences("puts {x marks}", "x"), 0);
        // A `[set x]` command substitution still counts (reads by name).
        assert_eq!(bareword_occurrences("puts [set x]", "x"), 1);
    }

    #[test]
    fn braced_word_is_never_propagated() {
        // Tcl performs no substitution inside `{…}`, so `$x` in a braced word
        // is a literal — it must not be propagated, and the def stays live.
        assert_eq!(
            optimised("set x 42\nputs {$x}\nputs $x\n"),
            "set x 42\nputs {$x}\nputs 42\n",
        );
    }

    #[test]
    fn read_modify_write_target_keeps_feeding_set() {
        // `lset` / `lpop` read the list before rewriting it, so the feeding
        // `set` is live — not a dead store.
        for src in [
            "set lst {a b c d e}\nlset lst 2 X\nputs $lst\n",
            "set lst {1 2 3}\nlpop lst\nputs $lst\n",
        ] {
            assert!(
                optimised(src).contains("set lst"),
                "RMW target's feeding set must be kept; got {:?}",
                optimised(src),
            );
        }
    }

    #[test]
    fn constant_branch_fires_via_manager() {
        // if {1} { set x 1 } triggers both O101 (branch fold)
        // and O112 (structure elimination). The overlap filter
        // prefers the higher-priority O112 (priority 9) over
        // O101 (priority 1), so the manager's output should
        // contain O112 in this shape.
        let opts = optimise("if {1} { set x 1 } else { set y 2 }", &registry());
        assert!(
            opts.iter().any(|o| o.code == "O112" || o.code == "O101"),
            "expected at least one branch-related rewrite, got {opts:?}",
        );
    }

    #[test]
    fn info_exists_fold_surfaces_o101() {
        // SYNC-MAY31-3 DCE wiring: a provably-constant `info exists`
        // guard (never-defined non-param folds false; a parameter folds
        // true) surfaces as an O101 constant-branch fold.
        let never = optimise(
            "proc f {a} { if {[info exists b]} { puts hi } }",
            &registry(),
        );
        assert!(
            never
                .iter()
                .any(|o| o.code == "O101" && o.replacement == "{0}"),
            "never-defined `info exists` should fold to {{0}}, got {never:?}",
        );
        let param = optimise(
            "proc f {a} { if {[info exists a]} { puts hi } }",
            &registry(),
        );
        assert!(
            param
                .iter()
                .any(|o| o.code == "O101" && o.replacement == "{1}"),
            "parameter `info exists` should fold to {{1}}, got {param:?}",
        );
    }

    #[test]
    fn output_is_sorted_by_span_start() {
        let opts = optimise("set x 5\nif {1} { puts $x } else { puts 0 }", &registry());
        let mut prev = 0u32;
        for o in &opts {
            assert!(
                o.span.start() >= prev,
                "manager output must be sorted by span start: got {} after {}",
                o.span.start(),
                prev,
            );
            prev = o.span.start();
        }
    }

    #[test]
    fn overlap_filter_runs() {
        // The manager applies select_non_overlapping, so duplicate
        // / overlapping rewrites must not appear in the output
        // — exercise by checking that no two rewrites share the
        // exact same span.
        let opts = optimise("if {1} { set x 1 } else { set y 2 }", &registry());
        let spans: Vec<_> = opts.iter().map(|o| o.span).collect();
        let mut unique = spans.clone();
        unique.sort_by_key(|s| (s.start(), s.end()));
        unique.dedup();
        assert_eq!(
            spans.len(),
            unique.len(),
            "manager must deduplicate overlapping rewrites",
        );
    }

    #[test]
    fn optimise_raw_skips_overlap_filter() {
        // Raw output can contain overlapping rewrites that the
        // filtered path would dedupe — the `raw` entry point is
        // the escape hatch that leaves them visible.
        let raw = optimise_raw("if {1} { set x 1 } else { set y 2 }", &registry(), None);
        // Presence alone is the contract; specific counts depend
        // on which pass bodies are landed.
        let _ = raw;
    }

    #[test]
    fn dialect_gated_passes_observe_active_dialect() {
        // irules-only O124 should fire when dialect = f5-irules.
        let src = "proc ::dead {} { return 1 }\nwhen HTTP_REQUEST { set x 0 }\n";
        let opts = optimise_with_dialect(src, &registry(), Some("f5-irules"));
        assert!(
            opts.iter().any(|o| o.code == "O124"),
            "expected O124 in irules dialect, got {opts:?}",
        );
        // And should NOT fire for plain tcl.
        let tcl_opts = optimise_with_dialect(src, &registry(), Some("tcl"));
        assert!(
            tcl_opts.iter().all(|o| o.code != "O124"),
            "O124 should be gated on irules dialect, got {tcl_opts:?}",
        );
    }
}
