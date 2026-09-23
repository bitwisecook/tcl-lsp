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

//! One document's findings under one policy — the function every surface
//! calls — and the rewrite loop that applies only what that policy shows.
//!
//! `docs/design/compiler/diagnostic-policy.md` § Adapters: an adapter reads
//! one [`Report`] and renders it, and none of them decides anything. The
//! producers a surface runs itself — the analyser, the compiler checks, the
//! optimiser, the XC walk, the BIG-IP model — arrive here already converted
//! to [`Finding`]s; the producers this crate owns — the source-style pass,
//! the byte-integrity pass and the `SslicTcl` projection — run here, so no
//! surface can forget one of them. What comes back is the report: every
//! finding, shown or suppressed, with its reason.
//!
//! The O111 brace-expression hint is a producer over the analyser's W100
//! findings, [`brace_expr_hints`]: a surface adds its findings right after
//! the analyser's, and the policy step decides W100 and O111 each on its own.
//!
//! A surface without the salsa database — `tcl diag`, the MCP tools — runs
//! the analyser, the O111 producer and the compiler checks through
//! [`standalone_findings`], so those surfaces run one producer set rather
//! than each assembling its own.

use std::collections::BTreeSet;
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, bidi_control_diagnostics};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::{Optimisation, apply_optimisations, optimise_with_dialect};
use tcl_compiler::unit_scope::CallSiteEvidence;
use tcl_core_types::{DiagCode, Severity};
use tcl_dialect::DialectProfile;
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_registry::CommandRegistry;

use crate::diagnostic_policy::{Directives, Finding, Policy, Producer, Report, apply};
use crate::source_decode::{DecodeReport, encoding_integrity_diagnostics};
use crate::source_style::{DEFAULT_LINE_ENDING, style_diagnostics};
use crate::sslictcl_diagnostics;

/// Which source-text producers run for a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePass {
    /// A Tcl document: the style pass (W111 line length, W112 trailing
    /// whitespace, W115 comment continuation, W118 line endings) with the
    /// byte-integrity checks (W107, W109). `line_length` is the W111
    /// threshold.
    Tcl {
        /// `tclLsp.style.lineLength`.
        line_length: usize,
    },
    /// A non-Tcl F5 model document (BIG-IP configuration, iApp APL), which
    /// never runs the Tcl analyser: only the integrity codes and W305, which
    /// apply to every text language. The style lints have separate
    /// syntax-policy questions and stay out.
    IntegrityOnly,
}

/// The document a report is about.
#[derive(Debug, Clone, Copy)]
pub struct DocumentSource<'a> {
    /// The client's exact buffer — what W118 reads, and what every span is
    /// lifted against.
    pub text: &'a str,
    /// The analysis form (a lone `\r` rewritten to `\n`), the same length as
    /// `text` — what the `SslicTcl` loader reads.
    pub analysis_text: &'a str,
    /// The byte decoder's report, when the surface has bytes.
    pub decode: Option<&'a DecodeReport>,
    /// The document's dialect.
    pub dialect: &'static DialectProfile,
    /// Which source-text producers run.
    pub pass: SourcePass,
}

/// The report for one document: `produced` — the findings the surface's own
/// producers emitted, in the order it wants them rendered — plus the
/// source-text and `SslicTcl` producers this crate owns, under `policy`.
///
/// Every step that hides or relabels a finding happens inside
/// [`apply`]; nothing here filters.
#[must_use]
pub fn document_report(
    doc: &DocumentSource<'_>,
    produced: Vec<Finding>,
    policy: &Policy,
) -> Report {
    let mut findings = produced;
    let line_index = LineIndex::new_lsp(doc.text);
    match doc.pass {
        SourcePass::Tcl { line_length } => {
            findings.extend(
                style_diagnostics(
                    doc.text,
                    line_length,
                    DEFAULT_LINE_ENDING,
                    doc.decode,
                    doc.dialect,
                )
                .into_iter()
                .map(|d| Finding::from_style(d, doc.text, &line_index)),
            );
        }
        SourcePass::IntegrityOnly => {
            findings.extend(
                encoding_integrity_diagnostics(doc.text, doc.decode)
                    .into_iter()
                    .map(|d| Finding::from_style(d, doc.text, &line_index)),
            );
            findings.extend(
                bidi_control_diagnostics(doc.text)
                    .into_iter()
                    .map(Finding::from),
            );
        }
    }
    if sslictcl_diagnostics::applies_to(doc.dialect) {
        findings.extend(sslictcl_diagnostics::diagnostics(doc.analysis_text));
    }
    apply(findings, policy)
}

