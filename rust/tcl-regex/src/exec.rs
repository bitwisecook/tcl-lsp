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

use crate::ast::{Anchor, CharSet, Node, Pref, case_variants};
use crate::defs::{Chr, DUPINF, REG_ICASE, REG_NLANCH, REG_NOTBOL, REG_NOTEOL};
use std::cell::{Cell, RefCell};
// A `BTreeMap` (not `HashMap`) keeps the engine free of any RNG/entropy
// dependency, so it embeds cleanly in freestanding / wasm / C-linked builds.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tcl_core_types::RecursionLimit;

/// Half-open character span `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Why a bounded search established neither a match nor a no-match
/// (`docs/design/compiler/value-evaluation.md` § *The typed precision
/// result*). A stopped search proves nothing: it is never a no-match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecStop {
    /// The shared work budget ran out, after `spent` units.
    Fuel {
        /// The units charged before the search stopped.
        spent: u64,
    },
    /// A recursion limit was reached.
    Depth {
        /// The limit that was reached.
        limit: u32,
    },
    /// The caller's cancellation token was set.
    Cancelled,
}

/// What one bounded search establishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecOutcome {
    /// The search ran to completion and matched: the capture spans, index 0
    /// the whole match, `None` a subexpression that did not participate.
    /// Every span comes from the exact path.
    Matched(Vec<Option<Span>>),
    /// The search ran to completion and did not match — the only answer
    /// that proves a negative.
    NoMatch,
    /// The search established neither, for the recorded reason.
    Stopped(ExecStop),
}

impl ExecOutcome {
    /// The capture spans of a completed match.
    #[must_use]
    pub fn matched(&self) -> Option<&[Option<Span>]> {
        match self {
            Self::Matched(groups) => Some(groups),
            Self::NoMatch | Self::Stopped(_) => None,
        }
    }
}

/// The limits one search runs under: the work budget, and the token a
/// caller sets to stop it. The token is read where the budget is charged,
/// so a stop reaches one long native match without a command boundary.
#[derive(Clone, Copy, Debug)]
pub struct ExecLimits<'c> {
    /// The work budget, in the engine's elementary steps.
    pub fuel: u64,
    /// Set by the caller to stop the search.
    pub cancel: Option<&'c AtomicBool>,
}

impl Default for ExecLimits<'_> {
    fn default() -> Self {
        Self {
            fuel: MATCH_FUEL,
            cancel: None,
        }
    }
}

/// Total work budget shared by both matching strategies, per search. The backtracking path
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
pub const MATCH_FUEL: u64 = 4_000_000;

/// Recursion budget for [`Matcher::dissect`]. Dissection walks a repeat's
/// iterations ([`Matcher::dissect_repeat`]) and a concatenation's items
/// ([`Matcher::dissect_seq`]) in loops, so its recursion depth is the
/// pattern's own structural nesting — groups within groups, a repeat's final
/// iteration within its operand — never the subject's length. An unguarded
/// per-iteration recursion overflowed the native stack (SIGABRT) between
/// depth 2400 and 2420 on a 2 MiB thread (`cargo test`'s per-test default);
/// 256 leaves better than 9x margin under that floor. A pattern nested deeper
/// than this stops the search with [`ExecStop::Depth`]: the captures it
/// could not assign are never approximated, and the match is never reported
/// with them missing.
const MAX_DISSECT_DEPTH: RecursionLimit = RecursionLimit(256);

/// Recursion budget for the backtracking matcher's mutually-recursive
/// [`Bt::m`]/[`Bt::m_seq`]/[`Bt::m_repeat`]/[`Bt::m_star`]/[`Bt::m_backref`] —
/// the separate matching path used only when a pattern contains a
/// backreference. A single-character operand's repeat and a run of literal
/// characters are matched in loops ([`Bt::m_run`], [`Bt::m_seq`]), and a
/// quantified backreference counts its repetitions, so only an operand with
/// choice points of its own recurses once per matched iteration. The fuel
/// charge is 1 unit per node visit regardless of how much native stack that
/// visit costs, so depth can run far ahead of the work budget: unguarded
/// input overflowed the native stack (SIGABRT) between depth 2200 and 2300
/// for `m_star` and between 2400 and 2500 for `m_backref` on a 2 MiB thread
/// (`cargo test`'s per-test default). 256 leaves better than 8x margin under
/// the lower of those. On trip the search records [`ExecStop::Depth`] and
/// unwinds: a branch it could not explore is never reported as a no-match.
const MAX_BT_DEPTH: RecursionLimit = RecursionLimit(256);

