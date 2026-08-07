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

//! Probe: with the `body_needs_enclosing_context` fallback bypassed, how far is
//! the per-item path from a full `analyse`?  Categorises the residual diagnostic
//! divergences across the tmp/ corpus so the fast path can be widened to cover
//! them.  Run: `cargo run --release -p tcl-compiler --example per_item_divergence`.

// Demonstration/experiment harness: percentage maths casts small counts to f64,
// and `main` drives the divergence categorisation inline — neither the pedantic
// precision-loss nor the length lint is worth acting on here.
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::types::{AnalysisResult, Diagnostic};

fn gather_tcl(dir: &Path, out: &mut Vec<PathBuf>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            gather_tcl(&p, out, cap);
        } else if p.extension().is_some_and(|x| x == "tcl") {
            out.push(p);
            if out.len() >= cap {
                return;
            }
        }
    }
}

fn diag_key(d: &Diagnostic) -> (u32, u32, String, String) {
    (
        d.span.start(),
        d.span.end(),
        d.code.to_string(),
        d.message.clone(),
    )
}

/// Symmetric diff of two diagnostic sets, keyed by (span, code, message).
fn diff_diags(want: &AnalysisResult, got: &AnalysisResult) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut w: BTreeMap<(u32, u32, String, String), Diagnostic> = BTreeMap::new();
    for d in &want.diagnostics {
        w.insert(diag_key(d), d.clone());
    }
    let mut g: BTreeMap<(u32, u32, String, String), Diagnostic> = BTreeMap::new();
    for d in &got.diagnostics {
        g.insert(diag_key(d), d.clone());
    }
    let missing: Vec<Diagnostic> = w
        .iter()
        .filter(|(k, _)| !g.contains_key(*k))
        .map(|(_, v)| v.clone())
        .collect();
    let extra: Vec<Diagnostic> = g
        .iter()
        .filter(|(k, _)| !w.contains_key(*k))
        .map(|(_, v)| v.clone())
        .collect();
    (missing, extra)
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    let dialect = "tcl8.6";
    let cap: usize = std::env::var("CAP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let mut files = Vec::new();
    gather_tcl(&root, &mut files, cap);
    println!("corpus: {} files (dialect {dialect})", files.len());

    let mut total = 0usize;
    let mut fast_path = 0usize;
    let mut identical = 0usize;
    let mut fell_back = 0usize; // still fell back for OTHER reasons (recovery/dup)
    let mut diverged = 0usize;
    // category -> (missing_count, extra_count, files_affected)
    let mut by_code: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    let mut worst: Vec<(usize, String)> = Vec::new();

    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.is_empty() {
            continue;
        }
        total += 1;
        let want = Analyser::new().analyse(&src, dialect);

        let mut a = Analyser::new();
        // Default: exercise the *production* path (narrow fallback active).  Set
        // SKIP_FALLBACK=1 to bypass `body_needs_enclosing_context` and measure
        // the raw residual divergence the fast path would have.
        a.probe_skip_enclosing_fallback = std::env::var("SKIP_FALLBACK").is_ok();
        let got = a.analyse_per_item(&src, dialect);
        if a.took_fast_path {
            fast_path += 1;
            if std::env::var("LIST_FAST").is_ok() && src.lines().count() > 1500 {
                println!(
                    "  FAST {:>5}L  {}",
                    src.lines().count(),
                    path.strip_prefix(&root).unwrap_or(path).display()
                );
            }
        }

        if got == want {
            identical += 1;
            continue;
        }
        let (missing, extra) = diff_diags(&want, &got);
        if missing.is_empty() && extra.is_empty() {
            // Diagnostics match but some other AnalysisResult field differs
            // (scopes/vars) — report which fields diverge.
            fell_back += 1;
            let mut fields = Vec::new();
            if want.global_scope != got.global_scope {
                fields.push("global_scope");
            }
            if want.all_procs != got.all_procs {
                fields.push("all_procs");
            }
            if want.all_classes != got.all_classes {
                fields.push("all_classes");
            }
            if want.all_variables != got.all_variables {
                fields.push("all_variables");
            }
            if want.command_invocations != got.command_invocations {
                fields.push("command_invocations");
            }
            if want.command_aliases != got.command_aliases {
                fields.push("command_aliases");
            }
            if want.instance_classes != got.instance_classes {
                fields.push("instance_classes");
            }
            if want.package_requires != got.package_requires {
                fields.push("package_requires");
            }
            if want.package_ifneededs != got.package_ifneededs {
                fields.push("package_ifneededs");
            }
            if want.package_provides != got.package_provides {
                fields.push("package_provides");
            }
            if want.namespace_imports != got.namespace_imports {
                fields.push("namespace_imports");
            }
            if want.regex_patterns != got.regex_patterns {
                fields.push("regex_patterns");
            }
            if want.suppressed_lines != got.suppressed_lines {
                fields.push("suppressed_lines");
            }
            if want.unknown_proc_info != got.unknown_proc_info {
                fields.push("unknown_proc_info");
            }
            if want.has_dynamic_providers != got.has_dynamic_providers {
                fields.push("has_dynamic_providers");
            }
            println!(
                "  STRUCT-DIFF {}  fields={fields:?}",
                path.strip_prefix(&root).unwrap_or(path).display()
            );
            continue;
        }
        diverged += 1;
        let mut codes = std::collections::BTreeSet::new();
        for d in &missing {
            let e = by_code.entry(format!("-{}", d.code)).or_default();
            e.0 += 1;
            codes.insert(format!("-{}", d.code));
        }
        for d in &extra {
            let e = by_code.entry(format!("+{}", d.code)).or_default();
            e.1 += 1;
            codes.insert(format!("+{}", d.code));
        }
        for c in codes {
            by_code.entry(c).or_default().2 += 1;
        }
        worst.push((
            missing.len() + extra.len(),
            format!(
                "{}  (-{} +{})",
                path.strip_prefix(&root).unwrap_or(path).display(),
                missing.len(),
                extra.len()
            ),
        ));
    }

    println!("\n== results ==");
    println!("  total non-empty:        {total}");
    println!(
        "  took FAST PATH:         {fast_path}  ({:.1}%)",
        100.0 * fast_path as f64 / total as f64
    );
    println!(
        "  byte-identical:         {identical}  ({:.1}%)",
        100.0 * identical as f64 / total as f64
    );
    println!("  diag-equal, struct-diff:{fell_back}");
    println!(
        "  diagnostic-diverged:    {diverged}  ({:.1}%)",
        100.0 * diverged as f64 / total as f64
    );

    println!("\n== divergence by code (-=missing in per_item, +=extra) ==");
    let mut codes: Vec<_> = by_code.into_iter().collect();
    codes.sort_by_key(|(_, v)| std::cmp::Reverse(v.0 + v.1));
    for (code, (m, e, files)) in codes.iter().take(40) {
        let (mc, ec) = if code.starts_with('-') {
            (*m, 0)
        } else {
            (0, *e)
        };
        let _ = (mc, ec);
        println!("  {code:<14} count={:<6} files={files}", m + e);
    }

    println!("\n== worst files ==");
    worst.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    for (n, f) in worst.iter().take(20) {
        println!("  {n:<6} {f}");
    }
}
