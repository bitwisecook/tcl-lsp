//! Byte-faithful port of Python's `difflib.unified_diff` (and the
//! `SequenceMatcher` machinery it rides on) restricted to the
//! line-oriented string-sequence use the CLI diff verbs need.
//!
//! The output — `--- ` / `+++ ` file headers, `@@ -a,b +c,d @@` hunk
//! headers, and the ` ` / `-` / `+` line prefixes — reproduces `CPython`'s
//! `difflib` exactly, including the `autojunk` popular-element purge that
//! kicks in for sequences of 200+ lines and the leading/trailing
//! context-group fixups in `get_grouped_opcodes`.
//!
//! Lines carry their own terminators (the caller splits with
//! `keepends=True` semantics), so this module never appends or strips
//! newlines on the data lines; only the control lines get the `lineterm`
//! (`"\n"`) appended, matching `unified_diff`'s default. Each generated
//! line is returned as its own `String` (mirroring Python's generator),
//! so the caller can `rstrip("\n")` per line and count them.

use std::collections::HashMap;

/// A matching block `a[i..i+size] == b[j..j+size]`.
#[derive(Clone, Copy)]
struct Match {
    i: usize,
    j: usize,
    size: usize,
}

/// An opcode 5-tuple `(tag, i1, i2, j1, j2)`.
#[derive(Clone, Copy)]
struct Opcode {
    tag: Tag,
    i1: usize,
    i2: usize,
    j1: usize,
    j2: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
    Equal,
    Replace,
    Delete,
    Insert,
}

/// Port of `difflib.SequenceMatcher` (with `isjunk=None`, `autojunk=True`).
struct SequenceMatcher<'a> {
    a: &'a [&'a str],
    b: &'a [&'a str],
    /// `b2j`: for each element of `b`, the sorted indices it occurs at, with
    /// "popular" autojunk elements purged.
    b2j: HashMap<&'a str, Vec<usize>>,
}

impl<'a> SequenceMatcher<'a> {
    fn new(a: &'a [&'a str], b: &'a [&'a str]) -> Self {
        let mut sm = SequenceMatcher {
            a,
            b,
            b2j: HashMap::new(),
        };
        sm.chain_b();
        sm
    }

    /// Port of `SequenceMatcher.__chain_b` (no `isjunk`; `autojunk=True`).
    fn chain_b(&mut self) {
        let b = self.b;
        let mut b2j: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, &elt) in b.iter().enumerate() {
            b2j.entry(elt).or_default().push(i);
        }
        // Purge popular elements that are not junk (autojunk).
        let n = b.len();
        if n >= 200 {
            let ntest = n / 100 + 1;
            let popular: Vec<&str> = b2j
                .iter()
                .filter(|(_, idxs)| idxs.len() > ntest)
                .map(|(elt, _)| *elt)
                .collect();
            for elt in popular {
                b2j.remove(elt);
            }
        }
        self.b2j = b2j;
    }

    /// Port of `SequenceMatcher.find_longest_match` (no junk, so the
    /// junk-extension passes are no-ops and omitted).
    #[allow(clippy::similar_names, clippy::needless_range_loop)]
    fn find_longest_match(&self, alo: usize, ahi: usize, blo: usize, bhi: usize) -> Match {
        let a = self.a;
        let b2j = &self.b2j;
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0usize);
        let mut j2len: HashMap<usize, usize> = HashMap::new();
        let empty: Vec<usize> = Vec::new();
        for i in alo..ahi {
            let mut newj2len: HashMap<usize, usize> = HashMap::new();
            let indices = b2j.get(a[i]).unwrap_or(&empty);
            for &j in indices {
                if j < blo {
                    continue;
                }
                if j >= bhi {
                    break;
                }
                let k = j2len.get(&j.wrapping_sub(1)).copied().unwrap_or(0) + 1;
                newj2len.insert(j, k);
                if k > bestsize {
                    besti = i + 1 - k;
                    bestj = j + 1 - k;
                    bestsize = k;
                }
            }
            j2len = newj2len;
        }
        while besti > alo && bestj > blo && a[besti - 1] == self.b[bestj - 1] {
            besti -= 1;
            bestj -= 1;
            bestsize += 1;
        }
        while besti + bestsize < ahi
            && bestj + bestsize < bhi
            && a[besti + bestsize] == self.b[bestj + bestsize]
        {
            bestsize += 1;
        }
        Match {
            i: besti,
            j: bestj,
            size: bestsize,
        }
    }

    /// Port of `SequenceMatcher.get_matching_blocks`.
    fn get_matching_blocks(&self) -> Vec<Match> {
        let la = self.a.len();
        let lb = self.b.len();
        let mut queue: Vec<(usize, usize, usize, usize)> = vec![(0, la, 0, lb)];
        let mut matching_blocks: Vec<Match> = Vec::new();
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let m = self.find_longest_match(alo, ahi, blo, bhi);
            let (i, j, k) = (m.i, m.j, m.size);
            if k > 0 {
                matching_blocks.push(m);
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i + k < ahi && j + k < bhi {
                    queue.push((i + k, ahi, j + k, bhi));
                }
            }
        }
        matching_blocks.sort_by_key(|m| (m.i, m.j, m.size));

        let (mut i1, mut j1, mut k1) = (0usize, 0usize, 0usize);
        let mut non_adjacent: Vec<Match> = Vec::new();
        for m in &matching_blocks {
            let (i2, j2, k2) = (m.i, m.j, m.size);
            if i1 + k1 == i2 && j1 + k1 == j2 {
                k1 += k2;
            } else {
                if k1 > 0 {
                    non_adjacent.push(Match {
                        i: i1,
                        j: j1,
                        size: k1,
                    });
                }
                i1 = i2;
                j1 = j2;
                k1 = k2;
            }
        }
        if k1 > 0 {
            non_adjacent.push(Match {
                i: i1,
                j: j1,
                size: k1,
            });
        }
        non_adjacent.push(Match {
            i: la,
            j: lb,
            size: 0,
        });
        non_adjacent
    }

    /// Port of `SequenceMatcher.get_opcodes`.
    fn get_opcodes(&self) -> Vec<Opcode> {
        let (mut i, mut j) = (0usize, 0usize);
        let mut answer: Vec<Opcode> = Vec::new();
        for m in self.get_matching_blocks() {
            let (ai, bj, size) = (m.i, m.j, m.size);
            let tag = if i < ai && j < bj {
                Some(Tag::Replace)
            } else if i < ai {
                Some(Tag::Delete)
            } else if j < bj {
                Some(Tag::Insert)
            } else {
                None
            };
            if let Some(tag) = tag {
                answer.push(Opcode {
                    tag,
                    i1: i,
                    i2: ai,
                    j1: j,
                    j2: bj,
                });
            }
            i = ai + size;
            j = bj + size;
            if size > 0 {
                answer.push(Opcode {
                    tag: Tag::Equal,
                    i1: ai,
                    i2: i,
                    j1: bj,
                    j2: j,
                });
            }
        }
        answer
    }

    /// Port of `SequenceMatcher.get_grouped_opcodes`.
    fn get_grouped_opcodes(&self, n: usize) -> Vec<Vec<Opcode>> {
        let mut codes = self.get_opcodes();
        if codes.is_empty() {
            codes = vec![Opcode {
                tag: Tag::Equal,
                i1: 0,
                i2: 1,
                j1: 0,
                j2: 1,
            }];
        }
        if codes[0].tag == Tag::Equal {
            let c = &mut codes[0];
            c.i1 = c.i1.max(c.i2.saturating_sub(n));
            c.j1 = c.j1.max(c.j2.saturating_sub(n));
        }
        let last = codes.len() - 1;
        if codes[last].tag == Tag::Equal {
            let c = &mut codes[last];
            c.i2 = c.i2.min(c.i1 + n);
            c.j2 = c.j2.min(c.j1 + n);
        }

        let nn = n + n;
        let mut groups: Vec<Vec<Opcode>> = Vec::new();
        let mut group: Vec<Opcode> = Vec::new();
        for code in codes {
            let Opcode {
                tag,
                mut i1,
                i2,
                mut j1,
                j2,
            } = code;
            if tag == Tag::Equal && i2 - i1 > nn {
                group.push(Opcode {
                    tag,
                    i1,
                    i2: i2.min(i1 + n),
                    j1,
                    j2: j2.min(j1 + n),
                });
                groups.push(std::mem::take(&mut group));
                i1 = i1.max(i2.saturating_sub(n));
                j1 = j1.max(j2.saturating_sub(n));
            }
            group.push(Opcode {
                tag,
                i1,
                i2,
                j1,
                j2,
            });
        }
        if !group.is_empty() && (group.len() != 1 || group[0].tag != Tag::Equal) {
            groups.push(group);
        }
        groups
    }
}

