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
//!    [`MEMBERSHIP`], and the single refused token below are all of it —
//!    so the translation table is total and a token outside it means the
//!    file is *refused*, never guessed at (U0).
//! 3. **The version word moves only when the body did** (U1). A `2.0`
//!    header over 1.x spellings is loadable but is a failed upgrade, so a
//!    file with any row left for the environment wire-up (U3) reports
//!    *partially upgraded* and keeps its 1.x header.
//!
//! With the P1 environment registry live, the environment-shaped halves
//! are real translations rather than markers:
//!
//! - **U3** — an environment-membership token maps to the environment's
//!   own ambient package provider (`f5-iapps` → `{package
//!   f5-iapps-cmds}`), read off the registry's placement rows; a token
//!   whose environment declares **no** ambient provider (`spectcl`,
//!   `bpf` — their surfaces are compiled, not package-provided) keeps
//!   the marker, because inventing a provider would re-scope the claim.
//! - **U4** — `ambient_package NAME VERSION` becomes an
//!   environment-scoped `ambient` placement inside an
//!   `environment OWNER -extend { … }` block, where OWNER is the pack's
//!   sole declared environment or its sole membership token; an
//!   ambiguous pack keeps the marker. Never guessed.
//! - **U5** — `file_extension … -dialect D` becomes a detection row
//!   inside `environment D -extend { … }`; an unresolvable `D` keeps
//!   the marker.
//!
//! The U9 gate widens with them: `--verify` compares command snapshots
//! across every axis a 1.x row can gate on **and** the
//! [`environment_effect_snapshot`] — the scoped detection and placement
//! rows both forms load to — so a translation that moved a row's home
//! must still mean the same thing to the registry.

use std::ops::Range;
use tcl_dialect::model::SpecProvider;

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_lexer::{LexerConfig, SourceMap, TokenType};

use crate::export::{ExportLoss, export_pack_reporting};
use crate::loader::{
    KNOWN_VOCABULARY_VERSIONS, NEWEST_VOCABULARY_VERSION, Pack, evaluate_pack, list_words,
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
    // `SpecSurface::TCL8X` bit has always meant.
    ("tcl8.x", "tcl 8.4-9.0"),
    ("f5-irules", "f5-irules"),
    // Review B11: Tk is a package on its own axis, never a Tcl release.
    ("tk", "package Tk"),
];

/// Tokens that are **environment membership**, not availability (U3).
///
/// Each names an environment whose ambient package provides the surface.
/// The environment registry answers *which* package at *which* placement:
/// a token whose environment declares exactly one ambient package
/// translates to `{package NAME}` (which the loader projects back onto
/// the same 1.x bit, so the spec is byte-equal — see
/// `loader::available`'s environment-derived table); a token whose
/// environment declares none stays a marker, because its surface is
/// compiled rather than package-provided and no `available` row can say
/// that yet.
const MEMBERSHIP: &[&str] = &["f5-iapps", "f5-tmsh", "expect", "spectcl", "bpf"];

/// The ambient package provider standing behind an environment-membership
/// token, when the environment registry declares exactly one.
fn membership_provider(token: &str) -> Option<String> {
    let environment = tcl_registry::model::resolve_known_environment(token)?;
    environment.definition.core?;
    let ambient: Vec<&str> = environment
        .definition
        .expected_packages
        .iter()
        .filter(|placement| placement.ambient)
        .map(|placement| placement.package.as_ref())
        .collect();
    match ambient.as_slice() {
        [sole] => Some((*sole).to_owned()),
        _ => None,
    }
}

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
    /// U6: hoist a uniform `required_package` (the pack-level default, or
    /// one identical row in every command) to a pack-level `provides`.
    /// Off by default — it changes shape, not spelling.
    pub infer_provides: bool,
    /// `--restyle` (ledger D13): once the rows are rewritten, re-emit the
    /// whole pack in **canonical** form through the same renderer `tcl spec
    /// export` uses — straight-line registration calls at the house
    /// layout, comments and author layout dropped. Off by default: the
    /// source rewrite above preserves every byte it does not translate
    /// (U8), and a restyle deliberately does not.
    ///
    /// Two refusals keep it honest. A **programmed** pack — one that queried
    /// `available?`, or whose `speclib` body holds a statement that is not
    /// one of the registration calls the snapshot recorded — is never
    /// rewritten (E-R12): writing its expansion over its program would
    /// silently delete the program. And a **partial** upgrade keeps its
    /// `# TODO(spectcl 2.0):` markers, which an export (which writes no
    /// comments) would drop, so it is not restyled.
    pub restyle: bool,
}

