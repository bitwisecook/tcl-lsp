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

//! The matcher. Tcl's ARE uses POSIX leftmost-longest semantics with a
//! hierarchical "longest earlier subexpression" rule for submatches — *not*
//! Perl leftmost-first. We get that with two phases, mirroring the C engine's
//! split between its DFA (find the overall extent) and `cdissect` (assign
//! submatches), but expressed idiomatically:
//!
//! 1. **Extent** — [`Matcher::reach`] computes, by NFA-style set simulation,
//!    every end position a node can reach from a start. The overall match is
//!    the leftmost start with any match, then the largest reachable end.
//! 2. **Dissection** — [`Matcher::dissect`] walks the tree over the fixed
//!    `[lo, hi)` extent, choosing each split to make the *earlier* part as long
//!    (or, for non-greedy operators, as short) as possible, recording captures.
//!
//! Backreferences make the language non-regular, so a regex containing them is
//! matched by a backtracking path ([`Matcher::bt`]) that threads capture state
//! and is ordered to honour the same longest/shortest preferences.

use crate::ast::{Anchor, Node, Pref, case_variants};
use crate::defs::{Chr, DUPINF, REG_ICASE, REG_NLANCH, REG_NOTBOL, REG_NOTEOL};
use std::cell::{Cell, RefCell};
// A `BTreeMap` (not `HashMap`) keeps the engine free of any RNG/entropy
// dependency, so it embeds cleanly in freestanding / wasm / C-linked builds.
use std::collections::BTreeMap;
use tcl_core_types::RecursionLimit;

/// Half-open character span `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Total work budget shared by both matching strategies. The backtracking path
/// ([`Bt`]) has exponential worst cases (`(a+)+\1$` on a run of "a"s) and the
/// set-simulation path ([`Matcher::reach`]) is super-linear (plain `a*` on a
/// long input is O(n²), `(a*)*` cubic). Counting every elementary step against
/// one budget lets a pathological input bail in bounded time instead of hanging
/// or appearing to lock up.
///
/// Each unit guards one frontier-point expansion or one backtracking node visit,
/// so the bound on real work is the cap times a small constant (the per-step set
/// bookkeeping). A few million units therefore keeps even the worst case to a
/// fraction of a second, while staying orders of magnitude above what any
/// realistic pattern/input needs — the `reg.test` corpus never comes close.
const MATCH_FUEL: u64 = 4_000_000;

/// Recursion budget for [`Matcher::dissect`]/[`Matcher::dissect_seq`]/
/// [`Matcher::dissect_repeat`] (issue #996). `dissect_repeat`'s `min == 0`
/// branch recurses once per matched iteration of a repeated sub-pattern —
/// so a shallow, everyday pattern like `.*` or `a+` matched against a long
/// subject produces recursion depth proportional to the subject length,
/// independent of [`MAX_PARSE_DEPTH`](crate::parser) (which bounds only how
/// deeply *groups* nest in the pattern text, not how many characters a
/// repeat matches). [`MATCH_FUEL`] happens to bound this too in the common
/// case — `dissect_repeat`'s per-level "where does this iteration end"
/// check re-scans the *remaining* subject via a fresh, unmemoized
/// `reach_repeat` call, so total fuel spent across the whole recursion is
/// quadratic in depth and self-limits it to roughly
/// `sqrt(2 * MATCH_FUEL)` (~2800) before the shared budget runs out — but
/// that is a coincidence of the current fuel accounting, not a depth
/// guarantee, and empirically the two limits are close enough to each other
/// that it is not a comfortable margin. With the fuel budget temporarily
/// raised to rule out fuel exhaustion as a confound, unguarded recursion
/// overflowed the native stack (SIGABRT) between depth 2400 and 2420 on a
/// 2 MiB thread (`cargo test`'s per-test default) — i.e. dissection was
/// already only ~15% of the way from its fuel-imposed "natural" ceiling to
/// an actual crash. 256 leaves better than 9x margin under that measured
/// floor. Tripping this cap does not turn a real match into "no match" —
/// [`Matcher::search`] has already fixed the overall match extent
/// (`caps[0]`) by the time `dissect` runs, so the cap only affects
/// submatch spans nested inside a repeat iterating this deep; the fallback
/// dissects the remaining `[lo, hi)` span as a single unit (the same
/// "can't cleanly locate the next iteration boundary" fallback the
/// algorithm already uses elsewhere in this function), a reasonable
/// approximation rather than leaving those captures unset.
const MAX_DISSECT_DEPTH: RecursionLimit = RecursionLimit(256);

