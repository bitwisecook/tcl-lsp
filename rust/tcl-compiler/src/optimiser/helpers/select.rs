//! Overlap-aware optimisation selection (manager final filter).
//!
//! Ported from `core/compiler/optimiser/_helpers.py::_select_non_overlapping_optimisations`.
//! Different passes may propose rewrites that overlap in source;
//! this helper picks a conflict-free subset deterministically:
//!
//! 1. Hint-only diagnostics never conflict — kept unconditionally.
//! 2. Rewrites are sorted by `(start, -priority, -length)` so
//!    earlier, higher-priority, longer rewrites win.
//! 3. Any rewrite overlapping a previously-kept rewrite is
//!    dropped. If the dropped rewrite had a group id, every
//!    surviving group member loses its group (all-or-nothing
//!    application).
//! 4. Output is sorted by `start` with hints merged back in.

use std::collections::HashSet;

use super::super::{opt_priority, Optimisation};

/// Return the overlap-free subset of `optimisations`, applying
/// the Python pass-order / invalidation rules.
#[must_use]
pub fn select_non_overlapping(optimisations: &[Optimisation]) -> Vec<Optimisation> {
    let hints: Vec<Optimisation> = optimisations.iter().filter(|o| o.hint_only).cloned().collect();
    let mut rewrite_opts: Vec<Optimisation> = optimisations
        .iter()
        .filter(|o| !o.hint_only)
        .cloned()
        .collect();

    // Sort key: (start, -priority, -length). Stable sort so ties
    // fall back on the original insertion order.
    rewrite_opts.sort_by(|a, b| {
        let sa = a.span.start();
        let sb = b.span.start();
        sa.cmp(&sb)
            .then_with(|| opt_priority(&b.code).cmp(&opt_priority(&a.code)))
            .then_with(|| (b.span.len()).cmp(&a.span.len()))
    });

    let mut selected: Vec<Optimisation> = Vec::new();
    let mut dropped_groups: HashSet<u32> = HashSet::new();

    for opt in rewrite_opts {
        let start = opt.span.start();
        let end = opt.span.end();
        let overlap = selected.iter().any(|kept| {
            // Spans overlap iff neither ends before the other
            // starts. Python compares inclusive-end offsets; Rust
            // `Span::end` is exclusive, so two spans `[a1, a2)`
            // and `[b1, b2)` overlap when `a1 < b2 && b1 < a2`.
            start < kept.span.end() && kept.span.start() < end
        });
        if overlap {
            if let Some(g) = opt.group {
                dropped_groups.insert(g);
            }
            continue;
        }
        selected.push(opt);
    }

    // Clear group from survivors whose sibling was dropped.
    if !dropped_groups.is_empty() {
        for opt in &mut selected {
            if let Some(g) = opt.group {
                if dropped_groups.contains(&g) {
                    opt.group = None;
                }
            }
        }
    }

    let mut out = selected;
    out.extend(hints);
    out.sort_by_key(|o| o.span.start());
    out
}

#[cfg(test)]
mod tests {
    use tcl_lexer::Span;

    use super::*;

    fn opt(code: &str, start: u32, end: u32) -> Optimisation {
        Optimisation::new(code, "m", Span::new(start, end), "r")
    }

    fn hint(code: &str, start: u32, end: u32) -> Optimisation {
        let mut o = opt(code, start, end);
        o.hint_only = true;
        o
    }

    fn grouped(code: &str, start: u32, end: u32, group: u32) -> Optimisation {
        let mut o = opt(code, start, end);
        o.group = Some(group);
        o
    }

    #[test]
    fn disjoint_rewrites_all_kept() {
        let out = select_non_overlapping(&[opt("O101", 0, 3), opt("O101", 5, 8)]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].span, Span::new(0, 3));
        assert_eq!(out[1].span, Span::new(5, 8));
    }

    #[test]
    fn overlapping_drops_lower_priority() {
        // O112 has higher priority than O101 — it wins on overlap.
        let out = select_non_overlapping(&[opt("O101", 0, 10), opt("O112", 0, 10)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, "O112");
    }

    #[test]
    fn overlapping_drops_shorter_at_same_priority() {
        // Both O101 (priority 1) — longer wins.
        let out = select_non_overlapping(&[opt("O101", 0, 3), opt("O101", 0, 10)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].span, Span::new(0, 10));
    }

    #[test]
    fn group_id_cleared_when_sibling_dropped() {
        // Two members of group 7. The later one overlaps a
        // higher-priority rewrite and is dropped → the surviving
        // member loses its group.
        let opts = vec![
            grouped("O101", 0, 5, 7),
            opt("O112", 10, 20),
            grouped("O101", 10, 15, 7),
        ];
        let out = select_non_overlapping(&opts);
        assert_eq!(out.len(), 2);
        let surviving = out.iter().find(|o| o.span == Span::new(0, 5)).unwrap();
        assert_eq!(
            surviving.group, None,
            "group should be cleared when sibling dropped",
        );
    }

    #[test]
    fn hints_always_kept_even_when_overlapping_rewrites() {
        let out = select_non_overlapping(&[
            hint("O100", 0, 5),
            opt("O101", 0, 5),
            opt("O101", 0, 10),
        ]);
        // The two rewrites overlap → longer kept; both the hint
        // and the surviving rewrite emerge.
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|o| o.hint_only));
        assert!(out.iter().any(|o| !o.hint_only && o.span.len() == 10));
    }

    #[test]
    fn output_sorted_by_start_offset() {
        let out = select_non_overlapping(&[
            opt("O101", 20, 25),
            opt("O101", 0, 5),
            opt("O101", 10, 15),
        ]);
        let starts: Vec<u32> = out.iter().map(|o| o.span.start()).collect();
        assert_eq!(starts, vec![0, 10, 20]);
    }

    #[test]
    fn touching_but_non_overlapping_spans_both_kept() {
        // [0, 5) and [5, 10) share an endpoint but don't overlap
        // — Rust exclusive-end semantics.
        let out = select_non_overlapping(&[opt("O101", 0, 5), opt("O101", 5, 10)]);
        assert_eq!(out.len(), 2);
    }
}
