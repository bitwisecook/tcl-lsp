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

//! `mro_delta` — W307/W308 delta + labeling-worksheet harness for the
//! class-lattice experiment.
//!
//! Two jobs, both over the FULL (cross-file, namespace-aware) resolver:
//!
//! 1. **W307/W308 delta.** Cross-tabulate the resolver's verdict at each
//!    `$obj method` site against the *shipping* diagnostic the analyser
//!    actually emits there (W307 non-literal-command / W308 unknown-method /
//!    none). This is the "false positives removed vs new true positives vs
//!    regressions" comparison against the current heuristic baseline —
//!    resolved against the real emitter, not a reimplementation.
//!
//! 2. **Labeling worksheet.** Emit a CSV of a deterministic ≥120-site
//!    sample (all resolved sites + a strided sample of abstentions) with
//!    enough source context (the dispatch line + the nearest preceding
//!    binding of the receiver) to hand-label ground truth. Feeds
//!    `experiments/mro_eval/labeled_sample.csv` + `score_labels.py`.
//!
//! Usage:
//! ```text
//! cargo run --release -p tcl-compiler --example mro_delta -- \
//!     experiments/corpus/{georgtree,adversarial,vendor,repo-fixtures} \
//!     > experiments/mro_eval/label_worksheet.csv
//! ```
//! The confusion matrix + delta go to stderr; the CSV worksheet to stdout.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tcl_compiler::analyser::class_lattice::{
    AblationConfig, DispatchVerdict, NsContext, analyse_dispatch,
};
use tcl_compiler::analyser::state::Analyser;
use tcl_compiler::analyser::types::{ClassDef, Diagnostic};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_core_types::DiagCode;
use tcl_dialect::DialectProfile;
use tcl_registry::CommandRegistry;

fn collect_tcl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(name, ".git" | "target" | "node_modules" | ".venv") {
                collect_tcl_files(&path, out);
            }
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|e| e == "tcl" || e == "test")
        {
            out.push(path);
        }
    }
}

struct FileData {
    path: PathBuf,
    src: String,
    diagnostics: Vec<Diagnostic>,
    sites: Vec<tcl_compiler::analyser::state::VarCommandSite>,
    classes: HashMap<String, ClassDef>,
    cu: CompilationUnit,
    ns: NsContext,
}

fn analyse_file(path: &Path, reg: &CommandRegistry) -> Option<FileData> {
    let src = std::fs::read_to_string(path).ok()?;
    if src.len() > 2_000_000 {
        return None;
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut a = Analyser::new();
        let result = a.analyse(&src, "tcl8.6");
        let cu = CompilationUnit::build_for(&src, reg, false)
            .with_interprocedural(reg, Some(DialectProfile::by_name("tcl8.6")));
        let ns = NsContext::from_result(&result);
        FileData {
            path: path.to_path_buf(),
            src,
            diagnostics: result.diagnostics,
            sites: a.var_command_sites.clone(),
            classes: result.all_classes,
            cu,
            ns,
        }
    }))
    .ok()
}