/// Recursion budget for the backtracking matcher's mutually-recursive
/// [`Bt::m`]/[`Bt::m_seq`]/[`Bt::m_repeat`]/[`Bt::m_star`]/[`Bt::m_backref`]
/// (issue #996) — the separate matching path used only when a pattern
/// contains a backreference. `m_star` recurses once per matched iteration
/// of a repeated sub-pattern (same shape as `dissect_repeat`, but via a
/// continuation closure rather than a plain call) and `m_backref` recurses
/// once per repetition of a quantified backreference (`\1*`); both are
/// independent of [`MATCH_FUEL`] as a *depth* bound — the fuel charge here
/// is 1 unit per node visit regardless of how much native stack that visit
/// costs, so a pattern like `a*(b)\1` or `(x)\1*` against a long subject
/// can recurse to a depth proportional to the subject length long before
/// the multi-million-unit budget is anywhere near exhausted. Empirically
/// (binary-searched via a throwaway probe spawning a worker thread with an
/// explicit stack size), unguarded input overflowed the native stack
/// (SIGABRT) between depth 2200 and 2300 for `m_star` and between 2400 and
/// 2500 for `m_backref`, both on a 2 MiB thread (`cargo test`'s per-test
/// default). 256 leaves better than 8x margin under the lower of those two
/// measured floors. On trip, every guarded function returns `false` —
/// mirroring the existing fuel-exhaustion fallback in [`Bt::m`]
/// (`if !self.spend_fuel() { return false; }`) immediately above — so a
/// pattern that recurses this deep cleanly reports no match along that
/// backtracking path rather than aborting the process.
const MAX_BT_DEPTH: RecursionLimit = RecursionLimit(256);

pub(crate) struct Matcher<'a> {
    subj: &'a [Chr],
    cflags: i32,
    eflags: i32,
    /// The whole RE prefers the shortest overall match (top-tree `SHORTER`).
    prefer_shortest: bool,
    memo: BTreeMap<(usize, usize), Vec<usize>>,
    /// Remaining work budget for the `reach` core (see [`MATCH_FUEL`]). Once it
    /// hits zero the reachability loops stop expanding, so a super-linear input
    /// terminates rather than hangs.
    fuel: u64,
}

fn is_word(c: Chr) -> bool {
    char::from_u32(c).is_some_and(|c| c.is_alphanumeric() || c == '_')
        || matches!(
            c,
            0x203F | 0x2040 | 0x2054 | 0xFE33 | 0xFE34 | 0xFE4D | 0xFE4E | 0xFE4F | 0xFF3F
        )
}

