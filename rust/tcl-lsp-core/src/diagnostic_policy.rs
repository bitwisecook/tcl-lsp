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
/// all render (`docs/design/lanes/diagnostic-policy.md` § Decisions taken,
/// D23).
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
    match &finding.data {
        Some(FindingData::Rewrite { group, .. }) => *group,
        _ => None,
    }
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
    /// `policy` gives for the code's absence ([`Policy::gap_reason`]). A code
    /// the policy would show is not recorded: there is nothing to explain a
    /// skip of it with, and a truth-table row over the producer is what
    /// catches that mismatch.
    pub fn declare_skipped(&mut self, codes: impl IntoIterator<Item = DiagCode>, policy: &Policy) {
        for code in codes {
            if let Some(reason) = policy.gap_reason(code) {
                self.skipped.insert(code, reason);
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
    /// optimisation code, each with the reason `policy` gives for its
    /// absence — [`Reason::OptimiserOff`] under a policy whose switch is off,
    /// unless an earlier step names another, as for every declared skip.
    /// What the diagnostics verbs and tools declare, which leave the
    /// rewrites to the rewrite surfaces: an O-code only the optimiser emits
    /// is then explained rather than read as clean, while one a compiler
    /// check or the O111 producer emitted keeps its finding — a code with a
    /// finding is no gap ([`Self::gaps`]).
    pub fn declare_optimiser_skip(&mut self, policy: &Policy) {
        self.declare_skipped(
            DiagCode::ALL
                .iter()
                .copied()
                .filter(|code| code.is_optimisation()),
            policy,
        );
    }

    /// The declared skips, by code.
    pub fn skipped(&self) -> impl Iterator<Item = (DiagCode, Reason)> + '_ {
        self.skipped.iter().map(|(code, reason)| (*code, *reason))
    }

    /// The declared skips whose code no finding in the report carries: the
    /// codes the policy turned off for this document that the report cannot
    /// show as a finding. A declared code some other producer did emit is
    /// not a gap — its findings carry their own reasons.
    pub fn gaps(&self) -> impl Iterator<Item = (DiagCode, Reason)> + '_ {
        self.skipped().filter(|(code, _)| {
            !self
                .outcomes
                .iter()
                .any(|(finding, _)| finding.code == *code)
        })
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
            let Some(FindingData::Rewrite {
                replacement,
                group,
                hint_only,
            }) = &finding.data
            else {
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
                Some(g) if whole.contains(g) => {
                    if let Some(&at) = group_at.get(g) {
                        out[at].members.push(finding);
                    } else {
                        group_at.insert(*g, out.len());
                        out.push(ApplicableRewrite {
                            group: Some(*g),
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
    /// shows — the rewrite loop's filter. Like [`Self::shown_items`], sound
    /// because [`apply`] is order-stable and keeps every finding.
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
            let Some(FindingData::Rewrite {
                group: Some(g),
                hint_only,
                ..
            }) = &finding.data
            else {
                continue;
            };
            seen.insert(*g);
            let shows = matches!(outcome, Outcome::Shown { .. });
            if !shows || (members == GroupMembers::Actionable && *hint_only) {
                broken.insert(*g);
            }
        }
        seen.difference(&broken).copied().collect()
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
    /// ([`Report::declare_skipped`]).
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
    /// § Producers that change), the analyser's `with_disabled_diagnostics`
    /// set on every surface. A disabled fact code is computed and then
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
    /// `tclLsp.<section>.<key>` keys are both read). Call lowest first:
    /// global, then editor or invocation, then project.
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

        let profile = self.requested_profile.unwrap_or_else(|| {
            optimiser_profile
                .as_ref()
                .and_then(Value::as_str)
                .map_or(self.default_profile, OptimisationProfile::parse)
        });
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

    #[test]
    fn every_reason_has_one_stable_spelling() {
        let cases: Vec<(Reason, &str)> = vec![
            (Reason::ReportingOff, "reporting-off"),
            (Reason::Excluded, "excluded"),
            (Reason::EncodingAbstention, "encoding-abstention"),
            (Reason::InlineDirective { line: 4 }, "inline-directive"),
            (Reason::FileDirective, "file-directive"),
            (Reason::Disabled(PolicyLayer::Editor), "disabled:editor"),
            (Reason::DefaultOff, "default-off"),
            (Reason::OptimiserOff, "optimiser-off"),
            (
                Reason::OptimiserProfile {
                    profile: OptimisationProfile::Readability,
                },
                "optimiser-profile:readability",
            ),
            (Reason::ShimmerOff, "shimmer-off"),
            (
                Reason::Overlap {
                    owner: OverlapOwner::Code(DiagCode::W110),
                },
                "overlap:W110",
            ),
            (
                Reason::Overlap {
                    owner: OverlapOwner::Producer(Producer::SslicTcl),
                },
                "overlap:sslictcl",
            ),
        ];
        for (reason, spelling) in cases {
            assert_eq!(reason.to_string(), spelling, "{reason:?}");
        }

        // `Disabled` and `Overlap` render one example above; every layer and
        // producer spelling in full here.
        for (layer, spelling) in [
            (PolicyLayer::Global, "global"),
            (PolicyLayer::Editor, "editor"),
            (PolicyLayer::Invocation, "invocation"),
            (PolicyLayer::Project, "project"),
        ] {
            assert_eq!(layer.as_str(), spelling);
            assert_eq!(
                Reason::Disabled(layer).to_string(),
                format!("disabled:{spelling}")
            );
        }
        for (producer, spelling) in [
            (Producer::Analyser, "analyser"),
            (Producer::CompilerCheck, "compiler-check"),
            (Producer::Optimiser, "optimiser"),
            (Producer::SourceStyle, "source-style"),
            (Producer::SourceDecode, "source-decode"),
            (Producer::SslicTcl, "sslictcl"),
            (Producer::Xc, "xc"),
            (Producer::BigipModel, "bigip-model"),
        ] {
            assert_eq!(producer.as_str(), spelling);
        }
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

    /// The owner's ruling: a profile the request names is the profile in
    /// force over every layer, the files' `optimiser.profile` is the default
    /// when it names none, and the switch and the per-code overrides keep
    /// the layer order (`docs/design/compiler/diagnostic-policy.md`
    /// § Configuration).
    #[test]
    fn a_requested_profile_is_in_force_over_every_layer() {
        let global = json!({"optimiser": {"profile": "aggressive"}});
        let invocation = json!({"optimiser": {"O114": false}});
        let project = json!({"optimiser": {"profile": "readability", "O109": true, "O114": true}});
        let layered = |requested: Option<OptimisationProfile>| {
            PolicyBuilder::new()
                .layer(PolicyLayer::Global, &global)
                .layer(PolicyLayer::Invocation, &invocation)
                .layer(PolicyLayer::Project, &project)
                .requested_profile(requested)
                .default_profile(OptimisationProfile::Full)
                .build()
        };
        let named = layered(Some(OptimisationProfile::Standard));
        assert_eq!(named.optimiser.profile, OptimisationProfile::Standard);
        assert!(
            !named.optimiser.disabled.contains(&DiagCode::O109),
            "the project's per-code `true` still applies over the named profile"
        );
        assert!(
            !named.optimiser.disabled.contains(&DiagCode::O114),
            "the project's per-code `true` still beats the invocation's `false`"
        );
        assert_eq!(
            layered(None).optimiser.profile,
            OptimisationProfile::Readability,
            "with none named, the project file supplies the profile"
        );

        let switched_off = PolicyBuilder::new()
            .layer(
                PolicyLayer::Project,
                &json!({"optimiser": {"enabled": false}}),
            )
            .requested_profile(Some(OptimisationProfile::Full))
            .build();
        assert!(
            !switched_off.optimiser.enabled,
            "the master switch stays on the layers"
        );

        let global_only = PolicyBuilder::new()
            .layer(
                PolicyLayer::Global,
                &json!({"optimiser": {"profile": "standard"}}),
            )
            .default_profile(OptimisationProfile::Full)
            .build();
        assert_eq!(
            global_only.optimiser.profile,
            OptimisationProfile::Standard,
            "the global file supplies the default too"
        );
        let unnamed = PolicyBuilder::new()
            .default_profile(OptimisationProfile::Full)
            .build();
        assert_eq!(
            unnamed.optimiser.profile,
            OptimisationProfile::Full,
            "the surface's own default when nothing names one"
        );
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
                scope: OverlapScope::WithinSpan,
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

    /// `Directives::hit` restates the bucket rule of the owner's
    /// `line_suppressed` rather than calling it, which needs a single-bucket
    /// predicate `tcl-compiler` does not offer (§ Decisions taken, D24).
    /// This keeps the two equal: an inline bucket or the file bucket, holding
    /// `*` or a code, silences a line code where `line_suppressed` says so,
    /// and a whole-file code only where the file bucket does.
    #[test]
    fn directives_agree_with_line_suppressed() {
        let text = "a\nb\nc\nd\n";
        let span_at = |line: u32| Span::new(line * 2, line * 2 + 1);
        let mut maps: Vec<HashMap<i32, HashSet<String>>> = Vec::new();
        let mut every: HashMap<i32, HashSet<String>> = HashMap::new();
        for key in [1, FILE_SUPPRESS_KEY] {
            for entry in ["*", "W210", "W112"] {
                maps.push(HashMap::from([(key, HashSet::from([entry.to_owned()]))]));
                every.entry(key).or_default().insert(entry.to_owned());
            }
        }
        maps.push(every);
        for map in maps {
            let directives = Directives::new(map.clone(), text);
            for code in [DiagCode::W210, DiagCode::W112, DiagCode::W118] {
                for line in 0..=3_u32 {
                    let key = if WHOLE_FILE_CODES.contains(&code) {
                        FILE_SUPPRESS_KEY
                    } else {
                        i32::try_from(line).expect("a small line")
                    };
                    assert_eq!(
                        directives.reason_for(code, span_at(line)).is_some(),
                        analyser::line_suppressed(code.as_str(), key, &map),
                        "{code} at line {line} under {map:?}"
                    );
                }
            }
        }
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

    /// A group's edits apply all-or-nothing (#2149): once a directive hides
    /// one member of an O127 pair, the other is neither offered nor applied —
    /// the inline without its delete runs the assignment twice.
    #[test]
    fn a_group_that_lost_a_member_is_neither_offered_nor_applied() {
        let text = "proc p {y} {\n    set x [llength $y]\n    # noqa\n    puts $x\n}\n";
        let at = |needle: &str| {
            let start = u32::try_from(text.find(needle).expect(needle)).unwrap();
            Span::new(start, start + u32::try_from(needle.len()).unwrap())
        };
        let mut delete = Optimisation::new(
            DiagCode::O127,
            "Inline the single-use assignment",
            at("set x [llength $y]"),
            "",
        );
        delete.group = Some(7);
        let mut inline = Optimisation::new(
            DiagCode::O127,
            "Inline the single-use assignment",
            at("$x"),
            "[llength $y]",
        );
        inline.group = Some(7);
        let findings = || vec![Finding::from(delete.clone()), Finding::from(inline.clone())];

        let mut lines: HashMap<i32, HashSet<String>> = HashMap::new();
        lines.insert(3, std::iter::once("*".to_owned()).collect());
        let directed = Policy {
            optimiser: OptimiserPolicy::all_on(),
            directives: Directives::new(lines, text),
            ..Policy::default()
        };
        let report = apply(findings(), &directed);
        assert_eq!(
            shown_codes(&report),
            vec![DiagCode::O127],
            "the directive hides the member on line 3 alone"
        );
        assert!(report.applicable_rewrites().is_empty());
        assert!(report.applicable_items(vec![0, 1]).is_empty());

        let plain = Policy {
            optimiser: OptimiserPolicy::all_on(),
            ..Policy::default()
        };
        let report = apply(findings(), &plain);
        let rewrites = report.applicable_rewrites();
        assert_eq!(rewrites.len(), 1, "{rewrites:?}");
        assert_eq!(rewrites[0].group, Some(7));
        assert_eq!(rewrites[0].members.len(), 2);
        assert_eq!(report.applicable_items(vec![0, 1]), vec![0, 1]);
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
    fn w110_owns_an_o120_whose_span_holds_it() {
        let w110 = analyser(DiagCode::W110, 10, 12);
        let o120_at = |start: u32, end: u32| {
            Finding::from(Optimisation::new(
                DiagCode::O120,
                "eq",
                Span::new(start, end),
                "eq",
            ))
        };
        // The analyser marks the operator, the optimiser the condition.
        let o120 = o120_at(4, 20);
        let same = o120_at(10, 12);
        let across = o120_at(11, 20);
        let elsewhere = o120_at(30, 32);
        let policy = Policy {
            optimiser: OptimiserPolicy::all_on(),
            ..Policy::default()
        };
        let owned = Some(Reason::Overlap {
            owner: OverlapOwner::Code(DiagCode::W110),
        });
        let report = apply(
            vec![w110.clone(), o120.clone(), same, across, elsewhere],
            &policy,
        );
        assert_eq!(report.reason_for(DiagCode::O120, Span::new(4, 20)), owned);
        assert_eq!(report.reason_for(DiagCode::O120, Span::new(10, 12)), owned);
        assert_eq!(
            report.reason_for(DiagCode::O120, Span::new(11, 20)),
            None,
            "W110 does not lie inside a span that starts after it"
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
    fn a_same_span_overlap_claims_only_the_span_it_shares() {
        let owner = analyser(DiagCode::W110, 10, 12);
        let superseded = |start: u32, end: u32| {
            Finding::from(Optimisation::new(
                DiagCode::O120,
                "eq",
                Span::new(start, end),
                "eq",
            ))
        };
        let policy = Policy {
            optimiser: OptimiserPolicy::all_on(),
            overlaps: vec![Overlap {
                owner: OverlapOwner::Code(DiagCode::W110),
                superseded: DiagCode::O120,
                scope: OverlapScope::SameSpan,
            }],
            ..Policy::default()
        };
        let report = apply(vec![owner, superseded(10, 12), superseded(4, 20)], &policy);
        assert_eq!(
            report.reason_for(DiagCode::O120, Span::new(10, 12)),
            Some(Reason::Overlap {
                owner: OverlapOwner::Code(DiagCode::W110)
            })
        );
        assert_eq!(
            report.reason_for(DiagCode::O120, Span::new(4, 20)),
            None,
            "a span that only holds the owner's is not the same span"
        );
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
    fn production_skip_is_the_per_code_decision_and_the_seed() {
        let mut policy = Policy::default();
        policy.codes.insert(
            DiagCode::W210,
            CodeDecision {
                enabled: false,
                layer: PolicyLayer::Global,
            },
        );
        policy.shimmer = false;
        let skip = policy.production_skip();
        assert!(skip.contains(&DiagCode::W210));
        assert!(
            skip.contains(&DiagCode::W242),
            "the default-off seed is skipped"
        );
        assert!(!skip.contains(&DiagCode::W100));
        // The family gates hide these codes, but no producer that honours a
        // skip emits them, so nothing is skipped on their account.
        assert_eq!(
            policy.code_reason(DiagCode::O107),
            Some(Reason::OptimiserProfile {
                profile: DEFAULT_EDITOR_PROFILE
            }),
            "the default readability profile hides O107"
        );
        assert!(!skip.contains(&DiagCode::O107));
        assert_eq!(policy.code_reason(DiagCode::S100), Some(Reason::ShimmerOff));
        assert!(!skip.contains(&DiagCode::S100));
    }

    #[test]
    fn production_skip_never_skips_a_fact_code() {
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &json!({ "diagnostics": { "W100": false, "W210": false } }),
            )
            .build();
        let skip = policy.production_skip();
        assert!(!skip.contains(&DiagCode::W100), "W100 is read as a fact");
        assert!(skip.contains(&DiagCode::W210));
        assert_eq!(
            policy.code_reason(DiagCode::W100),
            Some(Reason::Disabled(PolicyLayer::Editor)),
            "the policy step still hides it"
        );
        assert!(policy.disabled_codes().contains(&DiagCode::W100));
        // A top-of-file directive the analyser folds is the analyser's own
        // skip, fact code or not.
        let text = "# tcl-lsp: disable=W100\nputs ok\n";
        let directed = Policy {
            directives: Directives::scan(text, tcl9()),
            ..Policy::default()
        };
        assert!(directed.analyser_skip().contains(&DiagCode::W100));
    }

    #[test]
    fn a_gap_reason_follows_the_step_order() {
        let mut abstaining = Policy::default();
        abstaining.document.abstain = true;
        assert_eq!(
            abstaining.gap_reason(DiagCode::W210),
            Some(Reason::EncodingAbstention)
        );
        assert_eq!(
            abstaining.gap_reason(DiagCode::W109),
            None,
            "an integrity code survives abstention"
        );

        let text = "# tcl-lsp: disable=W210\nputs $y\n";
        let mut directed = Policy {
            directives: Directives::scan(text, tcl9()),
            ..Policy::default()
        };
        directed.codes.insert(
            DiagCode::W210,
            CodeDecision {
                enabled: false,
                layer: PolicyLayer::Global,
            },
        );
        assert_eq!(
            directed.gap_reason(DiagCode::W210),
            Some(Reason::FileDirective),
            "the file directive is step 3, ahead of the per-code decision"
        );
        assert_eq!(
            directed.gap_reason(DiagCode::W242),
            Some(Reason::DefaultOff)
        );

        directed.document.reporting = false;
        directed.document.abstain = true;
        assert_eq!(
            directed.gap_reason(DiagCode::W210),
            Some(Reason::ReportingOff),
            "reporting off answers before everything"
        );
    }

    #[test]
    fn the_analyser_skip_adds_the_codes_the_file_directive_names() {
        let text = "# tcl-lsp: disable=W210, *\nputs $y\n";
        let policy = Policy {
            directives: Directives::scan(text, tcl9()),
            ..Policy::default()
        };
        let production = policy.production_skip();
        let analyser = policy.analyser_skip();
        assert!(!production.contains(&DiagCode::W210));
        assert!(analyser.contains(&DiagCode::W210));
        let mut expected = production;
        expected.insert(DiagCode::W210);
        assert_eq!(
            analyser, expected,
            "`*` names no code, so it adds nothing to the skip"
        );
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

    /// A checks-emitted code can carry both a `Disabled` finding and a
    /// declared skip (`Policy::production_skip` declares every catalogued
    /// code the decision turns off, whichever producer emits it) —
    /// `gaps()` renders only the code no finding explains.
    #[test]
    fn a_gap_is_a_declared_skip_no_finding_explains() {
        let mut policy = Policy::default();
        for code in [DiagCode::W210, DiagCode::T100] {
            policy.codes.insert(
                code,
                CodeDecision {
                    enabled: false,
                    layer: PolicyLayer::Project,
                },
            );
        }
        let mut report = apply(
            vec![finding(
                DiagCode::T100,
                Span::new(0, 1),
                Producer::CompilerCheck,
            )],
            &policy,
        );
        report.declare_skipped([DiagCode::W210, DiagCode::T100], &policy);
        let gaps: Vec<DiagCode> = report.gaps().map(|(code, _)| code).collect();
        assert_eq!(gaps, vec![DiagCode::W210]);
    }
}
