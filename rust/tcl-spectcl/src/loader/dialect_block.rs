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

//! The `SpecTcl` 2.0 `dialect NAME { … }` pack-level block (§6.2, the
//! owner directive that `SpecTcl` declares **dialects**, not only
//! packages).
//!
//! ```text
//! dialect picol2 {
//!     release 2.0
//!     axis expand_syntax off
//!     axis braced_var    first-close
//!     axis numbers       tcl84
//! }
//! ```
//!
//! ## The closed axis vocabulary is the soundness boundary
//!
//! A pack **sets values for axes Rust defines**. Adding a new axis is a
//! Rust change, because the lexer has to implement it — so [`AXES`] is a
//! closed table, and an unknown axis, or an unknown value on a known axis,
//! is §6.1's **semantic** class: the whole `dialect` block is rejected and
//! the notice names the axis. The pack's other content still loads. That
//! is the fail-closed direction: a dialect read with one axis silently
//! dropped is a *wrong* lexer, not a less helpful one.
//!
//! ## The classification gate (§2)
//!
//! A `dialect` block whose axis values equal an existing family release is
//! rejected with a notice naming the environment it should have been. §2's
//! rule is that a *dialect* is a distinct grammar; something that merely
//! selects an existing grammar plus a package set is an **environment**,
//! and declaring it as a dialect would mint a second name for one grammar
//! — exactly the duplication the redesign exists to remove.
//!
//! ## Scope
//!
//! Parse, validate, carry, test. Nothing here mutates the compiled
//! catalogue, and nothing here registers: converting a [`PackDialect`]
//! into live runtime family data is
//! [`crate::dialect_conversion`]'s job, and the store it registers into
//! is [`tcl_dialect::model::dynamic`] — a *dynamic* family beside the
//! compiled [`Family`] ladder, never a new variant of it.

use tcl_dialect::model::{BuildProfileId, Family, Release, grammar};
use tcl_dialect::{
    BraceBackslashNewline, BraceLineContinuation, BracedVarStyle, EscapeSyntax, ExprCommentStyle,
    LexerGrammar, ListParse, NumberSyntax, QuoteTermination, VarSyntax, WordSeparators,
};

use super::{Log, Stmt, block, next_text};

/// One axis of the closed vocabulary: its name and every value it takes.
struct Axis {
    name: &'static str,
    values: &'static [&'static str],
}

/// The closed axis vocabulary (§6.2). Every value here is a value the
/// lexer can actually be built with, including the `jim*` spellings on
/// the `numbers` and `escapes` axes, which [`PackDialect::to_grammar`]
/// projects onto the Jim variants of [`NumberSyntax`] and
/// [`EscapeSyntax`].
const AXES: &[Axis] = &[
    Axis {
        name: "expand_syntax",
        values: &["on", "off"],
    },
    Axis {
        name: "braced_var",
        values: &["first-close", "tcl9-nesting"],
    },
    Axis {
        name: "array_index",
        values: &["tcl8", "tcl9"],
    },
    Axis {
        name: "expr_comments",
        values: &["none", "hash"],
    },
    Axis {
        name: "numbers",
        values: &["tcl84", "tcl85", "tcl90", "jim", "jim080"],
    },
    Axis {
        name: "escapes",
        values: &["tcl84", "tcl86", "tcl90", "jim"],
    },
    Axis {
        name: "irules_brace_separator",
        values: &["on", "off"],
    },
    Axis {
        name: "brace_line_continuation",
        values: &["on", "off"],
    },
    Axis {
        name: "bom_skip",
        values: &["on", "off"],
    },
    // The five axes on which Jim's parser — a reimplementation, not a
    // fork — differs from every C Tcl release. The lexer has implemented
    // all five since the Jim family landed; they were read by
    // [`PackDialect::to_grammar`] but missing here, so no pack could set
    // one and every non-default arm was unreachable (#2070). Without them
    // the §2 classification gate could not see a pack redeclaring Jim: the
    // nine axes above cannot express a Jim-shaped grammar, so
    // [`PackDialect::duplicates_compiled_release`] answered `None` for one.
    Axis {
        name: "word_separators",
        values: &["tcl", "jim"],
    },
    Axis {
        name: "brace_backslash_newline",
        values: &["folds", "literal"],
    },
    Axis {
        name: "quote_termination",
        values: &["strict", "concatenating"],
    },
    Axis {
        name: "var_syntax",
        values: &["tcl", "jim"],
    },
    Axis {
        name: "list_parse",
        values: &["strict", "lenient"],
    },
];