/// One document as a surface without the salsa database analyses it.
#[derive(Debug, Clone, Copy)]
pub struct StandaloneDocument<'a> {
    /// The analysis form of the text (lone `\r` rewritten).
    pub source: &'a str,
    /// The document's path, for the analyser's file-scoped facts.
    pub file_path: Option<&'a str>,
    /// The document's dialect.
    pub dialect: &'static DialectProfile,
    /// The registry the surface resolved for the dialect.
    pub registry: &'a CommandRegistry,
    /// `Analyser::with_pack_overlay`'s key.
    pub pack_overlay: u64,
    /// Cross-file call-site evidence, when the surface gathered any.
    pub external_call_sites: Option<&'a CallSiteEvidence>,
}

/// What [`standalone_findings`] produced.
#[derive(Debug)]
pub struct StandaloneFindings {
    /// The analysis, whose directive map feeds the policy.
    pub analysis: AnalysisResult,
    /// The analyser's findings, the O111 hints over them
    /// ([`brace_expr_hints`]), then the compiler checks', converted.
    pub produced: Vec<Finding>,
}

/// The analyser under `skip` — the policy's production skip — the O111
/// producer over its findings, and the compiler checks, over one
/// compilation unit.
///
/// The producers a surface without the salsa database runs itself, once,
/// so `tcl diag` and the MCP diagnostics tools run the same ones. It reads
/// no policy (rule 2 of `docs/design/compiler/diagnostic-policy.md`): the
/// skip is a producer input, as the analyser takes it, and the caller hands
/// the findings to [`document_report`] and declares the skip there
/// ([`Report::declare_analyser_skip`]).
///
/// One unit serves both producers, built with the document's own
/// environment grammar, the caller's cross-file evidence and the document's
/// stub declarations: the analyser's CFG/SSA tail would otherwise build its
/// own unit without the evidence, and its constant-branch findings (I230,
/// I231) would disagree with the checks'. A stubbed command's argument roles
/// reach both producers for the same reason.
#[must_use]
pub fn standalone_findings(
    doc: &StandaloneDocument<'_>,
    skip: &BTreeSet<DiagCode>,
) -> StandaloneFindings {
    let declared = tcl_compiler::analyser::utils::document_declared_surface(
        doc.source,
        doc.file_path,
        doc.dialect.name,
    );
    let unit = Arc::new(CompilationUnit::build_with_options(
        doc.source,
        UnitBuildOptions {
            registry: doc.registry,
            defer_top_level: false,
            config: LexerConfig::for_profile(Some(doc.dialect)),
            dialect: Some(doc.dialect),
            external_call_sites: doc.external_call_sites,
            declared_commands: Some(&declared),
        },
    ));
    let mut analyser =
        Analyser::with_disabled_diagnostics(skip.iter().map(ToString::to_string).collect())
            .with_file_path(doc.file_path.map(str::to_owned))
            .with_pack_overlay(doc.pack_overlay);
    analyser.set_cu_override(Arc::clone(&unit));
    let analysis = analyser.analyse(doc.source, doc.dialect.name);
    let mut produced: Vec<Finding> = analysis
        .diagnostics
        .iter()
        .cloned()
        .map(Finding::from)
        .collect();
    let hints = brace_expr_hints(&produced);
    produced.extend(hints);
    produced.extend(
        run_all_checks(unit.as_ref(), doc.registry, Some(doc.dialect))
            .into_iter()
            .map(Finding::from),
    );
    StandaloneFindings { analysis, produced }
}

