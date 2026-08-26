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

//! The 1.x → 2.0 **source rewriter** behind `tcl spec upgrade`
//! (`docs/design/dialect-and-package-registry-centralisation.md` §6).
//!
//! Three facts fix its shape, and all three are structural rather than
//! stylistic:
//!
//! 1. **It is a source rewriter, never a load-render round trip.** The
//!    renderer emits no pack-level rows and degrades 15 `DraftOpaque`
//!    fields to TODO comments, so rendering a loaded pack would delete
//!    parts of it. Edits are therefore content-range replacements located
//!    by the loader's own lexer — the [`speclib_version_span`] discipline:
//!    same lexing, same BOM handling, applied back-to-front, never
//!    reformatting. Author layout, comments, and delimiters survive, and
//!    the diff is reviewable (U8).
//! 2. **The 1.x dialect vocabulary is closed** — [`TRANSLATIONS`],
//!    [`DEFERRED`], and the single refused token below are all of it — so
//!    the translation table is total and a token outside it means the
//!    file is *refused*, never guessed at (U0).
//! 3. **The version word moves only when the body did** (U1). A `2.0`
//!    header over 1.x spellings is loadable but is a failed upgrade, so a
//!    file with any row left for the environment wire-up (U3) reports
//!    *partially upgraded* and keeps its 1.x header.
//!
//! What is **not** here: U3's environment-membership mapping, U4's
//! `ambient_package` rehoming, and U5's `file_extension -dialect` move all
//! need the P1 environment registry. Each is detected, left byte-identical,
//! and reported — with a `# TODO(spectcl 2.0):` marker written above the
//! row so the file itself says what is left.

use std::ops::Range;

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_lexer::{LexerConfig, SourceMap, TokenType};

use crate::loader::{
    KNOWN_VOCABULARY_VERSIONS, NEWEST_VOCABULARY_VERSION, list_words, load_pack,
    speclib_version_span,
};

/// The oldest vocabulary `--from` defaults to.
pub const OLDEST_VOCABULARY_VERSION: &str = "1.0";

/// The total 1.x → 2.0 token table (U2), for the tokens that translate
/// wholly onto the availability algebra.
///
/// The value is the `available` row's *content*: the tool writes it wrapped
/// in braces, so `all-tcl` becomes `available {tcl 8.4-}`.
const TRANSLATIONS: &[(&str, &str)] = &[
    ("tcl8.4", "tcl 8.4"),
    ("tcl8.5", "tcl 8.5"),
    ("tcl8.6", "tcl 8.6"),
    ("tcl9.0", "tcl 9.0"),
    ("tcl9.1", "tcl 9.1"),
    ("tcl8.4+", "tcl 8.4-"),
    ("tcl8.5+", "tcl 8.5-"),
    ("tcl8.6+", "tcl 8.6-"),
    ("tcl9.0+", "tcl 9.0-"),
    ("tcl9.1+", "tcl 9.1-"),
    ("all-tcl", "tcl 8.4-"),
    // The 8 series with its exclusive maximum stated, which is what the
    // `DialectSet::TCL8X` bit has always meant.
    ("tcl8.x", "tcl 8.4-9.0"),
    ("f5-irules", "f5-irules"),
    // Review B11: Tk is a package on its own axis, never a Tcl release.
    ("tk", "package Tk"),
];

/// Tokens that are **environment membership**, not availability (U3).
///
/// Each names an environment whose ambient package provides the surface;
/// spelling that as an `available` row needs the P1 environment registry to
/// say which package, at which placement. Until then the row is left
/// byte-identical and marked, because guessing here would silently
/// re-scope a claim.
const DEFERRED: &[&str] = &["f5-iapps", "f5-tmsh", "expect", "spectcl", "bpf"];

/// The one token that is an outright error (Q3): `f5-bigip` is a
/// configuration surface that leaves the Tcl axis, so no `available` row
/// can carry it.
const REFUSED: &str = "f5-bigip";

