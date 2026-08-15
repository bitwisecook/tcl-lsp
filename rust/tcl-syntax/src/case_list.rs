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
//! same construct. Several consumers walk it — the semantic-token walker (to
//! type each element), the iRules object-reference walker (to find objects
//! named inside a clause body), the reference scanner, and the fold walk (to
//! give each arm its own folding range) — and they must agree on where the
//! clauses are, or they disagree about what the code says.
//!
//! Two questions, two entry points: [`clause_list_call`] answers *which
//! argument* holds the list (skipping the command's options and subject
//! words), and [`split_case_list`] answers *where the clauses are* inside it.
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

impl<'a> CaseListShape<'a> {
    /// Resolve a clause-leading flag by its canonical spelling or a unique
    /// Expect-style abbreviation.  The descriptor supplies this vocabulary,
    /// so list and inline forms cannot disagree about where a pattern starts.
    #[must_use]
    pub fn resolve_flag(self, word: &str) -> Option<&'a str> {
        if let Some(&flag) = self.clause_flags.iter().find(|&&flag| flag == word) {
            return Some(flag);
        }
        // Tcl option abbreviations include the leading `-`, so `-r` may be a
        // unique prefix but a bare `-` is never a flag.
        if word.len() < 2 {
            return None;
        }
        let mut matches = self
            .clause_flags
            .iter()
            .copied()
            .filter(|flag| flag.starts_with(word));
        let flag = matches.next()?;
        matches.next().is_none().then_some(flag)
    }

    /// Whether a resolved canonical flag consumes one following word.
    #[must_use]
    pub fn flag_takes_value(self, flag: &str) -> bool {
        self.clause_value_flags.contains(&flag)
    }
}

/// How a clause-list command's *call* is shaped ahead of the list itself.
///
/// Built from the registry's `CaseListSpec` by the caller, for the same reason
/// [`CaseListShape`] is: this layer stays free of the registry.
#[derive(Debug, Clone, Copy, Default)]
pub struct CallShape<'a> {
    /// Non-option words between the options and the clause list — `switch`'s
    /// subject string is 1, `expect` has none.
    pub subject_args: usize,
    /// The command option that makes every pattern a regex (`switch -regexp`).
    pub regex_option: Option<&'a str>,
    /// Command options that consume a following value word (`-matchvar var`),
    /// so the value is not mistaken for a subject or for the list.
    pub value_options: &'a [&'a str],
}

/// Where the single braced clause list sits in a call, when the call is in
/// that shape at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClauseListCall {
    /// Index of the clause-list word, 0-based **after** the command name.
    pub index: usize,
    /// Whether the command-level regex option was given, so every pattern in
    /// the list is a regex.
    pub regexp: bool,
}

/// Locate the clause-list word of a call whose command carries a clause-list
/// shape — `switch ?opts? subject { pat body … }`, `expect ?opts? { pat body
/// … }`.
///
/// `args` is the call's words **after** the command name.  Returns `None`
/// when the call is in the inline `pat body pat body …` form instead (more
/// than one word remains after the options and subjects), or when no word
/// remains at all: those bodies are ordinary script arguments the caller's
/// generic role walk already reaches, and no clause-list unpacking applies.
///
/// The option skip is driven entirely by `shape`, never by a command name, so
/// every clause-list command — present and future — is located identically.
/// This is the one implementation: the semantic-token walker, the reference
/// scanner, and the fold walk all read it, and if they disagreed about where
/// the list is they would disagree about what the code says.
#[must_use]
pub fn clause_list_call(args: &[&str], shape: &CallShape<'_>) -> Option<ClauseListCall> {
    let mut i = 0usize;
    let mut regexp = false;
    while i < args.len() {
        let a = args[i];
        if shape.regex_option == Some(a) {
            regexp = true;
        }
        if a == "--" {
            i += 1;
            break;
        }
        if !a.starts_with('-') {
            break;
        }
        i += if shape.value_options.contains(&a) {
            2
        } else {
            1
        };
    }
    i += shape.subject_args;
    // Exactly one word left: the braced clause list.
    if i < args.len() && args.len() - i == 1 {
        Some(ClauseListCall { index: i, regexp })
    } else {
        None
    }
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
    /// Zero-based element ordinal of `pattern` in the enclosing list.
    pub pattern_index: Option<usize>,
    /// Zero-based element ordinal of `body` in the enclosing list.
    pub body_index: Option<usize>,
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
            let Some(flag) = (!e.braced).then(|| shape.resolve_flag(word)).flatten() else {
                break;
            };
            clause.flags.push(*e);
            i += 1;
            if shape.flag_takes_value(flag)
                && let Some(v) = elements.get(i)
            {
                clause.flags.push(*v);
                i += 1;
            }
        }

        clause.pattern_index = elements.get(i).map(|_| i);
        clause.pattern = elements.get(i).copied();
        i += 1;
        clause.body_index = elements.get(i).map(|_| i);
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

    mod locating_the_list {
        use super::super::{CallShape, ClauseListCall, clause_list_call};

        const SWITCH_CALL: CallShape<'static> = CallShape {
            subject_args: 1,
            regex_option: Some("-regexp"),
            value_options: &["-matchvar", "-indexvar"],
        };
        const EXPECT_CALL: CallShape<'static> = CallShape {
            subject_args: 0,
            regex_option: None,
            value_options: &["-timeout", "-i"],
        };

        fn call(args: &[&str], shape: &CallShape<'_>) -> Option<ClauseListCall> {
            clause_list_call(args, shape)
        }

        #[test]
        fn plain_switch_finds_the_trailing_list() {
            assert_eq!(
                call(&["$x", "{a b}"], &SWITCH_CALL),
                Some(ClauseListCall {
                    index: 1,
                    regexp: false
                })
            );
        }

        #[test]
        fn options_and_their_values_are_skipped() {
            assert_eq!(
                call(
                    &["-regexp", "-matchvar", "m", "--", "$x", "{a b}"],
                    &SWITCH_CALL
                ),
                Some(ClauseListCall {
                    index: 5,
                    regexp: true
                })
            );
        }

        #[test]
        fn the_inline_pairs_form_is_not_a_clause_list() {
            assert_eq!(call(&["$x", "a", "{b}", "c", "{d}"], &SWITCH_CALL), None);
        }

        #[test]
        fn a_call_with_no_list_word_at_all_is_none() {
            assert_eq!(call(&["$x"], &SWITCH_CALL), None);
            assert_eq!(call(&[], &SWITCH_CALL), None);
        }

        /// `expect` has no subject word, and `-timeout` / `-i` appear as
        /// *command* options too — stopping the scan on the value word would
        /// leave two trailing words and hide the list.
        #[test]
        fn expect_has_no_subject_and_skips_value_options() {
            assert_eq!(
                call(&["-timeout", "5", "{pat body}"], &EXPECT_CALL),
                Some(ClauseListCall {
                    index: 2,
                    regexp: false
                })
            );
        }

        /// A subject that happens to look like a flag is protected by `--`,
        /// and everything after it is positional.
        #[test]
        fn option_terminator_stops_the_option_scan() {
            assert_eq!(
                call(&["--", "-weird", "{a b}"], &SWITCH_CALL),
                Some(ClauseListCall {
                    index: 2,
                    regexp: false
                })
            );
        }
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