impl<'a> Matcher<'a> {
    pub(crate) fn new(
        subj: &'a [Chr],
        cflags: i32,
        eflags: i32,
        prefer_shortest: bool,
    ) -> Matcher<'a> {
        Matcher {
            subj,
            cflags,
            eflags,
            prefer_shortest,
            memo: BTreeMap::new(),
            fuel: MATCH_FUEL,
        }
    }

    fn len(&self) -> usize {
        self.subj.len()
    }

    /// Charge one unit of work against the reach budget, returning `false` once
    /// it is exhausted. Callers in the hot reachability loops use this to stop
    /// expanding the frontier on a pathological input (see [`MATCH_FUEL`]).
    fn spend_fuel(&mut self) -> bool {
        self.spend_fuel_n(1)
    }

    /// Charge `n` units at once, for loops whose per-step cost is proportional
    /// to the size of a reachable set (set copies, frontier expansions, dedup).
    /// Tying the charge to element count — not merely to call count — is what
    /// makes the budget a true bound on the cubic `(a*)*` blow-up, where each
    /// individual set can itself grow to O(input length).
    fn spend_fuel_n(&mut self, n: usize) -> bool {
        self.fuel = self.fuel.saturating_sub(n as u64);
        self.fuel > 0
    }

    fn lineanchor(&self) -> bool {
        self.cflags & REG_NLANCH != 0
    }

    /// Is the zero-width `anchor` satisfied at character position `pos`?
    fn anchor_ok(&self, anchor: Anchor, pos: usize) -> bool {
        let len = self.len();
        let left_word = pos > 0 && is_word(self.subj[pos - 1]);
        let right_word = pos < len && is_word(self.subj[pos]);
        match anchor {
            Anchor::Bol => {
                if pos == 0 {
                    self.eflags & REG_NOTBOL == 0
                } else {
                    self.lineanchor() && self.subj[pos - 1] == u32::from(b'\n')
                }
            }
            Anchor::Eol => {
                if pos == len {
                    self.eflags & REG_NOTEOL == 0
                } else {
                    self.lineanchor() && self.subj[pos] == u32::from(b'\n')
                }
            }
            Anchor::Bos => pos == 0,
            Anchor::Eos => pos == len,
            Anchor::WordBegin => !left_word && right_word,
            Anchor::WordEnd => left_word && !right_word,
            Anchor::WordBoundary => left_word != right_word,
            Anchor::NotWordBoundary => left_word == right_word,
        }
    }

    /// All end positions reachable by matching `node` starting at `pos`
    /// (sorted, deduplicated). Backreferences are treated optimistically here
    /// (see module note); the dissection / backtracking layer enforces them.
    fn reach(&mut self, node: &Node, pos: usize) -> Vec<usize> {
        let key = (std::ptr::from_ref(node) as usize, pos);
        if let Some(v) = self.memo.get(&key) {
            // A memo hit still copies the (possibly large) reachable set, which
            // is the per-step cost that drives the `(a*)*` cubic — charge for it
            // so the budget accounts for the copy, not just the lookup.
            let v = v.clone();
            self.spend_fuel_n(v.len());
            return v;
        }
        let out = self.reach_uncached(node, pos);
        self.memo.insert(key, out.clone());
        out
    }

    fn reach_uncached(&mut self, node: &Node, pos: usize) -> Vec<usize> {
        match node {
            Node::Empty => vec![pos],
            Node::Anchor(a) => {
                if self.anchor_ok(*a, pos) {
                    vec![pos]
                } else {
                    vec![]
                }
            }
            Node::Set(set) => {
                if pos < self.len() && set.matches(self.subj[pos]) {
                    vec![pos + 1]
                } else {
                    vec![]
                }
            }
            Node::Look { positive, sub } => {
                let sat = !self.reach(sub, pos).is_empty();
                if sat == *positive { vec![pos] } else { vec![] }
            }
            Node::Capture { sub, .. } => self.reach(sub, pos),
            Node::Concat(items) => self.reach_seq(items, pos),
            Node::Alt(branches) => {
                let mut set = Vec::new();
                for b in branches {
                    set.extend(self.reach(b, pos));
                }
                dedup_sorted(set)
            }
            Node::Repeat { sub, min, max, .. } => self.reach_repeat(sub, pos, *min, *max),
            Node::Backref { .. } => {
                // Optimistic: a backref can match its captured text; without the
                // capture we approximate it as reaching `pos` (a zero-width
                // match) regardless of `min`. The backtracking path handles real
                // backref matching.
                vec![pos]
            }
        }
    }

    /// Reachable ends after matching the concatenation `items[..]` from `pos`.
    fn reach_seq(&mut self, items: &[Node], pos: usize) -> Vec<usize> {
        let mut frontier = vec![pos];
        for it in items {
            let mut next = Vec::new();
            for &p in &frontier {
                // Charge per frontier point expanded; the sub-reach itself bills
                // for the set it copies. Bail (returning the partial frontier)
                // the moment the shared budget is spent.
                if !self.spend_fuel() {
                    return frontier;
                }
                next.extend(self.reach(it, p));
            }
            // The sort/dedup is proportional to the accumulated set size.
            self.spend_fuel_n(next.len());
            frontier = dedup_sorted(next);
            if frontier.is_empty() {
                break;
            }
        }
        frontier
    }

    /// Reachable ends after matching between `min` and `max` repetitions of
    /// `sub` from `pos` (`max == DUPINF` is unbounded). Empty iterations are
    /// kept (they hold the position but still count), so a nullable operand can
    /// satisfy a positive `min` — e.g. `()+` matches `""` and `(a?){2}` matches
    /// `"a"`. Termination: a fixpoint for unbounded `max`, a length+`min` cap
    /// otherwise.
    fn reach_repeat(&mut self, sub: &Node, pos: usize, min: i32, max: i32) -> Vec<usize> {
        let mut ends: Vec<usize> = Vec::new();
        if min <= 0 {
            ends.push(pos);
        }
        let mut frontier: Vec<usize> = vec![pos];
        let mut count = 0i32;
        // At most one progressing iteration per remaining char, plus enough
        // empty iterations to reach `min`, plus slack.
        let cap = if max >= crate::defs::DUPINF {
            (self.len() - pos) as i32 + min.max(0) + 2
        } else {
            max
        };
        while count < cap && !frontier.is_empty() {
            let mut next = Vec::new();
            for &p in &frontier {
                // Charge each operand re-reach; a nested star (`(a*)*`) makes
                // this loop cubic, so the budget bounds its total work.
                if !self.spend_fuel() {
                    return dedup_sorted(ends);
                }
                next.extend(self.reach(sub, p));
            }
            // The sort/dedup below scans the whole accumulated set; bill for it
            // so a frontier that grows with the input drains the budget.
            self.spend_fuel_n(next.len());
            next = dedup_sorted(next);
            count += 1;
            if count >= min {
                ends.extend(next.iter().copied());
            }
            if max < crate::defs::DUPINF && count >= max {
                break;
            }
            if next == frontier {
                // Fixpoint: more iterations add nothing new. If still short of
                // `min`, empty self-iterations pad up to it, so the stable set
                // is reachable at `min` as well.
                if count < min {
                    ends.extend(next.iter().copied());
                }
                break;
            }
            frontier = next;
        }
        dedup_sorted(ends)
    }

    /// Find the leftmost-longest match at or after `from`, returning capture
    /// spans (index 0 = whole match; `None` = non-participating).
    pub(crate) fn search(
        &mut self,
        root: &Node,
        nsub: usize,
        from: usize,
    ) -> Option<Vec<Option<Span>>> {
        for start in from..=self.len() {
            // Re-anchoring `reach` at every start makes the outer scan itself a
            // source of super-linear work; once the budget is spent, stop the
            // scan (a bailed search reports no match — the standard DoS guard).
            if !self.spend_fuel() {
                return None;
            }
            let ends = self.reach(root, start);
            // Leftmost start; then the overall length follows the top
            // preference — shortest for a non-greedy-led RE, else longest.
            let pick = if self.prefer_shortest {
                ends.iter().min()
            } else {
                ends.iter().max()
            };
            if let Some(&hi) = pick {
                let mut caps: Vec<Option<Span>> = vec![None; nsub + 1];
                caps[0] = Some(Span { start, end: hi });
                self.dissect(root, start, hi, 0, &mut caps);
                return Some(caps);
            }
        }
        None
    }

    /// Assign captures for `node` known to match exactly `[lo, hi)`.
    ///
    /// `depth` is the nesting level of this call (0 at the top, via
    /// [`Self::search`]); past [`MAX_DISSECT_DEPTH`] this stops recursing
    /// further — see that constant's doc comment for why that is safe.
    fn dissect(
        &mut self,
        node: &Node,
        lo: usize,
        hi: usize,
        depth: u32,
        caps: &mut [Option<Span>],
    ) {
        if MAX_DISSECT_DEPTH.exceeded(depth) {
            return;
        }
        match node {
            Node::Empty
            | Node::Anchor(_)
            | Node::Set(_)
            | Node::Look { .. }
            | Node::Backref { .. } => {}
            Node::Capture { subno, sub } => {
                caps[*subno] = Some(Span { start: lo, end: hi });
                self.dissect(sub, lo, hi, depth + 1, caps);
            }
            Node::Alt(branches) => {
                for b in branches {
                    if self.reach(b, lo).contains(&hi) {
                        self.dissect(b, lo, hi, depth + 1, caps);
                        return;
                    }
                }
            }
            Node::Concat(items) => self.dissect_seq(items, lo, hi, depth + 1, caps),
            Node::Repeat {
                sub,
                min,
                max,
                pref,
            } => {
                self.dissect_repeat(sub, lo, hi, *min, *max, *pref, depth + 1, caps);
            }
        }
    }

    fn dissect_seq(
        &mut self,
        items: &[Node],
        lo: usize,
        hi: usize,
        depth: u32,
        caps: &mut [Option<Span>],
    ) {
        if MAX_DISSECT_DEPTH.exceeded(depth) {
            return;
        }
        if items.is_empty() {
            return;
        }
        if items.len() == 1 {
            self.dissect(&items[0], lo, hi, depth + 1, caps);
            return;
        }
        let first = &items[0];
        let rest = &items[1..];
        // Candidate split points: ends of `first` that let `rest` finish at hi.
        let mut firsts = self.reach(first, lo);
        firsts.retain(|&m| m <= hi && self.reach_seq(rest, m).contains(&hi));
        // The earlier part is made as long (or, if it leads with a non-greedy
        // quantifier, as short) as possible.
        let pref = crate::ast::leading_pref(first).unwrap_or(Pref::Longer);
        let mid = match pref {
            Pref::Longer => firsts.into_iter().max(),
            Pref::Shorter => firsts.into_iter().min(),
        };
        if let Some(mid) = mid {
            self.dissect(first, lo, mid, depth + 1, caps);
            self.dissect_seq(rest, mid, hi, depth + 1, caps);
        }
    }

    fn dissect_repeat(
        &mut self,
        sub: &Node,
        lo: usize,
        hi: usize,
        min: i32,
        max: i32,
        pref: Pref,
        depth: u32,
        caps: &mut [Option<Span>],
    ) {
        if MAX_DISSECT_DEPTH.exceeded(depth) {
            // Can't cleanly locate the next iteration boundary within budget:
            // fall back to the same "dissect the remaining span as one unit"
            // approximation the algorithm already uses below when `firsts`
            // yields no candidate. This never changes whether the overall
            // match succeeded (`search` fixed `caps[0]` before `dissect` was
            // ever called) — only a capture nested this deep inside a single
            // repeat gets an approximate span instead of the exact final
            // iteration's.
            if lo < hi {
                self.dissect(sub, lo, hi, depth + 1, caps);
            }
            return;
        }
        if lo == hi && min == 0 {
            // Zero iterations: inner captures do not participate.
            return;
        }
        // POSIX records only the *final* iteration's submatches.
        let nmax = if max >= crate::defs::DUPINF {
            max
        } else {
            max - 1
        };
        if min >= 1 {
            // `x{m,n}` with m>=1 behaves as `x{m-1,n-1} x`: the final iteration
            // is a mandatory match after a (possibly long) prefix. Greedy
            // maximises the prefix, so the final iteration starts as late as
            // possible — and for a nullable operand it can be the empty match
            // at `hi` (e.g. `(a*)+` on "aaa" captures the inner group as the
            // empty `[3,3)`, and `(a+)+` on "foo" captures "o"). Candidate final
            // starts include `hi` itself.
            let prefix_ends = self.reach_repeat(sub, lo, min - 1, nmax);
            let starts: Vec<usize> = (lo..=hi)
                .filter(|&s| prefix_ends.contains(&s) && self.reach(sub, s).contains(&hi))
                .collect();
            let pick = match pref {
                Pref::Longer => starts.into_iter().max(),
                Pref::Shorter => starts.into_iter().min(),
            };
            if let Some(s) = pick {
                self.dissect(sub, s, hi, depth + 1, caps);
            }
            return;
        }
        // `x{0,n}` is a pure iteration with no mandatory final match: the last
        // recorded iteration is the last *non-empty* greedy chunk (no gratuitous
        // trailing empty iteration — e.g. `(a*)*` on "aaa" captures "aaa", not
        // the empty `[3,3)`). Walk forward, taking the longest (greedy) or
        // shortest (lazy) first iteration that still lets the rest complete,
        // then recurse; the final chunk reaching `hi` is the one we dissect.
        let firsts: Vec<usize> = self
            .reach(sub, lo)
            .into_iter()
            .filter(|&m| m > lo && m <= hi && self.reach_repeat(sub, m, 0, nmax).contains(&hi))
            .collect();
        let pick = match pref {
            Pref::Longer => firsts.into_iter().max(),
            Pref::Shorter => firsts.into_iter().min(),
        };
        match pick {
            Some(m) if m == hi => self.dissect(sub, lo, hi, depth + 1, caps),
            Some(m) => self.dissect_repeat(sub, m, hi, 0, nmax, pref, depth + 1, caps),
            None if lo < hi => self.dissect(sub, lo, hi, depth + 1, caps),
            None => {}
        }
    }
}