/// Convert a range to the unified-diff "ed" form — port of
/// `difflib._format_range_unified`.
fn format_range_unified(start: usize, stop: usize) -> String {
    let beginning = start + 1;
    let length = stop - start;
    if length == 1 {
        return beginning.to_string();
    }
    if length == 0 {
        return format!("{},{}", beginning - 1, length);
    }
    format!("{beginning},{length}")
}

/// Generate a unified diff of *a* vs *b* — port of `difflib.unified_diff`
/// with `lineterm="\n"` and the default empty file-dates. Returns each
/// yielded line as its own `String` (control lines carry the trailing
/// `"\n"`; data lines carry whatever terminator the caller's
/// [`splitlines_keepends`] preserved), mirroring `CPython`'s generator so a
/// caller can `rstrip` and count lines exactly.
#[must_use]
pub fn unified_diff(a: &[&str], b: &[&str], fromfile: &str, tofile: &str, n: usize) -> Vec<String> {
    let lineterm = "\n";
    let sm = SequenceMatcher::new(a, b);
    let mut out: Vec<String> = Vec::new();
    let mut started = false;
    for group in sm.get_grouped_opcodes(n) {
        if !started {
            started = true;
            out.push(format!("--- {fromfile}{lineterm}"));
            out.push(format!("+++ {tofile}{lineterm}"));
        }
        let first = group[0];
        let last = group[group.len() - 1];
        let file1_range = format_range_unified(first.i1, last.i2);
        let file2_range = format_range_unified(first.j1, last.j2);
        out.push(format!("@@ -{file1_range} +{file2_range} @@{lineterm}"));

        for op in &group {
            match op.tag {
                Tag::Equal => {
                    for line in &a[op.i1..op.i2] {
                        out.push(format!(" {line}"));
                    }
                }
                Tag::Replace => {
                    for line in &a[op.i1..op.i2] {
                        out.push(format!("-{line}"));
                    }
                    for line in &b[op.j1..op.j2] {
                        out.push(format!("+{line}"));
                    }
                }
                Tag::Delete => {
                    for line in &a[op.i1..op.i2] {
                        out.push(format!("-{line}"));
                    }
                }
                Tag::Insert => {
                    for line in &b[op.j1..op.j2] {
                        out.push(format!("+{line}"));
                    }
                }
            }
        }
    }
    out
}

/// Split *text* into lines keeping the trailing `\n` — mirrors Python's
/// `str.splitlines(keepends=True)` for `\n` endings. A final line with no
/// trailing newline is preserved without one.
#[must_use]
pub fn splitlines_keepends(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = text.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if c == b'\n' {
            out.push(&text[start..=i]);
            start = i + 1;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}
