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

//! `cargo xtask fp-sweep` — the false-positive audit harness (issue #1316).
//!
//! `docs/design/compiler/fp-audit-todo.md`'s documented method (dump every
//! firing of a code across the corpus, dialect-aware, grouped by site shape)
//! depended on a Python harness (`bench/fp_snippets.py`-style) that no longer
//! exists — `bench/` went away with the Python retirement. This subcommand
//! reproduces the method natively:
//!
//! - **Dialect-aware**: each corpus file's analysis dialect is resolved with
//!   [`tcl_cli_support::InputDocument::effective_dialect`] — the same
//!   detector (`# tcl-dialect:` / content signal / extension, falling back to
//!   `tcl8.6`) the `tcl` CLI and the LSP server use — so a version-gated
//!   command in a Tcl-9-only file does not produce a phantom W002/W004 the
//!   way a fixed-dialect sweep would (the harness-correctness note this file
//!   records for the old Python tool).
//! - **Every code, one pass**: both diagnostic sources the editor publishes —
//!   [`Analyser::analyse`]'s W/E/H-series checks and
//!   [`run_all_checks`]'s O/S/T-series compiler-checks pass — are run per
//!   file, mirroring `tcl diag`'s `collect_rows` (`rust/tcl-cli/src/commands/diag.rs`)
//!   minus its optimisation-code filter, since an FP sweep needs the O-series
//!   firings `tcl diag` deliberately drops.
//! - **Grouped by site shape**: firings for a code are bucketed by their
//!   normalised message (digit runs and single-quoted identifiers replaced
//!   with a placeholder) so repeated instances of the same pattern collapse
//!   into one row with a count, highest-volume shape first — the workflow
//!   the checklist's resolved entries describe ("corpus 3641 → ~700-900").
//!
//! Corpus discovery reuses [`tcl_cli_support::read_input_documents`] (the
//! same recursive walk + extension filter `tcl diag`/`tcl opt` use), so a
//! `--corpus` argument can name a directory or a file, same as the CLI.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tcl_cli_support::{InputDocument, read_input_documents};
use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::optimise_unit;
use tcl_core_types::DiagCode;
use tcl_lexer::LineIndex;
use tcl_lsp_core::source_style::{DEFAULT_LINE_ENDING, DEFAULT_LINE_LENGTH, style_diagnostics};
use tcl_registry::registry_for_dialect;

/// One firing of a swept code.
struct Firing {
    code: DiagCode,
    file: String,
    dialect: String,
    line: u32,
    column: u32,
    message: String,
}

/// Replace every run of ASCII digits and every single-quoted token with a
/// placeholder, so e.g. `"proc 'foo' shadows builtin 'set'"` and `"proc
/// 'bar' shadows builtin 'clock'"` collapse into one site-shape bucket
/// (`"proc '…' shadows builtin '…'"`) instead of each getting their own row.
fn shape_key(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            out.push('…');
            while chars.peek().is_some_and(char::is_ascii_digit) {
                chars.next();
            }
        } else if c == '\'' {
            out.push('\'');
            out.push('…');
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '\'' {
                    closed = true;
                    break;
                }
            }
            if closed {
                out.push('\'');
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Collect every firing of a code in `wanted` across `doc`, running every
/// diagnostic source the editor publishes for one document:
///
/// 1. [`Analyser::analyse`] — the W/E/H-series syntactic/semantic checks,
///    built over a default-config unit, mirroring `tcl diag`'s
///    `collect_rows`.
/// 2. [`run_all_checks`] + [`optimise_unit`] — the O/S/T-series
///    compiler-checks and optimiser passes, over one dialect-configured,
///    interprocedural-summarised unit — mirroring
///    `tcl_lsp_db::compiler_check_diagnostics_uncached`, the documented
///    "no salsa input" fallback that is byte-identical to the live server's
///    build. `tcl diag` only runs (1); an FP sweep needs the O-series
///    firings it deliberately drops, so this reproduces the fuller build
///    rather than diag.rs's simplified (non-interprocedural) one.
/// 3. [`style_diagnostics`] — the pure-text W111/W112/W115/W118 checks,
///    which read raw source and are not part of either compiler pass.
fn sweep_document(doc: &InputDocument, wanted: &[DiagCode], out: &mut Vec<Firing>) {
    let dialect = doc.effective_dialect(None);
    let registry = registry_for_dialect(&dialect);
    let line_index = LineIndex::new(&doc.source);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());

    let mut push = |code: DiagCode, span: tcl_lexer::Span, message: String| {
        if !wanted.contains(&code) {
            return;
        }
        let pos = line_index.position_at_utf16(span.start(), &doc.source);
        out.push(Firing {
            code,
            file: doc.label.clone(),
            dialect: dialect.clone(),
            line: pos.line + 1,
            column: pos.character.get() + 1,
            message,
        });
    };

    // (1) Analyser tail.
    let analysis_cu = Arc::new(CompilationUnit::build_with_options(
        &doc.source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::default(),
            dialect: &dialect,
            external_call_sites: None,
        },
    ));
    let mut analyser =
        Analyser::new().with_file_path(doc.path.as_ref().map(|p| p.display().to_string()));
    analyser.set_cu_override(Arc::clone(&analysis_cu));
    let result = analyser.analyse(&doc.source, &dialect);
    for d in &result.diagnostics {
        push(d.code, d.span, d.message.clone());
    }

    // (2) Compiler-checks + optimiser, over one interprocedural-summarised,
    // dialect-configured unit.
    let checks_cu = CompilationUnit::build_with_options(
        &doc.source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect(&dialect),
            dialect: &dialect,
            external_call_sites: None,
        },
    )
    .with_interprocedural(registry, dialect_opt);
    for d in run_all_checks(&checks_cu, registry, dialect_opt) {
        push(d.code, d.span, d.message);
    }
    for o in optimise_unit(&checks_cu, registry, dialect_opt) {
        push(o.code, o.span, o.message);
    }

    // (3) Pure-text checks — the style lints plus the W107 / W109 / W305
    // encoding-integrity set.  No suppression / user-disabled set, since the
    // sweep wants every firing regardless of what a hypothetical editor config
    // would silence.  The corpus is read through `read_input_documents`, so the
    // document carries the byte-level decode report and the encoding findings
    // come out at full precision (issue #1326).
    let no_disabled: std::collections::HashSet<String> = std::collections::HashSet::new();
    let no_suppressed: std::collections::HashMap<i32, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    for d in style_diagnostics(
        &doc.source,
        DEFAULT_LINE_LENGTH,
        DEFAULT_LINE_ENDING,
        &no_disabled,
        &no_suppressed,
        Some(&doc.decode),
    ) {
        let Ok(code) = DiagCode::from_str(d.code) else {
            continue;
        };
        if !wanted.contains(&code) {
            continue;
        }
        out.push(Firing {
            code,
            file: doc.label.clone(),
            dialect: dialect.clone(),
            line: d.range.start_line + 1,
            column: d.range.start_character + 1,
            message: d.message,
        });
    }
}

