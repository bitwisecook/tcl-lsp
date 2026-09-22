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

//! Diagnostic policy — one owner below every surface.
//!
//! A producer converts its own typed finding into one [`Finding`] and filters
//! nothing. The policy step pairs every finding with an [`Outcome`] and
//! deletes nothing, so the [`Report`] a surface renders carries the reason
//! for every finding it does not show. This module holds the finding shape,
//! the conversion from every producer type in the tree, and the outcome
//! vocabulary; the design is `docs/design/compiler/diagnostic-policy.md`.
//!
//! Every conversion lands on the one code space, [`DiagCode`]: that is what
//! makes the disabled set, the severity overrides, the tag table and the
//! overlap table one mechanism each. A producer that carries its code as a
//! string (`f5_xc::XcDiagnostic`, `tcl_bigip::validator::ConfigDiagnostic`)
//! converts fallibly, so an uncatalogued code is a conversion failure rather
//! than a value that silently skips every table.

use core::str::FromStr;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::Value;
use tcl_bigip::validator::{ConfigDiagnostic, DiagSeverity};
use tcl_compiler::analyser::utils::{
    parse_file_suppression, parse_noqa_line_suppressions_for_dialect,
};
use tcl_compiler::analyser::{self, AnalysisResult, FILE_SUPPRESS_KEY};
use tcl_compiler::compiler_checks;
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::optimiser::profiles::{
    DEFAULT_EDITOR_PROFILE, OptimisationProfile, profile_to_disabled,
};
use tcl_core_types::{DiagCode, DiagSection, DiagTag, Severity, UnknownDiagCode};
use tcl_dialect::DialectProfile;
use tcl_lexer::{LineIndex, Span, Utf16Col};

use crate::config_ini::{DEFAULT_OFF_CODES, parse_severity_value};
use crate::source_decode::{DecodeReport, should_abstain};
use crate::source_style::{StyleDiagnostic, StyleFix, StyleSeverity};

pub use tcl_compiler::analyser::FixSafety;

/// Which producer emitted a finding — for the report's explanation and for
/// the overlap table, never for ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Producer {
    /// The semantic analyser (`tcl_compiler::analyser`).
    Analyser,
    /// The compiler-checks pipeline (`run_all_checks`).
    CompilerCheck,
    /// The optimiser's rewrites.
    Optimiser,
    /// The source-text style pass (`source_style`).
    SourceStyle,
    /// The byte-integrity pass (`source_decode`).
    SourceDecode,
    /// The `SslicTcl` loader's projection (`sslictcl_diagnostics`).
    SslicTcl,
    /// The F5 Distributed Cloud translatability walk (`f5_xc`).
    Xc,
    /// The BIG-IP configuration and iApp model validators (`tcl_bigip`).
    BigipModel,
}

/// An edit a finding offers, whose range is independent of the finding's
/// span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    /// Byte range the edit replaces — zero-width for an insertion.
    pub span: Span,
    /// The replacement text.
    pub new_text: String,
    /// The action title.
    pub description: String,
    /// How much the edit changes the program's behaviour.
    pub safety: FixSafety,
}

impl From<analyser::CodeFix> for Fix {
    fn from(fix: analyser::CodeFix) -> Self {
        Self {
            span: fix.span,
            new_text: fix.new_text,
            description: fix.description,
            safety: fix.safety,
        }
    }
}

/// The extra payload an adapter may need beyond the finding itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingData {
    /// An optimiser rewrite.
    Rewrite {
        /// The text that replaces the finding's span.
        replacement: String,
        /// The all-or-nothing edit group the rewrite belongs to.
        group: Option<u32>,
        /// The span covers the consuming statement rather than a precise
        /// sub-span, so the replacement must never be advertised as
        /// auto-appliable.
        hint_only: bool,
    },
    /// A style fix already expressed as a document range.
    StyleFix {
        /// The fix as the style pass built it.
        fix: StyleFix,
    },
}

/// One producer's finding, before any policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code, as the one catalogue spells it.
    pub code: DiagCode,
    /// Byte offsets in the *analysis form* of the text (lone `\r`
    /// rewritten), which is what every producer reads.
    pub span: Span,
    /// The producer's default severity. Policy may relabel it; the producer
    /// never chooses the displayed one.
    pub severity: Severity,
    /// The one-line message.
    pub message: String,
    /// Edits whose range is independent of `span`.
    pub fixes: Vec<Fix>,
    /// The extra payload an adapter may need.
    pub data: Option<FindingData>,
    /// Which producer emitted it.
    pub producer: Producer,
}

impl From<analyser::Diagnostic> for Finding {
    fn from(d: analyser::Diagnostic) -> Self {
        Self {
            code: d.code,
            span: d.span,
            severity: d.severity,
            message: d.message,
            fixes: d.fixes.into_iter().map(Fix::from).collect(),
            data: None,
            producer: Producer::Analyser,
        }
    }
}

impl From<compiler_checks::Diagnostic> for Finding {
    /// `category` is dropped; a same-span `replacement` becomes a [`Fix`]
    /// titled with the message, classified [`FixSafety::RequiresReview`]
    /// because the check never proved more.
    fn from(d: compiler_checks::Diagnostic) -> Self {
        let mut fixes: Vec<Fix> = d.fixes.into_iter().map(Fix::from).collect();
        if let Some(replacement) = d.replacement {
            fixes.push(Fix {
                span: d.span,
                new_text: replacement,
                description: d.message.clone(),
                safety: FixSafety::RequiresReview,
            });
        }
        Self {
            code: d.code,
            span: d.span,
            severity: d.severity,
            message: d.message,
            fixes,
            data: None,
            producer: Producer::CompilerCheck,
        }
    }
}

impl From<Optimisation> for Finding {
    /// An optimisation is a `Hint`; its rewrite rides on
    /// [`FindingData::Rewrite`] rather than on `fixes`, because whether it
    /// may be applied is the adapter's reading of `hint_only`.
    fn from(o: Optimisation) -> Self {
        Self {
            code: o.code,
            span: o.span,
            severity: Severity::Hint,
            message: o.message,
            fixes: Vec::new(),
            data: Some(FindingData::Rewrite {
                replacement: o.replacement,
                group: o.group,
                hint_only: o.hint_only,
            }),
            producer: Producer::Optimiser,
        }
    }
}

impl Finding {
    /// A source-style or byte-integrity finding.
    ///
    /// The style pass reports LSP ranges; `line_index` (built over `text`
    /// with [`LineIndex::new_lsp`], or over the analysis form with
    /// [`LineIndex::new`] — the two agree) turns them back into byte offsets.
    /// W107 and W109 are the byte-integrity pass's, so they carry
    /// [`Producer::SourceDecode`]; every other code is the style pass's.
    #[must_use]
    pub fn from_style(d: StyleDiagnostic, text: &str, line_index: &LineIndex) -> Self {
        let offset = |line: u32, character: u32| {
            line_index.offset_at_utf16(line, Utf16Col::new(character), text)
        };
        let start = offset(d.range.start_line, d.range.start_character);
        let end = offset(d.range.end_line, d.range.end_character).max(start);
        let producer = match d.code {
            DiagCode::W107 | DiagCode::W109 => Producer::SourceDecode,
            _ => Producer::SourceStyle,
        };
        Self {
            code: d.code,
            span: Span::new(start, end),
            severity: match d.severity {
                StyleSeverity::Warning => Severity::Warning,
                StyleSeverity::Hint => Severity::Hint,
            },
            message: d.message,
            fixes: Vec::new(),
            data: d.fix.map(|fix| FindingData::StyleFix { fix }),
            producer,
        }
    }
}

impl TryFrom<&ConfigDiagnostic> for Finding {
    type Error = UnknownDiagCode;

    /// A BIG-IP configuration or iApp model finding. Its range carries byte
    /// offsets with an *inclusive* end, normalised to the exclusive span
    /// every other producer uses here rather than at each lift; `subject`
    /// is an object-report concern and is dropped.
    fn try_from(d: &ConfigDiagnostic) -> Result<Self, Self::Error> {
        let code = DiagCode::from_str(&d.code)?;
        let start = d.range.start.offset;
        let end = d.range.end.offset.saturating_add(1).max(start);
        Ok(Self {
            code,
            span: Span::new(start, end),
            severity: match d.severity {
                DiagSeverity::Warning => Severity::Warning,
                DiagSeverity::Hint => Severity::Hint,
            },
            message: d.message.clone(),
            fixes: Vec::new(),
            data: None,
            producer: Producer::BigipModel,
        })
    }
}

/// What the policy step decided for one finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The finding shows.
    Shown {
        /// The producer's default with the user's override applied.
        severity: Severity,
        /// The presentation tag the code table declares.
        tag: Option<DiagTag>,
    },
    /// The finding is hidden, for this reason.
    Suppressed(Reason),
}