pub(crate) struct Matcher<'a> {
    subj: &'a [Chr],
    cflags: i32,
    eflags: i32,
    /// The whole RE prefers the shortest overall match (top-tree `SHORTER`).
    prefer_shortest: bool,
    memo: BTreeMap<(usize, usize), Vec<usize>>,
    /// Whether each node's subtree holds a capture, by node address: a
    /// subtree without one has nothing to dissect.
    captures: BTreeMap<usize, bool>,
    /// Remaining work budget for the `reach` core (see [`MATCH_FUEL`]). Once it
    /// hits zero the reachability loops stop expanding, so a super-linear input
    /// terminates rather than hangs.
    fuel: u64,
    /// The budget the search started with, so a stop can say what it spent.
    budget: u64,
    /// The caller's cancellation token.
    cancel: Option<&'a AtomicBool>,
    /// Why the search stopped, once it has: its sets may be partial, so its
    /// answer proves nothing.
    stop: Option<ExecStop>,
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
        limits: &ExecLimits<'a>,
    ) -> Matcher<'a> {
        Matcher {
            subj,
            cflags,
            eflags,
            prefer_shortest,
            memo: BTreeMap::new(),
            captures: BTreeMap::new(),
            fuel: limits.fuel,
            budget: limits.fuel,
            cancel: limits.cancel,
            stop: None,
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
    ///
    /// This is also the search's one cancellation point: the caller's token
    /// is read here, where every frontier expansion and set copy already
    /// passes. Exhaustion and cancellation are recorded as different stops.
    fn spend_fuel_n(&mut self, n: usize) -> bool {
        if self.stop.is_some() {
            return false;
        }
        if self
            .cancel
            .is_some_and(|token| token.load(Ordering::Relaxed))
        {
            self.stop = Some(ExecStop::Cancelled);
            return false;
        }
        self.fuel = self.fuel.saturating_sub(n as u64);
        if self.fuel == 0 {
            self.stop = Some(ExecStop::Fuel { spent: self.budget });
            return false;
        }
        true
    }

    /// Record that dissection reached [`MAX_DISSECT_DEPTH`].
    fn stop_at_depth(&mut self) {
        if self.stop.is_none() {
            self.stop = Some(ExecStop::Depth {
                limit: MAX_DISSECT_DEPTH.0,
            });
        }
    }

    /// Whether `node`'s subtree holds a capture — what dissecting it can
    /// assign.
    fn has_capture(&mut self, node: &Node) -> bool {
        let key = std::ptr::from_ref(node) as usize;
        if let Some(&known) = self.captures.get(&key) {
            return known;
        }
        let found = match node {
            Node::Capture { .. } => true,
            // A lookahead's parentheses never capture, and dissection never
            // enters one.
            Node::Empty
            | Node::Set(_)
            | Node::Anchor(_)
            | Node::Backref { .. }
            | Node::Look { .. } => false,
            Node::Repeat { sub, .. } => self.has_capture(sub),
            Node::Concat(items) | Node::Alt(items) => {
                let mut found = false;
                for item in items {
                    if self.has_capture(item) {
                        found = true;
                        break;
                    }
                }
                found
            }
        };
        self.captures.insert(key, found);
        found
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
            if max >= crate::defs::DUPINF && count >= min {
                // Unbounded, and every further iteration count is admitted:
                // what remains is the closure of the frontier under one more
                // iteration, and each position needs expanding only once —
                // re-expanding the whole frontier per count made `(a+)+` over
                // n characters cubic.
                return self.reach_closure(sub, &frontier, ends);
            }
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

    /// Every end the frontier reaches under zero or more further iterations
    /// of `sub`, added to `ends`: a worklist that expands each position
    /// once. The same set the per-count loop of [`Self::reach_repeat`]
    /// converges to, since ends only ever move forward.
    fn reach_closure(&mut self, sub: &Node, frontier: &[usize], ends: Vec<usize>) -> Vec<usize> {
        let mut seen: Vec<usize> = frontier.to_vec();
        seen.sort_unstable();
        seen.dedup();
        let mut ends = ends;
        ends.extend(seen.iter().copied());
        let mut work = seen.clone();
        while let Some(p) = work.pop() {
            if !self.spend_fuel() {
                break;
            }
            for next in self.reach(sub, p) {
                if let Err(at) = seen.binary_search(&next) {
                    seen.insert(at, next);
                    ends.push(next);
                    work.push(next);
                }
            }
        }
        self.spend_fuel_n(ends.len());
        dedup_sorted(ends)
    }

    /// Find the leftmost-longest match at or after `from`: the capture spans
    /// (index 0 = whole match; `None` = non-participating), a completed
    /// no-match, or the stop that left the search incomplete — a partial
    /// reachable set proves neither the extent nor its absence.
    pub(crate) fn search(&mut self, root: &Node, nsub: usize, from: usize) -> ExecOutcome {
        for start in from..=self.len() {
            // Re-anchoring `reach` at every start makes the outer scan itself a
            // source of super-linear work; once the budget is spent, stop.
            if !self.spend_fuel() {
                break;
            }
            let ends = self.reach(root, start);
            if self.stop.is_some() {
                break;
            }
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
                return match self.stop {
                    Some(stop) => ExecOutcome::Stopped(stop),
                    None => ExecOutcome::Matched(caps),
                };
            }
        }
        match self.stop {
            Some(stop) => ExecOutcome::Stopped(stop),
            None => ExecOutcome::NoMatch,
        }
    }

    /// Assign captures for `node` known to match exactly `[lo, hi)`.
    ///
    /// `depth` is the nesting level of this call (0 at the top, via
    /// [`Self::search`]): the pattern's structural nesting, since repeats
    /// and concatenations are walked in loops. Past [`MAX_DISSECT_DEPTH`]
    /// the search stops ([`ExecStop::Depth`]) rather than leave a capture
    /// unassigned. A subtree without a capture has nothing to assign and is
    /// not walked.
    fn dissect(
        &mut self,
        node: &Node,
        lo: usize,
        hi: usize,
        depth: u32,
        caps: &mut [Option<Span>],
    ) {
        if self.stop.is_some() || !self.has_capture(node) {
            return;
        }
        if MAX_DISSECT_DEPTH.exceeded(depth) {
            self.stop_at_depth();
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

    /// Assign captures across the concatenation `items` known to match
    /// exactly `[lo, hi)`, item by item: each takes the longest (or, when it
    /// leads with a non-greedy quantifier, the shortest) extent that still
    /// lets the rest finish at `hi`.
    fn dissect_seq(
        &mut self,
        items: &[Node],
        lo: usize,
        hi: usize,
        depth: u32,
        caps: &mut [Option<Span>],
    ) {
        let mut lo = lo;
        for (i, first) in items.iter().enumerate() {
            if self.stop.is_some() {
                return;
            }
            let rest = &items[i + 1..];
            if rest.is_empty() {
                self.dissect(first, lo, hi, depth, caps);
                return;
            }
            // Candidate split points: ends of `first` that let `rest` finish
            // at hi.
            let mut firsts = self.reach(first, lo);
            firsts.retain(|&m| m <= hi && self.reach_seq(rest, m).contains(&hi));
            // The earlier part is made as long (or, if it leads with a
            // non-greedy quantifier, as short) as possible.
            let pref = crate::ast::leading_pref(first).unwrap_or(Pref::Longer);
            let mid = match pref {
                Pref::Longer => firsts.into_iter().max(),
                Pref::Shorter => firsts.into_iter().min(),
            };
            let Some(mid) = mid else {
                return;
            };
            self.dissect(first, lo, mid, depth, caps);
            lo = mid;
        }
    }

    /// Assign captures for `sub{min,max}` known to match exactly `[lo, hi)`.
    /// POSIX records only the final iteration's submatches, so the walk
    /// finds where the final iteration begins and dissects that one.
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
                self.dissect(sub, s, hi, depth, caps);
            }
            return;
        }
        // `x{0,n}` is a pure iteration with no mandatory final match: the last
        // recorded iteration is the last *non-empty* greedy chunk (no gratuitous
        // trailing empty iteration — e.g. `(a*)*` on "aaa" captures "aaa", not
        // the empty `[3,3)`). Walk forward, taking the longest (greedy) or
        // shortest (lazy) first iteration that still lets the rest complete;
        // the final chunk reaching `hi` is the one dissected. The walk is a
        // loop, so a repeat iterating once per subject character nests no
        // deeper than one that iterates once.
        //
        // Unbounded, "the rest completes from m" is one question per position,
        // answered once for all of them from `hi` backwards: the per-candidate
        // re-reach it replaces made a long run quadratic.
        let finishes = if max >= crate::defs::DUPINF {
            Some(self.finishing_positions(sub, lo, hi))
        } else {
            None
        };
        let (mut lo, mut max) = (lo, max);
        loop {
            if self.stop.is_some() || lo == hi {
                return;
            }
            let nmax = if max >= crate::defs::DUPINF {
                max
            } else {
                max - 1
            };
            let mut firsts = self.reach(sub, lo);
            firsts.retain(|&m| {
                m > lo
                    && m <= hi
                    && match &finishes {
                        Some(finishes) => finishes[m - lo],
                        None => self.reach_repeat(sub, m, 0, nmax).contains(&hi),
                    }
            });
            let pick = match pref {
                Pref::Longer => firsts.into_iter().max(),
                Pref::Shorter => firsts.into_iter().min(),
            };
            match pick {
                Some(m) if m == hi => {
                    self.dissect(sub, lo, hi, depth, caps);
                    return;
                }
                Some(m) => {
                    lo = m;
                    max = nmax;
                }
                None if lo < hi => {
                    self.dissect(sub, lo, hi, depth, caps);
                    return;
                }
                None => return,
            }
        }
    }

    /// For each position `p` in `[lo, hi]`, whether zero or more non-empty
    /// iterations of `sub` from `p` end exactly at `hi` (index `p - lo`).
    /// Iterations only move forward, so a backward pass from `hi` settles
    /// every position once.
    fn finishing_positions(&mut self, sub: &Node, lo: usize, hi: usize) -> Vec<bool> {
        let mut finishes = vec![false; hi - lo + 1];
        finishes[hi - lo] = true;
        for p in (lo..hi).rev() {
            if !self.spend_fuel() {
                break;
            }
            finishes[p - lo] = self
                .reach(sub, p)
                .into_iter()
                .any(|m| m > p && m <= hi && finishes[m - lo]);
        }
        finishes
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
    /// in greedy/lazy preference order. A search the budget, a recursion
    /// limit or the caller stopped is [`ExecOutcome::Stopped`], even when a
    /// later candidate matched: the preferred one may have been cut short.
    pub(crate) fn search_backref(&mut self, root: &Node, nsub: usize, from: usize) -> ExecOutcome {
        let bt = Bt {
            subj: self.subj,
            cflags: self.cflags,
            eflags: self.eflags,
            caps: RefCell::new(vec![None; nsub + 1]),
            fuel: Cell::new(self.budget),
            budget: self.budget,
            cancel: self.cancel,
            stop: Cell::new(None),
        };
        for start in from..=self.len() {
            // Try candidate end positions in preference order: shortest first
            // when the RE prefers the shortest match, else longest first.
            let mut found = None;
            let mut try_end = |end: usize| -> bool {
                if bt.stop.get().is_some() {
                    return true;
                }
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
            if let Some(stop) = bt.stop.get() {
                return ExecOutcome::Stopped(stop);
            }
            if hit && let Some(caps) = found {
                return ExecOutcome::Matched(caps);
            }
        }
        match bt.stop.get() {
            Some(stop) => ExecOutcome::Stopped(stop),
            None => ExecOutcome::NoMatch,
        }
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
    /// in [`Bt::m`] spends one unit, so an exponential search (`(a+)+\1$`)
    /// stops in bounded time rather than melting a core.
    fuel: Cell<u64>,
    /// The budget the search started with.
    budget: u64,
    /// The caller's cancellation token, read where the budget is charged.
    cancel: Option<&'a AtomicBool>,
    /// Why the search stopped, once it has: every path explored after that
    /// fails, so the search's answer proves nothing.
    stop: Cell<Option<ExecStop>>,
}

impl Bt<'_> {
    fn lineanchor(&self) -> bool {
        self.cflags & REG_NLANCH != 0
    }

    /// Spend one unit of backtracking budget, returning `false` when exhausted.
    /// Every [`Bt::m`] entry is one elementary step of the search, so charging
    /// here bounds the total number of backtracking states explored. The
    /// caller's cancellation token is read here too; exhaustion and
    /// cancellation are recorded as different stops.
    fn spend_fuel(&self) -> bool {
        if self.stop.get().is_some() {
            return false;
        }
        if self
            .cancel
            .is_some_and(|token| token.load(Ordering::Relaxed))
        {
            self.stop.set(Some(ExecStop::Cancelled));
            return false;
        }
        let remaining = self.fuel.get().saturating_sub(1);
        self.fuel.set(remaining);
        if remaining == 0 {
            self.stop.set(Some(ExecStop::Fuel { spent: self.budget }));
            return false;
        }
        true
    }

    /// Whether `depth` is past [`MAX_BT_DEPTH`], recording the stop when it
    /// is: the branch not explored is not a no-match.
    fn depth_exceeded(&self, depth: u32) -> bool {
        if !MAX_BT_DEPTH.exceeded(depth) {
            return false;
        }
        if self.stop.get().is_none() {
            self.stop.set(Some(ExecStop::Depth {
                limit: MAX_BT_DEPTH.0,
            }));
        }
        true
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
        // One step of the backtracking search. When the budget is gone, every
        // remaining path fails so an exponential pattern unwinds promptly; the
        // recorded stop makes the search's answer a stop, not a no-match.
        if !self.spend_fuel() {
            return false;
        }
        if self.depth_exceeded(depth) {
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
            // A single character repeated has no choice points of its own:
            // count the run and try each admissible length.
            Node::Repeat {
                sub,
                min,
                max,
                pref,
            } => match sub.as_ref() {
                Node::Set(set) => self.m_run(set, *min, *max, *pref, pos, hi, k),
                _ => self.m_repeat(sub, *min, *max, *pref, pos, hi, depth + 1, k),
            },
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
        if self.depth_exceeded(depth) {
            return false;
        }
        // A run of single characters has exactly one way to match, so it is
        // matched in a loop: a long literal nests no deeper than a short one.
        let (mut i, mut pos) = (i, pos);
        while let Some(Node::Set(set)) = items.get(i) {
            if !self.spend_fuel() || pos >= hi || !set.matches(self.subj[pos]) {
                return false;
            }
            i += 1;
            pos += 1;
        }
        if i == items.len() {
            return k(pos);
        }
        self.m(&items[i], pos, hi, depth + 1, &mut |p| {
            self.m_seq(items, i + 1, p, hi, depth + 1, k)
        })
    }

    /// Match `set{min,max}`: one character repeated has no choice points of
    /// its own, so the run is counted and each admissible length tried in
    /// preference order — longest first for greedy, shortest first for lazy
    /// — exactly the order [`Self::m_repeat`] and [`Self::m_star`] reach
    /// them in, without a native frame per iteration.
    fn m_run(
        &self,
        set: &CharSet,
        min: i32,
        max: i32,
        pref: Pref,
        pos: usize,
        hi: usize,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        let mut run = 0usize;
        while (max >= DUPINF || i32::try_from(run).is_ok_and(|run| run < max))
            && pos + run < hi
            && set.matches(self.subj[pos + run])
        {
            if !self.spend_fuel() {
                return false;
            }
            run += 1;
        }
        let least = usize::try_from(min.max(0)).unwrap_or(0);
        if run < least {
            return false;
        }
        match pref {
            Pref::Longer => (least..=run).rev().any(|reps| k(pos + reps)),
            Pref::Shorter => (least..=run).any(|reps| k(pos + reps)),
        }
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
        if self.depth_exceeded(depth) {
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
        if self.depth_exceeded(depth) {
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
    // plus the continuation; bundling into a struct would obscure the loop.
    //
    // `depth` still guards entry against unrelated deep nesting from the rest
    // of the pattern (e.g. this backref sitting inside many levels of
    // alternation/grouping) — see [`MAX_BT_DEPTH`] — but repetition itself no
    // longer recurses. A quantified backreference like `\1*` always matches
    // the same fixed `text` at every repetition (unlike `m_star`'s general
    // sub-pattern, which can have its own nested choice points per
    // iteration), so the whole repeat has no internal backtracking of its
    // own: greedily counting how far `text` repeats from `pos` and then
    // trying the continuation at each candidate stop count (longest-first for
    // greedy, shortest-first for lazy) is exactly equivalent to the original
    // one-native-frame-per-repetition recursion, without its native-stack
    // cost. This avoids a correctness hazard the original recursive form
    // had: capping recursion depth at [`MAX_BT_DEPTH`] (256) would make an
    // anchored pattern like
    // `(a)\1*$` spuriously fail to match ordinary, non-pathological input —
    // a run of 300 repeated characters is unremarkable in real text — instead
    // of merely bounding native stack use on truly pathological input.
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
        if self.depth_exceeded(depth) {
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
        // Greedily count the maximum further repetitions possible from `pos`,
        // capped by `max` (DUPINF means unbounded).
        let mut max_reps = 0i32;
        let mut p = pos;
        while (max >= DUPINF || count + max_reps < max)
            && p + l <= hi
            && chr_eq(&self.subj[p..p + l], &text, nocase)
        {
            max_reps += 1;
            p += l;
        }
        let try_stop = |reps: i32, k: &mut dyn FnMut(usize) -> bool| {
            count + reps >= min && k(pos + reps as usize * l)
        };
        // Greedy tries the most repetitions first and backs off; lazy tries
        // the fewest first and adds more — mirroring `m_star`/`m_repeat`'s
        // short-circuit order (load-bearing: capture side effects happen in
        // `k`, called at most once per candidate, in this exact order).
        match pref {
            Pref::Longer => (0..=max_reps).rev().any(|reps| try_stop(reps, k)),
            Pref::Shorter => (0..=max_reps).any(|reps| try_stop(reps, k)),
        }
    }
}