impl Default for UpgradeOptions {
    fn default() -> Self {
        Self {
            from: OLDEST_VOCABULARY_VERSION.to_owned(),
            to: NEWEST_VOCABULARY_VERSION.to_owned(),
            infer_provides: false,
            restyle: false,
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
    /// Nothing to translate, but `--restyle` re-emitted the file in
    /// canonical form, so it changed all the same.
    Restyled,
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
    /// Whether [`UpgradeOptions::restyle`] re-emitted the file in canonical
    /// form. `false` when it was not asked for, when the file was already
    /// canonical, or when the restyle was skipped (see
    /// [`Self::restyle_skipped`]).
    pub restyled: bool,
    /// Why an asked-for restyle did not happen while the upgrade itself
    /// still went ahead — a partial upgrade keeping its markers. A
    /// programmed pack is a refusal of the whole file instead.
    pub restyle_skipped: Option<String>,
    /// Words the canonical renderer had to quote rather than write
    /// verbatim during a restyle (see [`ExportLoss`]).
    pub restyle_losses: Vec<ExportLoss>,
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
        restyled: false,
        restyle_skipped: None,
        restyle_losses: Vec::new(),
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
    let mut markers: Vec<(usize, &'static str)> = Vec::new();
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
            SitePlan::Defer => markers.push((site.line_start, MARKER_U3)),
            SitePlan::Refuse => return outcome,
        }
    }
    if !outcome.refusals.is_empty() {
        return outcome;
    }

    // U4/U5: the pack-level environment-shaped rows, over the same lexer.
    let rows = pack_level_rows(source);
    let owner = source_owner(&rows, &sites);
    plan_ambient_rows(
        source,
        &rows,
        owner.as_deref(),
        &mut outcome,
        &mut edits,
        &mut markers,
    );
    plan_extension_rows(source, &rows, &mut outcome, &mut edits, &mut markers);
    if options.infer_provides {
        plan_infer_provides(source, &rows, &mut outcome, &mut edits);
    }

    emit_markers(source, markers, &mut edits);

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

    if options.restyle {
        restyle(source, &mut outcome);
    }

    outcome.above_target = words_above_declaration(&outcome.source);
    outcome
}

/// D13: re-emit the (rewritten) file in canonical form, or say why not.
fn restyle(source: &str, outcome: &mut UpgradeOutcome) {
    match outcome.status {
        UpgradeStatus::Upgraded | UpgradeStatus::AlreadyCurrent => {}
        UpgradeStatus::Partial => {
            outcome.restyle_skipped = Some(
                "the upgrade is partial and its `# TODO(spectcl 2.0):` markers would not \
                 survive a canonical re-emission, which writes no comments; finish the \
                 environment wire-up first"
                    .to_owned(),
            );
            return;
        }
        UpgradeStatus::Refused | UpgradeStatus::NotAPack | UpgradeStatus::Restyled => return,
    }
    let pack = evaluate_pack(&outcome.source);
    let refusal = if pack.load_error.is_some() {
        Some("the pack does not load, so there is no snapshot to re-emit".to_owned())
    } else if pack.target_dependent {
        Some(
            "the pack queried `available?` while registering, so its snapshot is one \
             analysis target's answer; a programmed pack is never rewritten (design E-R12)"
                .to_owned(),
        )
    } else if !straight_line(&outcome.source, &pack) {
        Some(
            "the `speclib` body runs rather than registers — a `proc`, `set`, `foreach` or \
             computed declaration — and writing its expansion over it would delete the \
             program; a programmed pack is never rewritten (design E-R12). Read the \
             expansion with `tcl spec export` instead"
                .to_owned(),
        )
    } else {
        None
    };
    if let Some(message) = refusal {
        outcome.refusals.push(Refusal {
            line: 1,
            message: format!("not restyled: {message}"),
        });
        outcome.status = UpgradeStatus::Refused;
        source.clone_into(&mut outcome.source);
        outcome.translated.clear();
        return;
    }
    let (canonical, losses) = export_pack_reporting(&pack);
    outcome.restyle_losses = losses;
    if canonical == outcome.source {
        return;
    }
    outcome.source = canonical;
    outcome.restyled = true;
    if outcome.status == UpgradeStatus::AlreadyCurrent {
        outcome.status = UpgradeStatus::Restyled;
    }
}

/// Whether every top-level statement of the `speclib` body is one of the
/// registration calls the snapshot recorded, in the same order — the
/// canonical-subset test (E-R11) a restyle needs before it may write a
/// snapshot back over its source. True for every straight-line pack, and
/// false the moment a statement *runs* rather than registers.
fn straight_line(source: &str, pack: &Pack) -> bool {
    let rows = pack_level_rows(source);
    rows.len() == pack.registrations.len()
        && rows.iter().zip(&pack.registrations).all(|(row, reg)| {
            row.words.first().map(String::as_str) == Some(reg.word())
                && row.words.get(1).map_or("", String::as_str) == reg.arg(1)
        })
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
        if MEMBERSHIP.contains(&token.as_str()) {
            // U3: the environment registry answers which package stands
            // behind the membership token. One ambient provider is the
            // unambiguous mapping; none means the environment's surface
            // is compiled, and the row keeps its marker.
            if let Some(package) = membership_provider(token) {
                rows.push(format!("package {package}"));
            } else {
                outcome.deferred.push(Deferred {
                    line: site.line,
                    token: token.clone(),
                    reason: format!(
                        "`{token}` names environment membership, and the `{token}` \
                         environment declares no ambient package provider (its surface \
                         is compiled, not package-provided), so no `available` row can \
                         carry the claim yet (upgrade spec U3)"
                    ),
                });
                defer = true;
            }
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

/// Write each marker as a `# TODO(spectcl 2.0)` comment block above its
/// row, at the row's own indent, one block per (row, reason) pair.
fn emit_markers(source: &str, mut markers: Vec<(usize, &'static str)>, edits: &mut Vec<Edit>) {
    markers.sort_unstable();
    markers.dedup();
    for (start, message) in markers {
        let indent: String = source[start..]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let mut text = String::new();
        for line in message.lines() {
            text.push_str(&indent);
            text.push_str("# ");
            text.push_str(line);
            text.push('\n');
        }
        edits.push(Edit {
            at: start..start,
            text,
        });
    }
}

/// The U3 marker: the row's tokens stayed because no ambient provider
/// stands behind them.
const MARKER_U3: &str = "TODO(spectcl 2.0): this row keeps an environment-membership token whose\n\
                         environment declares no ambient package provider; its surface is compiled,\n\
                         so the row has no `available` spelling yet (upgrade spec U3).";

/// The U4 marker: the pack's owning environment could not be inferred.
const MARKER_U4: &str = "TODO(spectcl 2.0): this `ambient_package` row needs the pack's owning\n\
                         environment and it could not be inferred (declare exactly one `environment`\n\
                         block, or gate the pack on one membership token); upgrade spec U4.";

/// The U5 marker: the routing target could not be resolved.
const MARKER_U5: &str = "TODO(spectcl 2.0): this `file_extension` row's `-dialect` names no\n\
                         catalogue environment, so its detection row has no `environment` block\n\
                         to move into; upgrade spec U5.";

// Pack-level rows (U4/U5/U6) — located through the same lexer as the sites

/// One pack-level statement, with the ranges a replacement edit needs.
struct PackRow {
    /// The decoded words.
    words: Vec<String>,
    /// Each word's **outer** byte range (delimiters included).
    word_ranges: Vec<Range<usize>>,
    /// Each word's **content** byte range (inside any delimiters).
    content_ranges: Vec<Range<usize>>,
    /// Whether each word was written braced.
    braced: Vec<bool>,
    /// The whole statement's byte range.
    range: Range<usize>,
    /// 1-based line of the statement.
    line: u32,
    /// Byte offset of the start of the statement's line.
    line_start: usize,
}

impl PackRow {
    fn word(&self, index: usize) -> &str {
        self.words.get(index).map_or("", String::as_str)
    }

    /// The statement's leading indentation, as written.
    fn indent<'s>(&self, source: &'s str) -> &'s str {
        let head = &source[self.line_start..self.range.start];
        if head.chars().all(|c| c == ' ' || c == '\t') {
            head
        } else {
            ""
        }
    }
}

/// Every statement directly inside the first `speclib` body — pack scope
/// only, no recursion — located through the loader's own lexer.
fn pack_level_rows(source: &str) -> Vec<PackRow> {
    let source_map = SourceMap::new(source);
    let (document, _warnings) = build_document(source, LexerConfig::default());
    let mut body: Option<Range<usize>> = None;
    for segment in segments_from_document(document, &source_map) {
        if segment.texts.first().map(String::as_str) != Some("speclib") {
            continue;
        }
        if let Some(token) = segment.argv.get(3) {
            let content = token.span.start() as usize + usize::from(token.content_offset);
            let close = token.span.end() as usize;
            if token.kind == TokenType::Str && !token.in_quote && close > content {
                body = Some(content..close);
            }
        }
        break;
    }
    let Some(body) = body else {
        return Vec::new();
    };
    let mut rows = rows_in(&source[body.clone()], body.start);
    locate_row_lines(source, &mut rows);
    rows
}

/// The statements of one block's text, with ranges absolute to the file.
fn rows_in(text: &str, base: usize) -> Vec<PackRow> {
    let source_map = SourceMap::new(text);
    let (document, _warnings) = build_document(text, LexerConfig::default());
    let mut out = Vec::new();
    for segment in segments_from_document(document, &source_map) {
        let mut words = Vec::new();
        let mut word_ranges = Vec::new();
        let mut content_ranges = Vec::new();
        let mut braced = Vec::new();
        for (index, token) in segment.argv.iter().enumerate() {
            let Some(word) = segment.texts.get(index) else {
                continue;
            };
            words.push(word.clone());
            let start = token.span.start() as usize;
            let end = token.span.end() as usize;
            word_ranges.push(base + start..base + outer_end(end, token.content_offset));
            content_ranges.push(base + start + usize::from(token.content_offset)..base + end);
            braced.push(token.kind == TokenType::Str && !token.in_quote);
        }
        let (Some(first), Some(last)) = (word_ranges.first(), word_ranges.last()) else {
            continue;
        };
        let range = first.start..last.end;
        out.push(PackRow {
            words,
            word_ranges,
            content_ranges,
            braced,
            range,
            line: 0,
            line_start: 0,
        });
    }
    out
}

/// Fill each row's line number and line start from the whole file.
fn locate_row_lines(source: &str, rows: &mut [PackRow]) {
    let line_starts = line_starts_of(source);
    for row in rows {
        let (line, line_start) = locate(&line_starts, row.range.start);
        row.line = line;
        row.line_start = line_start;
    }
}

fn line_starts_of(source: &str) -> Vec<usize> {
    let mut line_starts: Vec<usize> = vec![0];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(offset + 1);
        }
    }
    line_starts
}