/// Why a finding is hidden. The first reason that fires in the policy
/// step's fixed order is the one recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The whole document reports nothing: `features.diagnostics` is off.
    ReportingOff,
    /// `diagnostics.exclude` matches this file.
    Excluded,
    /// The bytes are not UTF-8 text, so only the integrity codes stand.
    EncodingAbstention,
    /// An inline `# noqa` on the command, at this 0-based line.
    InlineDirective {
        /// The line the directive's map keys the finding by.
        line: i32,
    },
    /// A top-of-file `# tcl-lsp: disable=`.
    FileDirective,
    /// A configuration or flag layer turned the code off.
    Disabled(PolicyLayer),
    /// The code is default-off and nothing turned it on.
    DefaultOff,
    /// The optimiser master switch is off.
    OptimiserOff,
    /// The active profile (with its per-code overrides) does not enable the
    /// code.
    OptimiserProfile {
        /// The profile in force.
        profile: OptimisationProfile,
    },
    /// `tclLsp.shimmer.enabled` is off.
    ShimmerOff,
    /// Another code or producer owns this site.
    Overlap {
        /// The owner that superseded the finding.
        owner: OverlapOwner,
    },
}

/// Which scope decided a code, lowest first.
///
/// Distinct from [`crate::config_ini::Layer`], which names the *file role*
/// of one INI parse (`[global]` versus `[project]`) and has no editor or
/// invocation spelling to name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PolicyLayer {
    /// The user's global `config.ini`.
    Global,
    /// Editor settings — the `workspace/configuration` payload.
    Editor,
    /// A surface's own flags (`--disable`, `--enable`, `--profile`, an MCP
    /// argument), which occupy the editor layer's slot in the order.
    Invocation,
    /// The project's `.tcl-lsp.ini`.
    Project,
}

/// What owns a site in the overlap table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlapOwner {
    /// One code owns the site: W110 over O120.
    Code(DiagCode),
    /// A whole producer owns it: in a `.sslictcl` document the loader owns
    /// W123 whether or not it emitted a finding of its own.
    Producer(Producer),
}

/// A shown finding with its resolved presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown<'a> {
    /// The finding.
    pub finding: &'a Finding,
    /// The severity to display.
    pub severity: Severity,
    /// The presentation tag, if the code carries one.
    pub tag: Option<DiagTag>,
}

/// Every finding paired with its outcome, in the producers' order, and the
/// codes a producer declared it left uncomputed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    outcomes: Vec<(Finding, Outcome)>,
    /// A producer's declared production-time skip, with the policy's reason
    /// for each code — so a gap where nothing was produced is explained
    /// rather than read as "clean" (`docs/design/compiler/diagnostic-policy.md`
    /// § Producers that change).
    skipped: BTreeMap<DiagCode, Reason>,
}

impl Report {
    /// The report over `outcomes`, with nothing declared skipped.
    #[must_use]
    pub fn new(outcomes: Vec<(Finding, Outcome)>) -> Self {
        Self {
            outcomes,
            skipped: BTreeMap::new(),
        }
    }

    /// Record that a producer skipped computing `codes`, each with the reason
    /// `policy` gives for the code. A code the policy would show is not
    /// recorded: there is nothing to explain a skip of it with, and a
    /// truth-table row over the producer is what catches that mismatch.
    pub fn declare_skipped(&mut self, codes: impl IntoIterator<Item = DiagCode>, policy: &Policy) {
        for code in codes {
            if let Some(reason) = policy.code_reason(code) {
                self.skipped.insert(code, reason);
            }
        }
    }

    /// The declared skips, by code.
    pub fn skipped(&self) -> impl Iterator<Item = (DiagCode, Reason)> + '_ {
        self.skipped.iter().map(|(code, reason)| (*code, *reason))
    }

    /// The findings that show, with their resolved severity and tag.
    pub fn shown(&self) -> impl Iterator<Item = Shown<'_>> {
        self.outcomes
            .iter()
            .filter_map(|(finding, outcome)| match outcome {
                Outcome::Shown { severity, tag } => Some(Shown {
                    finding,
                    severity: *severity,
                    tag: *tag,
                }),
                Outcome::Suppressed(_) => None,
            })
    }

    /// The findings that do not show, each with its reason.
    pub fn suppressed(&self) -> impl Iterator<Item = (&Finding, Reason)> {
        self.outcomes
            .iter()
            .filter_map(|(finding, outcome)| match outcome {
                Outcome::Suppressed(reason) => Some((finding, *reason)),
                Outcome::Shown { .. } => None,
            })
    }

    /// The outcome recorded for the first finding of `code` at `span`.
    #[must_use]
    pub fn outcome_for(&self, code: DiagCode, span: Span) -> Option<Outcome> {
        self.outcomes
            .iter()
            .find(|(finding, _)| finding.code == code && finding.span == span)
            .map(|(_, outcome)| *outcome)
    }

    /// Why the first finding of `code` at `span` is hidden, or — when no
    /// finding of `code` exists at all — why the producer skipped the code;
    /// `None` when the finding shows or nothing explains its absence.
    #[must_use]
    pub fn reason_for(&self, code: DiagCode, span: Span) -> Option<Reason> {
        match self.outcome_for(code, span) {
            Some(Outcome::Suppressed(reason)) => Some(reason),
            Some(Outcome::Shown { .. }) => None,
            None => self.skipped.get(&code).copied(),
        }
    }

    /// `items`, one per finding in the producers' order, kept where the
    /// finding shows — for a caller that keeps a producer's own record beside
    /// the converted finding (the optimiser's rewrite record, say) and needs
    /// the shown subset of those. [`apply`] is order-stable and keeps every
    /// finding, which is what makes the pairing sound.
    #[must_use]
    pub fn shown_items<T>(&self, items: Vec<T>) -> Vec<T> {
        debug_assert_eq!(items.len(), self.outcomes.len(), "one item per finding");
        self.outcomes
            .iter()
            .zip(items)
            .filter_map(|((_, outcome), item)| {
                matches!(outcome, Outcome::Shown { .. }).then_some(item)
            })
            .collect()
    }

    /// Every pair, in the producers' order.
    pub fn iter(&self) -> impl Iterator<Item = &(Finding, Outcome)> {
        self.outcomes.iter()
    }

    /// Every pair, in the producers' order, as a slice.
    #[must_use]
    pub fn outcomes(&self) -> &[(Finding, Outcome)] {
        &self.outcomes
    }

    /// Append `pairs` the caller decided itself, keeping the producers' order.
    pub fn extend(&mut self, pairs: impl IntoIterator<Item = (Finding, Outcome)>) {
        self.outcomes.extend(pairs);
    }

    /// How many findings the report holds, shown or not.
    #[must_use]
    pub fn len(&self) -> usize {
        self.outcomes.len()
    }

    /// Whether the report holds no finding at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }
}

/// The resolved decision for one code and the layer that won it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeDecision {
    /// Whether the code shows.
    pub enabled: bool,
    /// The scope whose value stands.
    pub layer: PolicyLayer,
}

/// The optimiser's master switch, profile and per-code set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimiserPolicy {
    /// `tclLsp.optimiser.enabled`.
    pub enabled: bool,
    /// The active profile.
    pub profile: OptimisationProfile,
    /// The profile's disabled set with the per-code overrides applied — as
    /// the server's `resolved_analysis_settings` builds it.
    pub disabled: BTreeSet<DiagCode>,
}

impl OptimiserPolicy {
    /// The policy `profile` alone selects: on, with the profile's disabled
    /// set and no per-code override.
    #[must_use]
    pub fn for_profile(profile: OptimisationProfile) -> Self {
        Self {
            enabled: true,
            profile,
            disabled: profile_to_disabled(profile)
                .into_iter()
                .filter_map(|code| DiagCode::from_str(code).ok())
                .collect(),
        }
    }

    /// Every optimisation shows: on, with nothing disabled, under the
    /// `full` profile.
    #[must_use]
    pub fn all_on() -> Self {
        Self {
            enabled: true,
            profile: OptimisationProfile::Full,
            disabled: BTreeSet::new(),
        }
    }
}

impl Default for OptimiserPolicy {
    /// The editor's default profile.
    fn default() -> Self {
        Self::for_profile(DEFAULT_EDITOR_PROFILE)
    }
}

/// One entry of the overlap table: `owner` supersedes `superseded` where
/// `scope` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overlap {
    /// What owns the site.
    pub owner: OverlapOwner,
    /// The code that loses.
    pub superseded: DiagCode,
    /// Where the owner wins.
    pub scope: OverlapScope,
}

/// Where an overlap's owner wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlapScope {
    /// Only where the two spans coincide — the W110 / O120 rule.
    SameSpan,
    /// Everywhere in the document.
    Document,
}

/// The codes that are whole-file verdicts: an inline `# noqa` has no line
/// to attach to, so only the top-of-file directive and the configuration
/// layers gate them. W118 reads the document's line terminators; W107 and
/// W109 read its bytes. Stated here once rather than in one pass's control
/// flow.
pub const WHOLE_FILE_CODES: &[DiagCode] = &[DiagCode::W107, DiagCode::W109, DiagCode::W118];

/// The overlap entries every dialect carries.
fn base_overlaps() -> Vec<Overlap> {
    vec![Overlap {
        owner: OverlapOwner::Code(DiagCode::W110),
        superseded: DiagCode::O120,
        scope: OverlapScope::SameSpan,
    }]
}

