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
//! string (`tcl_bigip::validator::ConfigDiagnostic`) converts fallibly, so an
//! uncatalogued code is a conversion failure rather than a value that
//! silently skips every table.

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

/// The truth table every adapter runs (`docs/design/compiler/diagnostic-policy.md`
/// § The truth table) — built for this crate's tests and, through the
/// `truth-table` feature, for the adapter crates' tests.
#[cfg(any(test, feature = "truth-table"))]
pub mod truth_table;

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

impl Producer {
    /// The spelling an `overlap:<producer>` [`Reason`] renders.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analyser => "analyser",
            Self::CompilerCheck => "compiler-check",
            Self::Optimiser => "optimiser",
            Self::SourceStyle => "source-style",
            Self::SourceDecode => "source-decode",
            Self::SslicTcl => "sslictcl",
            Self::Xc => "xc",
            Self::BigipModel => "bigip-model",
        }
    }
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
    /// The original analyser issuer, including its structured semantic subject.
    Analyser(Box<analyser::Diagnostic>),
    /// The original compiler issuer, including whole-source/config provenance.
    CompilerCheck(Box<compiler_checks::Diagnostic>),
    /// An actual finalized optimiser member with its sealed worker batch.
    IssuedOptimiser(Box<Optimisation>),
    /// An optimiser rewrite. Manual payloads have no source edit authority.
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
        let original = Box::new(d.clone());
        Self {
            code: d.code,
            span: d.span,
            severity: d.severity,
            message: d.message,
            fixes: d.fixes.into_iter().map(Fix::from).collect(),
            data: Some(FindingData::Analyser(original)),
            producer: Producer::Analyser,
        }
    }
}

impl From<compiler_checks::Diagnostic> for Finding {
    /// `category` is dropped; a same-span `replacement` becomes a [`Fix`]
    /// titled with the message, classified [`FixSafety::RequiresReview`]
    /// because the check never proved more.
    fn from(d: compiler_checks::Diagnostic) -> Self {
        let original = Box::new(d.clone());
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
            data: Some(FindingData::CompilerCheck(original)),
            producer: Producer::CompilerCheck,
        }
    }
}

impl From<Optimisation> for Finding {
    /// An optimisation is a `Hint`; its rewrite rides on
    /// [`FindingData::Rewrite`] rather than on `fixes`, because whether it
    /// may be applied is the adapter's reading of `hint_only`.
    fn from(o: Optimisation) -> Self {
        let original = o
            .original_rewrite_batch()
            .is_some()
            .then(|| Box::new(o.clone()));
        Self {
            code: o.code,
            span: o.span,
            severity: Severity::Hint,
            message: o.message,
            fixes: Vec::new(),
            data: Some(match original {
                Some(original) => FindingData::IssuedOptimiser(original),
                None => FindingData::Rewrite {
                    replacement: o.replacement,
                    group: o.group,
                    hint_only: o.hint_only,
                },
            }),
            producer: Producer::Optimiser,
        }
    }
}

impl Finding {
    /// Reporting payload shared by issued and manual optimiser findings.
    /// This view alone supplies no source correspondence or edit permission.
    #[must_use]
    pub fn rewrite_payload(&self) -> Option<(&str, Option<u32>, bool)> {
        match self.data.as_ref()? {
            FindingData::Rewrite {
                replacement,
                group,
                hint_only,
            } => Some((replacement, *group, *hint_only)),
            FindingData::IssuedOptimiser(original) => {
                Some((&original.replacement, original.group, original.hint_only))
            }
            _ => None,
        }
    }
    /// Actual finalized worker member, never reconstructed from report fields.
    #[must_use]
    pub fn original_optimiser_rewrite(&self) -> Option<&Optimisation> {
        match self.data.as_ref()? {
            FindingData::IssuedOptimiser(original) => Some(original),
            _ => None,
        }
    }