/// What `tcl spec upgrade` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeOptions {
    /// The vocabulary the file is expected to declare.
    pub from: String,
    /// The vocabulary to rewrite it to.
    pub to: String,
}

impl Default for UpgradeOptions {
    fn default() -> Self {
        Self {
            from: OLDEST_VOCABULARY_VERSION.to_owned(),
            to: NEWEST_VOCABULARY_VERSION.to_owned(),
        }
    }
}

/// One `dialects` row the rewriter translated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    /// 1-based line of the row.
    pub line: u32,
    /// The row as it was written.
    pub before: String,
    /// The row as it now reads.
    pub after: String,
}

/// One row left in place for a later phase, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deferred {
    /// 1-based line of the row.
    pub line: u32,
    /// The token that could not be translated yet.
    pub token: String,
    /// Why, in one line.
    pub reason: String,
}

/// Why a file was refused outright.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// 1-based line, or `1` when the refusal is about the file as a whole.
    pub line: u32,
    /// What is wrong, and what to do about it.
    pub message: String,
}

/// How far an upgrade got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeStatus {
    /// The file already declares the target vocabulary and has no 1.x
    /// spellings left.
    AlreadyCurrent,
    /// Every row translated and the version word moved.
    Upgraded,
    /// Rows translated, but at least one is left for a later phase, so the
    /// version word stays where it was (U1).
    Partial,
    /// Nothing was rewritten — see [`UpgradeOutcome::refusals`].
    Refused,
    /// The file has no `speclib` line at all.
    NotAPack,
}

/// The result of upgrading one file's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeOutcome {
    /// How far it got.
    pub status: UpgradeStatus,
    /// The rewritten source. Byte-identical to the input unless
    /// [`UpgradeStatus::Upgraded`] or [`UpgradeStatus::Partial`].
    pub source: String,
    /// The version word as the file declared it.
    pub declared_version: Option<String>,
    /// Rows translated, in source order.
    pub translated: Vec<Translated>,
    /// Rows left for a later phase, in source order.
    pub deferred: Vec<Deferred>,
    /// Why the file was refused, when it was.
    pub refusals: Vec<Refusal>,
    /// U7: sites in the rewritten file whose vocabulary is newer than its
    /// own declaration. Empty is the proof the upgrade is complete.
    pub above_target: Vec<String>,
}

/// One replacement, as a content range and the bytes to put there.
struct Edit {
    at: Range<usize>,
    text: String,
}