/// The overlap table for a document of `dialect`: W110 owns O120 at the
/// same span everywhere, and in a `.sslictcl` document the loader owns the
/// analyser codes [`crate::sslictcl_diagnostics::SUPERSEDED_ANALYSER_CODES`]
/// names, document-wide, whether or not it emitted a finding of its own.
#[must_use]
pub fn dialect_overlaps(dialect: &DialectProfile) -> Vec<Overlap> {
    let mut overlaps = base_overlaps();
    if crate::sslictcl_diagnostics::applies_to(dialect) {
        overlaps.extend(
            crate::sslictcl_diagnostics::SUPERSEDED_ANALYSER_CODES
                .iter()
                .map(|&superseded| Overlap {
                    owner: OverlapOwner::Producer(Producer::SslicTcl),
                    superseded,
                    scope: OverlapScope::Document,
                }),
        );
    }
    overlaps
}

/// The directive facts the front end parsed: the analyser's
/// `suppressed_lines` map — 0-based lines plus the [`FILE_SUPPRESS_KEY`]
/// bucket — over the line model of the text the findings' spans index.
#[derive(Debug, Clone, Default)]
pub struct Directives {
    lines: HashMap<i32, HashSet<String>>,
    line_index: Option<LineIndex>,
}

impl Directives {
    /// A map shaped like the analyser's, over `text` (the analysis form, or
    /// the client's buffer — the LSP line model is the same for both).
    #[must_use]
    pub fn new(lines: HashMap<i32, HashSet<String>>, text: &str) -> Self {
        Self {
            lines,
            line_index: Some(LineIndex::new_lsp(text)),
        }
    }

    /// The analyser's own map for `text`, for the surface that ran it.
    #[must_use]
    pub fn from_analysis(analysis: &AnalysisResult, text: &str) -> Self {
        Self::new(analysis.suppressed_lines.clone(), text)
    }

    /// The scan for a surface that did not run the analyser: the same
    /// `# noqa` comment scan and top-of-file directive parse the analyser
    /// folds into its map. The analyser additionally attributes a `# noqa`
    /// to every line of the command it precedes; this scan records it
    /// against the next line only.
    #[must_use]
    pub fn scan(text: &str, dialect: &'static DialectProfile) -> Self {
        let mut lines = parse_noqa_line_suppressions_for_dialect(text, dialect);
        let file = parse_file_suppression(text);
        if !file.is_empty() {
            lines.entry(FILE_SUPPRESS_KEY).or_default().extend(file);
        }
        Self::new(lines, text)
    }

    /// No directive at all.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// The map, as the analyser keys it.
    #[must_use]
    pub fn lines(&self) -> &HashMap<i32, HashSet<String>> {
        &self.lines
    }

    /// Why a directive silences `code` at `span`, if one does: an inline
    /// `# noqa` on the finding's line first, then the top-of-file
    /// directive. A code in [`WHOLE_FILE_CODES`] is a whole-file verdict
    /// with no line to attach an inline directive to, so only the file
    /// directive reaches it.
    #[must_use]
    pub fn reason_for(&self, code: DiagCode, span: Span) -> Option<Reason> {
        if !WHOLE_FILE_CODES.contains(&code) {
            let line = self.line_of(span);
            if self.hit(line, code) {
                return Some(Reason::InlineDirective { line });
            }
        }
        self.hit(FILE_SUPPRESS_KEY, code)
            .then_some(Reason::FileDirective)
    }

    /// The 0-based line the map keys a finding at `span` by.
    fn line_of(&self, span: Span) -> i32 {
        self.line_index.as_ref().map_or(0, |index| {
            i32::try_from(index.line_at(span.start())).unwrap_or(i32::MAX)
        })
    }

    /// Whether the bucket at `key` silences `code` — the one contract
    /// `line_suppressed` states: a `"*"` entry silences every code.
    fn hit(&self, key: i32, code: DiagCode) -> bool {
        self.lines
            .get(&key)
            .is_some_and(|codes| codes.contains("*") || codes.contains(code.as_str()))
    }
}

/// The document-level gates: whether this file reports at all, and whether
/// its bytes force abstention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentGates {
    /// `features.diagnostics` for this file's folder.
    pub reporting: bool,
    /// Whether `diagnostics.exclude` matches this file.
    pub excluded: bool,
    /// Byte evidence, through [`should_abstain`]: the codes that survive it
    /// are the integrity set, never a guess from whether W109 is displayed.
    pub abstain: bool,
}

impl Default for DocumentGates {
    /// Everything reports; nothing is excluded or abstains.
    fn default() -> Self {
        Self {
            reporting: true,
            excluded: false,
            abstain: false,
        }
    }
}

/// Every decision that can hide or relabel a finding, resolved once for one
/// document. Nothing here changes what a producer computes: a setting that
/// does (`genericVariablePatterns`, the non-ASCII mode, the style line
/// length) is a producer input and stays on the producers' arguments.
#[derive(Debug, Clone)]
pub struct Policy {
    /// The document-level gates.
    pub document: DocumentGates,
    /// The resolved per-code decision and the layer that won it. A code
    /// absent from the map is enabled unless it is in `default_off`.
    pub codes: BTreeMap<DiagCode, CodeDecision>,
    /// The default-off seed — the lowest layer, and the reason a code still
    /// off at the bottom reports [`Reason::DefaultOff`] rather than
    /// `Disabled(Global)`.
    pub default_off: &'static [DiagCode],
    /// `tclLsp.diagnosticSeverity.<CODE>`.
    pub severity_overrides: BTreeMap<DiagCode, Severity>,
    /// The optimiser master switch, profile and per-code set.
    pub optimiser: OptimiserPolicy,
    /// `tclLsp.shimmer.enabled`.
    pub shimmer: bool,
    /// The overlap table, resolved for the document's dialect.
    pub overlaps: Vec<Overlap>,
    /// The directive facts.
    pub directives: Directives,
}

impl Default for Policy {
    /// An unconfigured editor: everything reports, nothing is excluded or
    /// abstains, the catalogue's default-off seed applies, the optimiser
    /// runs its default profile, shimmer is on, and the dialect-independent
    /// overlaps stand.
    fn default() -> Self {
        Self {
            document: DocumentGates::default(),
            codes: BTreeMap::new(),
            default_off: DEFAULT_OFF_CODES,
            severity_overrides: BTreeMap::new(),
            optimiser: OptimiserPolicy::default(),
            shimmer: true,
            overlaps: base_overlaps(),
            directives: Directives::none(),
        }
    }
}

impl Policy {
    /// The policy under which every finding shows: no seed, no configured
    /// code, every optimisation on, no overlap, no directive.
    ///
    /// For a host that renders a producer's raw set and says so — a test
    /// host, or a surface that has not yet resolved its layers. It is not a
    /// way for an adapter to skip the policy step: an adapter reads a
    /// report, and what it renders is whatever that report shows.
    #[must_use]
    pub fn unrestricted() -> Self {
        Self {
            default_off: &[],
            optimiser: OptimiserPolicy::all_on(),
            overlaps: Vec::new(),
            ..Self::default()
        }
    }

    /// Steps 4 and 5 of [`apply`] for `code` alone — the per-code decision
    /// and the family gates — without the document gates or the directives,
    /// which need a finding to attach to. This is the reason a producer's
    /// declared skip is recorded with, and what [`Self::production_skip`] is
    /// built from.
    #[must_use]
    pub fn code_reason(&self, code: DiagCode) -> Option<Reason> {
        match self.codes.get(&code) {
            Some(CodeDecision {
                enabled: false,
                layer,
            }) => return Some(Reason::Disabled(*layer)),
            Some(CodeDecision { enabled: true, .. }) => {}
            None => {
                if self.default_off.contains(&code) {
                    return Some(Reason::DefaultOff);
                }
            }
        }
        if code.is_optimisation() {
            if !self.optimiser.enabled {
                return Some(Reason::OptimiserOff);
            }
            if self.optimiser.disabled.contains(&code) {
                return Some(Reason::OptimiserProfile {
                    profile: self.optimiser.profile,
                });
            }
        }
        if !self.shimmer && code.diag_section() == Some(DiagSection::Shimmer) {
            return Some(Reason::ShimmerOff);
        }
        None
    }

    /// The codes a producer may leave uncomputed: every catalogued code this
    /// policy hides by its per-code decision or a family gate, whatever the
    /// finding. Rule 2's permitted saving
    /// (`docs/design/compiler/diagnostic-policy.md` § Producers that change)
    /// — the analyser's `with_disabled_diagnostics` set on every surface —
    /// and what the caller declares back through [`Report::declare_skipped`].
    /// The directives are not in it: the analyser reads those itself, and a
    /// line-scoped one cannot skip a whole code.
    #[must_use]
    pub fn production_skip(&self) -> BTreeSet<DiagCode> {
        DiagCode::ALL
            .iter()
            .copied()
            .filter(|code| self.code_reason(*code).is_some())
            .collect()
    }
}

/// Builds one [`Policy`] from the configuration layers, lowest first, plus
/// the document facts.
///
/// The layers are taken one at a time rather than merged: only the unmerged
/// layers can name the layer that decided a code, which is what
/// [`Reason::Disabled`] and the per-code tri-state need. Within a section
/// the later layer wins per key exactly as [`crate::config_ini::merge_settings`]
/// merges, and a key a higher layer sets to an unusable value (a non-boolean
/// toggle, an unknown severity) resets the lower layers' decision, which is
/// what merge-then-parse did.
#[derive(Debug, Clone)]
pub struct PolicyBuilder {
    reporting: Option<bool>,
    excluded: bool,
    abstain: bool,
    layers: Vec<(PolicyLayer, Value)>,
    overlaps: Vec<Overlap>,
    directives: Directives,
}

