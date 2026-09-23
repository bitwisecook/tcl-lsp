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

//! Diagnostic verbs: `diag` / `lint` (identical) and `validate`.
//!
//! Drive the analyser in `tcl-compiler`. Unlike the transform verbs, these
//! analyse each input document separately (a per-file loop), each under its
//! own policy (`docs/design/compiler/diagnostic-policy.md` § Adapters): the
//! rows are the report's shown findings, and this verb decides nothing.

use std::collections::HashSet;

use serde::Serialize;
use tcl_cli_support::{
    InputDocument, OutputTarget, read_input_documents, registry_for_dialect, write_text_output,
};
use tcl_compiler::analyser::Severity;
use tcl_compiler::unit_scope::CallSiteEvidence;
use tcl_lexer::LineIndex;
use tcl_lsp_core::diagnostic_policy::{Directives, Policy, PolicyBuilder, Report};
use tcl_lsp_core::diagnostic_report::{
    DocumentSource, SourcePass, StandaloneDocument, document_report, standalone_findings,
};
use tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH;

use crate::cli::{DiagArgs, InputArgs};
use crate::commands::policy::{ConfigLayers, invocation_layer};

/// One diagnostic in the `diag` report (fields are emitted in a fixed order).
#[derive(Serialize)]
struct DiagItem {
    line: u32,
    column: u32,
    severity: &'static str,
    code: String,
    message: String,
}

/// Per-file diagnostic report entry.
#[derive(Serialize)]
struct FileReport {
    file: String,
    diagnostics: Vec<DiagItem>,
}

/// One error in the `validate` JSON payload (carries its file).
#[derive(Serialize)]
struct ValidateError {
    file: String,
    line: u32,
    column: u32,
    severity: &'static str,
    code: String,
    message: String,
}

/// `validate --json` payload.
#[derive(Serialize)]
struct ValidatePayload {
    ok: bool,
    inputs: usize,
    error_count: usize,
    errors: Vec<ValidateError>,
}

/// Map a severity to its lowercase name (`Severity.name.lower()`)
/// vocabulary (`error` / `warning` / `info` / `hint`). The `Suggestion`
/// tier corresponds to the `info` severity.
fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info | Severity::Suggestion => "info",
        Severity::Hint => "hint",
    }
}

fn is_problem(severity: Severity) -> bool {
    matches!(severity, Severity::Error | Severity::Warning)
}

/// Render one diagnostic text line (the
/// inline `diag` format): `label:line:col: severity<7> code<8> message`.
fn format_line(
    file: &str,
    line: u32,
    column: u32,
    severity: &str,
    code: &str,
    message: &str,
) -> String {
    let code = if code.is_empty() { "-" } else { code };
    format!("{file}:{line}:{column}: {severity:<7} {code:<8} {message}")
}

/// One collected diagnostic, pre-resolved to a 1-based line / column.
struct Row {
    line: u32,
    column: u32,
    severity: Severity,
    code: String,
    message: String,
}