/// The O111 producer over the unbraced-expression fact: one finding at the
/// span of every W100 the analyser emitted, whatever policy later decides
/// for either.
///
/// `docs/design/compiler/diagnostic-policy.md` § Producers that change: W100
/// and O111 consume the same fact, and the policy step decides each on its
/// own — a layer that turns W100 off or a `# noqa: W100` leaves O111
/// standing, and the optimiser's switch and profile reach O111 where they
/// reach every other O-code. It reads no policy. W100 is a fact code
/// ([`crate::diagnostic_policy::FACT_CODES`]), so no layer's skip takes the
/// input away; a `# tcl-lsp: disable=W100` still does, because the analyser
/// folds the file directive into its own skip.
#[must_use]
pub fn brace_expr_hints(produced: &[Finding]) -> Vec<Finding> {
    produced
        .iter()
        .filter(|f| f.producer == Producer::Analyser && f.code == DiagCode::W100)
        .map(|w100| Finding {
            code: DiagCode::O111,
            span: w100.span,
            severity: Severity::Info,
            message: BRACE_EXPR_HINT.to_owned(),
            fixes: Vec::new(),
            data: None,
            producer: Producer::Optimiser,
        })
        .collect()
}

/// The O111 message.
const BRACE_EXPR_HINT: &str = "Brace expression text (for example, `expr {...}` / `if {...}`) to pass \
                               a single static argument, enabling bytecode compilation and avoiding \
                               per-evaluation substitution/parsing overhead.";

/// What [`optimise_under_policy`] produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimisedSource {
    /// The rewritten text.
    pub text: String,
    /// The rewrites applied, every pass in order — the shown findings'
    /// records, so a rewrite whose finding was suppressed is not here.
    pub applied: Vec<Optimisation>,
    /// One per pass attempted, including the final pass that found nothing.
    pub iterations: usize,
}