impl Default for PolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyBuilder {
    /// No layer, no directive, the dialect-independent overlaps.
    #[must_use]
    pub fn new() -> Self {
        Self {
            reporting: None,
            excluded: false,
            abstain: false,
            layers: Vec::new(),
            overlaps: base_overlaps(),
            directives: Directives::none(),
        }
    }

    /// Add a configuration layer — the `tclLsp` content shape
    /// [`crate::config_ini::settings_from_ini`] produces and the editor
    /// delivers (a `{"tclLsp": …}` wrapper and flat-dotted
    /// `tclLsp.<section>.<key>` keys are both read). Call lowest first:
    /// global, then editor or invocation, then project.
    #[must_use]
    pub fn layer(mut self, layer: PolicyLayer, settings: &Value) -> Self {
        self.layers.push((layer, settings.clone()));
        self
    }

    /// `features.diagnostics` for this file's folder, when the caller has
    /// resolved it itself; otherwise the layers' `features.diagnostics`
    /// decides, defaulting to on.
    #[must_use]
    pub fn reporting(mut self, on: bool) -> Self {
        self.reporting = Some(on);
        self
    }

    /// Whether `diagnostics.exclude` matches this file.
    #[must_use]
    pub fn excluded(mut self, yes: bool) -> Self {
        self.excluded = yes;
        self
    }

    /// The byte decoder's report for the text, when the caller has one.
    #[must_use]
    pub fn decode(mut self, report: Option<&DecodeReport>) -> Self {
        self.abstain = should_abstain(report);
        self
    }

    /// Resolve the overlap table for the document's dialect.
    #[must_use]
    pub fn dialect(mut self, dialect: &DialectProfile) -> Self {
        self.overlaps = dialect_overlaps(dialect);
        self
    }

    /// The directive facts for the document.
    #[must_use]
    pub fn directives(mut self, directives: Directives) -> Self {
        self.directives = directives;
        self
    }

    /// Resolve the layers into one policy.
    #[must_use]
    pub fn build(self) -> Policy {
        let mut codes: BTreeMap<DiagCode, CodeDecision> = BTreeMap::new();
        let mut severity_overrides: BTreeMap<DiagCode, Severity> = BTreeMap::new();
        let mut optimiser_enabled: Option<Value> = None;
        let mut optimiser_profile: Option<Value> = None;
        let mut optimiser_overrides: BTreeMap<DiagCode, Value> = BTreeMap::new();
        let mut shimmer: Option<Value> = None;
        let mut reporting: Option<Value> = None;

        for (layer, settings) in &self.layers {
            for (key, value) in section_entries(settings, "diagnostics") {
                let Ok(code) = DiagCode::from_str(&key) else {
                    continue;
                };
                match value.as_bool() {
                    Some(enabled) => {
                        codes.insert(
                            code,
                            CodeDecision {
                                enabled,
                                layer: *layer,
                            },
                        );
                    }
                    None => {
                        codes.remove(&code);
                    }
                }
            }
            for (key, value) in section_entries(settings, "diagnosticSeverity") {
                let Ok(code) = DiagCode::from_str(&key) else {
                    continue;
                };
                match value.as_str().and_then(parse_severity_value) {
                    Some(severity) => {
                        severity_overrides.insert(code, severity);
                    }
                    None => {
                        severity_overrides.remove(&code);
                    }
                }
            }
            for (key, value) in section_entries(settings, "optimiser") {
                match key.as_str() {
                    "enabled" => optimiser_enabled = Some(value),
                    "profile" => optimiser_profile = Some(value),
                    code => {
                        if let Ok(code) = DiagCode::from_str(code) {
                            optimiser_overrides.insert(code, value);
                        }
                    }
                }
            }
            for (key, value) in section_entries(settings, "shimmer") {
                if key == "enabled" {
                    shimmer = Some(value);
                }
            }
            for (key, value) in section_entries(settings, "features") {
                if key == "diagnostics" {
                    reporting = Some(value);
                }
            }
        }

        let profile = optimiser_profile
            .as_ref()
            .and_then(Value::as_str)
            .map_or(DEFAULT_EDITOR_PROFILE, OptimisationProfile::parse);
        let mut optimiser = OptimiserPolicy::for_profile(profile);
        optimiser.enabled = optimiser_enabled
            .as_ref()
            .and_then(Value::as_bool)
            .unwrap_or(true);
        for (code, value) in &optimiser_overrides {
            match value.as_bool() {
                Some(true) => {
                    optimiser.disabled.remove(code);
                }
                Some(false) => {
                    optimiser.disabled.insert(*code);
                }
                None => {}
            }
        }

        Policy {
            document: DocumentGates {
                reporting: self
                    .reporting
                    .unwrap_or_else(|| reporting.as_ref().and_then(Value::as_bool).unwrap_or(true)),
                excluded: self.excluded,
                abstain: self.abstain,
            },
            codes,
            default_off: DEFAULT_OFF_CODES,
            severity_overrides,
            optimiser,
            shimmer: shimmer.as_ref().and_then(Value::as_bool).unwrap_or(true),
            overlaps: self.overlaps,
            directives: self.directives,
        }
    }
}

/// The `(key, value)` entries of `section` in one settings layer, from the
/// nested object (`{"diagnostics": {…}}`, optionally under a `tclLsp`
/// wrapper) and from flat-dotted keys (`tclLsp.diagnostics.W001` or
/// `diagnostics.W001`), in that order.
fn section_entries(settings: &Value, section: &str) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    let unwrapped = settings.get("tclLsp").unwrap_or(settings);
    if let Some(map) = unwrapped.get(section).and_then(Value::as_object) {
        out.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    if let Some(map) = settings.as_object() {
        let prefixes = [format!("tclLsp.{section}."), format!("{section}.")];
        for (k, v) in map {
            if let Some(key) = prefixes.iter().find_map(|p| k.strip_prefix(p.as_str())) {
                out.push((key.to_owned(), v.clone()));
            }
        }
    }
    out
}

/// The codes that survive encoding abstention: the integrity findings the
/// bytes themselves justify, and W305, which reads the decoded text as text.
const ABSTENTION_SURVIVORS: &[DiagCode] = &[DiagCode::W107, DiagCode::W109, DiagCode::W305];

impl Overlap {
    /// Whether the claim holds with no owner finding at all: a producer
    /// owns the document whether or not it emitted anything.
    fn unconditional(self) -> bool {
        matches!(
            (self.owner, self.scope),
            (OverlapOwner::Producer(_), OverlapScope::Document)
        )
    }

    /// Whether a standing `owner` finding establishes the claim over a
    /// superseded finding at `site`.
    fn claimed_by(self, owner: &Finding, site: Span) -> bool {
        let is_owner = match self.owner {
            OverlapOwner::Code(code) => owner.code == code,
            OverlapOwner::Producer(producer) => owner.producer == producer,
        };
        is_owner && (self.scope == OverlapScope::Document || owner.span == site)
    }
}

/// Pair every finding with its outcome under `policy`, in the producers'
/// order. Nothing is deleted.
///
/// The steps run in one fixed order, and the order is part of the contract
/// because the recorded reason depends on it — the first reason that fires
/// wins:
///
/// 1. `reporting`, then `excluded` — the document-wide gates.
/// 2. `abstain` — everything but W107, W109 and W305 becomes
///    [`Reason::EncodingAbstention`], ahead of the directives on purpose:
///    when the positions in the file are decoding artefacts, "nothing here
///    is analysed" is the honest explanation.
/// 3. The inline directive, then the file directive
///    ([`Directives::reason_for`]; a code in [`WHOLE_FILE_CODES`] skips the
///    inline one).
/// 4. The per-code decision: [`Reason::Disabled`] for the winning layer, or
///    [`Reason::DefaultOff`] at the seed.
/// 5. The family gates: [`Reason::OptimiserOff`] then
///    [`Reason::OptimiserProfile`] for an optimisation code,
///    [`Reason::ShimmerOff`] for the shimmer family.
/// 6. The overlap table, entry by entry in table order, over the findings
///    still standing — so an owner that policy has already suppressed cannot
///    supersede anything.
/// 7. Everything left is shown, at the producer's severity with the user's
///    override applied, tagged as the code table declares.
#[must_use]
pub fn apply(findings: Vec<Finding>, policy: &Policy) -> Report {
    let mut reasons: Vec<Option<Reason>> = findings
        .iter()
        .map(|finding| own_reason(finding, policy))
        .collect();
    for overlap in &policy.overlaps {
        let candidates: Vec<usize> = (0..findings.len())
            .filter(|&i| reasons[i].is_none() && findings[i].code == overlap.superseded)
            .collect();
        for i in candidates {
            let site = findings[i].span;
            let owned = overlap.unconditional()
                || findings
                    .iter()
                    .zip(&reasons)
                    .any(|(owner, reason)| reason.is_none() && overlap.claimed_by(owner, site));
            if owned {
                reasons[i] = Some(Reason::Overlap {
                    owner: overlap.owner,
                });
            }
        }
    }
    Report::new(
        findings
            .into_iter()
            .zip(reasons)
            .map(|(finding, reason)| {
                let outcome = match reason {
                    Some(reason) => Outcome::Suppressed(reason),
                    None => Outcome::Shown {
                        severity: policy
                            .severity_overrides
                            .get(&finding.code)
                            .copied()
                            .unwrap_or(finding.severity),
                        tag: finding.code.lsp_tag(),
                    },
                };
                (finding, outcome)
            })
            .collect(),
    )
}