/// Cross-file call-site evidence across every input document, plus the
/// project-wide procedure-name set the scan resolved against.
///
/// `tcl diag a.tcl b.tcl` is a multi-file compilation just as much as the
/// editor's workspace is: without this, `a.tcl` would fold a parameter that
/// `b.tcl` calls with a different literal.  `None` for a single input — one
/// file is not a project, and asserting a closed world from it would be
/// wrong.
fn cross_file_call_site_evidence(
    documents: &[InputDocument],
    dialect_override: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<CallSiteEvidence> {
    if documents.len() < 2 {
        return None;
    }
    let mut known: HashSet<String> = HashSet::new();
    for document in documents {
        let dialect = document.effective_dialect(dialect_override);
        known.extend(document_proc_names(document, dialect));
    }
    // Naming several files on one command line asserts they are one program,
    // so an unenumerable dispatch in any of them may reach any of their
    // procedures.  The editor derives the same bound from the `source` graph
    // (`tcl_lsp_db::file_dispatch_reach`); here the user has stated it.
    let mut reach: Vec<String> = known.iter().cloned().collect();
    reach.sort();
    let mut merged = CallSiteEvidence::default();
    for document in documents {
        let dialect = document.effective_dialect(dialect_override);
        let source = document.analysis_source();
        let file_path = document.path.as_deref().map(|p| p.display().to_string());
        // Each file's call sites are read on its *own* command surface: a
        // stub declared there names the callbacks and script bodies this scan
        // must credit, exactly as the catalogue does for a shipped command.
        let declared = tcl_compiler::analyser::utils::document_declared_surface(
            &source,
            file_path.as_deref(),
            dialect.name,
        );
        merged.merge_from(&tcl_compiler::unit_scope::scan_source_call_sites(
            &source,
            &registry_for_dialect(dialect.name),
            Some(&declared),
            dialect,
            &known,
            &reach,
        ));
    }
    Some(merged)
}

/// The qualified names of every procedure `document` declares, from the same
/// [`tcl_compiler::signature_scan`] the analyser and the LSP index use.
fn document_proc_names(
    document: &InputDocument,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<String> {
    tcl_compiler::signature_scan::extract_signatures(
        &document.analysis_source(),
        &registry_for_dialect(dialect.name),
    )
    .procs
    .into_keys()
    .collect()
}

/// Every diagnostic the editor surfaces for one document, as rows: the
/// analyser's syntactic / semantic checks, the compiler-checks pass (shimmer
/// `S1xx`, taint `T1xx` / `W2xx`, iRules data-flow) and — the report's own —
/// the source-text pass (`W111` line length, `W112` trailing whitespace,
/// `W115` comment continuation, `W118` line endings, plus the byte-backed
/// `W107` / `W109` integrity checks) and, for a `.sslictcl` document, the
/// loader's `SSLIC1xxx` findings; all under the document's policy from
/// `layers` and its own directives. The same producer set and the same
/// policy step the server's publish paths take, so the CLI and the editor
/// report the same set. Optimiser `O1xx` rewrites are the domain of the
/// `optimise` verb — see [`diag_policy`]. Rows come back in a deterministic
/// `(line, column, code)` order.
fn collect_rows(
    document: &InputDocument,
    dialect: &'static tcl_dialect::DialectProfile,
    layers: &ConfigLayers,
    external_call_sites: Option<&CallSiteEvidence>,
) -> Vec<Row> {
    // The *analysis* form of the document, not the bytes on disk — see
    // `InputDocument::analysis_source`. `LineIndex` is built over it too:
    // `LineIndex::new(normalise_lone_cr(t))` is byte-identical to
    // `LineIndex::new_lsp(t)`, so the lexer's line model and the client's
    // coincide, and a span from either text resolves to the same position.
    let source = document.analysis_source();
    let source = source.as_ref();
    let line_index = LineIndex::new(source);
    let base = layers
        .builder_for(document.path.as_deref())
        .decode(Some(&document.decode))
        .dialect(dialect);

    // A document whose bytes are not UTF-8 text: everything derived from the
    // decoded text would be about decoding artefacts rather than about the
    // user's code, pointing at positions the file does not have, so the
    // analyser never runs. The report carries what the bytes themselves
    // justify — the integrity codes and W305, the codes the editor's
    // abstention keeps — under the directives scanned from the text.
    if document.abstains_on_encoding() {
        let policy = diag_policy(base.directives(Directives::scan(&document.source, dialect)));
        let doc = DocumentSource {
            text: &document.source,
            analysis_text: source,
            decode: Some(&document.decode),
            dialect,
            pass: SourcePass::IntegrityOnly,
        };
        return rows_of(
            &document_report(&doc, Vec::new(), &policy),
            source,
            &line_index,
        );
    }

    // The analyser's production-time skip: the codes the policy hides by a
    // configuration layer or the default-off seed, which it need not compute
    // (`docs/design/compiler/diagnostic-policy.md` § Producers that change)
    // — the same seeded set the editor's `file_analysis` passes. Declared to
    // the report below with the codes the analyser's own file-directive fold
    // skips, so a gap is explained rather than read as clean.
    let skip = diag_policy(base.clone()).production_skip();

    // The analyser and the compiler checks — the same `run_all_checks` set
    // the server lifts via `compiler_check_diagnostics` — over one unit built
    // with the cross-file evidence the caller gathered: the producer run the
    // MCP diagnostics tools share. Built once per document; `diag` is a batch
    // verb, not latency-sensitive.
    let registry = registry_for_dialect(dialect.name);
    let file_path = document.path.as_deref().map(|p| p.display().to_string());
    let standalone = standalone_findings(
        &StandaloneDocument {
            source,
            file_path: file_path.as_deref(),
            dialect,
            registry: &registry,
            pack_overlay: tcl_cli_support::spec_pack_key(dialect.name),
            external_call_sites,
        },
        &skip,
    );
    let policy =
        diag_policy(base.directives(Directives::from_analysis(&standalone.analysis, source)));

    // The style pass reads `document.source` — the bytes as read, not the
    // analysis form — because W118 is the one lint whose subject *is* the
    // line terminators; the loader reads the analysis form. The line length
    // and expected ending are the server's defaults: the CLI has no
    // per-document style settings to resolve.
    let doc = DocumentSource {
        text: &document.source,
        analysis_text: source,
        decode: Some(&document.decode),
        dialect,
        pass: SourcePass::Tcl {
            line_length: DEFAULT_LINE_LENGTH,
        },
    };
    let mut report = document_report(&doc, standalone.produced, &policy);
    report.declare_analyser_skip(&policy);
    rows_of(&report, source, &line_index)
}

/// This verb's policy over `builder`: the optimiser off, because the
/// rewrites are the `optimise` verb's — every O-code the checks pass emits is
/// then an `OptimiserOff` suppression in the report rather than a finding
/// that silently never existed.
fn diag_policy(builder: PolicyBuilder) -> Policy {
    let mut policy = builder.build();
    policy.optimiser.enabled = false;
    policy
}

/// The shown findings of `report` as rows — 1-based line and column from
/// `line_index`, the resolved severity — in the deterministic
/// `(line, column, code)` order.
fn rows_of(report: &Report, source: &str, line_index: &LineIndex) -> Vec<Row> {
    let mut rows: Vec<Row> = report
        .shown()
        .map(|shown| {
            let pos = line_index.position_at_utf16(shown.finding.span.start(), source);
            Row {
                line: pos.line + 1,
                column: pos.character.get() + 1,
                severity: shown.severity,
                code: shown.finding.code.to_string(),
                message: shown.finding.message.clone(),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        (a.line, a.column, a.code.as_str()).cmp(&(b.line, b.column, b.code.as_str()))
    });
    rows
}

/// `tcl diag` / `tcl lint` — report every diagnostic across all inputs.
pub fn run_diag(input: &InputArgs, diag: &DiagArgs) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let layers = ConfigLayers::new(invocation_layer(&diag.disable, &diag.enable, "diagnostics"));

    let mut report: Vec<FileReport> = Vec::with_capacity(documents.len());
    let mut problem_count = 0usize;
    let mut diagnostic_count = 0usize;

    let explicit_dialect = input.dialect_profile()?;
    let evidence = cross_file_call_site_evidence(&documents, explicit_dialect);
    for document in &documents {
        let dialect = document.effective_dialect(explicit_dialect);
        let declared = document_proc_names(document, dialect);
        let slice = evidence
            .as_ref()
            .map(|all| all.slice_for(declared.iter().map(String::as_str)));
        let rows = collect_rows(document, dialect, &layers, slice.as_ref());
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            diagnostic_count += 1;
            if is_problem(r.severity) {
                problem_count += 1;
            }
            items.push(DiagItem {
                line: r.line,
                column: r.column,
                severity: severity_label(r.severity),
                code: r.code,
                message: r.message,
            });
        }
        report.push(FileReport {
            file: document.label.clone(),
            diagnostics: items,
        });
    }

    // Honour the shared `-o/--output FILE` flag (default stdout) like every
    // other verb, rather than always printing to stdout (issue 196).
    let target = OutputTarget::from_arg(input.output.as_deref());
    let rendered = if diag.json {
        tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&report)?)
    } else {
        let mut lines: Vec<String> = Vec::new();
        for item in &report {
            for d in &item.diagnostics {
                lines.push(format_line(
                    &item.file, d.line, d.column, d.severity, &d.code, &d.message,
                ));
            }
        }
        if diagnostic_count == 0 {
            lines.push("no diagnostics".to_owned());
        }
        lines.join("\n")
    };
    write_text_output(&target, &rendered)?;

    eprintln!(
        "diagnostics={diagnostic_count} across {} input(s)",
        documents.len()
    );
    Ok(u8::from(problem_count > 0))
}

