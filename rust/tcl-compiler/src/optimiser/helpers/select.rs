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

//! Overlap-aware optimisation selection (manager final filter).
//!
//! Different passes may propose rewrites that overlap in source;
//! this helper picks a conflict-free subset deterministically:
//!
//! 1. Hint-only diagnostics never conflict — kept unconditionally.
//! 2. Rewrites are sorted by `(start, -priority, -length)` so
//!    earlier, higher-priority, longer rewrites win.
//! 3. Any rewrite overlapping a previously-kept rewrite is
//!    dropped. If the dropped rewrite had a group id, every
//!    surviving member of that group is dropped too — the group is
//!    applied all-or-nothing, never partially.
//! 4. Output is sorted by `start` with hints merged back in.

use std::collections::HashSet;

use super::super::{Optimisation, opt_priority};

/// Return the overlap-free subset of `optimisations`, applying
/// the pass-order / invalidation rules described above.
#[must_use]
pub fn select_non_overlapping(optimisations: &[Optimisation]) -> Vec<Optimisation> {
    let hints: Vec<Optimisation> = optimisations
        .iter()
        .filter(|o| o.hint_only)
        .cloned()
        .collect();
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
            .then_with(|| opt_priority(b.code).cmp(&opt_priority(a.code)))
            .then_with(|| (b.span.len()).cmp(&a.span.len()))
    });

    let mut selected: Vec<Optimisation> = Vec::new();
    let mut dropped_groups: HashSet<u32> = HashSet::new();

    for opt in rewrite_opts {
        let start = opt.span.start();
        let end = opt.span.end();
        let overlap = selected.iter().any(|kept| {
            // Spans overlap iff neither ends before the other
            // starts. `Span::end` is exclusive, so two spans
            // `[a1, a2)` and `[b1, b2)` overlap when
            // `a1 < b2 && b1 < a2`.
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

    // Drop survivors whose sibling was dropped: a group is applied
    // all-or-nothing. Merely clearing the group and keeping the survivors would
    // apply the group *partially* — e.g. a fold that deletes
    // `set s ""; append s foo` and rewrites the final `append s bar`, if the
    // rewrite loses an overlap, would leave the deletions in place and change
    // the result to `s == "bar"` (issue 153).
    if !dropped_groups.is_empty() {
        selected.retain(|opt| opt.group.is_none_or(|g| !dropped_groups.contains(&g)));
    }

    let mut out = selected;
    out.extend(hints);
    out.sort_by_key(|o| o.span.start());
    out
}

#[cfg(test)]
mod tests {
    use tcl_core_types::DiagCode;
    use tcl_lexer::Span;

    use super::*;

    fn opt(code: DiagCode, start: u32, end: u32) -> Optimisation {
        Optimisation::new(code, "m", Span::new(start, end), "r")
    }

    fn hint(code: DiagCode, start: u32, end: u32) -> Optimisation {
        let mut o = opt(code, start, end);
        o.hint_only = true;
        o
    }

    fn grouped(code: DiagCode, start: u32, end: u32, group: u32) -> Optimisation {
        let mut o = opt(code, start, end);
        o.group = Some(group);
        o
    }

    #[test]
    fn disjoint_rewrites_all_kept() {
        let out = select_non_overlapping(&[opt(DiagCode::O101, 0, 3), opt(DiagCode::O101, 5, 8)]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].span, Span::new(0, 3));
        assert_eq!(out[1].span, Span::new(5, 8));
    }

    #[test]
    fn overlapping_drops_lower_priority() {
        // O112 has higher priority than O101 — it wins on overlap.
        let out = select_non_overlapping(&[opt(DiagCode::O101, 0, 10), opt(DiagCode::O112, 0, 10)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, DiagCode::O112);
    }

    #[test]
    fn overlapping_drops_shorter_at_same_priority() {
        // Both O101 (priority 1) — longer wins.
        let out = select_non_overlapping(&[opt(DiagCode::O101, 0, 3), opt(DiagCode::O101, 0, 10)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].span, Span::new(0, 10));
    }

    #[test]
    fn group_dropped_entirely_when_sibling_dropped() {
        // Two members of group 7. The later one overlaps a higher-priority
        // rewrite and is dropped → the *surviving* member must be dropped too,
        // so the group is never applied partially (issue 153). Only the
        // higher-priority non-group rewrite remains.
        let opts = vec![
            grouped(DiagCode::O101, 0, 5, 7),
            opt(DiagCode::O112, 10, 20),
            grouped(DiagCode::O101, 10, 15, 7),
        ];
        let out = select_non_overlapping(&opts);
        assert_eq!(out.len(), 1, "surviving group member must be dropped: {out:?}");
        assert_eq!(out[0].code, DiagCode::O112);
        assert!(
            !out.iter().any(|o| o.span == Span::new(0, 5)),
            "the group-7 survivor must not be applied",
        );
    }

    #[test]
    fn intact_group_all_kept() {
        // FP-guard: when no member of a group is dropped, every member survives
        // with its group intact.
        let opts = vec![
            grouped(DiagCode::O101, 0, 5, 7),
            grouped(DiagCode::O101, 10, 15, 7),
        ];
        let out = select_non_overlapping(&opts);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|o| o.group == Some(7)));
    }

    #[test]
    fn hints_always_kept_even_when_overlapping_rewrites() {
        let out = select_non_overlapping(&[
            hint(DiagCode::O100, 0, 5),
            opt(DiagCode::O101, 0, 5),
            opt(DiagCode::O101, 0, 10),
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
            opt(DiagCode::O101, 20, 25),
            opt(DiagCode::O101, 0, 5),
            opt(DiagCode::O101, 10, 15),
        ]);
        let starts: Vec<u32> = out.iter().map(|o| o.span.start()).collect();
        assert_eq!(starts, vec![0, 10, 20]);
    }

    #[test]
    fn touching_but_non_overlapping_spans_both_kept() {
        // [0, 5) and [5, 10) share an endpoint but don't overlap
        // — Rust exclusive-end semantics.
        let out = select_non_overlapping(&[opt(DiagCode::O101, 0, 5), opt(DiagCode::O101, 5, 10)]);
        assert_eq!(out.len(), 2);
    }
}