/// Steps 1 to 5 of [`apply`] for one finding: the first reason that fires.
fn own_reason(finding: &Finding, policy: &Policy) -> Option<Reason> {
    if !policy.document.reporting {
        return Some(Reason::ReportingOff);
    }
    if policy.document.excluded {
        return Some(Reason::Excluded);
    }
    if policy.document.abstain && !ABSTENTION_SURVIVORS.contains(&finding.code) {
        return Some(Reason::EncodingAbstention);
    }
    if let Some(reason) = policy.directives.reason_for(finding.code, finding.span) {
        return Some(reason);
    }
    policy.code_reason(finding.code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::LspRange;

    #[test]
    fn analyser_diagnostic_converts_one_to_one() {
        let d = analyser::Diagnostic::new(
            DiagCode::W210,
            Span::new(4, 9),
            "Variable read before set",
            Severity::Warning,
        )
        .with_fix(analyser::CodeFix::equivalent(
            Span::new(4, 9),
            "x",
            "Rename",
        ));
        let f = Finding::from(d);
        assert_eq!(f.code, DiagCode::W210);
        assert_eq!(f.span, Span::new(4, 9));
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.producer, Producer::Analyser);
        assert_eq!(f.fixes.len(), 1);
        assert_eq!(f.fixes[0].safety, FixSafety::SemanticsEquivalent);
        assert!(f.data.is_none());
    }

    #[test]
    fn compiler_check_replacement_becomes_a_same_span_fix() {
        let d = compiler_checks::Diagnostic {
            span: Span::new(0, 7),
            code: DiagCode::W201,
            category: "path".to_owned(),
            severity: Severity::Warning,
            message: "Use file join".to_owned(),
            replacement: Some("[file join a b]".to_owned()),
            fixes: Vec::new(),
        };
        let f = Finding::from(d);
        assert_eq!(f.producer, Producer::CompilerCheck);
        assert_eq!(f.fixes.len(), 1);
        assert_eq!(f.fixes[0].span, Span::new(0, 7));
        assert_eq!(f.fixes[0].new_text, "[file join a b]");
        assert_eq!(f.fixes[0].safety, FixSafety::RequiresReview);
        assert!(f.data.is_none());
    }

    #[test]
    fn optimisation_carries_its_rewrite_as_data() {
        let mut o = Optimisation::new(DiagCode::O114, "incr", Span::new(2, 20), "incr x");
        o.group = Some(3);
        let f = Finding::from(o);
        assert_eq!(f.severity, Severity::Hint);
        assert_eq!(f.producer, Producer::Optimiser);
        assert_eq!(
            f.data,
            Some(FindingData::Rewrite {
                replacement: "incr x".to_owned(),
                group: Some(3),
                hint_only: false,
            })
        );
        assert!(f.fixes.is_empty());
    }

    #[test]
    fn style_ranges_become_byte_spans_and_pick_the_producer_by_code() {
        // `set 😀  ` — the two trailing spaces start at UTF-16 column 6 and
        // byte offset 8.
        let text = "set 😀  \nputs ok\n";
        let line_index = LineIndex::new_lsp(text);
        let range = LspRange {
            start_line: 0,
            start_character: 6,
            end_line: 0,
            end_character: 8,
        };
        let style = StyleDiagnostic {
            range,
            message: "Trailing whitespace".to_owned(),
            severity: StyleSeverity::Hint,
            code: DiagCode::W112,
            fix: Some(StyleFix {
                range,
                new_text: String::new(),
                description: "Remove trailing whitespace".to_owned(),
            }),
        };
        let f = Finding::from_style(style.clone(), text, &line_index);
        assert_eq!(f.span, Span::new(8, 10));
        assert_eq!(f.producer, Producer::SourceStyle);
        assert_eq!(f.severity, Severity::Hint);
        assert_eq!(
            f.data,
            Some(FindingData::StyleFix {
                fix: style.fix.clone().unwrap()
            })
        );
        // The round trip back to the LSP range is exact.
        let start = line_index.position_at_utf16(f.span.start(), text);
        let end = line_index.position_at_utf16(f.span.end(), text);
        assert_eq!((start.line, start.character.get()), (0, 6));
        assert_eq!((end.line, end.character.get()), (0, 8));

        let integrity = StyleDiagnostic {
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            message: "not UTF-8".to_owned(),
            severity: StyleSeverity::Warning,
            code: DiagCode::W109,
            fix: None,
        };
        let f = Finding::from_style(integrity, text, &line_index);
        assert_eq!(f.producer, Producer::SourceDecode);
        assert_eq!(f.span, Span::new(0, 0));
    }

    #[test]
    fn config_diagnostic_normalises_its_inclusive_end() {
        let d = ConfigDiagnostic {
            code: "BIGIP6002".to_owned(),
            message: "pool missing".to_owned(),
            severity: DiagSeverity::Warning,
            subject: tcl_bigip::validator::ConfigDiagnosticSubject::IRule,
            range: tcl_bigip::Range::from_offsets(
                "pool my_pool\n",
                &LineIndex::new("pool my_pool\n"),
                5,
                11,
            ),
        };
        let f = Finding::try_from(&d).expect("catalogued code");
        assert_eq!(f.code, DiagCode::Bigip6002);
        assert_eq!(f.span, Span::new(5, 12));
        assert_eq!(f.producer, Producer::BigipModel);
        assert_eq!(f.severity, Severity::Warning);

        let unknown = ConfigDiagnostic {
            code: "BIGIP9999".to_owned(),
            ..d
        };
        assert_eq!(Finding::try_from(&unknown), Err(UnknownDiagCode));
    }

    fn finding(code: DiagCode, span: Span) -> Finding {
        Finding {
            code,
            span,
            severity: Severity::Warning,
            message: String::new(),
            fixes: Vec::new(),
            data: None,
            producer: Producer::Analyser,
        }
    }

    #[test]
    fn report_views_split_shown_from_suppressed_and_keep_order() {
        let report = Report::new(vec![
            (
                finding(DiagCode::W210, Span::new(0, 1)),
                Outcome::Shown {
                    severity: Severity::Error,
                    tag: None,
                },
            ),
            (
                finding(DiagCode::W211, Span::new(2, 3)),
                Outcome::Suppressed(Reason::FileDirective),
            ),
            (
                finding(DiagCode::W220, Span::new(4, 5)),
                Outcome::Shown {
                    severity: Severity::Warning,
                    tag: Some(DiagTag::Unnecessary),
                },
            ),
        ]);
        let shown: Vec<(DiagCode, Severity, Option<DiagTag>)> = report
            .shown()
            .map(|s| (s.finding.code, s.severity, s.tag))
            .collect();
        assert_eq!(
            shown,
            vec![
                (DiagCode::W210, Severity::Error, None),
                (
                    DiagCode::W220,
                    Severity::Warning,
                    Some(DiagTag::Unnecessary)
                ),
            ]
        );
        let suppressed: Vec<(DiagCode, Reason)> =
            report.suppressed().map(|(f, r)| (f.code, r)).collect();
        assert_eq!(suppressed, vec![(DiagCode::W211, Reason::FileDirective)]);
        assert_eq!(
            report.reason_for(DiagCode::W211, Span::new(2, 3)),
            Some(Reason::FileDirective)
        );
        assert_eq!(report.reason_for(DiagCode::W210, Span::new(0, 1)), None);
        assert_eq!(report.outcome_for(DiagCode::W210, Span::new(9, 9)), None);
        assert_eq!(report.len(), 3);
        assert!(!report.is_empty());
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    use serde_json::json;

    fn decision(policy: &Policy, code: DiagCode) -> Option<CodeDecision> {
        policy.codes.get(&code).copied()
    }

    #[test]
    fn the_builder_names_the_layer_that_decided_a_code() {
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"diagnostics": {"W111": false}}),
            )
            .layer(PolicyLayer::Editor, &json!({"diagnostics": {"W111": true}}))
            .build();
        assert_eq!(
            decision(&policy, DiagCode::W111),
            Some(CodeDecision {
                enabled: true,
                layer: PolicyLayer::Editor
            }),
            "the editor turns the global file's disable back on"
        );
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"diagnostics": {"W111": false}}),
            )
            .layer(PolicyLayer::Editor, &json!({"diagnostics": {"W111": true}}))
            .layer(
                PolicyLayer::Project,
                &json!({"diagnostics": {"W111": false}}),
            )
            .build();
        assert_eq!(
            decision(&policy, DiagCode::W111),
            Some(CodeDecision {
                enabled: false,
                layer: PolicyLayer::Project
            })
        );
        // A layer silent on the code leaves the lower decision standing.
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"diagnostics": {"W111": false}}),
            )
            .layer(
                PolicyLayer::Project,
                &json!({"diagnostics": {"W112": false}}),
            )
            .build();
        assert_eq!(
            decision(&policy, DiagCode::W111).map(|d| d.layer),
            Some(PolicyLayer::Global)
        );
        assert_eq!(
            decision(&policy, DiagCode::W112).map(|d| d.layer),
            Some(PolicyLayer::Project)
        );
    }

    #[test]
    fn the_default_off_seed_is_the_lowest_layer() {
        let policy = PolicyBuilder::new().build();
        assert!(policy.default_off.contains(&DiagCode::W242));
        assert_eq!(decision(&policy, DiagCode::W242), None);
        let policy = PolicyBuilder::new()
            .layer(PolicyLayer::Global, &json!({"diagnostics": {"W242": true}}))
            .build();
        assert_eq!(
            decision(&policy, DiagCode::W242),
            Some(CodeDecision {
                enabled: true,
                layer: PolicyLayer::Global
            })
        );
    }

    #[test]
    fn an_unusable_higher_value_resets_the_lower_decision() {
        // Merge-then-parse semantics: the higher layer's value replaced the
        // lower one in the merged JSON, and the parse then skipped it.
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"diagnostics": {"W111": false}}),
            )
            .layer(
                PolicyLayer::Project,
                &json!({"diagnostics": {"W111": "no"}}),
            )
            .build();
        assert_eq!(decision(&policy, DiagCode::W111), None);
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"diagnosticSeverity": {"W211": "warning"}}),
            )
            .layer(
                PolicyLayer::Editor,
                &json!({"diagnosticSeverity": {"W211": "default"}}),
            )
            .build();
        assert!(policy.severity_overrides.is_empty());
    }

    #[test]
    fn wrapped_and_flat_dotted_shapes_are_read() {
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &json!({"tclLsp": {"diagnostics": {"W108": false}}}),
            )
            .layer(
                PolicyLayer::Project,
                &json!({"tclLsp.diagnostics.W210": false, "diagnosticSeverity.W211": "error"}),
            )
            .build();
        assert_eq!(
            decision(&policy, DiagCode::W108).map(|d| d.layer),
            Some(PolicyLayer::Editor)
        );
        assert_eq!(
            decision(&policy, DiagCode::W210).map(|d| d.layer),
            Some(PolicyLayer::Project)
        );
        assert_eq!(
            policy.severity_overrides.get(&DiagCode::W211),
            Some(&Severity::Error)
        );
        // Keys that are not codes (`exclude`, `genericVariablePatterns`)
        // and codes the catalogue lacks are ignored.
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Project,
                &json!({"diagnostics": {"exclude": ["a/**"], "W122": false, "W999": false}}),
            )
            .build();
        assert!(policy.codes.is_empty());
    }

    #[test]
    fn the_optimiser_profile_and_overrides_resolve_in_layer_order() {
        let policy = PolicyBuilder::new().build();
        assert!(policy.optimiser.enabled);
        assert_eq!(policy.optimiser.profile, DEFAULT_EDITOR_PROFILE);
        assert!(policy.optimiser.disabled.contains(&DiagCode::O109));
        assert!(!policy.optimiser.disabled.contains(&DiagCode::O114));

        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"optimiser": {"profile": "full", "O114": false}}),
            )
            .layer(
                PolicyLayer::Project,
                &json!({"optimiser": {"enabled": false, "O109": false, "O114": true}}),
            )
            .build();
        assert!(!policy.optimiser.enabled);
        assert_eq!(policy.optimiser.profile, OptimisationProfile::Full);
        assert!(policy.optimiser.disabled.contains(&DiagCode::O109));
        assert!(
            !policy.optimiser.disabled.contains(&DiagCode::O114),
            "the project's `true` wins over the global `false`"
        );
        // An unknown profile name falls back to the editor default, as the
        // server's parse does.
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &json!({"optimiser": {"profile": "nope"}}),
            )
            .build();
        assert_eq!(policy.optimiser.profile, DEFAULT_EDITOR_PROFILE);
    }

    #[test]
    fn the_family_switches_and_the_document_gates_resolve() {
        let policy = PolicyBuilder::new()
            .layer(PolicyLayer::Global, &json!({"shimmer": {"enabled": false}}))
            .layer(
                PolicyLayer::Project,
                &json!({"features": {"diagnostics": false}}),
            )
            .excluded(true)
            .build();
        assert!(!policy.shimmer);
        assert!(!policy.document.reporting);
        assert!(policy.document.excluded);
        assert!(!policy.document.abstain);
        // The caller's own answer for `features.diagnostics` wins.
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Project,
                &json!({"features": {"diagnostics": false}}),
            )
            .reporting(true)
            .build();
        assert!(policy.document.reporting);
    }

    #[test]
    fn the_decode_report_drives_abstention() {
        let (_, report) = crate::source_decode::decode_source(&[0xFF, 0xFE, b'p', 0, b'u', 0]);
        assert!(
            PolicyBuilder::new()
                .decode(Some(&report))
                .build()
                .document
                .abstain
        );
        let (_, clean) = crate::source_decode::decode_source(b"puts ok\n");
        assert!(
            !PolicyBuilder::new()
                .decode(Some(&clean))
                .build()
                .document
                .abstain
        );
        assert!(!PolicyBuilder::new().decode(None).build().document.abstain);
    }

    #[test]
    fn the_overlap_table_follows_the_dialect() {
        let plain = dialect_overlaps(crate::profile_for_dialect("tcl9.0"));
        assert_eq!(plain, base_overlaps());
        assert_eq!(
            plain[0],
            Overlap {
                owner: OverlapOwner::Code(DiagCode::W110),
                superseded: DiagCode::O120,
                scope: OverlapScope::SameSpan,
            }
        );
        let sslic = dialect_overlaps(crate::profile_for_dialect("sslictcl"));
        assert!(sslic.contains(&Overlap {
            owner: OverlapOwner::Producer(Producer::SslicTcl),
            superseded: DiagCode::W123,
            scope: OverlapScope::Document,
        }));
        let built = PolicyBuilder::new()
            .dialect(crate::profile_for_dialect("sslictcl"))
            .build();
        assert_eq!(built.overlaps, sslic);
    }

    #[test]
    fn directives_scan_reads_both_directive_spellings() {
        let text = "# tcl-lsp: disable=W111, O114\nset a 1\n# noqa: W210\nputs $b\n";
        let directives = Directives::scan(text, crate::profile_for_dialect("tcl9.0"));
        let file = &directives.lines()[&FILE_SUPPRESS_KEY];
        assert!(file.contains("W111") && file.contains("O114"));
        assert!(directives.lines()[&3].contains("W210"));
        assert!(directives.hit(FILE_SUPPRESS_KEY, DiagCode::W111));
        assert!(!directives.hit(FILE_SUPPRESS_KEY, DiagCode::W112));
        assert!(directives.hit(3, DiagCode::W210));
        // `puts $b` starts at byte 51 — line 3.
        assert_eq!(directives.line_of(Span::new(51, 58)), 3);
        // The analyser's own map is the same shape.
        let analysis = analyser::Analyser::new().analyse(text, "tcl9.0");
        let from_analysis = Directives::from_analysis(&analysis, text);
        assert!(from_analysis.hit(3, DiagCode::W210));
        assert!(from_analysis.hit(FILE_SUPPRESS_KEY, DiagCode::W111));
        // No text at all: nothing hits, and every span is on line 0.
        let none = Directives::none();
        assert!(!none.hit(0, DiagCode::W210));
        assert_eq!(none.line_of(Span::new(51, 58)), 0);
    }

    #[test]
    fn directives_answer_inline_before_file_and_skip_inline_for_whole_file_codes() {
        let text = "# tcl-lsp: disable=W112\n# noqa\nset x 1   \nputs $y\n";
        let directives = Directives::scan(text, crate::profile_for_dialect("tcl9.0"));
        // `set x 1   ` is line 2, bytes 31..41; `puts $y` is line 3, 42..49.
        assert_eq!(
            directives.reason_for(DiagCode::W210, Span::new(31, 41)),
            Some(Reason::InlineDirective { line: 2 })
        );
        // Both directives cover W112 on line 2; the inline one is reported.
        assert_eq!(
            directives.reason_for(DiagCode::W112, Span::new(38, 41)),
            Some(Reason::InlineDirective { line: 2 })
        );
        assert_eq!(
            directives.reason_for(DiagCode::W112, Span::new(42, 49)),
            Some(Reason::FileDirective)
        );
        assert_eq!(
            directives.reason_for(DiagCode::W210, Span::new(42, 49)),
            None
        );
        // A whole-file code ignores the inline bucket and honours the file one.
        let mut lines: HashMap<i32, HashSet<String>> = HashMap::new();
        lines.insert(0, std::iter::once("*".to_owned()).collect());
        let inline_only = Directives::new(lines.clone(), "a\r\nb\r\n");
        assert_eq!(
            inline_only.reason_for(DiagCode::W118, Span::new(0, 0)),
            None
        );
        assert_eq!(
            inline_only.reason_for(DiagCode::W111, Span::new(0, 0)),
            Some(Reason::InlineDirective { line: 0 })
        );
        lines.insert(
            FILE_SUPPRESS_KEY,
            std::iter::once("W118".to_owned()).collect(),
        );
        let with_file = Directives::new(lines, "a\r\nb\r\n");
        assert_eq!(
            with_file.reason_for(DiagCode::W118, Span::new(0, 0)),
            Some(Reason::FileDirective)
        );
        for code in WHOLE_FILE_CODES {
            assert!(!code.is_optimisation(), "{code}");
        }
    }

    #[test]
    fn a_wildcard_bucket_silences_every_code() {
        let mut lines: HashMap<i32, HashSet<String>> = HashMap::new();
        lines.insert(0, std::iter::once("*".to_owned()).collect());
        let directives = Directives::new(lines, "set x 1\n");
        assert!(directives.hit(0, DiagCode::W210));
        assert!(directives.hit(0, DiagCode::O114));
        assert!(!directives.hit(1, DiagCode::W210));
    }
}