/// `tcl validate` — error-severity diagnostics only, fail-fast exit code.
pub fn run_validate(input: &InputArgs, diag: &DiagArgs) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let layers = ConfigLayers::new(invocation_layer(&diag.disable, &diag.enable, "diagnostics"));

    let mut errors: Vec<ValidateError> = Vec::new();
    let explicit_dialect = input.dialect_profile()?;
    let evidence = cross_file_call_site_evidence(&documents, explicit_dialect);
    for document in &documents {
        let dialect = document.effective_dialect(explicit_dialect);
        let declared = document_proc_names(document, dialect);
        let slice = evidence
            .as_ref()
            .map(|all| all.slice_for(declared.iter().map(String::as_str)));
        for r in collect_rows(document, dialect, &layers, slice.as_ref()) {
            if r.severity == Severity::Error {
                errors.push(ValidateError {
                    file: document.label.clone(),
                    line: r.line,
                    column: r.column,
                    severity: severity_label(r.severity),
                    code: r.code,
                    message: r.message,
                });
            }
        }
    }

    let target = OutputTarget::from_arg(input.output.as_deref());
    if diag.json {
        let payload = ValidatePayload {
            ok: errors.is_empty(),
            inputs: documents.len(),
            error_count: errors.len(),
            errors,
        };
        write_text_output(
            &target,
            &tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&payload)?),
        )?;
        return Ok(u8::from(!payload.ok));
    }

    if errors.is_empty() {
        eprintln!("validation ok");
        return Ok(0);
    }

    let rendered = errors
        .iter()
        .map(|e| format_line(&e.file, e.line, e.column, e.severity, &e.code, &e.message))
        .collect::<Vec<_>>()
        .join("\n");
    write_text_output(&target, &rendered)?;
    eprintln!("validation failed: {} error(s)", errors.len());
    Ok(1)
}
