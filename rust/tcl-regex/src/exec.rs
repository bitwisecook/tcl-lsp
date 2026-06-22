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
use std::cell::RefCell;
use std::collections::HashMap;

/// Half-open character span `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

pub(crate) struct Matcher<'a> {
    subj: &'a [Chr],
    cflags: i32,
    eflags: i32,
    /// The whole RE prefers the shortest overall match (top-tree `SHORTER`).
    prefer_shortest: bool,
    memo: HashMap<(usize, usize), Vec<usize>>,
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
            memo: HashMap::new(),
        }
    }

    fn len(&self) -> usize {
        self.subj.len()
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
            return v.clone();
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
                next.extend(self.reach(it, p));
            }
            frontier = dedup_sorted(next);
            if frontier.is_empty() {
                break;
            }
        }
        frontier
    }

    /// Reachable ends after matching between `min` and `max` repetitions of
    /// `sub` from `pos` (`max == DUPINF` is unbounded). Guards against
    /// zero-width iterations looping forever.
    fn reach_repeat(&mut self, sub: &Node, pos: usize, min: i32, max: i32) -> Vec<usize> {
        let mut ends: Vec<usize> = Vec::new();
        let mut frontier: Vec<usize> = vec![pos];
        let mut count = 0i32;
        if min <= 0 {
            ends.push(pos);
        }
        let cap = if max >= crate::defs::DUPINF {
            // Bounded by subject length: at most len-pos progressing iterations.
            (self.len() - pos) as i32 + 2
        } else {
            max
        };
        let mut seen: Vec<usize> = vec![pos];
        while count < cap && !frontier.is_empty() {
            let mut next = Vec::new();
            for &p in &frontier {
                for e in self.reach(sub, p) {
                    if e > p {
                        next.push(e);
                    }
                }
            }
            next = dedup_sorted(next);
            // Keep only progress we haven't already recorded as a frontier, to
            // terminate on cycles.
            next.retain(|e| !seen.contains(e));
            if next.is_empty() {
                break;
            }
            count += 1;
            seen.extend(next.iter().copied());
            if count + 1 > min || min <= count + 1 {
                // record once count of completed iterations >= min
            }
            if count >= min {
                ends.extend(next.iter().copied());
            }
            if max < crate::defs::DUPINF && count >= max {
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
                self.dissect(root, start, hi, &mut caps);
                return Some(caps);
            }
        }
        None
    }

    /// Assign captures for `node` known to match exactly `[lo, hi)`.
    fn dissect(&mut self, node: &Node, lo: usize, hi: usize, caps: &mut [Option<Span>]) {
        match node {
            Node::Empty
            | Node::Anchor(_)
            | Node::Set(_)
            | Node::Look { .. }
            | Node::Backref { .. } => {}
            Node::Capture { subno, sub } => {
                caps[*subno] = Some(Span { start: lo, end: hi });
                self.dissect(sub, lo, hi, caps);
            }
            Node::Alt(branches) => {
                for b in branches {
                    if self.reach(b, lo).contains(&hi) {
                        self.dissect(b, lo, hi, caps);
                        return;
                    }
                }
            }
            Node::Concat(items) => self.dissect_seq(items, lo, hi, caps),
            Node::Repeat {
                sub,
                min,
                max,
                pref,
            } => {
                self.dissect_repeat(sub, lo, hi, *min, *max, *pref, caps);
            }
        }
    }

    fn dissect_seq(&mut self, items: &[Node], lo: usize, hi: usize, caps: &mut [Option<Span>]) {
        if items.is_empty() {
            return;
        }
        if items.len() == 1 {
            self.dissect(&items[0], lo, hi, caps);
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
            self.dissect(first, lo, mid, caps);
            self.dissect_seq(rest, mid, hi, caps);
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
        caps: &mut [Option<Span>],
    ) {
        if lo == hi && min == 0 {
            // Zero iterations: inner captures do not participate.
            return;
        }
        // POSIX records only the *final* iteration's submatches. Find the start
        // `s` of that final iteration: a position in `[lo, hi)` where `sub`
        // matches `[s, hi)` and the prefix `[lo, s)` decomposes into the
        // remaining (min-1 .. max-1) iterations. A greedy outer quantifier
        // maximises the prefix (so the final iteration starts as late as
        // possible — e.g. `(([a-z]+)+)` on "foo" captures the inner group as
        // "o", not "foo"); a lazy one minimises it.
        let nmin = (min - 1).max(0);
        let nmax = if max >= crate::defs::DUPINF {
            max
        } else {
            max - 1
        };
        let prefix_ends = self.reach_repeat(sub, lo, nmin, nmax);
        let starts: Vec<usize> = (lo..hi)
            .filter(|&s| prefix_ends.contains(&s) && self.reach(sub, s).contains(&hi))
            .collect();
        let pick = match pref {
            Pref::Longer => starts.into_iter().max(),
            Pref::Shorter => starts.into_iter().min(),
        };
        if let Some(s) = pick {
            self.dissect(sub, s, hi, caps);
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
    /// leftmost start, then the longest total match (trying ends descending),
    /// recording captures in greedy/lazy preference order.
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
        };
        for start in from..=self.len() {
            for end in (start..=self.len()).rev() {
                *bt.caps.borrow_mut() = vec![None; nsub + 1];
                if bt.m(root, start, end, &mut |p| p == end) {
                    let mut caps = bt.caps.borrow().clone();
                    caps[0] = Some(Span { start, end });
                    return Some(caps);
                }
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
}

impl Bt<'_> {
    fn lineanchor(&self) -> bool {
        self.cflags & REG_NLANCH != 0
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
    fn m(&self, node: &Node, pos: usize, hi: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        match node {
            Node::Empty => k(pos),
            Node::Anchor(a) => self.anchor_ok(*a, pos) && k(pos),
            Node::Set(s) => pos < hi && s.matches(self.subj[pos]) && k(pos + 1),
            Node::Look { positive, sub } => {
                let mut matched = false;
                self.m(sub, pos, self.subj.len(), &mut |_| {
                    matched = true;
                    true
                });
                (matched == *positive) && k(pos)
            }
            Node::Capture { subno, sub } => {
                let sn = *subno;
                let start = pos;
                self.m(sub, pos, hi, &mut |end| {
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
            Node::Concat(items) => self.m_seq(items, 0, pos, hi, k),
            Node::Alt(branches) => {
                for b in branches {
                    if self.m(b, pos, hi, k) {
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
            } => self.m_repeat(sub, *min, *max, *pref, 0, pos, hi, k),
            Node::Backref {
                subno,
                min,
                max,
                pref,
            } => self.m_backref(*subno, *min, *max, *pref, 0, pos, hi, k),
        }
    }

    fn m_seq(
        &self,
        items: &[Node],
        i: usize,
        pos: usize,
        hi: usize,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        if i == items.len() {
            return k(pos);
        }
        self.m(&items[i], pos, hi, &mut |p| {
            self.m_seq(items, i + 1, p, hi, k)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn m_repeat(
        &self,
        sub: &Node,
        min: i32,
        max: i32,
        pref: Pref,
        count: i32,
        pos: usize,
        hi: usize,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        let can_more = max >= DUPINF || count < max;
        let can_stop = count >= min;
        let more = |k: &mut dyn FnMut(usize) -> bool| {
            can_more
                && self.m(sub, pos, hi, &mut |p| {
                    p != pos && self.m_repeat(sub, min, max, pref, count + 1, p, hi, k)
                })
        };
        // The two arms differ only in evaluation order, but `more`/`k` have
        // side effects (they record captures) and short-circuit, so the order
        // is what implements greedy-vs-lazy. They are NOT interchangeable.
        #[allow(clippy::match_same_arms)]
        match pref {
            Pref::Longer => more(k) || (can_stop && k(pos)),
            Pref::Shorter => (can_stop && k(pos)) || more(k),
        }
    }

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
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
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
            can_more && self.m_backref(subno, min, max, pref, count + 1, pos + l, hi, k)
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