/// 1-based line number for a byte offset.
fn line_of(src: &str, offset: u32) -> usize {
    src[..(offset as usize).min(src.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

/// The source line containing `offset`, trimmed and CSV-safe.
fn line_text(src: &str, offset: u32) -> String {
    let o = (offset as usize).min(src.len());
    let start = src[..o].rfind('\n').map_or(0, |i| i + 1);
    let end = src[o..].find('\n').map_or(src.len(), |i| o + i);
    csv_cell(src[start..end].trim())
}

/// The nearest `set <var> …` binding of `var` before `offset`, if any.
fn nearest_binding(src: &str, var: &str, offset: u32) -> String {
    let needle = format!("set {var} ");
    let o = (offset as usize).min(src.len());
    if let Some(pos) = src[..o].rfind(&needle) {
        let end = src[pos..].find('\n').map_or(src.len(), |i| pos + i);
        csv_cell(src[pos..end].trim())
    } else {
        String::new()
    }
}

fn csv_cell(s: &str) -> String {
    let s = s.replace('"', "'");
    if s.contains(',') || s.contains('"') {
        format!("\"{s}\"")
    } else {
        s
    }
}

/// The shipping W307/W308 diagnostic at a dispatch span, if any.
fn shipping_code(diags: &[Diagnostic], offset: u32) -> &'static str {
    for d in diags {
        if d.span.start() == offset {
            if d.code == DiagCode::W307 {
                return "W307";
            }
            if d.code == DiagCode::W308 {
                return "W308";
            }
        }
    }
    "none"
}

fn main() {
    let roots = parse_roots();
    let files = gather_files(&roots);
    let reg = CommandRegistry::build_default();
    let (data, merged) = analyse_all(&files, &reg);
    let ws = build_worksheet(&data, &merged);
    print_delta(data.len(), merged.len(), &ws.confusion);
    print_worksheet(&ws.resolved_rows, &ws.abstain_rows);
}

/// Parse corpus-directory arguments, exiting with a usage message if none.
fn parse_roots() -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        eprintln!("usage: mro_delta <corpus dirs...>  (worksheet CSV → stdout, delta → stderr)");
        std::process::exit(1);
    }
    roots
}

/// Collect and sort every `.tcl`/`.test` file under the given roots.
fn gather_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for r in roots {
        collect_tcl_files(r, &mut files);
    }
    files.sort();
    files
}

/// Analyse every file and merge the discovered classes into one index.
fn analyse_all(
    files: &[PathBuf],
    reg: &CommandRegistry,
) -> (Vec<FileData>, HashMap<String, ClassDef>) {
    let mut data: Vec<FileData> = Vec::new();
    let mut merged: HashMap<String, ClassDef> = HashMap::new();
    for p in files {
        if let Some(fd) = analyse_file(p, reg) {
            for (k, v) in &fd.classes {
                merged.entry(k.clone()).or_insert_with(|| v.clone());
            }
            data.push(fd);
        }
    }
    (data, merged)
}

/// The confusion matrix plus the resolved/abstain worksheet records.
struct Worksheet {
    // Confusion matrix: resolver verdict × shipping code.
    // resolver rows: resolved-known, resolved-unknown (W308-cand), abstain
    // shipping cols: W307, W308, none
    confusion: HashMap<(&'static str, &'static str), usize>,
    resolved_rows: Vec<String>,
    abstain_rows: Vec<String>,
}

/// Run the dispatch resolver over every file and tabulate the confusion
/// matrix alongside the per-site worksheet rows.
fn build_worksheet(data: &[FileData], merged: &HashMap<String, ClassDef>) -> Worksheet {
    let mut confusion: HashMap<(&'static str, &'static str), usize> = HashMap::new();
    let mut resolved_rows: Vec<String> = Vec::new();
    let mut abstain_rows: Vec<String> = Vec::new();

    let cfg = AblationConfig::full();
    for fd in data {
        let (reports, _stats) = analyse_dispatch(&fd.cu, merged, &fd.sites, &fd.ns, &cfg);
        for r in reports {
            let Some(method) = &r.method else { continue };
            let ship = shipping_code(&fd.diagnostics, r.offset);
            let (res_kind, classes) = match &r.verdict {
                DispatchVerdict::Resolved {
                    method_known: true,
                    classes,
                    ..
                } => (
                    "resolved-known",
                    classes.iter().cloned().collect::<Vec<_>>().join("|"),
                ),
                DispatchVerdict::Resolved {
                    method_known: false,
                    classes,
                    ..
                } => (
                    "resolved-unknown",
                    classes.iter().cloned().collect::<Vec<_>>().join("|"),
                ),
                DispatchVerdict::Abstain(reason) => ("abstain", reason.to_string()),
            };
            *confusion.entry((res_kind, ship)).or_insert(0) += 1;

            let file = fd.path.display();
            let line = line_of(&fd.src, r.offset);
            let row = format!(
                "{},{},{},{},{},{},{},{},{}",
                csv_cell(&file.to_string()),
                line,
                csv_cell(&r.var),
                csv_cell(method),
                res_kind,
                csv_cell(&classes),
                ship,
                line_text(&fd.src, r.offset),
                nearest_binding(&fd.src, &r.var, r.offset),
            );
            if res_kind == "abstain" {
                abstain_rows.push(row);
            } else {
                resolved_rows.push(row);
            }
        }
    }
    Worksheet {
        confusion,
        resolved_rows,
        abstain_rows,
    }
}