/// Iteratively optimise `source` until a fixpoint or `max_iterations` is
/// reached, applying on every pass only the rewrites `policy` shows — and a
/// grouped rewrite only with every member of its group.
///
/// The optimiser's rewrites are findings like any other: a `# noqa` on the
/// command, a file-wide `# tcl-lsp: disable=*`, a code the profile or a
/// configuration layer turned off — each means to a rewrite what it means to
/// a squiggle. The directives are the analyser's own map over the current
/// text, read afresh on every pass: an applied rewrite moves the lines the
/// next pass's directives attach to, and the analyser attributes a `# noqa`
/// to every line of the command it precedes, which is what the editor's
/// squiggles are decided under (#2119). A single-pass profile is
/// `max_iterations == 1`.
///
/// This is the shared loop behind `tcl opt`, the MCP `optimize` tool and
/// the server's `tcl-lsp.optimiseDocument` command.
#[must_use]
pub fn optimise_under_policy(
    source: &str,
    registry: &CommandRegistry,
    dialect: Option<&'static DialectProfile>,
    max_iterations: usize,
    policy: &Policy,
) -> OptimisedSource {
    let directive_dialect = dialect.map_or("", |d| d.name);
    let mut current = source.to_owned();
    let mut applied: Vec<Optimisation> = Vec::new();
    let mut iterations = 0;
    for _ in 0..max_iterations {
        iterations += 1;
        let opts = optimise_with_dialect(&current, registry, dialect);
        let mut pass_policy = policy.clone();
        pass_policy.directives = Directives::from_analysis(
            &Analyser::new().analyse(&current, directive_dialect),
            &current,
        );
        let report = apply(
            opts.iter().cloned().map(Finding::from).collect(),
            &pass_policy,
        );
        // A group applies whole or not at all: a directive over one member
        // of an O127 pair keeps the other off too (#2149).
        let kept = report.applicable_items(opts);
        if kept.is_empty() {
            break;
        }
        let next = apply_optimisations(&current, &kept);
        applied.extend(kept);
        if next == current {
            break;
        }
        current = next;
    }
    OptimisedSource {
        text: current,
        applied,
        iterations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_policy::{OptimiserPolicy, Outcome, PolicyBuilder, PolicyLayer, Reason};
    use tcl_lexer::Span;

    fn tcl9() -> &'static DialectProfile {
        crate::profile_for_dialect("tcl9.0")
    }

    fn tcl_doc(text: &str) -> DocumentSource<'_> {
        DocumentSource {
            text,
            analysis_text: text,
            decode: None,
            dialect: tcl9(),
            pass: SourcePass::Tcl { line_length: 120 },
        }
    }

    fn codes(report: &Report) -> Vec<DiagCode> {
        report.shown().map(|s| s.finding.code).collect()
    }

    #[test]
    fn the_style_pass_and_the_produced_findings_share_one_report() {
        let text = "set x 1   \nputs $y\n";
        let analysis = Analyser::new().analyse(text, "tcl9.0");
        let produced: Vec<Finding> = analysis
            .diagnostics
            .iter()
            .cloned()
            .map(Finding::from)
            .collect();
        let policy = PolicyBuilder::new()
            .directives(Directives::from_analysis(&analysis, text))
            .build();
        let report = document_report(&tcl_doc(text), produced, &policy);
        let shown = codes(&report);
        assert!(shown.contains(&DiagCode::W112), "{shown:?}");
        assert!(shown.contains(&DiagCode::W210), "{shown:?}");
    }

    /// The analyser folds the top-of-file directive into its own skip, so a
    /// code the directive names is never computed; the declared skip is what
    /// explains its absence.
    #[test]
    fn a_file_directive_gap_is_declared() {
        let text = "# tcl-lsp: disable=W210\nputs $y\n";
        let base = PolicyBuilder::new().build();
        let skip = base
            .production_skip()
            .iter()
            .map(ToString::to_string)
            .collect();
        let analysis = Analyser::with_disabled_diagnostics(skip).analyse(text, "tcl9.0");
        let policy = PolicyBuilder::new()
            .directives(Directives::from_analysis(&analysis, text))
            .build();
        let produced: Vec<Finding> = analysis
            .diagnostics
            .iter()
            .cloned()
            .map(Finding::from)
            .collect();
        let mut report = document_report(&tcl_doc(text), produced, &policy);
        assert!(
            report.iter().all(|(f, _)| f.code != DiagCode::W210),
            "the analyser never computed W210: {report:?}"
        );
        assert_eq!(report.reason_for(DiagCode::W210, Span::new(0, 0)), None);
        report.declare_analyser_skip(&policy);
        assert_eq!(
            report.reason_for(DiagCode::W210, Span::new(0, 0)),
            Some(Reason::FileDirective)
        );
    }

    /// The byte-integrity finding sits at the decoder's U+FFFD in a file whose
    /// lines end with a lone `\r`: the pass positions it on the client's line
    /// model, so the conversion back to a byte span lands where it started.
    #[test]
    fn w107_sits_at_the_replacement_character_in_a_lone_cr_file() {
        let (text, decode) =
            crate::source_decode::decode_source(b"set a 1\rset b 2\rputs \"\xff bad\"\n");
        let analysis_text = tcl_lexer::normalise_lone_cr(&text);
        let doc = DocumentSource {
            text: &text,
            analysis_text: &analysis_text,
            decode: Some(&decode),
            dialect: tcl9(),
            pass: SourcePass::Tcl { line_length: 120 },
        };
        let report = document_report(&doc, Vec::new(), &PolicyBuilder::new().build());
        let at = u32::try_from(text.find('\u{fffd}').expect("a replacement character"))
            .expect("offset fits");
        let w107 = report
            .iter()
            .find(|(f, _)| f.code == DiagCode::W107)
            .map(|(f, _)| f.span);
        assert_eq!(
            w107,
            Some(Span::new(at, at + 3)),
            "W107 covers the U+FFFD, not the end of the first line: {report:?}"
        );
    }

    #[test]
    fn a_file_directive_reaches_the_style_pass_through_the_report() {
        let text = "# tcl-lsp: disable=W112\nset x 1   \n";
        let policy = PolicyBuilder::new()
            .directives(Directives::scan(text, tcl9()))
            .build();
        let report = document_report(&tcl_doc(text), Vec::new(), &policy);
        assert!(codes(&report).is_empty());
        let w112 = report
            .iter()
            .find(|(f, _)| f.code == DiagCode::W112)
            .map(|(_, o)| *o);
        assert_eq!(w112, Some(Outcome::Suppressed(Reason::FileDirective)));
    }

    #[test]
    fn an_integrity_only_document_carries_the_bidi_finding() {
        let text = "ltm rule /Common/r {\n # \u{202e}hidden\n}\n";
        let doc = DocumentSource {
            pass: SourcePass::IntegrityOnly,
            ..tcl_doc(text)
        };
        let report = document_report(&doc, Vec::new(), &PolicyBuilder::new().build());
        assert_eq!(codes(&report), [DiagCode::W305]);
    }

    #[test]
    fn the_sslictcl_projection_runs_for_its_dialect_only() {
        let text = "SSLIC::unknown_thing\n";
        let sslic = DocumentSource {
            dialect: crate::profile_for_dialect("sslictcl"),
            ..tcl_doc(text)
        };
        let with = document_report(&sslic, Vec::new(), &PolicyBuilder::new().build());
        let without = document_report(&tcl_doc(text), Vec::new(), &PolicyBuilder::new().build());
        assert!(
            with.iter().any(|(f, _)| f.producer == Producer::SslicTcl),
            "{with:?}"
        );
        assert!(
            without
                .iter()
                .all(|(f, _)| f.producer != Producer::SslicTcl)
        );
    }

    #[test]
    fn the_brace_expr_hint_follows_every_w100_the_analyser_finds() {
        // The analyser's findings, the hints over them, and the directive map
        // the analyser read.
        let produce = |text: &str| {
            let analysis = Analyser::new().analyse(text, "tcl9.0");
            let mut produced: Vec<Finding> = analysis
                .diagnostics
                .iter()
                .cloned()
                .map(Finding::from)
                .collect();
            let hints = brace_expr_hints(&produced);
            produced.extend(hints);
            (produced, Directives::from_analysis(&analysis, text))
        };
        let spans = |report: &Report, code: DiagCode| -> Vec<Span> {
            report
                .iter()
                .filter(|(f, _)| f.code == code)
                .map(|(f, _)| f.span)
                .collect()
        };
        let decided = |layer: serde_json::Value, directives: Directives| {
            let mut policy = PolicyBuilder::new()
                .layer(PolicyLayer::Editor, &layer)
                .directives(directives)
                .build();
            policy.optimiser = OptimiserPolicy::all_on();
            policy
        };

        let text = "set a 1\nif [expr $a + 1] { puts x }\n";
        let (produced, directives) = produce(text);
        let hints: Vec<&Finding> = produced
            .iter()
            .filter(|f| f.code == DiagCode::O111)
            .collect();
        assert!(!hints.is_empty(), "the fixture must carry a W100");
        for hint in &hints {
            assert_eq!(hint.producer, Producer::Optimiser);
            assert_eq!(hint.severity, Severity::Info);
            assert_eq!(hint.message, BRACE_EXPR_HINT);
        }
        let shown = apply(
            produced.clone(),
            &decided(serde_json::json!({}), directives.clone()),
        );
        assert_eq!(spans(&shown, DiagCode::O111), spans(&shown, DiagCode::W100));
        let w100 = spans(&shown, DiagCode::W100)[0];
        let o111_shows = |report: &Report, at: Span| {
            matches!(
                report.outcome_for(DiagCode::O111, at),
                Some(Outcome::Shown { .. })
            )
        };
        assert!(o111_shows(&shown, w100), "{shown:?}");

        // A layer that turns W100 off leaves O111 standing.
        let off = apply(
            produced.clone(),
            &decided(
                serde_json::json!({ "diagnostics": { "W100": false } }),
                directives.clone(),
            ),
        );
        assert_eq!(
            off.reason_for(DiagCode::W100, w100),
            Some(Reason::Disabled(PolicyLayer::Editor))
        );
        assert!(o111_shows(&off, w100), "{off:?}");

        // So does a `# noqa: W100` over the command.
        let marked = "set a 1\n# noqa: W100\nif [expr $a + 1] { puts x }\n";
        let (marked_produced, marked_directives) = produce(marked);
        let noqa = apply(
            marked_produced,
            &decided(serde_json::json!({}), marked_directives),
        );
        let marked_w100 = spans(&noqa, DiagCode::W100)[0];
        assert!(
            matches!(
                noqa.reason_for(DiagCode::W100, marked_w100),
                Some(Reason::InlineDirective { .. })
            ),
            "{noqa:?}"
        );
        assert!(o111_shows(&noqa, marked_w100), "{noqa:?}");

        // The optimiser's switch reaches O111 as it reaches every O-code.
        let mut switched_off = decided(serde_json::json!({}), directives);
        switched_off.optimiser.enabled = false;
        let gated = apply(produced, &switched_off);
        assert_eq!(
            gated.reason_for(DiagCode::O111, w100),
            Some(Reason::OptimiserOff)
        );
        assert!(
            matches!(
                gated.outcome_for(DiagCode::W100, w100),
                Some(Outcome::Shown { .. })
            ),
            "{gated:?}"
        );
    }

    #[test]
    fn optimise_under_policy_skips_a_rewrite_a_directive_silences() {
        let registry = CommandRegistry::build_default();
        let dialect = Some(tcl9());
        let mut policy = PolicyBuilder::new().build();
        policy.optimiser = OptimiserPolicy::all_on();
        let plain = "set x [expr {1 + 2}]\n";
        let folded = optimise_under_policy(plain, &registry, dialect, 1, &policy);
        assert!(folded.text.contains("set x 3"), "{}", folded.text);
        assert!(folded.applied.iter().any(|o| o.code == DiagCode::O101));

        let marked = "# noqa: O101\nset x [expr {1 + 2}]\n";
        let kept = optimise_under_policy(marked, &registry, dialect, 1, &policy);
        assert_eq!(
            kept.text, marked,
            "a `# noqa` on the command keeps the rewrite off"
        );
        assert!(kept.applied.is_empty());

        let whole = "# tcl-lsp: disable=*\nset x [expr {1 + 2}]\n";
        let untouched = optimise_under_policy(whole, &registry, dialect, 5, &policy);
        assert_eq!(untouched.text, whole);

        let mut off = policy.clone();
        off.optimiser.enabled = false;
        let switched = optimise_under_policy(plain, &registry, dialect, 5, &off);
        assert_eq!(switched.text, plain, "the master switch reaches a rewrite");
    }

    /// A `# noqa` covers every line of the command it precedes, as the
    /// analyser's map — the one the editor's squiggles are decided under —
    /// attributes it, not only the next line.
    #[test]
    fn a_noqa_reaches_every_line_of_the_command_it_precedes() {
        let registry = CommandRegistry::build_default();
        let dialect = Some(tcl9());
        let mut policy = PolicyBuilder::new().build();
        policy.optimiser = OptimiserPolicy::all_on();
        let marked = "# noqa: O101\nproc p {} {\n    return [expr {1 + 2}]\n}\nputs [p]\n";
        let out = optimise_under_policy(marked, &registry, dialect, 1, &policy);
        assert!(out.text.contains("return [expr {1 + 2}]"), "{}", out.text);
        let plain = "proc p {} {\n    return [expr {1 + 2}]\n}\nputs [p]\n";
        let out = optimise_under_policy(plain, &registry, dialect, 1, &policy);
        assert!(out.text.contains("return 3"), "{}", out.text);
    }

    /// A line-keyed `# noqa` over one member of an O127 pair must not leave
    /// the other applicable: the delete without its inline removes a store
    /// the use site still reads (#2149).
    #[test]
    fn optimise_under_policy_never_applies_half_a_group() {
        let registry = CommandRegistry::build_default();
        let dialect = Some(crate::profile_for_dialect("tcl8.6"));
        let mut policy = PolicyBuilder::new().build();
        policy.optimiser = OptimiserPolicy::all_on();
        let marked = "proc p {y} {\n    set x [llength $y]\n    # noqa\n    puts $x\n}\n";
        let out = optimise_under_policy(marked, &registry, dialect, 1, &policy);
        assert!(out.text.contains("set x [llength $y]"), "{}", out.text);
        assert!(
            !out.applied.iter().any(|o| o.code == DiagCode::O127),
            "{:?}",
            out.applied
        );
        // Positive control: without the directive the pair applies whole.
        let plain = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
        let out = optimise_under_policy(plain, &registry, dialect, 1, &policy);
        assert_eq!(
            out.applied
                .iter()
                .filter(|o| o.code == DiagCode::O127)
                .count(),
            2,
            "{:?}",
            out.applied
        );
    }
}