fn dedup_sorted(mut v: Vec<usize>) -> Vec<usize> {
    v.sort_unstable();
    v.dedup();
    v
}

/// Compare two codepoint slices, optionally case-insensitively (for `-nocase`
/// backreferences).
fn chr_eq(a: &[Chr], b: &[Chr], nocase: bool) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b)
        .all(|(&x, &y)| x == y || (nocase && case_variants(x).contains(&y)))
}

impl Matcher<'_> {
    /// Backtracking search for patterns containing backreferences. Finds the
    /// leftmost start, then — honouring the top preference — the longest (or,
    /// for a non-greedy-led RE, the shortest) total match, recording captures
    /// in greedy/lazy preference order.
    pub(crate) fn search_backref(
        &mut self,
        root: &Node,
        nsub: usize,
        from: usize,
    ) -> Option<Vec<Option<Span>>> {
        let bt = Bt {
            subj: self.subj,
            cflags: self.cflags,
            eflags: self.eflags,
            caps: RefCell::new(vec![None; nsub + 1]),
            fuel: Cell::new(MATCH_FUEL),
        };
        for start in from..=self.len() {
            // Try candidate end positions in preference order: shortest first
            // when the RE prefers the shortest match, else longest first.
            let mut found = None;
            let mut try_end = |end: usize| -> bool {
                *bt.caps.borrow_mut() = vec![None; nsub + 1];
                if bt.m(root, start, end, 0, &mut |p| p == end) {
                    let mut caps = bt.caps.borrow().clone();
                    caps[0] = Some(Span { start, end });
                    found = Some(caps);
                    true
                } else {
                    false
                }
            };
            let hit = if self.prefer_shortest {
                (start..=self.len()).any(&mut try_end)
            } else {
                (start..=self.len()).rev().any(&mut try_end)
            };
            if hit {
                return found;
            }
        }
        None
    }
}