fn locate(line_starts: &[usize], offset: usize) -> (u32, usize) {
    let index = match line_starts.binary_search(&offset) {
        Ok(found) => found,
        Err(next) => next.saturating_sub(1),
    };
    (
        u32::try_from(index + 1).unwrap_or(u32::MAX),
        line_starts[index],
    )
}

/// The pack's owning environment, from its source (U4's cannot-infer
/// rule): the sole declared (non-`-extend`) `environment` block wins;
/// with none declared, the sole environment-membership token across the
/// pack's `dialects` rows; anything else is ambiguous. Never guessed.
fn source_owner(rows: &[PackRow], sites: &[Site]) -> Option<String> {
    let mut declared: Vec<&str> = rows
        .iter()
        .filter(|row| {
            row.word(0) == "environment"
                && !row.word(1).is_empty()
                && row.word(2) != "-extend"
                && row.words.len() >= 3
        })
        .map(|row| row.word(1))
        .collect();
    declared.sort_unstable();
    declared.dedup();
    if let [sole] = declared.as_slice() {
        return Some((*sole).to_owned());
    }
    if !declared.is_empty() {
        return None;
    }
    let mut tokens: Vec<String> = sites
        .iter()
        .flat_map(|site| list_words(&site.value_text))
        .filter(|token| MEMBERSHIP.contains(&token.as_str()))
        .collect();
    tokens.sort_unstable();
    tokens.dedup();
    match tokens.as_slice() {
        [sole] => Some(sole.clone()),
        _ => None,
    }
}