/// One `axis NAME VALUE` row, resolved against [`AXES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackDialectAxis {
    /// The axis name, interned from the closed vocabulary.
    pub axis: &'static str,
    /// The value, interned from that axis's closed value set.
    pub value: &'static str,
    /// The declaring line.
    pub line: u32,
}

/// One `release R ?-build P?` ladder row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackDialectRelease {
    /// The release's spelling on the declared ladder.
    pub release: String,
    /// The build profile the row names.
    pub build: BuildProfileId,
    /// The declaring line.
    pub line: u32,
}

/// A parsed `dialect NAME { … }` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackDialect {
    /// The family name the block declares.
    pub name: String,
    /// The `release` ladder, in declaration order.
    pub releases: Vec<PackDialectRelease>,
    /// The `axis` rows, in declaration order, last-wins per axis.
    pub axes: Vec<PackDialectAxis>,
    /// The declaring line.
    pub line: u32,
}

impl PackDialect {
    /// The value set for `axis`, when the block names it.
    #[must_use]
    pub fn axis(&self, axis: &str) -> Option<&'static str> {
        self.axes
            .iter()
            .rev()
            .find(|row| row.axis == axis)
            .map(|row| row.value)
    }

    /// The [`LexerGrammar`] the block's axes describe, when every value
    /// has a Rust backing.
    ///
    /// `None` means the block uses a reserved `jim*` value, which the
    /// lexer cannot be built with until the Jim branch lands. Unset axes
    /// take the type's own default, which is the modern permissive value —
    /// so a block that sets nothing describes the default grammar, and the
    /// §2 classification gate below will reject it for that reason.
    #[must_use]
    pub fn to_grammar(&self) -> Option<LexerGrammar> {
        let flag = |axis: &str, default: bool| match self.axis(axis) {
            Some("on") => Some(true),
            Some("off") => Some(false),
            Some(_) => None,
            None => Some(default),
        };
        let braced_var = match self.axis("braced_var") {
            Some("first-close") => BracedVarStyle::FirstClose,
            Some("tcl9-nesting") | None => BracedVarStyle::Tcl9Nesting,
            Some(_) => return None,
        };
        let array_index = match self.axis("array_index") {
            Some("tcl8") => tcl_dialect::ArrayIndexSyntax::Tcl8,
            Some("tcl9") | None => tcl_dialect::ArrayIndexSyntax::Tcl9,
            Some(_) => return None,
        };
        let expr_comments = match self.axis("expr_comments") {
            Some("none") => ExprCommentStyle::None,
            Some("hash") | None => ExprCommentStyle::Hash,
            Some(_) => return None,
        };
        let numbers = match self.axis("numbers") {
            Some("tcl84") => NumberSyntax::Tcl84,
            Some("tcl85") => NumberSyntax::Tcl85,
            Some("tcl90") | None => NumberSyntax::Tcl90,
            Some("jim") => NumberSyntax::Jim,
            Some("jim080") => NumberSyntax::Jim080,
            Some(_) => return None,
        };
        let escapes = match self.axis("escapes") {
            Some("tcl84") => EscapeSyntax::Tcl84,
            Some("tcl86") => EscapeSyntax::Tcl86,
            Some("tcl90") | None => EscapeSyntax::Tcl90,
            Some("jim") => EscapeSyntax::Jim,
            Some(_) => return None,
        };
        let word_separators = match self.axis("word_separators") {
            Some("tcl") | None => WordSeparators::Tcl,
            Some("jim") => WordSeparators::Jim,
            Some(_) => return None,
        };
        let brace_backslash_newline = match self.axis("brace_backslash_newline") {
            Some("folds") | None => BraceBackslashNewline::Folds,
            Some("literal") => BraceBackslashNewline::Literal,
            Some(_) => return None,
        };
        let quote_termination = match self.axis("quote_termination") {
            Some("strict") | None => QuoteTermination::Strict,
            Some("concatenating") => QuoteTermination::Concatenating,
            Some(_) => return None,
        };
        let var_syntax = match self.axis("var_syntax") {
            Some("tcl") | None => VarSyntax::Tcl,
            Some("jim") => VarSyntax::Jim,
            Some(_) => return None,
        };
        let list_parse = match self.axis("list_parse") {
            Some("strict") | None => ListParse::Strict,
            Some("lenient") => ListParse::Lenient,
            Some(_) => return None,
        };
        Some(LexerGrammar {
            expand_syntax: flag("expand_syntax", true)?,
            irules_brace_separator: flag("irules_brace_separator", false)?,
            brace_line_continuation: if flag("brace_line_continuation", false)? {
                BraceLineContinuation::Continues
            } else {
                BraceLineContinuation::Terminates
            },
            braced_var,
            array_index,
            script_skips_leading_bom: flag("bom_skip", true)?,
            expr_comments,
            numbers,
            escapes,
            word_separators,
            brace_backslash_newline,
            quote_termination,
            var_syntax,
            list_parse,
        })
    }

    /// The compiled `(family, release)` this block's axes duplicate, when
    /// they duplicate one — the §2 classification gate.
    #[must_use]
    pub fn duplicates_compiled_release(&self) -> Option<(Family, Release)> {
        let declared = self.to_grammar()?;
        Family::ALL.iter().find_map(|family| {
            family
                .releases()
                .iter()
                .find(|release| grammar(*family, **release) == declared)
                .map(|release| (*family, *release))
        })
    }
}