    /// Structured original analyser issuer; reporting text is never parsed.
    #[must_use]
    pub fn original_analyser_diagnostic(&self) -> Option<&analyser::Diagnostic> {
        match self.data.as_ref()? {
            FindingData::Analyser(original) => Some(original),
            _ => None,
        }
    }

    /// Independent compiler source issuer, retained through report policy.
    #[must_use]
    pub fn original_compiler_diagnostic(&self) -> Option<&compiler_checks::Diagnostic> {
        match self.data.as_ref()? {
            FindingData::CompilerCheck(original) => Some(original),
            _ => None,
        }
    }

    /// Consumer DTO from the original semantic subject, independent of prose.
    #[must_use]
    pub fn structured_data(&self) -> Option<serde_json::Value> {
        if let Some(original) = self.original_analyser_diagnostic() {
            return crate::diagnostic_subject::diagnostic_subject_data(original);
        }
        crate::code_actions::ContextDiagnosticData::from_compiler_diagnostic(
            self.original_compiler_diagnostic()?,
        )
        .map(|subject| subject.to_value())
    }

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
/// The declaration order is rule 5's precedence order, stated once:
/// [`PolicyBuilder::build`] resolves the layers by it, whatever order a
/// surface adds them in. `Editor` and `Invocation` share the slot between
/// the global file and the project file; no surface adds both.
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
    /// A surface's own flags (`--disable`, `--enable`, an MCP `disable` or
    /// `enable` argument), which occupy the editor layer's slot in the
    /// order. A named optimiser profile is not a layer: it is the request's
    /// own ([`PolicyBuilder::requested_profile`]).
    Invocation,
    /// The project's `.tcl-lsp.ini`.
    Project,
}

impl PolicyLayer {
    /// The spelling a `disabled:<layer>` [`Reason`] renders.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Editor => "editor",
            Self::Invocation => "invocation",
            Self::Project => "project",
        }
    }
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

/// One spelling for every reason, lower-case and hyphenated with an
/// optional `:detail` — what the CLI rows, the MCP JSON and the truth table
/// all render (`docs/design/compiler/diagnostic-policy.md` § Adapters, the
/// reason spellings).
impl core::fmt::Display for Reason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ReportingOff => write!(f, "reporting-off"),
            Self::Excluded => write!(f, "excluded"),
            Self::EncodingAbstention => write!(f, "encoding-abstention"),
            // The finding's own row carries its line; the reason names only
            // the kind of directive.
            Self::InlineDirective { .. } => write!(f, "inline-directive"),
            Self::FileDirective => write!(f, "file-directive"),
            Self::Disabled(layer) => write!(f, "disabled:{}", layer.as_str()),
            Self::DefaultOff => write!(f, "default-off"),
            Self::OptimiserOff => write!(f, "optimiser-off"),
            Self::OptimiserProfile { profile } => {
                write!(f, "optimiser-profile:{}", profile.name())
            }
            Self::ShimmerOff => write!(f, "shimmer-off"),
            Self::Overlap {
                owner: OverlapOwner::Code(code),
            } => write!(f, "overlap:{code}"),
            Self::Overlap {
                owner: OverlapOwner::Producer(producer),
            } => write!(f, "overlap:{}", producer.as_str()),
        }
    }
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

/// A rewrite a surface may offer: one shown ungrouped rewrite, or every
/// member of an optimisation group all of whose members show.
#[derive(Debug, Clone)]
pub struct ApplicableRewrite<'a> {
    /// The group, for a grouped rewrite.
    pub group: Option<u32>,
    /// The members in the producers' order — one for an ungrouped rewrite.
    pub members: Vec<&'a Finding>,
}

/// The optimisation group `finding`'s rewrite belongs to, if any.
fn rewrite_group(finding: &Finding) -> Option<u32> {
    finding.rewrite_payload().and_then(|(_, group, _)| group)
}

/// A producer a surface did not run, and the codes only it emits that the
/// report explains with one reason — rendered as one row, not one per code
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters, "What a
/// diagnostics surface did not run").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotRun {
    /// The producer the surface did not run.
    pub producer: Producer,
    /// The policy's reason, the same for every code here.
    pub reason: Reason,
    /// The codes, in catalogue order.
    pub codes: Vec<DiagCode>,
}

