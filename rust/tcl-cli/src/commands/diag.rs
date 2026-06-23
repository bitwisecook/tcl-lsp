//! Diagnostic verbs: `diag` / `lint` (identical) and `validate`.
//!
//! Ports of the handlers in `tooling/tcl/verbs/diag.py`, driving the analyser
//! in `tcl-compiler`. Unlike the transform verbs, these analyse each input
//! document separately (matching the Python per-file loop).

use std::collections::HashSet;

use serde::Serialize;
use tcl_cli_support::read_input_documents;
use tcl_compiler::analyser::{Analyser, Severity};
use tcl_lexer::LineIndex;

use crate::cli::{DiagArgs, InputArgs};

/// One diagnostic in the `diag` report (field order matches the Python dict).
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

/// Map a Rust analyser severity to the Python `Severity.name.lower()`
/// vocabulary (`error` / `warning` / `info` / `hint`). The Rust `Suggestion`
/// tier corresponds to the Python `INFO`.
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

/// Render one diagnostic text line (mirrors `_format_diagnostic_line` and the
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

/// `tcl diag` / `tcl lint` — report every diagnostic across all inputs.
pub fn run_diag(input: &InputArgs, diag: &DiagArgs) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let disabled = resolve_disabled(&diag.disable, &diag.enable);

    let mut report: Vec<FileReport> = Vec::with_capacity(documents.len());
    let mut problem_count = 0usize;
    let mut diagnostic_count = 0usize;

    for document in &documents {
        let result = Analyser::new().analyse(&document.source, &input.dialect);
        let line_index = LineIndex::new(&document.source);
        let mut items = Vec::new();
        for d in &result.diagnostics {
            if disabled.contains(&d.code.to_ascii_uppercase()) {
                continue;
            }
            let pos = line_index.position_at_utf16(d.span.start(), &document.source);
            diagnostic_count += 1;
            if is_problem(d.severity) {
                problem_count += 1;
            }
            items.push(DiagItem {
                line: pos.line + 1,
                column: pos.character + 1,
                severity: severity_label(d.severity),
                code: d.code.clone(),
                message: d.message.clone(),
            });
        }
        report.push(FileReport {
            file: document.label.clone(),
            diagnostics: items,
        });
    }

    if diag.json {
        println!(
            "{}",
            tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&report)?)
        );
    } else {
        for item in &report {
            for d in &item.diagnostics {
                println!(
                    "{}",
                    format_line(
                        &item.file, d.line, d.column, d.severity, &d.code, &d.message
                    )
                );
            }
        }
        if diagnostic_count == 0 {
            println!("no diagnostics");
        }
    }

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
    for document in &documents {
        let result = Analyser::new().analyse(&document.source, &input.dialect);
        let line_index = LineIndex::new(&document.source);
        for d in &result.diagnostics {
            if d.severity == Severity::Error && !disabled.contains(&d.code.to_ascii_uppercase()) {
                let pos = line_index.position_at_utf16(d.span.start(), &document.source);
                errors.push(ValidateError {
                    file: document.label.clone(),
                    line: pos.line + 1,
                    column: pos.character + 1,
                    severity: severity_label(d.severity),
                    code: d.code.clone(),
                    message: d.message.clone(),
                });
            }
        }
    }

    if diag.json {
        let payload = ValidatePayload {
            ok: errors.is_empty(),
            inputs: documents.len(),
            error_count: errors.len(),
            errors,
        };
        println!(
            "{}",
            tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&payload)?)
        );
        return Ok(u8::from(!payload.ok));
    }

    if errors.is_empty() {
        eprintln!("validation ok");
        return Ok(0);
    }

    for e in &errors {
        println!(
            "{}",
            format_line(&e.file, e.line, e.column, e.severity, &e.code, &e.message)
        );
    }
    eprintln!("validation failed: {} error(s)", errors.len());
    Ok(1)
}
