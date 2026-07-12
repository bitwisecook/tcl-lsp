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

//! Splitting a `{pattern body pattern body …}` clause list into its clauses.
//!
//! `switch { pat body … }` and Expect's `expect { ?-flags? pat body … }` are the
//! same construct. Two consumers walk it — the semantic-token walker (to type
//! each element) and the iRules object-reference walker (to find objects named
//! inside a clause body) — and they must agree on where the clauses are, or the
//! two disagree about what the code says.
//!
//! A clause list is a **list**, not a script: `;` and `#` are ordinary pattern
//! elements, not a command separator / comment (Tcl's "comments don't work in
//! `switch`" gotcha). So it is split with the list grammar
//! ([`crate::list::find_element`]), never the command segmenter.
//!
//! The walk is clause-by-clause rather than strict pattern/body alternation,
//! because Expect lets a clause carry leading flags (`-re`, `-timeout 5`) that
//! would otherwise shift every following element by one — turning patterns into
//! bodies and bodies into patterns.
//!
//! The *shape* (which flags exist, which take a value) is registry data; this
//! module takes it as plain slices so the syntax layer stays free of the
//! registry.

/// Which clause-leading flags a clause list admits.
///
/// Built from the registry's `CaseListSpec` by the caller. `switch` declares no
/// clause flags; Expect declares `-re` / `-gl` / `-ex` / `-nocase` / `-timeout`
/// / `-i` / `--`, of which `-timeout` and `-i` consume a following value word.
#[derive(Debug, Clone, Copy, Default)]
pub struct CaseListShape<'a> {
    /// Flags that may precede a pattern inside the list.
    pub clause_flags: &'a [&'a str],
    /// Of those, the ones that consume a following value word (`-timeout 5`).
    pub clause_value_flags: &'a [&'a str],
}

/// One element of a clause list, as byte offsets into the list's *content*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    /// Byte offset of the element, **including** its opening `{` when braced.
    pub start: usize,
    /// Byte offset one past the element's value — for a braced element this is
    /// the position *of* its closing `}`, matching the lexer's inner-end
    /// convention (`span.end()` sits at the closer; `content_offset` strips the
    /// opener).
    pub end: usize,
    /// Whether the element was written braced (`{…}`).
    pub braced: bool,
}

/// One clause: its leading flags, its pattern, and its body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Clause {
    /// Leading flag words (and their value words, where a flag takes one).
    pub flags: Vec<Element>,
    /// The pattern element. `None` only on a malformed trailing clause.
    pub pattern: Option<Element>,
    /// The body element. `None` when the list ends after a pattern.
    pub body: Option<Element>,
}

/// Split the *content* of a clause list into clauses.
///
/// `inner` is the text between the list's braces; every offset returned is
/// relative to it.
#[must_use]
pub fn split_case_list(inner: &str, shape: &CaseListShape<'_>) -> Vec<Clause> {
    let elements = elements_of(inner);
    let text = |e: &Element| inner.get(e.start..e.end).unwrap_or_default();

    let mut clauses = Vec::new();
    let mut i = 0usize;
    while i < elements.len() {
        let mut clause = Clause::default();

        // Leading clause flags. A braced element is never a flag — `{-re}` is a
        // pattern that happens to look like one.
        while let Some(e) = elements.get(i) {
            let word = text(e).trim_start_matches('{');
            if e.braced || !shape.clause_flags.contains(&word) {
                break;
            }
            clause.flags.push(*e);
            i += 1;
            if shape.clause_value_flags.contains(&word)
                && let Some(v) = elements.get(i)
            {
                clause.flags.push(*v);
                i += 1;
            }
        }

        clause.pattern = elements.get(i).copied();
        i += 1;
        clause.body = elements.get(i).copied();
        i += 1;

        if clause.pattern.is_none() && clause.flags.is_empty() {
            break;
        }
        clauses.push(clause);
    }
    clauses
}

/// Every element of the list, in source order.
fn elements_of(inner: &str) -> Vec<Element> {
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut scan = 0usize;
    while let Ok(Some(el)) = crate::list::find_element(inner, scan) {
        let braced = el.value.start > 0 && bytes.get(el.value.start - 1) == Some(&b'{');
        out.push(Element {
            start: if braced {
                el.value.start - 1
            } else {
                el.value.start
            },
            end: el.value.end,
            braced,
        });
        if el.next <= scan {
            break;
        }
        scan = el.next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{CaseListShape, split_case_list};

    const EXPECT: CaseListShape<'static> = CaseListShape {
        clause_flags: &["-re", "-gl", "-ex", "-nocase", "-timeout", "-i", "--"],
        clause_value_flags: &["-timeout", "-i"],
    };
    const SWITCH: CaseListShape<'static> = CaseListShape {
        clause_flags: &[],
        clause_value_flags: &[],
    };

    fn shape_of<'a>(
        inner: &'a str,
        shape: &CaseListShape<'_>,
    ) -> Vec<(Vec<&'a str>, &'a str, &'a str)> {
        split_case_list(inner, shape)
            .into_iter()
            .map(|c| {
                let t = |e: Option<super::Element>| {
                    e.map(|e| &inner[e.start..e.end]).unwrap_or_default()
                };
                (
                    c.flags.iter().map(|f| &inner[f.start..f.end]).collect(),
                    t(c.pattern),
                    t(c.body),
                )
            })
            .collect()
    }

    #[test]
    fn switch_is_plain_pattern_body_alternation() {
        assert_eq!(
            shape_of("{^a} {puts 1} default {puts 2}", &SWITCH),
            vec![(vec![], "{^a", "{puts 1"), (vec![], "default", "{puts 2"),]
        );
    }

    /// The clause-flag walk is the whole point: strict alternation would read
    /// `-re` as a pattern and `{ye+s}` as a body, shifting every clause after it.
    #[test]
    fn expect_clause_flags_do_not_shift_the_clauses() {
        assert_eq!(
            shape_of(
                "-re {ye+s} {send y} timeout {puts slow} eof {puts done}",
                &EXPECT
            ),
            vec![
                (vec!["-re"], "{ye+s", "{send y"),
                (vec![], "timeout", "{puts slow"),
                (vec![], "eof", "{puts done"),
            ]
        );
    }

    /// A flag that takes a value consumes the value word too.
    #[test]
    fn value_flags_consume_their_value() {
        assert_eq!(
            shape_of("-timeout 5 eof {puts done}", &EXPECT),
            vec![(vec!["-timeout", "5"], "eof", "{puts done")]
        );
    }

    /// A braced element is a pattern even when it looks like a flag.
    #[test]
    fn a_braced_flag_lookalike_is_a_pattern() {
        assert_eq!(
            shape_of("{-re} {puts 1}", &EXPECT),
            vec![(vec![], "{-re", "{puts 1")]
        );
    }

    #[test]
    fn a_trailing_pattern_without_a_body_does_not_panic() {
        assert_eq!(
            shape_of("a {puts 1} b", &SWITCH),
            vec![(vec![], "a", "{puts 1"), (vec![], "b", "")]
        );
    }
}