/// Rewrite `source` from one vocabulary to another.
#[must_use]
pub fn upgrade_source(source: &str, options: &UpgradeOptions) -> UpgradeOutcome {
    let mut outcome = UpgradeOutcome {
        status: UpgradeStatus::Refused,
        source: source.to_owned(),
        declared_version: None,
        translated: Vec::new(),
        deferred: Vec::new(),
        refusals: Vec::new(),
        above_target: Vec::new(),
    };
    if let Some(refusal) = check_versions(options) {
        outcome.refusals.push(refusal);
        return outcome;
    }
    let Some((version_span, declared)) = speclib_version_span(source) else {
        outcome.status = UpgradeStatus::NotAPack;
        return outcome;
    };
    outcome.declared_version = Some(declared.clone());
    if !KNOWN_VOCABULARY_VERSIONS.contains(&declared.as_str()) {
        outcome.refusals.push(Refusal {
            line: 1,
            message: format!(
                "`{declared}` names no SpecTcl vocabulary (the library's own version?) — \
                 not rewritten; fix the `speclib` line by hand"
            ),
        });
        return outcome;
    }

    let sites = dialects_sites(source);
    let mut edits: Vec<Edit> = Vec::new();
    let mut todo_lines: Vec<usize> = Vec::new();
    for site in &sites {
        match plan_site(site, &mut outcome) {
            SitePlan::Translate(rows) => {
                let before = format!("{} {}", site.keyword.0, site.value_text);
                let after = render_row_word(&rows, site.is_flag);
                outcome.translated.push(Translated {
                    line: site.line,
                    before,
                    after: format!("{} {after}", available_word(site.is_flag)),
                });
                edits.push(Edit {
                    at: site.keyword.1.clone(),
                    text: available_word(site.is_flag).to_owned(),
                });
                edits.push(Edit {
                    at: site.value_word.clone(),
                    text: after,
                });
            }
            SitePlan::Defer => todo_lines.push(site.line_start),
            SitePlan::Refuse => return outcome,
        }
    }
    if !outcome.refusals.is_empty() {
        return outcome;
    }

    todo_lines.sort_unstable();
    todo_lines.dedup();
    for start in todo_lines {
        let indent: String = source[start..]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        edits.push(Edit {
            at: start..start,
            text: format!(
                "{indent}# TODO(spectcl 2.0): this row's tokens name environment \
                 membership, not availability;\n{indent}# they become the environment's \
                 ambient package once the environment registry lands (upgrade spec U3).\n"
            ),
        });
    }

    // U1: the version word moves only when the body rewrite completed on
    // this file. A `2.0` header over rows still spelled 1.x is loadable but
    // is a failed upgrade, so a partial file keeps its old header and says
    // so.
    let complete = outcome.deferred.is_empty();
    if complete && declared != options.to {
        edits.push(Edit {
            at: version_span,
            text: options.to.clone(),
        });
    }

    if edits.is_empty() {
        outcome.status = UpgradeStatus::AlreadyCurrent;
    } else {
        outcome.source = apply(source, edits);
        outcome.status = if complete {
            UpgradeStatus::Upgraded
        } else {
            UpgradeStatus::Partial
        };
    }

    outcome.above_target = words_above_declaration(&outcome.source);
    outcome
}

/// U10: `--from`/`--to` must both name a vocabulary, and a downgrade is
/// refused — an unsupported major fails closed, so a 2.0 → 1.x rewrite is a
/// silent capability loss rather than a conversion.
fn check_versions(options: &UpgradeOptions) -> Option<Refusal> {
    for version in [&options.from, &options.to] {
        if !KNOWN_VOCABULARY_VERSIONS.contains(&version.as_str()) {
            return Some(Refusal {
                line: 1,
                message: format!(
                    "`{version}` is not a SpecTcl vocabulary this build knows ({})",
                    KNOWN_VOCABULARY_VERSIONS.join(", ")
                ),
            });
        }
    }
    if tcl_registry::version::compare(&options.to, &options.from).is_lt() {
        return Some(Refusal {
            line: 1,
            message: format!(
                "refusing to downgrade {} → {}: an unsupported major fails closed, so a \
                 downgrade is a silent capability loss, not a conversion (upgrade spec U10)",
                options.from, options.to
            ),
        });
    }
    None
}

/// What to do with one located `dialects` row.
enum SitePlan {
    /// Replace the keyword and the value with these provider rows.
    Translate(Vec<String>),
    /// Leave it byte-identical and mark it (U3).
    Defer,
    /// Stop: the whole file is refused.
    Refuse,
}

fn plan_site(site: &Site, outcome: &mut UpgradeOutcome) -> SitePlan {
    let tokens = list_words(&site.value_text);
    if tokens.is_empty() {
        outcome.refusals.push(Refusal {
            line: site.line,
            message: format!("`{}` names no dialect; fix it by hand", site.keyword.0),
        });
        return SitePlan::Refuse;
    }
    let mut rows = Vec::with_capacity(tokens.len());
    let mut defer = false;
    for token in &tokens {
        if token == REFUSED {
            outcome.refusals.push(Refusal {
                line: site.line,
                message: format!(
                    "`{REFUSED}` leaves the Tcl axis entirely and has no `available` \
                     spelling (design §2, Q3); the file is not rewritten"
                ),
            });
            return SitePlan::Refuse;
        }
        if DEFERRED.contains(&token.as_str()) {
            outcome.deferred.push(Deferred {
                line: site.line,
                token: token.clone(),
                reason: format!(
                    "`{token}` names environment membership, not availability; it becomes \
                     that environment's ambient package once the environment registry \
                     lands (upgrade spec U3)"
                ),
            });
            defer = true;
            continue;
        }
        let Some((_, row)) = TRANSLATIONS
            .iter()
            .find(|(legacy, _)| *legacy == token.as_str())
        else {
            outcome.refusals.push(Refusal {
                line: site.line,
                message: format!(
                    "`{token}` is not a SpecTcl 1.x dialect word; the file is not \
                     rewritten (the 1.x vocabulary is closed — upgrade spec U0)"
                ),
            });
            return SitePlan::Refuse;
        };
        rows.push((*row).to_owned());
    }
    if defer {
        SitePlan::Defer
    } else {
        SitePlan::Translate(rows)
    }
}