/// stderr: the delta / confusion matrix.
fn print_delta(
    files: usize,
    merged_classes: usize,
    confusion: &HashMap<(&'static str, &'static str), usize>,
) {
    let rows = ["resolved-known", "resolved-unknown", "abstain"];
    let get = |r: &str, c: &str| confusion.get(&(row_key(r), c)).copied().unwrap_or(0);
    eprintln!("# mro_delta — resolver × shipping-diagnostic confusion matrix");
    eprintln!("# files={files} merged_classes={merged_classes}");
    eprintln!(
        "{:<18} {:>7} {:>7} {:>7}",
        "resolver \\ shipping", "W307", "W308", "none"
    );
    for r in rows {
        eprintln!(
            "{:<18} {:>7} {:>7} {:>7}",
            r,
            get(r, "W307"),
            get(r, "W308"),
            get(r, "none")
        );
    }
    eprintln!();
    eprintln!("# Interpretation (resolver as a hypothetical replacement):");
    eprintln!(
        "#  resolved-known × W308  = {} → W308 the resolver would REMOVE (FP-removed if class really has the method)",
        get("resolved-known", "W308")
    );
    eprintln!(
        "#  resolved-unknown × none = {} → NEW W308 the resolver would ADD (TP if method really absent, else FP)",
        get("resolved-unknown", "none")
    );
    eprintln!(
        "#  resolved-unknown × W308 = {} → agree with shipping W308",
        get("resolved-unknown", "W308")
    );
    eprintln!(
        "#  abstain × W307         = {} → resolver makes no claim where shipping fired W307",
        get("abstain", "W307")
    );
    eprintln!(
        "#  abstain × W308         = {} → resolver abstains where shipping fired W308 (removes a claim)",
        get("abstain", "W308")
    );
}

/// stdout: labeling worksheet CSV (header + all resolved sites + a strided
/// abstain sample, targeting >= 120 rows total).
fn print_worksheet(resolved_rows: &[String], abstain_rows: &[String]) {
    println!(
        "file,line,var,method,resolver_verdict,resolver_classes,shipping,dispatch_line,nearest_binding,truth_class,method_exists,notes"
    );
    let emit = |rows: &[String]| {
        for row in rows {
            println!("{row},,,");
        }
    };
    emit(resolved_rows);
    // Stride the abstentions so the sample spans the corpus, aiming to top
    // up to ~120 total labeled rows.
    let want_abstain = 120usize.saturating_sub(resolved_rows.len()).max(40);
    let stride = (abstain_rows.len() / want_abstain.max(1)).max(1);
    let sampled: Vec<String> = abstain_rows.iter().step_by(stride).cloned().collect();
    emit(&sampled);
    eprintln!(
        "\n# worksheet: {} resolved + {} sampled abstentions = {} rows (of {} total abstentions)",
        resolved_rows.len(),
        sampled.len(),
        resolved_rows.len() + sampled.len(),
        abstain_rows.len(),
    );
}

fn row_key(r: &str) -> &'static str {
    match r {
        "resolved-known" => "resolved-known",
        "resolved-unknown" => "resolved-unknown",
        _ => "abstain",
    }
}