impl NotRun {
    /// The row's sentence, the same on every rendering: `optimiser not run
    /// on this surface`.
    #[must_use]
    pub fn message(&self) -> String {
        format!("{} not run on this surface", self.producer.as_str())
    }
}

/// One declared skip: the policy's reason for the code's absence and, when
/// the surface did not run the producer at all, that producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeclaredSkip {
    reason: Reason,
    not_run: Option<Producer>,
}

/// Every finding paired with its outcome, in the producers' order, and the
/// codes a producer declared it left uncomputed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    analysis_context_unavailable: Option<tcl_registry::model::OverlayMiss>,
    outcomes: Vec<(Finding, Outcome)>,
    /// The declared skips, with the policy's reason for each code — so a
    /// gap where nothing was produced is explained rather than read as
    /// "clean" (`docs/design/compiler/diagnostic-policy.md` § Producers that
    /// change).
    skipped: BTreeMap<DiagCode, DeclaredSkip>,
}

impl Report {
    /// A refused generation is distinct from a successful analysis with no findings.
    pub fn retain_analysis_context(&mut self, analysis: &analyser::AnalysisResult) {
        self.analysis_context_unavailable
            .clone_from(&analysis.analysis_context_unavailable);
    }
    /// Original generation failure; display text supplies no replacement context.
    #[must_use]
    pub fn analysis_context_unavailable(&self) -> Option<&tcl_registry::model::OverlayMiss> {
        self.analysis_context_unavailable.as_ref()
    }
    /// Reporting-only typed unavailable input fields.
    #[must_use]
    pub fn analysis_context_status_data(&self) -> Option<serde_json::Value> {
        self.analysis_context_unavailable().map(|miss| serde_json::json!({"kind":"analysisContextUnavailable", "environment":miss.environment, "overlay":miss.overlay}))
    }

    /// The report over `outcomes`, with nothing declared skipped.
    #[must_use]
    pub fn new(outcomes: Vec<(Finding, Outcome)>) -> Self {
        Self {
            outcomes,
            skipped: BTreeMap::new(),
            analysis_context_unavailable: None,
        }
    }

    /// Record that a producer skipped computing `codes`, each with the reason
    /// `policy` gives for the code's absence ([`Policy::gap_reason`]). A code
    /// the policy would show is not recorded: there is nothing to explain a
    /// skip of it with, and a truth-table row over the producer is what
    /// catches that mismatch. A code a producer the surface did not run
    /// already explains ([`Self::declare_not_run`]) keeps that declaration.
    pub fn declare_skipped(&mut self, codes: impl IntoIterator<Item = DiagCode>, policy: &Policy) {
        for code in codes {
            if let Some(reason) = policy.gap_reason(code) {
                self.skipped.entry(code).or_insert(DeclaredSkip {
                    reason,
                    not_run: None,
                });
            }
        }
    }

    /// Record that the surface did not run `producer`, which alone emits
    /// `codes`, each with the reason `policy` gives for its absence. Where a
    /// code is also declared skipped by name ([`Self::declare_skipped`] — a
    /// layer's `false` or the file directive puts every code in the
    /// analyser's skip, whichever producer emits it), this declaration
    /// stands, in either order: the code is absent because its producer
    /// never ran, and the reason is the policy's either way.
    pub fn declare_not_run(
        &mut self,
        producer: Producer,
        codes: impl IntoIterator<Item = DiagCode>,
        policy: &Policy,
    ) {
        for code in codes {
            if let Some(reason) = policy.gap_reason(code) {
                self.skipped.insert(
                    code,
                    DeclaredSkip {
                        reason,
                        not_run: Some(producer),
                    },
                );
            }
        }
    }