/// The 2.0 spelling of the keyword at this scope.
fn available_word(is_flag: bool) -> &'static str {
    if is_flag { "-available" } else { "available" }
}

/// The provider rows as one written word.
///
/// A flag takes exactly one value word, so several rows nest inside one
/// braced list; a property statement takes them as separate words, which is
/// how the design writes them (`available {tcl 8.6-} {package Tk}`).
fn render_row_word(rows: &[String], is_flag: bool) -> String {
    let braced: Vec<String> = rows.iter().map(|row| format!("{{{row}}}")).collect();
    if is_flag && braced.len() > 1 {
        format!("{{{}}}", braced.join(" "))
    } else {
        braced.join(" ")
    }
}

/// Apply every edit back-to-front, so no earlier offset is disturbed.
fn apply(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.at.start));
    let mut out = source.to_owned();
    for edit in edits {
        out.replace_range(edit.at, &edit.text);
    }
    out
}

/// U7: every site in `source` whose word is newer than the file's own
/// `speclib` declaration, as the loader reports them.
fn words_above_declaration(source: &str) -> Vec<String> {
    load_pack(source)
        .notices
        .into_iter()
        .filter(|notice| notice.message.contains("but this pack declares vocabulary"))
        .map(|notice| format!("{}:{}: {}", notice.context, notice.line, notice.message))
        .collect()
}

/// One located `dialects` / `-dialects` row.
struct Site {
    /// The keyword's spelling and its byte range.
    keyword: (String, Range<usize>),
    /// Whether it was written as a flag (`-dialects`).
    is_flag: bool,
    /// The value word's **whole** range, delimiters included.
    value_word: Range<usize>,
    /// The value's decoded text, exactly as the loader reads it.
    value_text: String,
    /// 1-based line of the keyword.
    line: u32,
    /// Byte offset of the start of the keyword's line.
    line_start: usize,
}

/// Every `dialects` / `-dialects` row in a pack source, located through the
/// loader's own lexer.
fn dialects_sites(source: &str) -> Vec<Site> {
    let mut out = Vec::new();
    scan(source, 0, &mut out);
    out.sort_by_key(|site| site.keyword.1.start);
    locate_lines(source, &mut out);
    out
}

/// Recursive descent over the CST, mirroring the loader's own.
///
/// `base` is the absolute offset of `text` inside the whole file, so every
/// recorded range is a range in the file rather than in the block.
fn scan(text: &str, base: usize, out: &mut Vec<Site>) {
    let source_map = SourceMap::new(text);
    let (document, _warnings) = build_document(text, LexerConfig::default());
    for segment in segments_from_document(document, &source_map) {
        // A `hook` body is arbitrary Tcl, not pack vocabulary; descending
        // into it would let a body's own words be rewritten as if they were
        // declarations.
        let is_hook = segment.texts.first().map(String::as_str) == Some("hook");
        for (index, token) in segment.argv.iter().enumerate() {
            let Some(word) = segment.texts.get(index) else {
                continue;
            };
            let braced = token.kind == TokenType::Str && !token.in_quote;
            let is_flag = word == "-dialects";
            if (is_flag || word == "dialects") && index + 1 < segment.argv.len() {
                let value = &segment.argv[index + 1];
                let start = value.span.start() as usize;
                let end = outer_end(value.span.end() as usize, value.content_offset);
                let absolute = base + token.span.start() as usize;
                out.push(Site {
                    keyword: (word.clone(), absolute..base + token.span.end() as usize),
                    is_flag,
                    value_word: base + start..base + end,
                    value_text: segment.texts[index + 1].clone(),
                    line: 0,
                    line_start: 0,
                });
                continue;
            }
            if braced && !is_hook {
                let content = token.span.start() as usize + usize::from(token.content_offset);
                let close = token.span.end() as usize;
                if close > content {
                    scan(&text[content..close], base + content, out);
                }
            }
        }
    }
}