#[cfg(test)]
mod apply_tests {
    use super::*;
    use crate::source_style::{DEFAULT_LINE_ENDING, DEFAULT_LINE_LENGTH, style_diagnostics};
    use serde_json::json;

    fn finding(code: DiagCode, span: Span, producer: Producer) -> Finding {
        Finding {
            code,
            span,
            severity: Severity::Warning,
            message: format!("{code}"),
            fixes: Vec::new(),
            data: None,
            producer,
        }
    }

    fn analyser(code: DiagCode, start: u32, end: u32) -> Finding {
        finding(code, Span::new(start, end), Producer::Analyser)
    }

    fn outcomes(report: &Report) -> Vec<(DiagCode, Outcome)> {
        report.iter().map(|(f, o)| (f.code, *o)).collect()
    }

    fn shown_codes(report: &Report) -> Vec<DiagCode> {
        report.shown().map(|s| s.finding.code).collect()
    }

    fn tcl9() -> &'static DialectProfile {
        crate::profile_for_dialect("tcl9.0")
    }

    #[test]
    fn reporting_off_then_excluded_hide_the_whole_document_with_a_reason() {
        let findings = vec![
            analyser(DiagCode::W210, 0, 3),
            analyser(DiagCode::E002, 4, 8),
        ];
        let off = Policy {
            document: DocumentGates {
                reporting: false,
                excluded: true,
                abstain: false,
            },
            ..Policy::default()
        };
        let report = apply(findings.clone(), &off);
        assert_eq!(report.len(), 2, "nothing is deleted");
        assert!(
            report
                .iter()
                .all(|(_, o)| *o == Outcome::Suppressed(Reason::ReportingOff))
        );
        let excluded = Policy {
            document: DocumentGates {
                excluded: true,
                ..DocumentGates::default()
            },
            ..Policy::default()
        };
        let report = apply(findings, &excluded);
        assert!(
            report
                .iter()
                .all(|(_, o)| *o == Outcome::Suppressed(Reason::Excluded))
        );
    }

    #[test]
    fn abstention_keeps_the_integrity_codes_and_beats_a_directive() {
        let text = "# noqa\nset x 1   \n";
        let findings = vec![
            finding(DiagCode::W109, Span::new(0, 0), Producer::SourceDecode),
            finding(DiagCode::W107, Span::new(0, 3), Producer::SourceDecode),
            analyser(DiagCode::W305, 0, 1),
            finding(DiagCode::W112, Span::new(14, 17), Producer::SourceStyle),
            analyser(DiagCode::W210, 7, 10),
        ];
        let policy = Policy {
            document: DocumentGates {
                abstain: true,
                ..DocumentGates::default()
            },
            directives: Directives::scan(text, tcl9()),
            ..Policy::default()
        };
        let report = apply(findings, &policy);
        assert_eq!(
            shown_codes(&report),
            vec![DiagCode::W109, DiagCode::W107, DiagCode::W305]
        );
        // The `# noqa` covers line 1 too, but abstention is the reason.
        assert_eq!(
            report.reason_for(DiagCode::W210, Span::new(7, 10)),
            Some(Reason::EncodingAbstention)
        );
        assert_eq!(
            report.reason_for(DiagCode::W112, Span::new(14, 17)),
            Some(Reason::EncodingAbstention)
        );
    }

    #[test]
    fn directives_beat_the_layers_and_the_inline_one_is_reported_first() {
        // Line 0 carries the file directive for W112; line 2 (`puts $y`)
        // carries an inline `# noqa` for W210; the project layer enables
        // both codes and the global layer disables W111.
        let text = "# tcl-lsp: disable=W112\n# noqa: W210\nputs $y\nset z 1   \n";
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"diagnostics": {"W111": false}}),
            )
            .layer(
                PolicyLayer::Project,
                &json!({"diagnostics": {"W210": true, "W112": true}}),
            )
            .directives(Directives::scan(text, tcl9()))
            .build();
        let findings = vec![
            analyser(DiagCode::W210, 37, 44),
            finding(DiagCode::W112, Span::new(52, 55), Producer::SourceStyle),
            finding(DiagCode::W111, Span::new(45, 55), Producer::SourceStyle),
        ];
        let report = apply(findings, &policy);
        assert_eq!(
            outcomes(&report),
            vec![
                (
                    DiagCode::W210,
                    Outcome::Suppressed(Reason::InlineDirective { line: 2 })
                ),
                (DiagCode::W112, Outcome::Suppressed(Reason::FileDirective)),
                (
                    DiagCode::W111,
                    Outcome::Suppressed(Reason::Disabled(PolicyLayer::Global))
                ),
            ]
        );
    }

    #[test]
    fn a_layer_names_itself_and_the_seed_reports_default_off() {
        let findings = vec![
            analyser(DiagCode::W242, 0, 3),
            analyser(DiagCode::W211, 4, 8),
        ];
        let seeded = apply(findings.clone(), &Policy::default());
        assert_eq!(
            seeded.reason_for(DiagCode::W242, Span::new(0, 3)),
            Some(Reason::DefaultOff)
        );
        assert_eq!(seeded.reason_for(DiagCode::W211, Span::new(4, 8)), None);
        for layer in [
            PolicyLayer::Global,
            PolicyLayer::Editor,
            PolicyLayer::Project,
        ] {
            let policy = PolicyBuilder::new()
                .layer(
                    layer,
                    &json!({"diagnostics": {"W242": true, "W211": false}}),
                )
                .build();
            let report = apply(findings.clone(), &policy);
            assert_eq!(shown_codes(&report), vec![DiagCode::W242], "{layer:?}");
            assert_eq!(
                report.reason_for(DiagCode::W211, Span::new(4, 8)),
                Some(Reason::Disabled(layer))
            );
        }
        // The project turns back on what the global file disabled.
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"diagnostics": {"W211": false}}),
            )
            .layer(
                PolicyLayer::Project,
                &json!({"diagnostics": {"W211": true}}),
            )
            .build();
        assert_eq!(shown_codes(&apply(findings, &policy)), vec![DiagCode::W211]);
    }

    #[test]
    fn the_optimiser_gates_fire_in_order_and_after_a_directive() {
        let text = "set x 1\n# noqa: O109\nset y 2\n";
        let rewrite = |code: DiagCode, start: u32, end: u32| {
            Finding::from(Optimisation::new(
                code,
                "rewrite",
                Span::new(start, end),
                "x",
            ))
        };
        let findings = vec![
            rewrite(DiagCode::O114, 0, 7),
            rewrite(DiagCode::O109, 21, 28),
            analyser(DiagCode::W210, 0, 7),
        ];
        let off = Policy {
            optimiser: OptimiserPolicy {
                enabled: false,
                ..OptimiserPolicy::for_profile(OptimisationProfile::Full)
            },
            ..Policy::default()
        };
        let report = apply(findings.clone(), &off);
        assert_eq!(
            report.reason_for(DiagCode::O114, Span::new(0, 7)),
            Some(Reason::OptimiserOff)
        );
        assert_eq!(shown_codes(&report), vec![DiagCode::W210]);

        let readability = Policy {
            optimiser: OptimiserPolicy::for_profile(OptimisationProfile::Readability),
            directives: Directives::scan(text, tcl9()),
            ..Policy::default()
        };
        let report = apply(findings, &readability);
        assert_eq!(
            report.reason_for(DiagCode::O114, Span::new(0, 7)),
            None,
            "a readability rewrite shows under the readability profile"
        );
        assert_eq!(
            report.reason_for(DiagCode::O109, Span::new(21, 28)),
            Some(Reason::InlineDirective { line: 2 }),
            "the directive is reported ahead of the profile"
        );
        let no_directive = Policy {
            directives: Directives::none(),
            ..readability
        };
        let report = apply(
            vec![Finding::from(Optimisation::new(
                DiagCode::O109,
                "dead store",
                Span::new(21, 28),
                "",
            ))],
            &no_directive,
        );
        assert_eq!(
            report.reason_for(DiagCode::O109, Span::new(21, 28)),
            Some(Reason::OptimiserProfile {
                profile: OptimisationProfile::Readability
            })
        );
    }

    #[test]
    fn the_shimmer_switch_hides_the_family_only() {
        let findings = vec![
            finding(DiagCode::S100, Span::new(0, 3), Producer::CompilerCheck),
            finding(DiagCode::S110, Span::new(4, 8), Producer::CompilerCheck),
            finding(DiagCode::T100, Span::new(9, 12), Producer::CompilerCheck),
        ];
        let policy = Policy {
            shimmer: false,
            ..Policy::default()
        };
        let report = apply(findings, &policy);
        assert_eq!(shown_codes(&report), vec![DiagCode::T100]);
        assert_eq!(
            report.reason_for(DiagCode::S100, Span::new(0, 3)),
            Some(Reason::ShimmerOff)
        );
    }

    #[test]
    fn a_same_span_overlap_needs_a_standing_owner() {
        let w110 = analyser(DiagCode::W110, 10, 12);
        let o120 = Finding::from(Optimisation::new(
            DiagCode::O120,
            "eq",
            Span::new(10, 12),
            "eq",
        ));
        let elsewhere = Finding::from(Optimisation::new(
            DiagCode::O120,
            "eq",
            Span::new(30, 32),
            "eq",
        ));
        let policy = Policy {
            optimiser: OptimiserPolicy::all_on(),
            ..Policy::default()
        };
        let report = apply(vec![w110.clone(), o120.clone(), elsewhere.clone()], &policy);
        assert_eq!(
            report.reason_for(DiagCode::O120, Span::new(10, 12)),
            Some(Reason::Overlap {
                owner: OverlapOwner::Code(DiagCode::W110)
            })
        );
        assert_eq!(report.reason_for(DiagCode::O120, Span::new(30, 32)), None);
        // An owner that policy suppressed cannot supersede anything.
        let mut disabled = Policy {
            optimiser: OptimiserPolicy::all_on(),
            ..Policy::default()
        };
        disabled.codes.insert(
            DiagCode::W110,
            CodeDecision {
                enabled: false,
                layer: PolicyLayer::Editor,
            },
        );
        let report = apply(vec![w110, o120], &disabled);
        assert_eq!(shown_codes(&report), vec![DiagCode::O120]);
    }

    #[test]
    fn a_producer_owns_the_document_without_a_finding_of_its_own() {
        let policy = Policy {
            overlaps: dialect_overlaps(crate::profile_for_dialect("sslictcl")),
            ..Policy::default()
        };
        let findings = vec![
            analyser(DiagCode::W123, 0, 4),
            analyser(DiagCode::E003, 5, 9),
        ];
        let report = apply(findings, &policy);
        assert_eq!(
            report.reason_for(DiagCode::W123, Span::new(0, 4)),
            Some(Reason::Overlap {
                owner: OverlapOwner::Producer(Producer::SslicTcl)
            })
        );
        assert_eq!(shown_codes(&report), vec![DiagCode::E003]);
    }

    #[test]
    fn a_shown_finding_carries_the_override_and_the_tag() {
        let findings = vec![
            analyser(DiagCode::W214, 0, 1),
            analyser(DiagCode::W210, 2, 3),
        ];
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &json!({"diagnosticSeverity": {"W214": "error"}}),
            )
            .build();
        let report = apply(findings, &policy);
        let shown: Vec<(DiagCode, Severity, Option<DiagTag>)> = report
            .shown()
            .map(|s| (s.finding.code, s.severity, s.tag))
            .collect();
        assert_eq!(
            shown,
            vec![
                (DiagCode::W214, Severity::Error, Some(DiagTag::Unnecessary)),
                (DiagCode::W210, Severity::Warning, None),
            ]
        );
    }

    #[test]
    fn the_wildcard_reaches_both_directive_spellings() {
        let inline = "# noqa\nset x 1   \n";
        let report = apply(
            vec![
                analyser(DiagCode::W210, 7, 10),
                finding(DiagCode::W112, Span::new(14, 17), Producer::SourceStyle),
            ],
            &Policy {
                directives: Directives::scan(inline, tcl9()),
                ..Policy::default()
            },
        );
        assert!(report.shown().next().is_none(), "{:?}", outcomes(&report));
        let file = "# tcl-lsp: disable=*\nset x 1   \n";
        let report = apply(
            vec![
                analyser(DiagCode::W210, 21, 24),
                finding(DiagCode::W118, Span::new(0, 0), Producer::SourceStyle),
            ],
            &Policy {
                directives: Directives::scan(file, tcl9()),
                ..Policy::default()
            },
        );
        assert!(
            report
                .iter()
                .all(|(_, o)| *o == Outcome::Suppressed(Reason::FileDirective))
        );
    }

    #[test]
    fn the_style_pass_through_apply_keeps_the_orchestrator_rules() {
        // What the style pass used to decide for itself: a line-0 `*`
        // hides the line-0 W112 but never the file-level W118; a file
        // directive hides a named code; a disabled set hides a code.
        let src = "set x 1   \r\n";
        let style = style_diagnostics(src, DEFAULT_LINE_LENGTH, DEFAULT_LINE_ENDING, None, tcl9());
        let codes: Vec<DiagCode> = style.iter().map(|d| d.code).collect();
        assert_eq!(codes, vec![DiagCode::W112, DiagCode::W118]);
        let line_index = LineIndex::new_lsp(src);
        let findings = |style: &[StyleDiagnostic]| -> Vec<Finding> {
            style
                .iter()
                .cloned()
                .map(|d| Finding::from_style(d, src, &line_index))
                .collect()
        };

        let mut inline: HashMap<i32, HashSet<String>> = HashMap::new();
        inline.insert(0, std::iter::once("*".to_owned()).collect());
        let policy = Policy {
            directives: Directives::new(inline, src),
            ..Policy::default()
        };
        let report = apply(findings(&style), &policy);
        assert_eq!(shown_codes(&report), vec![DiagCode::W118]);
        assert_eq!(report.shown_items(style.clone()).len(), 1);

        let mut file: HashMap<i32, HashSet<String>> = HashMap::new();
        file.insert(
            FILE_SUPPRESS_KEY,
            std::iter::once("W112".to_owned()).collect(),
        );
        let policy = Policy {
            directives: Directives::new(file, src),
            ..Policy::default()
        };
        assert_eq!(
            shown_codes(&apply(findings(&style), &policy)),
            vec![DiagCode::W118]
        );

        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &json!({"diagnostics": {"W118": false}}),
            )
            .build();
        let report = apply(findings(&style), &policy);
        assert_eq!(shown_codes(&report), vec![DiagCode::W112]);
        assert_eq!(
            report.reason_for(DiagCode::W118, Span::new(0, 0)),
            Some(Reason::Disabled(PolicyLayer::Editor))
        );
    }

    #[test]
    fn the_report_keeps_the_producers_order() {
        let findings = vec![
            analyser(DiagCode::W211, 9, 10),
            analyser(DiagCode::W210, 0, 1),
            analyser(DiagCode::E002, 5, 6),
        ];
        let expected: Vec<DiagCode> = findings.iter().map(|f| f.code).collect();
        let report = apply(findings, &Policy::default());
        let got: Vec<DiagCode> = report.iter().map(|(f, _)| f.code).collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn code_reason_is_the_per_code_decision_and_the_family_gates() {
        let mut policy = Policy::default();
        assert_eq!(policy.code_reason(DiagCode::W210), None);
        assert_eq!(policy.code_reason(DiagCode::W242), Some(Reason::DefaultOff));
        policy.codes.insert(
            DiagCode::W210,
            CodeDecision {
                enabled: false,
                layer: PolicyLayer::Project,
            },
        );
        assert_eq!(
            policy.code_reason(DiagCode::W210),
            Some(Reason::Disabled(PolicyLayer::Project))
        );
        policy.optimiser.enabled = false;
        assert_eq!(
            policy.code_reason(DiagCode::O100),
            Some(Reason::OptimiserOff)
        );
        policy.shimmer = false;
        assert_eq!(policy.code_reason(DiagCode::S100), Some(Reason::ShimmerOff));
    }

    #[test]
    fn production_skip_is_every_code_the_policy_hides_without_a_finding() {
        let mut policy = Policy::default();
        policy.codes.insert(
            DiagCode::W210,
            CodeDecision {
                enabled: false,
                layer: PolicyLayer::Global,
            },
        );
        let skip = policy.production_skip();
        assert!(skip.contains(&DiagCode::W210));
        assert!(
            skip.contains(&DiagCode::W242),
            "the default-off seed is skipped"
        );
        assert!(!skip.contains(&DiagCode::W100));
        // The default readability profile hides the non-readability rewrites.
        assert!(skip.contains(&DiagCode::O107));
        assert!(!skip.contains(&DiagCode::O120));
    }

    #[test]
    fn a_declared_skip_explains_a_code_no_finding_carries() {
        let mut policy = Policy::default();
        policy.codes.insert(
            DiagCode::W210,
            CodeDecision {
                enabled: false,
                layer: PolicyLayer::Invocation,
            },
        );
        let mut report = apply(Vec::new(), &policy);
        report.declare_skipped([DiagCode::W210, DiagCode::W100], &policy);
        assert_eq!(
            report.reason_for(DiagCode::W210, Span::new(0, 1)),
            Some(Reason::Disabled(PolicyLayer::Invocation))
        );
        assert_eq!(
            report.reason_for(DiagCode::W100, Span::new(0, 1)),
            None,
            "a code the policy would show has no skip to declare"
        );
        assert_eq!(report.skipped().count(), 1);
    }
}