/// Report every firing of `code` in `firings`, grouped by message shape,
/// highest-volume shape first, with up to `examples` sample locations per
/// shape and a per-dialect firing-count breakdown.
fn report_code(code: DiagCode, firings: &[Firing], examples: usize) {
    let mine: Vec<&Firing> = firings.iter().filter(|f| f.code == code).collect();
    println!("== {} — {} firing(s) ==", code.as_str(), mine.len());
    if mine.is_empty() {
        println!("  (no firings in this corpus)\n");
        return;
    }

    let mut by_dialect: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &mine {
        *by_dialect.entry(f.dialect.as_str()).or_insert(0) += 1;
    }
    print!("  by dialect:");
    for (dialect, n) in &by_dialect {
        print!(" {dialect}={n}");
    }
    println!();

    let mut by_shape: BTreeMap<String, Vec<&Firing>> = BTreeMap::new();
    for f in &mine {
        by_shape.entry(shape_key(&f.message)).or_default().push(f);
    }
    let mut shapes: Vec<(&String, &Vec<&Firing>)> = by_shape.iter().collect();
    shapes.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    for (shape, group) in shapes {
        println!("  [{:>5}] {shape}", group.len());
        for f in group.iter().take(examples) {
            println!(
                "           {}:{}:{}: {}",
                f.file, f.line, f.column, f.message
            );
        }
        if group.len() > examples {
            println!("           … {} more", group.len() - examples);
        }
    }
    println!();
}

/// `cargo xtask fp-sweep --code CODE [--code CODE...] --corpus PATH [--corpus PATH...] [--examples N]`
///
/// Prints, per swept code: total firing count, a dialect breakdown, and
/// every distinct message shape ranked by frequency with up to `examples`
/// sample locations — the "dump every firing, group by site/shape" method
/// the checklist specifies.
pub fn run(codes: &[String], corpus: &[PathBuf], examples: usize) -> Result<ExitCode> {
    if codes.is_empty() {
        bail!("pass at least one --code CODE");
    }
    if corpus.is_empty() {
        bail!("pass at least one --corpus PATH (a directory or a file)");
    }

    let wanted: Vec<DiagCode> = codes
        .iter()
        .map(|c| DiagCode::from_str(c).map_err(|_| anyhow::anyhow!("unknown diagnostic code: {c}")))
        .collect::<Result<_>>()?;

    let documents = read_input_documents(corpus, &[], true)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading corpus from {corpus:?}"))?;

    let mut firings: Vec<Firing> = Vec::new();
    for doc in &documents {
        sweep_document(doc, &wanted, &mut firings);
    }

    println!(
        "fp-sweep: {} file(s) scanned, {} code(s) requested, {} firing(s) found\n",
        documents.len(),
        wanted.len(),
        firings.len()
    );

    for code in &wanted {
        report_code(*code, &firings, examples);
    }
    Ok(ExitCode::SUCCESS)
}