/// Parse one `dialect NAME { … }` block, or reject it.
pub(super) fn parse(stmt: &Stmt, log: &mut Log) -> Option<PackDialect> {
    let rows = stmt.arg(2).map(block);
    parse_rows(stmt, rows.as_deref(), log)
}

/// Parse one `dialect` declaration from its header and its already-read
/// rows, or reject it.
///
/// The single owner of the block's validation and notices, shared by the
/// literal reader above and the evaluation loader, which runs the body as
/// a script and passes the rows the script registered. `rows` is `None`
/// when the declaration carried no `{ … }` body word at all.
pub(super) fn parse_rows(stmt: &Stmt, rows: Option<&[Stmt]>, log: &mut Log) -> Option<PackDialect> {
    let name = stmt.word_text(1);
    if name.is_empty() || stmt.words.get(1).is_some_and(|word| word.braced) {
        log.say(stmt.line, "`dialect` needs a name and a `{ … }` block");
        return None;
    }
    let Some(rows) = rows else {
        log.say(
            stmt.line,
            format!("`dialect {name}` has no `{{ … }}` block; the block is rejected"),
        );
        return None;
    };
    let mut dialect = PackDialect {
        name: name.to_owned(),
        releases: Vec::new(),
        axes: Vec::new(),
        line: stmt.line,
    };
    let mut rejected = false;
    log.scoped(format!("dialect {name}"), |log| {
        for row in rows {
            if !read_row(&mut dialect, row, log) {
                rejected = true;
            }
        }
    });
    if rejected {
        return None;
    }
    if let Some((family, release)) = dialect.duplicates_compiled_release() {
        log.say(
            stmt.line,
            format!(
                "`dialect {name}` declares the grammar of {family} {release}, which is not a \
                 new dialect but a selection of an existing one (design §2); declare it as \
                 `environment {name} {{ core {} {} … }}` instead — the block is rejected",
                family.name(),
                release.as_str()
            ),
        );
        return None;
    }
    Some(dialect)
}

/// Read one row. `false` rejects the whole block (the §6.1 semantic
/// class).
fn read_row(dialect: &mut PackDialect, stmt: &Stmt, log: &mut Log) -> bool {
    match stmt.word_text(0) {
        "release" => release_row(dialect, stmt, log),
        "axis" => axis_row(dialect, stmt, log),
        other => {
            log.say(
                stmt.line,
                format!(
                    "unknown `dialect` row `{}` is semantic-class vocabulary \
                     (design §6.1); the dialect block is rejected",
                    super::quotable(other)
                ),
            );
            false
        }
    }
}

fn release_row(dialect: &mut PackDialect, stmt: &Stmt, log: &mut Log) -> bool {
    let release = stmt.word_text(1);
    if release.is_empty() {
        log.say(
            stmt.line,
            "`release` needs a release spelling; the dialect block is rejected",
        );
        return false;
    }
    let mut build = BuildProfileId::Canonical;
    let words = &stmt.words;
    let mut index = 2;
    while index < words.len() {
        match words[index].text.as_str() {
            "-build" => {
                let named = next_text(words, &mut index);
                match named.as_str() {
                    "Canonical" => build = BuildProfileId::Canonical,
                    "Unknown" => build = BuildProfileId::Unknown,
                    other => {
                        log.say(
                            stmt.line,
                            format!(
                                "`-build {other}` is not a build profile (`Canonical`, \
                                 `Unknown`); the dialect block is rejected"
                            ),
                        );
                        return false;
                    }
                }
            }
            other => {
                log.unknown_flag("release", stmt.line, other);
            }
        }
        index += 1;
    }
    if dialect
        .releases
        .iter()
        .any(|prior| prior.release == release)
    {
        log.say(
            stmt.line,
            format!("`release {release}` is declared twice; the dialect block is rejected"),
        );
        return false;
    }
    dialect.releases.push(PackDialectRelease {
        release: release.to_owned(),
        build,
        line: stmt.line,
    });
    true
}

