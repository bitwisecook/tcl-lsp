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
//! analyse each input
//! document separately (a per-file loop).

use std::collections::HashSet;

use serde::Serialize;
use tcl_cli_support::{
    InputDocument, OutputTarget, read_input_documents, registry_for_dialect, write_text_output,
};
use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::unit_scope::CallSiteEvidence;
use tcl_lexer::LineIndex;
use tcl_lsp_core::source_style::StyleSeverity;

use crate::cli::{DiagArgs, InputArgs};

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

/// Build the disabled-code set from `--disable` / `--enable` (comma-separated,
/// upper-cased), mirroring `_resolve_disabled_diagnostics` sans config file.
fn resolve_disabled(disable: &[String], enable: &[String]) -> HashSet<String> {
    let mut set = HashSet::new();
    for raw in disable {
        for code in raw.split(',') {
            let code = code.trim();
            if !code.is_empty() {
                set.insert(code.to_ascii_uppercase());
            }
        }
    }
    for raw in enable {
        for code in raw.split(',') {
            let code = code.trim();
            if !code.is_empty() {
                set.remove(&code.to_ascii_uppercase());
            }
        }
    }
    set
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
/// project-wide procedure-name set the scan resolved against (issue #977).
///
/// `tcl diag a.tcl b.tcl` is a multi-file compilation just as much as the
/// editor's workspace is: without this, `a.tcl` would fold a parameter that
/// `b.tcl` calls with a different literal.  `None` for a single input — one
/// file is not a project, and asserting a closed world from it would be the
/// very claim issue #977 is about.
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
        merged.merge_from(&tcl_compiler::unit_scope::scan_source_call_sites(
            &document.source,
            &registry_for_dialect(dialect.name),
            dialect.name,
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
    dialect: &tcl_dialect::DialectProfile,
) -> Vec<String> {
    tcl_compiler::signature_scan::extract_signatures(
        &document.source,
        &registry_for_dialect(dialect.name),
    )
    .procs
    .into_keys()
    .collect()
}

/// Collect every diagnostic the editor surfaces for one document: the analyser's
/// syntactic / semantic checks plus the compiler-checks pass (shimmer `S1xx`,
/// taint `T1xx` / `W2xx`, iRules data-flow). Mirrors the server's
/// `lift_analyser_diagnostics` + `lift_compiler_diagnostics` concatenation so the
/// CLI and the editor report the same set. Optimiser `O1xx` rewrites are the
/// domain of the `optimise` verb, so they are dropped here — the same split the
/// server draws with its optimiser toggle. Rows come back in a deterministic
/// `(line, column, code)` order; `disabled` removes `--disable`d codes.
fn collect_rows(
    document: &InputDocument,
    dialect: &tcl_dialect::DialectProfile,
    disabled: &HashSet<String>,
    external_call_sites: Option<&CallSiteEvidence>,
) -> Vec<Row> {
    let source = document.source.as_str();
    let line_index = LineIndex::new(source);
    let mut rows: Vec<Row> = Vec::new();

    // Byte-backed source integrity first (issue #1326): W107 / W109 say whether
    // the text below is the file on disk at all, so they belong ahead of
    // anything derived from it. W305 comes from the analyser below. When the
    // file is not UTF-8 text, the byte-backed diagnostics are all we report —
    // abstaining is the honest answer, and it is what turns a
    // three-line UTF-16 iRule from 87 nonsense findings into one accurate one.
    for d in document.encoding_diagnostics() {
        if disabled.contains(d.code) {
            continue;
        }
        rows.push(Row {
            line: d.range.start_line + 1,
            column: d.range.start_character + 1,
            severity: match d.severity {
                StyleSeverity::Warning => Severity::Warning,
                StyleSeverity::Hint => Severity::Hint,
            },
            code: d.code.to_owned(),
            message: d.message,
        });
    }
    if document.abstains_on_encoding() {
        return rows;
    }

    // One compilation unit for both consumers, built with whatever cross-file
    // call-site evidence the caller gathered (issue #977).  The analyser's
    // CFG/SSA tail would otherwise build its **own** unit — with no evidence —
    // and its I230 / I231 constant-branch findings would disagree with the
    // compiler-checks pass below.  `LexerConfig::default()` matches what
    // `emit_cfg_ssa_diagnostics` builds for itself, mirroring the server's
    // `set_cu_override` seam in `tcl_lsp_db::file_analysis_incremental`.
    let registry = registry_for_dialect(dialect.name);
    let analysis_cu = std::sync::Arc::new(CompilationUnit::build_with_options(
        source,
        UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::default(),
            dialect: dialect.name,
            external_call_sites,
        },
    ));

    let file_path = document.path.as_deref().map(|p| p.display().to_string());
    let mut analyser = Analyser::new()
        .with_file_path(file_path)
        .with_pack_overlay(tcl_cli_support::spec_pack_key(dialect.name));
    analyser.set_cu_override(std::sync::Arc::clone(&analysis_cu));
    let result = analyser.analyse(source, dialect.name);
    for d in &result.diagnostics {
        if disabled.contains(d.code.as_str()) {
            continue;
        }
        let pos = line_index.position_at_utf16(d.span.start(), source);
        rows.push(Row {
            line: pos.line + 1,
            column: pos.character.get() + 1,
            severity: d.severity,
            code: d.code.to_string(),
            message: d.message.clone(),
        });
    }

    // Compiler-checks pass — the same `run_all_checks` set the server lifts via
    // `compiler_check_diagnostics`. Built once per document; `diag` is a batch
    // verb, not latency-sensitive.
    // The checks pass lowers under the document's own dialect config (`{*}` /
    // iRules braces), which coincides with the analyser tail's default for
    // every dialect but `tcl8.4` / `f5-irules` — reuse the unit above when it
    // does, exactly as the server's shared `compilation_unit` query does.
    let checks_config = tcl_lexer::LexerConfig::for_dialect(dialect.name);
    let checks_cu = if checks_config == tcl_lexer::LexerConfig::default() {
        std::sync::Arc::clone(&analysis_cu)
    } else {
        std::sync::Arc::new(CompilationUnit::build_with_options(
            source,
            UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: checks_config,
                dialect: dialect.name,
                external_call_sites,
            },
        ))
    };
    let cu = checks_cu.as_ref();
    let dialect_opt = Some(dialect);
    for d in run_all_checks(cu, &registry, dialect_opt) {
        if d.code.is_optimisation() || disabled.contains(d.code.as_str()) {
            continue;
        }
        let pos = line_index.position_at_utf16(d.span.start(), source);
        rows.push(Row {
            line: pos.line + 1,
            column: pos.character.get() + 1,
            severity: d.severity,
            code: d.code.to_string(),
            message: d.message,
        });
    }

    rows.sort_by(|a, b| {
        (a.line, a.column, a.code.as_str()).cmp(&(b.line, b.column, b.code.as_str()))
    });
    rows
}

/// `tcl diag` / `tcl lint` — report every diagnostic across all inputs.
pub fn run_diag(input: &InputArgs, diag: &DiagArgs) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let disabled = resolve_disabled(&diag.disable, &diag.enable);

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
        let rows = collect_rows(document, dialect, &disabled, slice.as_ref());
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
    let disabled = resolve_disabled(&diag.disable, &diag.enable);

    let mut errors: Vec<ValidateError> = Vec::new();
    let explicit_dialect = input.dialect_profile()?;
    let evidence = cross_file_call_site_evidence(&documents, explicit_dialect);
    for document in &documents {
        let dialect = document.effective_dialect(explicit_dialect);
        let declared = document_proc_names(document, dialect);
        let slice = evidence
            .as_ref()
            .map(|all| all.slice_for(declared.iter().map(String::as_str)));
        for r in collect_rows(document, dialect, &disabled, slice.as_ref()) {
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
