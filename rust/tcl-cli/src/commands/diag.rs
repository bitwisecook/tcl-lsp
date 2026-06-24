//! Diagnostic verbs: `diag` / `lint` (identical) and `validate`.
//!
//! Drive the analyser in `tcl-compiler`. Unlike the transform verbs, these
//! analyse each input
//! document separately (a per-file loop).

use std::collections::HashSet;

use serde::Serialize;
use tcl_cli_support::{read_input_documents, registry_for_dialect};
use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_lexer::LineIndex;

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

/// Collect every diagnostic the editor surfaces for one document: the analyser's
/// syntactic / semantic checks plus the compiler-checks pass (shimmer `S1xx`,
/// taint `T1xx` / `W2xx`, iRules data-flow). Mirrors the server's
/// `lift_analyser_diagnostics` + `lift_compiler_diagnostics` concatenation so the
/// CLI and the editor report the same set. Optimiser `O1xx` rewrites are the
/// domain of the `optimise` verb, so they are dropped here — the same split the
/// server draws with its optimiser toggle. Rows come back in a deterministic
/// `(line, column, code)` order; `disabled` removes `--disable`d codes.
fn collect_rows(source: &str, dialect: &str, disabled: &HashSet<String>) -> Vec<Row> {
    let line_index = LineIndex::new(source);
    let mut rows: Vec<Row> = Vec::new();

    let result = Analyser::new().analyse(source, dialect);
    for d in &result.diagnostics {
        if disabled.contains(&d.code.to_ascii_uppercase()) {
            continue;
        }
        let pos = line_index.position_at_utf16(d.span.start(), source);
        rows.push(Row {
            line: pos.line + 1,
            column: pos.character.get() + 1,
            severity: d.severity,
            code: d.code.clone(),
            message: d.message.clone(),
        });
    }

    // Compiler-checks pass — the same `run_all_checks` set the server lifts via
    // `compiler_check_diagnostics`. Built once per document; `diag` is a batch
    // verb, not latency-sensitive.
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(source, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.starts_with('O') || disabled.contains(&d.code.to_ascii_uppercase()) {
            continue;
        }
        let pos = line_index.position_at_utf16(d.span.start(), source);
        rows.push(Row {
            line: pos.line + 1,
            column: pos.character.get() + 1,
            severity: d.severity,
            code: d.code,
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

    for document in &documents {
        let rows = collect_rows(&document.source, &input.dialect, &disabled);
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
        for r in collect_rows(&document.source, &input.dialect, &disabled) {
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