fn axis_row(dialect: &mut PackDialect, stmt: &Stmt, log: &mut Log) -> bool {
    let name = stmt.word_text(1);
    let Some(axis) = AXES.iter().find(|axis| axis.name == name) else {
        log.say(
            stmt.line,
            format!(
                "`axis {}` is not in the closed axis vocabulary (a new axis is a Rust \
                 change — the lexer has to implement it); the dialect block is rejected \
                 (design §6.1, §6.2)",
                super::quotable(name)
            ),
        );
        return false;
    };
    let value = stmt.word_text(2);
    let Some(interned) = axis.values.iter().find(|known| **known == value) else {
        log.say(
            stmt.line,
            format!(
                "`axis {name} {}` is not a value of `{name}` ({}); the dialect block is \
                 rejected (design §6.1)",
                super::quotable(value),
                axis.values.join(", ")
            ),
        );
        return false;
    };
    for extra in stmt.words.iter().skip(3) {
        log.unknown_flag("axis", stmt.line, &extra.text);
    }
    dialect.axes.push(PackDialectAxis {
        axis: axis.name,
        value: interned,
        line: stmt.line,
    });
    true
}

#[cfg(test)]
mod tests {
    use super::{AXES, PackDialect, PackDialectAxis, PackDialectRelease};
    use std::fmt::Write as _;
    use tcl_dialect::model::{BuildProfileId, Family};

    /// Evaluate a pack declaring one `dialect` block with exactly `rows`.
    fn dialect_pack(rows: &str) -> crate::loader::Pack {
        crate::loader::evaluate_pack(&format!(
            "speclib probe 2.0 {{\n dialect probed {{\n  release 1.0\n{rows} }}\n}}"
        ))
    }

    /// A block setting exactly `axes`, without going through the loader —
    /// so the §2 classification gate does not reject it for reproducing a
    /// compiled grammar, which is a separate question from whether the
    /// axis is in the vocabulary.
    fn block_with(axes: &[(&'static str, &'static str)]) -> PackDialect {
        PackDialect {
            name: "probed".to_owned(),
            releases: vec![PackDialectRelease {
                release: "1.0".to_owned(),
                build: BuildProfileId::default(),
                line: 3,
            }],
            axes: axes
                .iter()
                .map(|(axis, value)| PackDialectAxis {
                    axis,
                    value,
                    line: 4,
                })
                .collect(),
            line: 2,
        }
    }

    /// #2070's defect class, pinned at its source: **every axis name
    /// `to_grammar` reads is in [`AXES`]**.
    ///
    /// This is the assertion the other two cannot make. Both of those
    /// iterate `AXES`, so an axis that `to_grammar` reads and `AXES` omits
    /// is invisible to them — which is exactly how five of them survived.
    /// The two lists live in one file and drifted apart there, so the test
    /// reads that file: `self.axis("NAME")` is the only way an axis value
    /// is fetched, and every name so fetched must be settable.
    #[test]
    fn axes_lists_every_axis_to_grammar_reads_issue_2070() {
        // Bounded to `to_grammar`'s own body, so this test's marker
        // literals below do not match themselves.
        let source = include_str!("dialect_block.rs");
        let from = source
            .find("pub fn to_grammar")
            .expect("`to_grammar` by that name");
        let to = source[from..]
            .find("pub fn duplicates_compiled_release")
            .expect("`duplicates_compiled_release` follows it");
        let body = &source[from..from + to];

        // Two ways an axis is read: `self.axis("NAME")` directly, and the
        // `flag("NAME", default)` closure for the on/off axes.
        let mut read: Vec<&str> = Vec::new();
        for marker in ["self.axis(\"", "flag(\""] {
            read.extend(body.match_indices(marker).filter_map(|(at, marker)| {
                let rest = &body[at + marker.len()..];
                rest.find('"').map(|end| &rest[..end])
            }));
        }

        assert_eq!(
            read.len(),
            AXES.len(),
            "`to_grammar` reads {} axes and AXES lists {} — {read:?}",
            read.len(),
            AXES.len()
        );
        for name in read {
            assert!(
                AXES.iter().any(|axis| axis.name == name),
                "`to_grammar` reads the `{name}` axis, but it is not in AXES, \
                 so no pack can set it and the non-default arms are unreachable"
            );
        }
    }