    /// Record the analyser's skip under `policy` ([`Policy::analyser_skip`]):
    /// the production skip the surface handed it and the codes the file
    /// directive folded away. What every surface that ran the analyser
    /// declares, so a code it left uncomputed is explained rather than read
    /// as clean.
    pub fn declare_analyser_skip(&mut self, policy: &Policy) {
        self.declare_skipped(policy.analyser_skip(), policy);
    }

    /// Record that the surface did not run the optimiser: every catalogued
    /// optimisation code but [`PRODUCED_WITHOUT_THE_OPTIMISER`], each with
    /// the reason `policy` gives for its absence — [`Reason::OptimiserOff`]
    /// under a policy whose switch is off, unless an earlier step names
    /// another, as for every declared skip. What the diagnostics verbs and
    /// tools declare, which leave the rewrites to the rewrite surfaces: an
    /// O-code only the optimiser emits is then explained rather than read as
    /// clean ([`Self::not_run`]). A code the compiler checks or the O111
    /// producer emit is not declared: those producers ran, so its absence is
    /// a clean answer, and a finding of it carries its own reason.
    pub fn declare_optimiser_skip(&mut self, policy: &Policy) {
        self.declare_not_run(
            Producer::Optimiser,
            DiagCode::ALL.iter().copied().filter(|code| {
                code.is_optimisation() && !PRODUCED_WITHOUT_THE_OPTIMISER.contains(code)
            }),
            policy,
        );
    }

