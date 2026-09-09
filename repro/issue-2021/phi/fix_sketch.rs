// SKETCH ONLY -- not applied, does not compile standalone.
// Proposed replacement for `phi_can_undef` in
// rust/tcl-compiler/src/analyser/diagnostics/helpers.rs

/// Startup facts for one variable name.  They depend only on `name`, which
/// never changes down `phi_can_undef`'s recursion, so they are computed once
/// per top-level query instead of once per visited node.
#[derive(Clone, Copy)]
pub(super) struct StartupFacts {
    global_binding: bool,
    readable_at_startup: bool,
    rematerialises_after_unset: bool,
}

impl StartupFacts {
    fn for_name(name: &str, ctx: &PhiUndefCtx<'_>) -> Self {
        let startup_name = startup_var_name(name);
        let global_binding =
            has_global_startup_binding(name, ctx.initial_global, ctx.global_aliases);
        Self {
            global_binding,
            readable_at_startup: global_binding
                && tcl_registry::special_vars::is_readable_at_startup(startup_name, ctx.dialect),
            rematerialises_after_unset: global_binding
                && tcl_registry::special_vars::is_lazily_readable(startup_name, ctx.dialect),
        }
    }
}

/// Memo for [`phi_can_undef`], shared across every query on one function.
///
/// Only holds *path-independent* answers: a node whose result was reached
/// without cutting a back-edge has the same answer from every ancestor, so it
/// is safe to reuse.  A node whose result used the "a back-edge is not undef"
/// rule is valid only while the ancestors it cut against are open on the DFS
/// stack, so it is deliberately left out — that keeps the existing semantics
/// bit-for-bit while still collapsing the acyclic (overwhelmingly common) part
/// of the phi DAG.  Loop-header phi cycles are small, so the residual repeat
/// work is bounded by the SCC size, not by the whole function.
#[derive(Default)]
pub(super) struct PhiUndefMemo {
    // Ideally keyed on `(crate::ssa::Symbol, Version)` rather than
    // `(String, Version)`: `build_phi_undef_index` already has the symbol and
    // the `String` key costs an allocation on *every* visit (visible as
    // `__GI___libc_free` under `phi_can_undef` in the gdb samples).
    done: FxHashMap<(String, crate::ssa::Version), bool>,
    facts: FxHashMap<String, StartupFacts>,
}

pub(super) fn phi_can_undef(
    name: &str,
    version: crate::ssa::Version,
    ctx: &PhiUndefCtx<'_>,
    memo: &mut PhiUndefMemo,
) -> bool {
    let facts = match memo.facts.get(name) {
        Some(f) => *f,
        None => {
            let f = StartupFacts::for_name(name, ctx);
            memo.facts.insert(name.to_string(), f);
            f
        }
    };
    let mut seen = FxHashSet::default();
    walk(name, version, ctx, facts, &mut seen, memo).0
}

/// Returns `(can_undef, used_back_edge_cut)`.  The second flag is what keeps
/// the cycle rule's path-dependence out of the memo.
fn walk(
    name: &str,
    version: crate::ssa::Version,
    ctx: &PhiUndefCtx<'_>,
    facts: StartupFacts,
    seen: &mut FxHashSet<(String, crate::ssa::Version)>,
    memo: &mut PhiUndefMemo,
) -> (bool, bool) {
    let key = (name.to_string(), version);

    if ctx.killed.contains(&key) {
        return (!facts.rematerialises_after_unset, false);
    }
    if version == 0 {
        return (!facts.readable_at_startup, false);
    }
    if seen.contains(&key) {
        // Cycle (loop-header phi): unchanged semantics -- treat the back edge
        // as not-undef -- but tell the caller its answer is path-dependent.
        return (false, true);
    }
    if let Some(&cached) = memo.done.get(&key) {
        return (cached, false);
    }
    let Some(phi) = ctx.phi_def.get(&key) else {
        return (false, false); // concrete definition
    };

    let this_block = ctx.phi_block.get(&key).copied();
    seen.insert(key.clone());
    let mut result = false;
    let mut cyclic = false;
    for (&pred, &incoming_ver) in &phi.incoming {
        if !ctx.considered.contains(&pred) {
            continue;
        }
        if let Some(blk) = this_block
            && !ctx.executable_edges.is_empty()
            && !ctx.executable_edges.contains(&(pred, blk))
        {
            continue;
        }
        if ctx
            .exists_guards
            .iter()
            .any(|(gv, gblk)| gv == name && block_dominated_by(ctx.ssa, pred, *gblk))
        {
            continue;
        }
        let (sub, sub_cyclic) = walk(name, incoming_ver, ctx, facts, seen, memo);
        if sub {
            // A `true` reached without a cut is unconditionally true, so it is
            // cacheable even if an *earlier* operand was cyclic.
            result = true;
            cyclic = sub_cyclic;
            break;
        }
        cyclic |= sub_cyclic;
    }
    seen.remove(&key);

    if !cyclic {
        memo.done.insert(key, result);
    }
    (result, cyclic)
}

// Call-site changes ----------------------------------------------------------
//
// helpers.rs `build_undef_suppression` (~line 930): hoist the memo out of the
// driver loop so the whole `phi_def.keys()` sweep is one linear pass:
//
//     let mut memo = PhiUndefMemo::default();
//     for key in phi_def.keys() {
//         if phi_can_undef(&key.0, key.1, &undef_ctx, &mut memo) {
//             can_undef.insert(key.clone());
//         }
//     }
//     let loop_entry_only_undef =
//         build_loop_entry_only_undef(fu, &can_undef, &undef_ctx, rules, &mut memo);
//
// helpers.rs `build_loop_entry_only_undef` (~line 1045): thread the same memo
// through the fixpoint instead of a fresh `seen` per operand per pass.
//
// dataflow.rs `return_read_fires_w210` (~line 1589): build one
// `PhiUndefMemo` in `emit_return_phi_undef_w210` next to `PhiUndefIndex` and
// pass it down, so the per-return-block sweep also shares the memo.