    /// #2070, first half: every axis in the closed table is *settable*.
    ///
    /// `axis_row` rejects a name absent from [`AXES`] and the notice names
    /// it, rejecting the whole block — so five axes `to_grammar` reads
    /// (`word_separators`, `brace_backslash_newline`, `quote_termination`,
    /// `var_syntax`, `list_parse`) could never be set by any pack.
    ///
    /// A §2 classification notice is not a vocabulary rejection and is
    /// expected here: most single-axis blocks do reproduce a compiled
    /// grammar. Only the vocabulary notice is the failure.
    #[test]
    fn every_axis_is_in_the_closed_vocabulary_issue_2070() {
        for axis in AXES {
            for value in axis.values {
                let pack = dialect_pack(&format!("  axis {} {}\n", axis.name, value));
                let rejected: Vec<_> = pack
                    .notices
                    .iter()
                    .filter(|notice| notice.message.contains("closed axis vocabulary"))
                    .collect();
                assert!(
                    rejected.is_empty(),
                    "`axis {} {}` is not in the closed vocabulary: {rejected:?}",
                    axis.name,
                    value
                );
            }
        }
    }

    /// #2070, second half: every value on every axis reaches a distinct
    /// grammar, so no match arm in `to_grammar` is unreachable.
    #[test]
    fn every_axis_value_reaches_a_distinct_grammar_issue_2070() {
        for axis in AXES {
            let mut seen = Vec::new();
            for value in axis.values {
                let grammar = block_with(&[(axis.name, value)])
                    .to_grammar()
                    .unwrap_or_else(|| panic!("`axis {} {}` has no grammar", axis.name, value));
                assert!(
                    !seen.contains(&grammar),
                    "`axis {} {}` produces a grammar another value on the same axis \
                     already produced — the match arm is unreachable",
                    axis.name,
                    value
                );
                seen.push(grammar);
            }
        }
    }

    /// #2070's consequence, not merely its dead code: the §2 classification
    /// gate could not see a pack redeclaring Jim.
    ///
    /// The five missing axes are exactly the ones on which Jim's parser —
    /// a reimplementation, not a fork — differs from every C Tcl release
    /// (see `GRAMMAR_JIM` in `tcl-dialect`). A pack setting all nine
    /// reachable axes to Jim's values still produced a Tcl-shaped grammar,
    /// so `duplicates_compiled_release` answered `None` and the block was
    /// accepted as a brand-new dialect. It is now rejected, naming Jim.
    #[test]
    fn a_pack_declaring_jims_grammar_is_classified_as_jim_issue_2070() {
        const JIM: &[(&str, &str)] = &[
            ("expand_syntax", "on"),
            ("braced_var", "first-close"),
            ("array_index", "tcl8"),
            ("expr_comments", "none"),
            ("numbers", "jim"),
            ("escapes", "jim"),
            ("irules_brace_separator", "off"),
            ("brace_line_continuation", "off"),
            ("bom_skip", "off"),
            ("word_separators", "jim"),
            ("brace_backslash_newline", "literal"),
            ("quote_termination", "concatenating"),
            ("var_syntax", "jim"),
            ("list_parse", "lenient"),
        ];

        assert_eq!(
            block_with(JIM)
                .duplicates_compiled_release()
                .map(|(family, _)| family),
            Some(Family::Jim),
            "grammar was {:?}",
            block_with(JIM).to_grammar()
        );

        let mut rows = String::new();
        for (axis, value) in JIM {
            let _ = writeln!(rows, "  axis {axis} {value}");
        }
        let pack = dialect_pack(&rows);
        assert!(
            pack.dialects.is_empty(),
            "the block should be rejected by the §2 gate"
        );
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.message.contains("grammar of jim")),
            "{:?}",
            pack.notices
        );
    }

    /// The `dialect` spec's own documentation restates the axis names, and
    /// it had already drifted — it omitted `array_index` and
    /// `brace_line_continuation` before #2070 added five more. Pin the two
    /// lists together so the next addition cannot forget the user-facing
    /// half.
    #[test]
    fn the_registry_spec_documents_every_axis_issue_2070() {
        let specs = tcl_registry::commands::spectcl::spectcl_command_specs();
        let dialect = specs
            .iter()
            .find(|spec| spec.name == "dialect")
            .expect("a `dialect` statement spec");
        let hover = dialect.hover.as_ref().expect("a hover snippet");
        let documentation = format!("{} {}", hover.summary, hover.snippet);
        for axis in AXES {
            assert!(
                documentation.contains(&format!("`{}`", axis.name)),
                "the `dialect` spec does not document the `{}` axis",
                axis.name
            );
        }
    }
}