/// A word token's end **including** its closing delimiter.
///
/// Word tokens exclude the closing delimiter from `span.end` and carry the
/// opening one via `content_offset`, so a wrapped word runs one byte past
/// the token and a bare word does not.
fn outer_end(end: usize, content_offset: u8) -> usize {
    if content_offset > 0 { end + 1 } else { end }
}

/// Fill in each site's line number and line start, which need the whole
/// file's text rather than the block the site was found in.
fn locate_lines(source: &str, sites: &mut [Site]) {
    let mut line_starts: Vec<usize> = vec![0];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(offset + 1);
        }
    }
    for site in sites {
        let index = match line_starts.binary_search(&site.keyword.1.start) {
            Ok(found) => found,
            Err(next) => next.saturating_sub(1),
        };
        site.line = u32::try_from(index + 1).unwrap_or(u32::MAX);
        site.line_start = line_starts[index];
    }
}

#[cfg(test)]
mod tests {
    use super::{UpgradeOptions, UpgradeStatus, upgrade_source};
    use crate::loader::load_pack;

    /// A pack with a `dialects` row at each of the three written shapes —
    /// property, flag, multi-token — comes out spelling `available`, with
    /// every other byte where the author left it.
    #[test]
    fn dialects_rows_translate_and_nothing_else_moves() {
        let source = "# a comment the rewriter must not touch\n\
                      speclib probe 1.2 {\n\
                      \x20   command demo {\n\
                      \x20       arity 1..\n\
                      \x20       dialects {tcl8.6+ f5-irules}\n\
                      \x20       option -x -dialects tcl8.x   ;# trailing comment\n\
                      \x20       subcommand sub {\n\
                      \x20           arity 0\n\
                      \x20           dialects tk\n\
                      \x20       }\n\
                      \x20   }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
        assert_eq!(outcome.translated.len(), 3, "{:#?}", outcome.translated);
        assert!(outcome.deferred.is_empty(), "{:#?}", outcome.deferred);
        assert!(outcome.refusals.is_empty(), "{:#?}", outcome.refusals);
        assert_eq!(
            outcome.source,
            "# a comment the rewriter must not touch\n\
             speclib probe 2.0 {\n\
             \x20   command demo {\n\
             \x20       arity 1..\n\
             \x20       available {tcl 8.6-} {f5-irules}\n\
             \x20       option -x -available {tcl 8.4-9.0}   ;# trailing comment\n\
             \x20       subcommand sub {\n\
             \x20           arity 0\n\
             \x20           available {package Tk}\n\
             \x20       }\n\
             \x20   }\n\
             }\n"
        );
        // U7: nothing in the rewritten file needs a vocabulary above its
        // own declaration.
        assert!(
            outcome.above_target.is_empty(),
            "{:#?}",
            outcome.above_target
        );
    }