/// Backtracking matcher with capture state, used only when backreferences are
/// present. Continuations carry "what must match after"; captures live in a
/// `RefCell` so nested continuations can record and restore them.
struct Bt<'a> {
    subj: &'a [Chr],
    cflags: i32,
    eflags: i32,
    caps: RefCell<Vec<Option<Span>>>,
    /// Remaining backtracking budget (see [`MATCH_FUEL`]). Held in a `Cell`
    /// because the matcher threads everything through `&self`; each node visit
    /// in [`Bt::m`] spends one unit, so an exponential search (`(a+)+\1$`) gives
    /// up in bounded time and reports no match rather than melting a core.
    fuel: Cell<u64>,
}

impl Bt<'_> {
    fn lineanchor(&self) -> bool {
        self.cflags & REG_NLANCH != 0
    }

    /// Spend one unit of backtracking budget, returning `false` when exhausted.
    /// Every [`Bt::m`] entry is one elementary step of the search, so charging
    /// here bounds the total number of backtracking states explored.
    fn spend_fuel(&self) -> bool {
        let remaining = self.fuel.get().saturating_sub(1);
        self.fuel.set(remaining);
        remaining > 0
    }

    fn anchor_ok(&self, anchor: Anchor, pos: usize) -> bool {
        let len = self.subj.len();
        let left_word = pos > 0 && is_word(self.subj[pos - 1]);
        let right_word = pos < len && is_word(self.subj[pos]);
        match anchor {
            Anchor::Bol => {
                if pos == 0 {
                    self.eflags & REG_NOTBOL == 0
                } else {
                    self.lineanchor() && self.subj[pos - 1] == u32::from(b'\n')
                }
            }
            Anchor::Eol => {
                if pos == len {
                    self.eflags & REG_NOTEOL == 0
                } else {
                    self.lineanchor() && self.subj[pos] == u32::from(b'\n')
                }
            }
            Anchor::Bos => pos == 0,
            Anchor::Eos => pos == len,
            Anchor::WordBegin => !left_word && right_word,
            Anchor::WordEnd => left_word && !right_word,
            Anchor::WordBoundary => left_word != right_word,
            Anchor::NotWordBoundary => left_word == right_word,
        }
    }

    /// Match `node` from `pos` (not consuming past `hi`); call `k(end)` for each
    /// way it matches, returning `true` as soon as `k` accepts.
    ///
    /// `depth` is the nesting level of this call (0 at the top, via
    /// [`Matcher::search_backref`]); past [`MAX_BT_DEPTH`] every guarded
    /// function in this `impl` returns `false` — see that constant's doc
    /// comment.
    fn m(
        &self,
        node: &Node,
        pos: usize,
        hi: usize,
        depth: u32,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        // One step of the backtracking search. When the budget is gone, treat
        // every remaining path as a non-match so an exponential pattern unwinds
        // promptly (the search then reports no match — the standard ReDoS guard).
        if !self.spend_fuel() {
            return false;
        }
        if MAX_BT_DEPTH.exceeded(depth) {
            return false;
        }
        match node {
            Node::Empty => k(pos),
            Node::Anchor(a) => self.anchor_ok(*a, pos) && k(pos),
            Node::Set(s) => pos < hi && s.matches(self.subj[pos]) && k(pos + 1),
            Node::Look { positive, sub } => {
                let mut matched = false;
                self.m(sub, pos, self.subj.len(), depth + 1, &mut |_| {
                    matched = true;
                    true
                });
                (matched == *positive) && k(pos)
            }
            Node::Capture { subno, sub } => {
                let sn = *subno;
                let start = pos;
                self.m(sub, pos, hi, depth + 1, &mut |end| {
                    let prev = self.caps.borrow()[sn];
                    self.caps.borrow_mut()[sn] = Some(Span { start, end });
                    if k(end) {
                        true
                    } else {
                        self.caps.borrow_mut()[sn] = prev;
                        false
                    }
                })
            }
            Node::Concat(items) => self.m_seq(items, 0, pos, hi, depth + 1, k),
            Node::Alt(branches) => {
                for b in branches {
                    if self.m(b, pos, hi, depth + 1, k) {
                        return true;
                    }
                }
                false
            }
            Node::Repeat {
                sub,
                min,
                max,
                pref,
            } => self.m_repeat(sub, *min, *max, *pref, pos, hi, depth + 1, k),
            Node::Backref {
                subno,
                min,
                max,
                pref,
            } => self.m_backref(*subno, *min, *max, *pref, 0, pos, hi, depth + 1, k),
        }
    }

    fn m_seq(
        &self,
        items: &[Node],
        i: usize,
        pos: usize,
        hi: usize,
        depth: u32,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        if MAX_BT_DEPTH.exceeded(depth) {
            return false;
        }
        if i == items.len() {
            return k(pos);
        }
        self.m(&items[i], pos, hi, depth + 1, &mut |p| {
            self.m_seq(items, i + 1, p, hi, depth + 1, k)
        })
    }

    /// Match `sub{min,max}` in the backtracking matcher, recording captures.
    ///
    /// Mirrors the C dissector (`citerdissect`) — and our own `dissect_repeat`:
    ///
    /// * `min >= 1` is Spencer's transform `x{m,n}` → `x{m-1,n-1} x`: a prefix
    ///   repeat followed by a **mandatory** final `x`. The final iteration is
    ///   the one whose captures stick, and for a nullable operand it can be the
    ///   empty match at the end (`(a*)+` on "aaa" captures `[3,3)`; `(a?){2}\1`
    ///   ends with the empty `[1,1)`). Greedy makes the prefix as long as
    ///   possible, so the final `x` lands as late as possible.
    /// * `min == 0` is a pure star: a zero-width iteration is **never** taken
    ///   (Tcl rejects zero-length matches in a min-0 repeat), so `(a*)?`/`(a*)*`
    ///   over "" take zero iterations and do not enter — hence do not capture —
    ///   their operand. This is why `((a*)?){2}\2` fails to match "": the inner
    ///   group never participates, so the backreference has nothing to match.
    fn m_repeat(
        &self,
        sub: &Node,
        min: i32,
        max: i32,
        pref: Pref,
        pos: usize,
        hi: usize,
        depth: u32,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        if MAX_BT_DEPTH.exceeded(depth) {
            return false;
        }
        if min >= 1 {
            let pmax = if max >= DUPINF { max } else { max - 1 };
            self.m_repeat(sub, min - 1, pmax, pref, pos, hi, depth + 1, &mut |mid| {
                self.m(sub, mid, hi, depth + 1, k)
            })
        } else {
            self.m_star(sub, max, pref, 0, pos, hi, depth + 1, k)
        }
    }

    /// Match a pure `sub{0,max}` star. Only **non-empty** iterations are taken
    /// (an empty one would not progress and is never recorded); `k(pos)` is the
    /// zero-or-more-iterations stop. Greedy tries another iteration first, lazy
    /// stops first.
    fn m_star(
        &self,
        sub: &Node,
        max: i32,
        pref: Pref,
        count: i32,
        pos: usize,
        hi: usize,
        depth: u32,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        if MAX_BT_DEPTH.exceeded(depth) {
            return false;
        }
        let can_more = max >= DUPINF || count < max;
        let more = |k: &mut dyn FnMut(usize) -> bool| {
            can_more
                && self.m(sub, pos, hi, depth + 1, &mut |p| {
                    p > pos && self.m_star(sub, max, pref, count + 1, p, hi, depth + 1, k)
                })
        };
        // The arms differ only in short-circuit order, which is load-bearing
        // (greedy takes more iterations first, lazy stops first; both have
        // capture side effects).
        #[allow(clippy::match_same_arms)]
        match pref {
            Pref::Longer => more(k) || k(pos),
            Pref::Shorter => k(pos) || more(k),
        }
    }

    // Threads the full backref-repeat state (subno/min/max/pref/count/pos/hi)
    // plus the continuation; bundling into a struct would obscure the recursion.
    //
    // `depth` guards this against the same unbounded-recursion class as
    // `m`/`m_seq`/`m_repeat`/`m_star` (issue #996): a quantified
    // backreference like `\1*` recurses once per repetition, independent of
    // any other function in this `impl` — see [`MAX_BT_DEPTH`].
    #[allow(clippy::too_many_arguments)]
    fn m_backref(
        &self,
        subno: usize,
        min: i32,
        max: i32,
        pref: Pref,
        count: i32,
        pos: usize,
        hi: usize,
        depth: u32,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        if MAX_BT_DEPTH.exceeded(depth) {
            return false;
        }
        let text: Vec<Chr> = match self.caps.borrow()[subno] {
            // A non-participating group makes the backreference fail outright
            // (Tcl/POSIX): it is not the same as a group that captured "".
            None => return false,
            Some(s) => self.subj[s.start..s.end].to_vec(),
        };
        if text.is_empty() {
            // A group that captured the empty string: the backref matches empty.
            return k(pos);
        }
        let l = text.len();
        let nocase = self.cflags & REG_ICASE != 0;
        let one = pos + l <= hi && chr_eq(&self.subj[pos..pos + l], &text, nocase);
        let can_more = (max >= DUPINF || count < max) && one;
        let can_stop = count >= min;
        let more = |k: &mut dyn FnMut(usize) -> bool| {
            can_more && self.m_backref(subno, min, max, pref, count + 1, pos + l, hi, depth + 1, k)
        };
        // See `m_repeat`: the arms differ only in short-circuit order, which is
        // load-bearing (greedy vs lazy, plus capture side effects).
        #[allow(clippy::match_same_arms)]
        match pref {
            Pref::Longer => more(k) || (can_stop && k(pos)),
            Pref::Shorter => (can_stop && k(pos)) || more(k),
        }
    }
}