/// U4: rehome each `ambient_package NAME VERSION` row into the owning
/// environment's `-extend` block, or mark it.
fn plan_ambient_rows(
    source: &str,
    rows: &[PackRow],
    owner: Option<&str>,
    outcome: &mut UpgradeOutcome,
    edits: &mut Vec<Edit>,
    markers: &mut Vec<(usize, &'static str)>,
) {
    for row in rows.iter().filter(|row| row.word(0) == "ambient_package") {
        let name = row.word(1);
        let version = row.word(2);
        if name.is_empty() || version.is_empty() {
            // The loader already drops an incomplete row with its own
            // notice; there is nothing translatable here.
            continue;
        }
        let plain_version = tcl_dialect::model::Version::parse(version).is_ok();
        let (Some(owner), true) = (owner, plain_version) else {
            outcome.deferred.push(Deferred {
                line: row.line,
                token: format!("ambient_package {name}"),
                reason: if plain_version {
                    format!(
                        "`ambient_package {name}` never named its environment and the \
                         pack's owner is ambiguous; declare exactly one `environment` \
                         block or gate the pack on one membership token (upgrade spec U4)"
                    )
                } else {
                    format!(
                        "`ambient_package {name} {version}` does not carry a plain \
                         version, so its placement row cannot be spelled without \
                         changing meaning (upgrade spec U4)"
                    )
                },
            });
            markers.push((row.line_start, MARKER_U4));
            continue;
        };
        let indent = row.indent(source);
        // The row's own bytes past the head word, verbatim, so the
        // author's spelling of name and version survives.
        let rest = &source[row.word_ranges[1].start..row.range.end];
        let before = source[row.range.clone()].to_owned();
        let after =
            format!("environment {owner} -extend {{\n{indent}    ambient {rest}\n{indent}}}");
        outcome.translated.push(Translated {
            line: row.line,
            before,
            after: after.clone(),
        });
        edits.push(Edit {
            at: row.range.clone(),
            text: after,
        });
    }
}

/// U5: move each `file_extension … -dialect D` row's detection into
/// `environment D -extend { … }`, or mark it.
fn plan_extension_rows(
    source: &str,
    rows: &[PackRow],
    outcome: &mut UpgradeOutcome,
    edits: &mut Vec<Edit>,
    markers: &mut Vec<(usize, &'static str)>,
) {
    for row in rows.iter().filter(|row| row.word(0) == "file_extension") {
        let Some(flag) = row.words.iter().position(|word| word == "-dialect") else {
            // No routing claim: the row is display-only and stays valid
            // 2.0 vocabulary where it is.
            continue;
        };
        let Some(value_range) = row.word_ranges.get(flag + 1) else {
            continue;
        };
        let dialect = row.word(flag + 1);
        // The same resolution the loader's own row reader applies: only a
        // catalogue environment is a routing target.
        let Some(profile) = crate::environment::catalogue_profile_for_dialect(dialect) else {
            outcome.deferred.push(Deferred {
                line: row.line,
                token: format!("file_extension {}", row.word(1)),
                reason: format!(
                    "`-dialect {dialect}` names no catalogue environment, so the \
                     detection row has no `environment` block to move into \
                     (upgrade spec U5)"
                ),
            });
            markers.push((row.line_start, MARKER_U5));
            continue;
        };
        // The row minus the `-dialect D` flag (and the whitespace before
        // it), verbatim.
        let mut cut_start = row.word_ranges[flag].start;
        while cut_start > row.range.start
            && matches!(source.as_bytes()[cut_start - 1], b' ' | b'\t')
        {
            cut_start -= 1;
        }
        let inner = format!(
            "{}{}",
            &source[row.range.start..cut_start],
            &source[value_range.end..row.range.end]
        );
        let indent = row.indent(source);
        let before = source[row.range.clone()].to_owned();
        let after = format!(
            "environment {} -extend {{\n{indent}    {}\n{indent}}}",
            profile.name,
            inner.trim_end()
        );
        outcome.translated.push(Translated {
            line: row.line,
            before,
            after: after.clone(),
        });
        edits.push(Edit {
            at: row.range.clone(),
            text: after,
        });
    }
}

/// U6 (`--infer-provides`): hoist a uniform `required_package` to a
/// pack-level `provides` — the pack-level default when one exists, else
/// one identical row in every command.
fn plan_infer_provides(
    source: &str,
    rows: &[PackRow],
    outcome: &mut UpgradeOutcome,
    edits: &mut Vec<Edit>,
) {
    if rows.iter().any(|row| row.word(0) == "provides") {
        return;
    }
    // The pack-level default is the simple hoist: same fallback rule at
    // load (`provides` feeds the same default), so the snapshot is
    // unchanged.
    if let Some(row) = rows
        .iter()
        .find(|row| row.word(0) == "default" && row.word(1) == "required_package")
    {
        let name = row.word(2);
        if name.is_empty() {
            return;
        }
        let before = source[row.range.clone()].to_owned();
        let after = format!("provides {name}");
        outcome.translated.push(Translated {
            line: row.line,
            before,
            after: after.clone(),
        });
        edits.push(Edit {
            at: row.range.clone(),
            text: after,
        });
        return;
    }
    // Else: one identical `required_package` row in every command.
    let commands: Vec<&PackRow> = rows.iter().filter(|row| row.word(0) == "command").collect();
    if commands.is_empty() {
        return;
    }
    let mut name: Option<String> = None;
    let mut row_sites: Vec<PackRow> = Vec::new();
    for command in &commands {
        let Some(body) = (2..command.words.len()).find(|&index| command.braced[index]) else {
            return;
        };
        let content = command.content_ranges[body].clone();
        let mut body_rows = rows_in(&source[content.clone()], content.start);
        locate_row_lines(source, &mut body_rows);
        let required: Vec<PackRow> = body_rows
            .into_iter()
            .filter(|row| row.word(0) == "required_package" && !row.word(1).is_empty())
            .collect();
        let [row] = &required[..] else {
            return;
        };
        match &name {
            None => name = Some(row.word(1).to_owned()),
            Some(prior) if prior == row.word(1) => {}
            Some(_) => return,
        }
        row_sites.extend(required);
    }
    let Some(name) = name else {
        return;
    };
    for row in &row_sites {
        // Delete the whole line when the row owns it; otherwise just the
        // statement's bytes.
        let line_end = source[row.range.end..]
            .find('\n')
            .map_or(source.len(), |at| row.range.end + at + 1);
        let alone = source[row.line_start..row.range.start]
            .chars()
            .all(|c| c == ' ' || c == '\t')
            && source[row.range.end..line_end.saturating_sub(1)]
                .chars()
                .all(|c| c == ' ' || c == '\t');
        let at = if alone {
            row.line_start..line_end
        } else {
            row.range.clone()
        };
        outcome.translated.push(Translated {
            line: row.line,
            before: source[row.range.clone()].to_owned(),
            after: String::new(),
        });
        edits.push(Edit {
            at,
            text: String::new(),
        });
    }
    let first = commands[0];
    let indent = first.indent(source).to_owned();
    outcome.translated.push(Translated {
        line: first.line,
        before: String::new(),
        after: format!("provides {name}"),
    });
    edits.push(Edit {
        at: first.line_start..first.line_start,
        text: format!("{indent}provides {name}\n\n"),
    });
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

// The environment-effect snapshot (the U5 extension of the U9 gate)

/// The environment-scoped rows a loaded pack means to the registry —
/// detection routing and package placements — normalised so the 1.x
/// spelling (`file_extension … -dialect`, pack-level `ambient_package`)
/// and its 2.0 rehoming (`environment … -extend { … }`) render
/// identically. `--verify` compares this string across the rewrite, which
/// is what lets U4/U5 move a row's home while proving the registry effect
/// did not move.
///
/// A pack-level `ambient_package` row never named its environment; it is
/// attributed to the same owner the U4 rewrite would scope it to
/// ([`source_owner`]'s loaded-pack twin), and `?` when that owner is
/// ambiguous — on both sides of a comparison, so ambiguity itself
/// round-trips.
#[must_use]
pub fn environment_effect_snapshot(pack: &crate::loader::Pack) -> String {
    effect_snapshot(
        &pack.file_extensions,
        &pack.environments,
        &pack.ambient_packages,
        &pack.commands,
    )
}

/// [`environment_effect_snapshot`] over a merged pack — the form
/// `--verify` compares, since it loads through the real merge.
#[must_use]
pub fn merged_environment_effect_snapshot(pack: &crate::pack::MergedPack) -> String {
    effect_snapshot(
        &pack.file_extensions,
        &pack.environments,
        &pack.ambient_packages,
        &pack.commands,
    )
}

fn effect_snapshot(
    file_extensions: &[crate::loader::FileExtension],
    environments: &[crate::loader::PackEnvironment],
    ambient_packages: &[crate::loader::AmbientPackage],
    commands: &[crate::loader::PackCommand],
) -> String {
    let owner = pack_owner(environments, commands);
    let mut rows: Vec<String> = Vec::new();
    for extension in file_extensions {
        if let Some(dialect) = extension.dialect {
            rows.push(format!(
                "detect {dialect} extension {} name {}",
                extension.extension,
                extension
                    .display_name
                    .as_deref()
                    .unwrap_or(&extension.extension)
            ));
        }
    }
    for environment in environments {
        for claim in &environment.file_extensions {
            rows.push(format!(
                "detect {} extension {} name {}",
                environment.id, claim.extension, claim.display_name
            ));
        }
        for filename in &environment.filenames {
            rows.push(format!("detect {} filename {filename}", environment.id));
        }
        for signature in &environment.signatures {
            rows.push(format!("detect {} signature {signature}", environment.id));
        }
        for placement in &environment.placements {
            rows.push(format!(
                "placement {} {} {} {}",
                environment.id,
                if placement.ambient {
                    "ambient"
                } else {
                    "hosted"
                },
                placement.package,
                placement_text(&placement.version),
            ));
        }
    }
    for ambient in ambient_packages {
        let version = tcl_dialect::model::Version::parse(ambient.version)
            .map_or_else(|_| ambient.version.to_owned(), |v| format!("pinned {v}"));
        rows.push(format!(
            "placement {} ambient {} {version}",
            owner.as_deref().unwrap_or("?"),
            ambient.name,
        ));
    }
    rows.sort_unstable();
    let mut out = String::new();
    for row in rows {
        out.push_str(&row);
        out.push('\n');
    }
    out
}

fn placement_text(placement: &tcl_dialect::model::Placement) -> String {
    use tcl_dialect::model::Placement;
    match placement {
        Placement::Pinned(version) => format!("pinned {version}"),
        Placement::TracksBase => "tracks-base".to_owned(),
        Placement::Keyed(axis) => format!("keyed {axis:?}"),
        Placement::Requirement(set) => format!("requirement {set:?}"),
    }
}

/// [`source_owner`]'s loaded-pack twin, deriving the same answer from a
/// loaded pack's pieces: the sole declared environment block, else the
/// sole membership gate across the pack's specs — which survives the U2/U3
/// rewrite because the translated `available` rows project back onto the
/// same 1.x bits.
fn pack_owner(
    environments: &[crate::loader::PackEnvironment],
    commands: &[crate::loader::PackCommand],
) -> Option<String> {
    let declared: Vec<&crate::loader::PackEnvironment> = environments
        .iter()
        .filter(|environment| !environment.extends)
        .collect();
    if let [sole] = declared.as_slice() {
        return Some(sole.id.clone());
    }
    if !declared.is_empty() {
        return None;
    }
    // The providers every command in the pack names, taken together: a pack
    // whose commands all come from one membership surface converts to that
    // environment, and one that spans several is left for the author.
    let mut providers: Vec<SpecProvider> = Vec::new();
    for command in commands {
        for row in command.spec.surface.unwrap_or_default() {
            if !providers.contains(&row.provider) {
                providers.push(row.provider);
            }
        }
    }
    let members: Vec<&str> = MEMBERSHIP
        .iter()
        .copied()
        .filter(|token| {
            crate::catalogue::dialect_surface(token)
                .is_some_and(|rows| rows.iter().any(|row| providers.contains(&row.provider)))
        })
        .collect();
    match members.as_slice() {
        [sole] => Some((*sole).to_owned()),
        _ => None,
    }
}

/// U7: every site in `source` whose word is newer than the file's own
/// `speclib` declaration, as the loader reports them.
fn words_above_declaration(source: &str) -> Vec<String> {
    evaluate_pack(source)
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
    use super::{UpgradeOptions, UpgradeStatus, environment_effect_snapshot, upgrade_source};
    use crate::loader::evaluate_pack;

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

    /// D13: `--restyle` re-emits the upgraded pack in canonical form —
    /// comments and author layout gone, rows at the house margins — and the
    /// restyled pack loads to the same snapshot as the plain rewrite.
    #[test]
    fn restyle_emits_canonical_form_that_loads_to_the_same_snapshot() {
        let source = "# a comment the restyle drops\n\
                      speclib probe 1.1 {\n\
                      display_name {Probe}\n\
                      command demo {\n\
                            arity 1..  ;# odd indent\n\
                        dialects {tcl8.6+ tk}\n\
                        subcommand sub { arity 0 }\n\
                      }\n\
                      }\n";
        let plain = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(plain.status, UpgradeStatus::Upgraded, "{plain:#?}");
        let restyled = upgrade_source(
            source,
            &UpgradeOptions {
                restyle: true,
                ..UpgradeOptions::default()
            },
        );
        assert_eq!(restyled.status, UpgradeStatus::Upgraded, "{restyled:#?}");
        assert!(restyled.restyled);
        assert!(
            restyled.restyle_losses.is_empty(),
            "{:?}",
            restyled.restyle_losses
        );
        assert_eq!(
            restyled.source,
            "speclib probe 1.1 {\n\
             display_name {Probe}\n\
             \n\
             command demo {\n\
             \x20   arity 1..\n\
             \x20   available {tcl 8.6-} {package Tk}\n\
             \n\
             \x20   subcommand sub {\n\
             \x20       arity 0\n\
             \x20   }\n\
             }\n\
             \n\
             }\n"
            .replace("probe 1.1", "probe 2.0")
        );
        assert!(
            restyled.above_target.is_empty(),
            "{:?}",
            restyled.above_target
        );
        let plain_pack = evaluate_pack(&plain.source);
        let restyled_pack = evaluate_pack(&restyled.source);
        assert!(
            restyled_pack.notices.is_empty(),
            "{:?}",
            restyled_pack.notices
        );
        assert_eq!(restyled_pack.display_name.as_deref(), Some("Probe"));
        assert_eq!(
            format!("{:?}", plain_pack.command("demo").expect("demo").spec),
            format!("{:?}", restyled_pack.command("demo").expect("demo").spec),
        );
        assert_eq!(
            environment_effect_snapshot(&plain_pack),
            environment_effect_snapshot(&restyled_pack)
        );
        // A restyled pack is a fixed point of the restyle.
        let again = upgrade_source(
            &restyled.source,
            &UpgradeOptions {
                restyle: true,
                ..UpgradeOptions::default()
            },
        );
        assert_eq!(again.status, UpgradeStatus::AlreadyCurrent, "{again:#?}");
        assert!(!again.restyled);
    }

    /// D13 / E-R12: a programmed pack is never rewritten. Its rows are not
    /// translated either — the whole file is refused, byte-identical.
    #[test]
    fn restyle_refuses_a_programmed_pack() {
        let source = "speclib probe 2.0 {\n\
                      set version 1.0\n\
                      foreach name {alpha beta} {\n\
                      \x20   command $name { arity 0 }\n\
                      }\n\
                      }\n";
        let outcome = upgrade_source(
            source,
            &UpgradeOptions {
                restyle: true,
                ..UpgradeOptions::default()
            },
        );
        assert_eq!(outcome.status, UpgradeStatus::Refused, "{outcome:#?}");
        assert_eq!(outcome.source, source, "nothing is rewritten");
        assert!(!outcome.restyled);
        assert!(
            outcome
                .refusals
                .iter()
                .any(|refusal| refusal.message.contains("E-R12")),
            "{:?}",
            outcome.refusals
        );
        // Without `--restyle` the same file is simply current.
        let plain = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(plain.status, UpgradeStatus::AlreadyCurrent, "{plain:#?}");
    }

    /// D13: a partial upgrade keeps its markers and is not restyled, while
    /// the row rewrite still goes ahead and says why the restyle waited.
    #[test]
    fn restyle_is_skipped_on_a_partial_upgrade() {
        let source = "speclib probe 1.2 {\n\
                      command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6 bpf}\n\
                      }\n\
                      }\n";
        let outcome = upgrade_source(
            source,
            &UpgradeOptions {
                restyle: true,
                ..UpgradeOptions::default()
            },
        );
        assert_eq!(outcome.status, UpgradeStatus::Partial, "{outcome:#?}");
        assert!(!outcome.restyled);
        assert!(outcome.restyle_skipped.is_some());
        assert!(
            outcome.source.contains("TODO(spectcl 2.0)"),
            "{}",
            outcome.source
        );
    }

    /// D13: an already-current pack with nothing to translate is still
    /// restyled when asked, and reports as such.
    #[test]
    fn restyle_of_a_current_pack_reports_restyled() {
        let source = "speclib probe 2.0 {\n  command demo {\n      arity 1\n  }\n}\n";
        let outcome = upgrade_source(
            source,
            &UpgradeOptions {
                restyle: true,
                ..UpgradeOptions::default()
            },
        );
        assert_eq!(outcome.status, UpgradeStatus::Restyled, "{outcome:#?}");
        assert!(outcome.restyled);
        assert_eq!(
            outcome.source,
            "speclib probe 2.0 {\n\ncommand demo {\n    arity 1\n}\n\n}\n"
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
        let before = evaluate_pack(source);
        let after = evaluate_pack(&outcome.source);
        assert!(after.notices.is_empty(), "{:?}", after.notices);
        assert_eq!(
            format!("{:?}", before.command("demo").expect("demo").spec),
            format!("{:?}", after.command("demo").expect("demo").spec),
        );
    }

    /// U3: a membership token whose environment declares one ambient
    /// package translates to that package's `available` row, and the
    /// rewritten pack loads to the same specs (the environment-derived
    /// package↔bit table in `loader::available`).
    #[test]
    fn a_membership_token_translates_to_its_environments_ambient_package() {
        let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6 f5-iapps}\n\
                      \x20 }\n\
                      \x20 command duo {\n\
                      \x20   arity 1\n\
                      \x20   dialects f5-tmsh\n\
                      \x20 }\n\
                      \x20 command trio {\n\
                      \x20   arity 1\n\
                      \x20   dialects expect\n\
                      \x20 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
        assert!(
            outcome
                .source
                .contains("available {tcl 8.6} {package f5-iapps-cmds}"),
            "{}",
            outcome.source
        );
        assert!(
            outcome.source.contains("available {package f5-tmsh-cmds}"),
            "{}",
            outcome.source
        );
        assert!(
            outcome.source.contains("available {package Expect}"),
            "{}",
            outcome.source
        );
        let before = evaluate_pack(source);
        let after = evaluate_pack(&outcome.source);
        for name in ["demo", "duo", "trio"] {
            assert_eq!(
                format!("{:?}", before.command(name).expect(name).spec),
                format!("{:?}", after.command(name).expect(name).spec),
                "{name}"
            );
        }
        assert!(
            outcome.above_target.is_empty(),
            "{:#?}",
            outcome.above_target
        );
    }

    /// U3: a membership token whose environment declares **no** ambient
    /// provider (its surface is compiled) is left byte-identical, marked
    /// in place, and reported — and the version word does **not** move,
    /// because the file is only partially upgraded (U1).
    #[test]
    fn an_environment_membership_token_is_deferred_not_guessed() {
        let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6 spectcl}\n\
                      \x20 }\n\
                      \x20 command duo {\n\
                      \x20   arity 1\n\
                      \x20   dialects bpf\n\
                      \x20 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Partial, "{outcome:#?}");
        assert_eq!(outcome.deferred.len(), 2, "{:#?}", outcome.deferred);
        assert_eq!(outcome.deferred[0].token, "spectcl");
        assert_eq!(outcome.deferred[1].token, "bpf");
        assert!(
            outcome.source.contains("# TODO(spectcl 2.0):"),
            "{}",
            outcome.source
        );
        assert!(
            outcome.source.contains("dialects {tcl8.6 spectcl}"),
            "the row itself is untouched: {}",
            outcome.source
        );
        assert!(
            outcome.source.contains("speclib probe 1.2"),
            "a partial upgrade keeps its 1.x header: {}",
            outcome.source
        );
    }

    /// U4: an `ambient_package` row rehomes into the owning environment's
    /// `-extend` block — here the owner is the pack's sole membership
    /// token — and the environment-effect snapshot is unchanged across
    /// the rewrite.
    #[test]
    fn an_ambient_package_row_rehomes_into_the_owning_environment() {
        let source = "speclib probe 1.2 {\n\
                      ambient_package iapp_utils 1.4\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects f5-iapps\n\
                      \x20 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
        assert!(
            outcome
                .source
                .contains("environment f5-iapps -extend {\n    ambient iapp_utils 1.4\n}"),
            "{}",
            outcome.source
        );
        let before = evaluate_pack(source);
        let after = evaluate_pack(&outcome.source);
        assert!(
            after.ambient_packages.is_empty(),
            "{:#?}",
            after.ambient_packages
        );
        assert_eq!(after.environments.len(), 1, "{:#?}", after.notices);
        assert!(after.environments[0].extends);
        assert_eq!(
            super::environment_effect_snapshot(&before),
            super::environment_effect_snapshot(&after),
            "the registry effect must not move"
        );
        assert!(
            outcome.above_target.is_empty(),
            "{:#?}",
            outcome.above_target
        );
    }

    /// U4's cannot-infer rule: with no declared environment and no sole
    /// membership token, the row keeps its marker and the file reports
    /// partial.
    #[test]
    fn an_ambient_package_row_with_no_owner_is_deferred() {
        let source = "speclib probe 1.2 {\n\
                      ambient_package iapp_utils 1.4\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects tcl8.6\n\
                      \x20 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Partial, "{outcome:#?}");
        assert!(
            outcome.source.contains("ambient_package iapp_utils 1.4"),
            "{}",
            outcome.source
        );
        assert!(
            outcome
                .source
                .contains("# TODO(spectcl 2.0): this `ambient_package` row"),
            "{}",
            outcome.source
        );
    }

    /// U5: a `file_extension … -dialect D` row's detection moves into
    /// `environment D -extend { … }`, the flag itself dropped, everything
    /// else verbatim — and the effect snapshot is unchanged.
    #[test]
    fn a_file_extension_dialect_row_moves_into_the_environment_block() {
        let source = "speclib probe 1.1 {\n\
                      file_extension upf -name {Unified Power Format} -dialect synopsys-eda-tcl\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20 }\n\
                      }\n";
        let outcome = upgrade_source(source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
        assert!(
            outcome.source.contains(
                "environment synopsys-eda-tcl -extend {\n\
                 \x20   file_extension upf -name {Unified Power Format}\n\
                 }"
            ),
            "{}",
            outcome.source
        );
        let before = evaluate_pack(source);
        let after = evaluate_pack(&outcome.source);
        assert!(after.file_extensions.is_empty());
        assert_eq!(
            super::environment_effect_snapshot(&before),
            super::environment_effect_snapshot(&after),
        );
        assert!(
            outcome.above_target.is_empty(),
            "{:#?}",
            outcome.above_target
        );
    }

    /// U6: `--infer-provides` hoists the pack-level default, and the
    /// rewritten pack loads to the same specs through the `provides`
    /// fallback.
    #[test]
    fn infer_provides_hoists_the_pack_level_default() {
        let source = "speclib probe 1.2 {\n\
                      default required_package upf\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20 }\n\
                      }\n";
        let options = UpgradeOptions {
            infer_provides: true,
            ..UpgradeOptions::default()
        };
        let outcome = upgrade_source(source, &options);
        assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
        assert!(
            outcome.source.contains("provides upf"),
            "{}",
            outcome.source
        );
        assert!(
            !outcome.source.contains("default required_package"),
            "{}",
            outcome.source
        );
        let before = evaluate_pack(source);
        let after = evaluate_pack(&outcome.source);
        assert_eq!(after.provides.len(), 1);
        assert_eq!(
            before.command("demo").expect("demo").spec.required_package,
            after.command("demo").expect("demo").spec.required_package,
        );
    }

    /// U6's per-command shape: a uniform `required_package` row in every
    /// command hoists to one pack-level `provides`, and specs are
    /// unchanged.
    #[test]
    fn infer_provides_hoists_uniform_per_command_rows() {
        let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   required_package upf\n\
                      \x20 }\n\
                      \x20 command duo {\n\
                      \x20   arity 1\n\
                      \x20   required_package upf\n\
                      \x20 }\n\
                      }\n";
        let options = UpgradeOptions {
            infer_provides: true,
            ..UpgradeOptions::default()
        };
        let outcome = upgrade_source(source, &options);
        assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
        assert!(
            outcome.source.contains("provides upf"),
            "{}",
            outcome.source
        );
        assert!(
            !outcome.source.contains("required_package upf"),
            "{}",
            outcome.source
        );
        let before = evaluate_pack(source);
        let after = evaluate_pack(&outcome.source);
        for name in ["demo", "duo"] {
            assert_eq!(
                before.command(name).expect(name).spec.required_package,
                after.command(name).expect(name).spec.required_package,
                "{name}"
            );
        }
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
                ..UpgradeOptions::default()
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