    /// U9 in miniature: the rewritten pack loads to the same specs.
    #[test]
    fn the_rewritten_pack_loads_to_the_same_specs() {
        let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6+ tk}\n\
                      \x20 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
        let before = load_pack(source);
        let after = load_pack(&outcome.source);
        assert!(after.notices.is_empty(), "{:?}", after.notices);
        assert_eq!(
            format!("{:?}", before.command("demo").expect("demo").spec),
            format!("{:?}", after.command("demo").expect("demo").spec),
        );
    }

    /// U3: an environment-membership token is left byte-identical, marked
    /// in place, and reported — and the version word does **not** move,
    /// because the file is only partially upgraded (U1).
    #[test]
    fn an_environment_membership_token_is_deferred_not_guessed() {
        let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6 f5-iapps}\n\
                      \x20 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Partial, "{outcome:#?}");
        assert_eq!(outcome.deferred.len(), 1);
        assert_eq!(outcome.deferred[0].token, "f5-iapps");
        assert!(
            outcome.source.contains("# TODO(spectcl 2.0):"),
            "{}",
            outcome.source
        );
        assert!(
            outcome.source.contains("dialects {tcl8.6 f5-iapps}"),
            "the row itself is untouched: {}",
            outcome.source
        );
        assert!(
            outcome.source.contains("speclib probe 1.2"),
            "a partial upgrade keeps its 1.x header: {}",
            outcome.source
        );
    }

    /// Q3: `f5-bigip` refuses the whole file rather than being translated
    /// into a claim about the Tcl axis.
    #[test]
    fn f5_bigip_refuses_the_file() {
        let source = "speclib probe 1.2 {\n command demo {\n arity 1\n \
                      dialects f5-bigip\n }\n}\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Refused);
        assert_eq!(outcome.source, source, "a refused file is not rewritten");
        assert!(
            outcome.refusals[0].message.contains("leaves the Tcl axis"),
            "{:#?}",
            outcome.refusals
        );
    }

    /// U0: the 1.x vocabulary is closed, so a word outside it refuses the
    /// file rather than being carried through unread.
    #[test]
    fn a_non_vocabulary_word_refuses_the_file() {
        let source = "speclib probe 1.2 {\n command demo {\n arity 1\n \
                      dialects klingon\n }\n}\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Refused);
        assert_eq!(outcome.source, source);
        assert!(
            outcome.refusals[0]
                .message
                .contains("is not a SpecTcl 1.x dialect word"),
            "{:#?}",
            outcome.refusals
        );
    }

    /// U10: downgrades are refused.
    #[test]
    fn a_downgrade_is_refused() {
        let outcome = upgrade_source(
            "speclib probe 2.0 {\n command demo { arity 1 }\n}\n",
            &UpgradeOptions {
                from: "2.0".to_owned(),
                to: "1.2".to_owned(),
            },
        );
        assert_eq!(outcome.status, UpgradeStatus::Refused);
        assert!(
            outcome.refusals[0]
                .message
                .contains("refusing to downgrade 2.0 → 1.2"),
            "{:#?}",
            outcome.refusals
        );
    }

    /// A file with no `speclib` line is skipped, not rewritten.
    #[test]
    fn a_file_with_no_speclib_line_is_skipped() {
        let outcome = upgrade_source("# just a comment\n", &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::NotAPack);
    }

    /// A hook body is arbitrary Tcl, not pack vocabulary: its words are
    /// never rewritten, even when one of them reads like a declaration.
    #[test]
    fn a_hook_body_is_not_descended_into() {
        let source = "speclib probe 1.2 {\n\
                      \x20 hook probe {ctx} {\n\
                      \x20   set dialects tcl8.6\n\
                      \x20 }\n\
                      \x20 command demo { arity 1 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert!(outcome.translated.is_empty(), "{:#?}", outcome.translated);
        assert!(
            outcome.source.contains("set dialects tcl8.6"),
            "{}",
            outcome.source
        );
    }

    /// A pack already at the target vocabulary with no 1.x spellings left
    /// is finished, and its bytes are untouched.
    #[test]
    fn an_already_current_pack_is_left_alone() {
        let source = "speclib probe 2.0 {\n command demo {\n arity 1\n \
                      available {tcl 8.6-}\n }\n}\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::AlreadyCurrent);
        assert_eq!(outcome.source, source);
    }
}