    /// The declared skips, by code.
    pub fn skipped(&self) -> impl Iterator<Item = (DiagCode, Reason)> + '_ {
        self.skipped.iter().map(|(code, skip)| (*code, skip.reason))
    }

    /// The codes declared skipped by name whose code no finding in the
    /// report carries: the codes the policy turned off for this document
    /// that the report cannot show as a finding. A declared code some other
    /// producer did emit is not a gap — its findings carry their own
    /// reasons. A code a producer the surface did not run explains is in
    /// [`Self::not_run`] instead, so every declared code no finding carries
    /// is in exactly one of the two.
    pub fn gaps(&self) -> impl Iterator<Item = (DiagCode, Reason)> + '_ {
        self.skipped
            .iter()
            .filter(|(code, skip)| skip.not_run.is_none() && !self.carries(**code))
            .map(|(code, skip)| (*code, skip.reason))
    }

    /// Every producer the surface declared it did not run, with the codes no
    /// finding carries, one entry per producer and reason — in the order of
    /// each entry's first code, its codes in catalogue order.
    #[must_use]
    pub fn not_run(&self) -> Vec<NotRun> {
        let mut rows: Vec<NotRun> = Vec::new();
        for (code, skip) in &self.skipped {
            let Some(producer) = skip.not_run else {
                continue;
            };
            if self.carries(*code) {
                continue;
            }
            match rows
                .iter_mut()
                .find(|row| row.producer == producer && row.reason == skip.reason)
            {
                Some(row) => row.codes.push(*code),
                None => rows.push(NotRun {
                    producer,
                    reason: skip.reason,
                    codes: vec![*code],
                }),
            }
        }
        rows
    }

    /// Whether any finding in the report carries `code`.
    fn carries(&self, code: DiagCode) -> bool {
        self.outcomes
            .iter()
            .any(|(finding, _)| finding.code == code)
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
            None => self.skipped.get(&code).map(|skip| skip.reason),
        }
    }

    /// The rewrites a surface may offer or publish as an edit. Ungrouped: a
    /// shown [`FindingData::Rewrite`] that is not `hint_only` and has a
    /// non-empty replacement. Grouped: a group every member of which shows
    /// and none of which is `hint_only` (a member's replacement may be empty
    /// — a deletion). A group that lost a member to the policy — a directive
    /// on one member's line, a per-code toggle — is not applicable at all:
    /// its edits apply all-or-nothing, and offering the survivor alone is
    /// the corruption #2149 describes (O127's inline without its delete runs
    /// the assignment twice).
    #[must_use]
    pub fn applicable_rewrites(&self) -> Vec<ApplicableRewrite<'_>> {
        let whole = self.whole_groups(GroupMembers::Actionable);
        let mut out: Vec<ApplicableRewrite<'_>> = Vec::new();
        let mut group_at: BTreeMap<u32, usize> = BTreeMap::new();
        for (finding, outcome) in &self.outcomes {
            let Some((replacement, group, hint_only)) = finding.rewrite_payload() else {
                continue;
            };
            match group {
                None => {
                    if matches!(outcome, Outcome::Shown { .. })
                        && !hint_only
                        && !replacement.is_empty()
                    {
                        out.push(ApplicableRewrite {
                            group: None,
                            members: vec![finding],
                        });
                    }
                }
                Some(g) if whole.contains(&g) => {
                    if let Some(&at) = group_at.get(&g) {
                        out[at].members.push(finding);
                    } else {
                        group_at.insert(g, out.len());
                        out.push(ApplicableRewrite {
                            group: Some(g),
                            members: vec![finding],
                        });
                    }
                }
                Some(_) => {}
            }
        }
        out
    }

    /// `items`, one per finding in the producers' order, kept where the
    /// finding shows and, for a grouped rewrite, where its whole group
    /// shows — the rewrite loop's filter, which keeps the optimiser's own
    /// record beside each converted finding. Sound because [`apply`] is
    /// order-stable and keeps every finding.
    #[must_use]
    pub fn applicable_items<T>(&self, items: Vec<T>) -> Vec<T> {
        debug_assert_eq!(items.len(), self.outcomes.len(), "one item per finding");
        let whole = self.whole_groups(GroupMembers::Shown);
        self.outcomes
            .iter()
            .zip(items)
            .filter_map(|((finding, outcome), item)| {
                (matches!(outcome, Outcome::Shown { .. })
                    && rewrite_group(finding).is_none_or(|g| whole.contains(&g)))
                .then_some(item)
            })
            .collect()
    }

    /// The optimisation groups every member of which passes `members`. The
    /// report keeps every finding, so it knows each group's full size
    /// without a second list.
    fn whole_groups(&self, members: GroupMembers) -> BTreeSet<u32> {
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut broken: BTreeSet<u32> = BTreeSet::new();
        for (finding, outcome) in &self.outcomes {
            let Some((_, Some(g), hint_only)) = finding.rewrite_payload() else {
                continue;
            };
            seen.insert(g);
            let shows = matches!(outcome, Outcome::Shown { .. });
            if !shows || (members == GroupMembers::Actionable && hint_only) {
                broken.insert(g);
            }
        }
        seen.difference(&broken).copied().collect()
    }

    /// Every pair, in the producers' order.
    pub fn iter(&self) -> impl Iterator<Item = &(Finding, Outcome)> {
        self.outcomes.iter()
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

/// Which members a whole optimisation group needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupMembers {
    /// Every member shows — the rewrite loop's reading, where the optimiser
    /// itself skips a `hint_only` record.
    Shown,
    /// Every member shows and none is `hint_only` — what a surface may offer
    /// or publish as an edit.
    Actionable,
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
    /// The profile in force: the one the request names, else the layers'
    /// `optimiser.profile`, else the surface's default
    /// ([`PolicyBuilder::requested_profile`]). It decides the category set
    /// here and the pass count wherever the optimiser runs.
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

    /// The policy the layers resolve: `profile`, the profile in force, with
    /// the layers' switch (`enabled`, on unless a layer says otherwise) and
    /// their per-code overrides applied over its disabled set — a `true`
    /// turns a code back on, a `false` off, anything else leaves the
    /// profile's choice.
    fn resolved(
        profile: OptimisationProfile,
        enabled: Option<&Value>,
        overrides: &BTreeMap<DiagCode, Value>,
    ) -> Self {
        let mut optimiser = Self::for_profile(profile);
        optimiser.enabled = enabled.and_then(Value::as_bool).unwrap_or(true);
        for (code, value) in overrides {
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
        optimiser
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
    /// Only where the two spans coincide.
    SameSpan,
    /// Where the owner's span lies inside the superseded finding's span —
    /// the W110 / O120 rule. Each producer anchors its own fact where it
    /// belongs, the analyser W110 on the `==` operator and the optimiser
    /// O120 on the whole condition it rewrites, so the owner sits inside
    /// what it supersedes rather than on it.
    WithinSpan,
    /// Everywhere in the document.
    Document,
}

/// The codes that are whole-file verdicts: an inline `# noqa` has no line
/// to attach to, so only the top-of-file directive and the configuration
/// layers gate them. W118 reads the document's line terminators; W107 and
/// W109 read its bytes. Stated here once rather than in one pass's control
/// flow.
pub const WHOLE_FILE_CODES: &[DiagCode] = &[DiagCode::W107, DiagCode::W109, DiagCode::W118];

/// Codes whose findings another producer reads as a fact — O111 reads W100's
/// sites — so the analyser computes them whatever the policy says; the policy
/// step still decides whether they show.
///
/// The analyser's production-time skip is safe only because its findings are
/// not read as facts (`docs/design/compiler/diagnostic-policy.md` § Failure
/// modes); W100 and O111 consume the same unbraced-expression fact (§ Producers
/// that change), so disabling W100 must not take O111's input away.
pub const FACT_CODES: &[DiagCode] = &[DiagCode::W100];

/// The optimisation codes a producer other than the optimiser emits: the
/// compiler checks' SCCP (O100) and GVN (O105, O106) findings and the O111
/// hints ([`crate::diagnostic_report::brace_expr_hints`]). A surface that
/// runs those producers without the optimiser — `tcl diag`, `tcl lint`, the
/// MCP diagnostics tools — computed these codes, so it never declares them
/// among the optimiser's ([`Report::declare_optimiser_skip`]): with no
/// finding, their absence is a clean answer, not a gap.
pub const PRODUCED_WITHOUT_THE_OPTIMISER: &[DiagCode] = &[
    DiagCode::O100,
    DiagCode::O105,
    DiagCode::O106,
    DiagCode::O111,
];

/// The overlap entries every dialect carries.
fn base_overlaps() -> Vec<Overlap> {
    vec![Overlap {
        owner: OverlapOwner::Code(DiagCode::W110),
        superseded: DiagCode::O120,
        scope: OverlapScope::WithinSpan,
    }]
}

/// The overlap table for a document of `dialect`: W110 owns an O120 whose
/// span holds it everywhere, and in a `.sslictcl` document the loader owns
/// the analyser codes [`crate::sslictcl_diagnostics::SUPERSEDED_ANALYSER_CODES`]
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
        self.file_reason(code)
    }

    /// [`Reason::FileDirective`] when the top-of-file directive names `code`
    /// or `*` — the half of [`Self::reason_for`] that needs no span.
    #[must_use]
    pub fn file_reason(&self, code: DiagCode) -> Option<Reason> {
        self.hit(FILE_SUPPRESS_KEY, code)
            .then_some(Reason::FileDirective)
    }

    /// The catalogued codes the top-of-file directive names, spelled as the
    /// catalogue spells them. `*` is not a code and is not among them; nor
    /// is a spelling the catalogue lacks.
    pub fn file_codes(&self) -> impl Iterator<Item = DiagCode> + '_ {
        self.lines
            .get(&FILE_SUPPRESS_KEY)
            .into_iter()
            .flatten()
            .filter_map(|code| DiagCode::from_str(code).ok())
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

    /// Steps 1 and 2 of [`apply`] for `code`: the document-wide gates, then
    /// encoding abstention, which spares only [`ABSTENTION_SURVIVORS`].
    fn document_reason(&self, code: DiagCode) -> Option<Reason> {
        if !self.document.reporting {
            Some(Reason::ReportingOff)
        } else if self.document.excluded {
            Some(Reason::Excluded)
        } else if self.document.abstain && !ABSTENTION_SURVIVORS.contains(&code) {
            Some(Reason::EncodingAbstention)
        } else {
            None
        }
    }

    /// Step 4 of [`apply`] for `code` alone: [`Reason::Disabled`] for the
    /// layer whose value stands, or [`Reason::DefaultOff`] when the code is
    /// in the seed and no layer turned it on.
    fn decision_reason(&self, code: DiagCode) -> Option<Reason> {
        match self.codes.get(&code) {
            Some(CodeDecision {
                enabled: false,
                layer,
            }) => Some(Reason::Disabled(*layer)),
            Some(CodeDecision { enabled: true, .. }) => None,
            None => self
                .default_off
                .contains(&code)
                .then_some(Reason::DefaultOff),
        }
    }

    /// Steps 4 and 5 of [`apply`] for `code` alone — the per-code decision
    /// and the family gates — without the document gates or the directives.
    #[must_use]
    pub fn code_reason(&self, code: DiagCode) -> Option<Reason> {
        if let Some(reason) = self.decision_reason(code) {
            return Some(reason);
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

    /// Why a code with no finding at all is absent, in [`apply`]'s order for
    /// the steps that need no span: the document gates, abstention (unless
    /// the code survives it), the top-of-file directive (which names the code
    /// or `*`), then [`Self::code_reason`]. An inline directive is keyed by a
    /// line, so it cannot explain a whole code's absence. This is the reason
    /// a producer's declared skip is recorded with
    /// ([`Report::declare_skipped`], [`Report::declare_not_run`]).
    #[must_use]
    pub fn gap_reason(&self, code: DiagCode) -> Option<Reason> {
        self.document_reason(code)
            .or_else(|| self.directives.file_reason(code))
            .or_else(|| self.code_reason(code))
    }

    /// Every catalogued code the per-code decision turns off — a layer's
    /// `false`, or the default-off seed no layer turned on — whatever a
    /// producer then computes. What a surface reports as the codes the
    /// configuration disables (`getEffectiveConfig`, the INI export).
    #[must_use]
    pub fn disabled_codes(&self) -> BTreeSet<DiagCode> {
        DiagCode::ALL
            .iter()
            .copied()
            .filter(|code| self.decision_reason(*code).is_some())
            .collect()
    }

    /// The codes a producer may leave uncomputed: [`Self::disabled_codes`]
    /// less [`FACT_CODES`], which another producer reads as a fact. Rule 2's
    /// permitted saving (`docs/design/compiler/diagnostic-policy.md`
    /// § What the producers leave to the policy), the analyser's
    /// `with_disabled_diagnostics` set on every surface. A disabled fact code is computed and then
    /// suppressed, so its reason is recorded on the finding itself.
    ///
    /// The family gates are not in it. No producer that honours a skip emits
    /// an optimisation or a shimmer code — the analyser emits neither, and
    /// the compiler checks and the optimiser always run — so a family-gated
    /// code is never skipped, and declaring it skipped would explain a gap
    /// that does not exist. Nor are the directives: the analyser reads those
    /// itself ([`Self::analyser_skip`]), and a line-scoped one cannot skip a
    /// whole code.
    #[must_use]
    pub fn production_skip(&self) -> BTreeSet<DiagCode> {
        let mut skip = self.disabled_codes();
        for code in FACT_CODES {
            skip.remove(code);
        }
        skip
    }

    /// What the analyser leaves uncomputed under this policy:
    /// [`Self::production_skip`], which the surface hands it, plus every
    /// catalogued code the top-of-file directive names, because
    /// `Analyser::analyse` folds `parse_file_suppression` into its own
    /// disabled set. The directive's `*` names no code and skips nothing —
    /// the analyser compares codes exactly — so it is not in the set; the
    /// policy step hides those findings instead. This is the set a surface
    /// that ran the analyser declares ([`Report::declare_analyser_skip`]).
    #[must_use]
    pub fn analyser_skip(&self) -> BTreeSet<DiagCode> {
        let mut skip = self.production_skip();
        skip.extend(self.directives.file_codes());
        skip
    }
}

/// Builds one [`Policy`] from the configuration layers, resolved lowest
/// first in [`PolicyLayer`]'s order, plus the document facts.
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
    requested_profile: Option<OptimisationProfile>,
    default_profile: OptimisationProfile,
    overlaps: Vec<Overlap>,
    directives: Directives,
}

impl Default for PolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyBuilder {
    /// No layer, no directive, the dialect-independent overlaps, and the
    /// editor's default profile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            reporting: None,
            excluded: false,
            abstain: false,
            layers: Vec::new(),
            requested_profile: None,
            default_profile: DEFAULT_EDITOR_PROFILE,
            overlaps: base_overlaps(),
            directives: Directives::none(),
        }
    }

    /// Add a configuration layer — the `tclLsp` content shape
    /// [`crate::config_ini::settings_from_ini`] produces and the editor
    /// delivers (a `{"tclLsp": …}` wrapper and flat-dotted
    /// `tclLsp.<section>.<key>` keys are both read). In any order:
    /// [`Self::build`] resolves the layers in [`PolicyLayer`]'s order —
    /// global, then editor or invocation, then project — and two layers of
    /// one scope in the order they were added.
    #[must_use]
    pub fn layer(mut self, layer: PolicyLayer, settings: &Value) -> Self {
        self.layers.push((layer, settings.clone()));
        self
    }

    /// The optimiser profile the request itself names — `tcl opt
    /// --profile`, the MCP `optimize` tool's `profile`, the
    /// `tcl-lsp.optimiseDocument` command's argument. A named profile is in
    /// force over every layer's `optimiser.profile`, which applies only when
    /// the request names none: the profile is a request parameter with a
    /// project default, not a layered decision
    /// (`docs/design/compiler/diagnostic-policy.md` § Configuration). The
    /// master switch and the per-code overrides stay on the layers. `None`
    /// leaves the layers to decide.
    #[must_use]
    pub fn requested_profile(mut self, profile: Option<OptimisationProfile>) -> Self {
        self.requested_profile = profile;
        self
    }

    /// The profile in force when neither the request nor any layer names
    /// one — the surface's own default (`full` for `tcl opt` and the MCP
    /// `optimize` tool). Unset, it is the editor's default profile.
    #[must_use]
    pub fn default_profile(mut self, profile: OptimisationProfile) -> Self {
        self.default_profile = profile;
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

    /// Resolve the layers into one policy, lowest first in
    /// [`PolicyLayer`]'s order — the one place rule 5's order is applied, so
    /// no surface's push order can change a decision.
    #[must_use]
    pub fn build(mut self) -> Policy {
        let mut codes: BTreeMap<DiagCode, CodeDecision> = BTreeMap::new();
        let mut severity_overrides: BTreeMap<DiagCode, Severity> = BTreeMap::new();
        let mut optimiser_enabled: Option<Value> = None;
        let mut optimiser_profile: Option<Value> = None;
        let mut optimiser_overrides: BTreeMap<DiagCode, Value> = BTreeMap::new();
        let mut shimmer: Option<Value> = None;
        let mut reporting: Option<Value> = None;

        // Stable: two layers of one scope keep the order they were added in.
        self.layers.sort_by_key(|(layer, _)| *layer);
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

        let profile = self.requested_profile.unwrap_or_else(|| {
            optimiser_profile
                .as_ref()
                .and_then(Value::as_str)
                .map_or(self.default_profile, OptimisationProfile::parse)
        });
        let optimiser =
            OptimiserPolicy::resolved(profile, optimiser_enabled.as_ref(), &optimiser_overrides);

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
        is_owner
            && match self.scope {
                OverlapScope::SameSpan => owner.span == site,
                OverlapScope::WithinSpan => {
                    site.start() <= owner.span.start() && owner.span.end() <= site.end()
                }
                OverlapScope::Document => true,
            }
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
    policy
        .document_reason(finding.code)
        .or_else(|| policy.directives.reason_for(finding.code, finding.span))
        .or_else(|| policy.code_reason(finding.code))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod policy_tests;

#[cfg(test)]
mod apply_tests;
