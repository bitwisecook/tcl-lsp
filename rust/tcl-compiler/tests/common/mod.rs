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

//! Shared corpus/fuzz test helpers.
//!
//! [`Progress`] gives long corpus sweeps **durable, flushed progress**: one line
//! per completed file (a "chunk") and one per finding, written *and flushed* to a
//! log file as they happen and mirrored to stderr. So a sweep can be watched live
//! (`tail -f target/fuzz-progress/<name>.log`), and a `SIGKILL` mid-run preserves
//! every completed chunk's result plus every finding so far — instead of losing
//! everything because the test only asserts at the very end.
//!
//! Runs are also **resumable**: [`Progress::skip`] reads `$TCL_FUZZ_SKIP` so a
//! killed sweep can restart roughly where it stopped, and [`Progress::limit`]
//! reads `$TCL_FUZZ_LIMIT` to cap a chunk to a bounded slice.
#![allow(dead_code)]

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

/// Durable, flushed progress log for one corpus sweep.
pub struct Progress {
    file: Option<File>,
    start: Instant,
    findings: usize,
}

impl Progress {
    /// Open (truncate) `$TCL_FUZZ_PROGRESS_DIR/<name>.log` (default
    /// `<repo>/target/fuzz-progress/<name>.log`) and write a START line.
    pub fn new(name: &str) -> Self {
        let dir = std::env::var("TCL_FUZZ_PROGRESS_DIR").map_or_else(
            |_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/fuzz-progress"),
            PathBuf::from,
        );
        let _ = fs::create_dir_all(&dir);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dir.join(format!("{name}.log")))
            .ok();
        let mut p = Self {
            file,
            start: Instant::now(),
            findings: 0,
        };
        p.line(&format!(
            "START {name} (skip={}, limit={:?})",
            Self::skip(),
            Self::limit()
        ));
        p
    }

    fn line(&mut self, msg: &str) {
        let out = format!("[{:>6}s] {msg}", self.start.elapsed().as_secs());
        eprintln!("{out}");
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "{out}");
            let _ = f.flush();
        }
    }

    /// Record one completed chunk (usually a file). Flushed immediately.
    pub fn tick(&mut self, done: usize, total: usize, detail: &str) {
        self.line(&format!(
            "{done}/{total} findings={} {detail}",
            self.findings
        ));
    }

    /// Record a divergence/finding — written and flushed at once, so a kill can
    /// never lose it. Returns the running finding count.
    pub fn finding(&mut self, msg: &str) -> usize {
        self.findings += 1;
        self.line(&format!("FINDING #{}: {msg}", self.findings));
        self.findings
    }

    pub fn findings(&self) -> usize {
        self.findings
    }

    /// Final summary line.
    pub fn finish(&mut self, summary: &str) {
        self.line(&format!("DONE {summary} total_findings={}", self.findings));
    }

    /// Starting file index from `$TCL_FUZZ_SKIP` (default 0) — resume a killed
    /// sweep roughly where it stopped.
    pub fn skip() -> usize {
        std::env::var("TCL_FUZZ_SKIP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Optional cap on files processed from `$TCL_FUZZ_LIMIT` — bound a chunk.
    pub fn limit() -> Option<usize> {
        std::env::var("TCL_FUZZ_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
    }

    /// Apply `skip`/`limit` to a gathered file list, returning the slice this
    /// chunk should process along with `(absolute_start_index, total_len)`.
    pub fn slice<T: Clone>(files: &[T]) -> (Vec<T>, usize, usize) {
        let total = files.len();
        let start = Self::skip().min(total);
        let end = Self::limit().map_or(total, |n| (start + n).min(total));
        (files[start..end].to_vec(), start, total)
    }
}

use tcl_compiler::analyser::types::AnalysisResult;

/// A compact but **actionable** description of how `got` (per-item) diverges from
/// `want` (full) — which fields differ, with sample keys/entries — so a finding
/// written to the progress log is enough to start debugging without re-running.
pub fn describe_analysis_divergence(
    name: &str,
    got: &AnalysisResult,
    want: &AnalysisResult,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    macro_rules! map_field {
        ($f:ident) => {
            if got.$f != want.$f {
                let gk: std::collections::BTreeSet<_> = got.$f.keys().cloned().collect();
                let wk: std::collections::BTreeSet<_> = want.$f.keys().cloned().collect();
                let only_pi: Vec<_> = gk.difference(&wk).take(6).cloned().collect();
                let only_full: Vec<_> = wk.difference(&gk).take(6).cloned().collect();
                let mut valdiff: Vec<String> = Vec::new();
                for k in gk.intersection(&wk) {
                    if got.$f[k] != want.$f[k] {
                        valdiff.push(format!("{k:?}"));
                        if valdiff.len() >= 6 {
                            break;
                        }
                    }
                }
                parts.push(format!(
                    "{}[+per_item={only_pi:?} -full={only_full:?} valdiff={valdiff:?}]",
                    stringify!($f)
                ));
            }
        };
    }
    macro_rules! vec_field {
        ($f:ident) => {
            if got.$f != want.$f {
                let g: Vec<String> = got.$f.iter().map(|x| format!("{x:?}")).collect();
                let w: Vec<String> = want.$f.iter().map(|x| format!("{x:?}")).collect();
                let only_pi: Vec<_> = g
                    .iter()
                    .filter(|x| !w.contains(x))
                    .take(4)
                    .cloned()
                    .collect();
                let only_full: Vec<_> = w
                    .iter()
                    .filter(|x| !g.contains(x))
                    .take(4)
                    .cloned()
                    .collect();
                parts.push(format!(
                    "{}[len {}->{} +per_item={only_pi:?} -full={only_full:?}]",
                    stringify!($f),
                    w.len(),
                    g.len()
                ));
            }
        };
    }
    macro_rules! scalar_field {
        ($f:ident) => {
            if got.$f != want.$f {
                parts.push(format!(
                    "{}[per_item={:?} full={:?}]",
                    stringify!($f),
                    got.$f,
                    want.$f
                ));
            }
        };
    }

    map_field!(all_procs);
    map_field!(all_classes);
    map_field!(all_variables);
    map_field!(command_aliases);
    map_field!(instance_classes);
    map_field!(suppressed_lines);
    vec_field!(diagnostics);
    vec_field!(command_invocations);
    vec_field!(package_requires);
    vec_field!(package_provides);
    vec_field!(source_targets);
    vec_field!(namespace_imports);
    vec_field!(auto_path_entries);
    vec_field!(stub_commands);
    vec_field!(stub_expr_defs);
    vec_field!(regex_patterns);
    vec_field!(unresolved_command_sites);
    scalar_field!(has_dynamic_providers);
    scalar_field!(unknown_proc_info);
    if got.global_scope != want.global_scope {
        parts.push(format!(
            "global_scope[{}]",
            first_debug_diff(&got.global_scope, &want.global_scope)
        ));
    }

    if parts.is_empty() {
        parts.push("!= but no field pinpointed (a field not covered by the describer)".to_owned());
    }
    format!("{name}: {}", parts.join(" ; "))
}

/// The first line where the pretty-`Debug` of two values differs — a bounded,
/// deterministic pinpoint for tree-shaped fields (`Scope`) whose full diff is huge.
fn first_debug_diff<T: std::fmt::Debug>(got: &T, want: &T) -> String {
    let g = format!("{got:#?}");
    let w = format!("{want:#?}");
    for (i, (gl, wl)) in g.lines().zip(w.lines()).enumerate() {
        if gl != wl {
            return format!("line {i}: per_item={} full={}", gl.trim(), wl.trim());
        }
    }
    format!("len {} vs {}", g.len(), w.len())
}

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;

/// The merged analyser + compiler-checks diagnostic codes for `src` under
/// `dialect`, excluding optimisation-only findings — the canonical helper the
/// `analyser` / `checks` / `fp` test files share (previously copy-pasted).
pub fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if diagnostic `code` appears in [`codes`] for `src` under `dialect`.
pub fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// How many times diagnostic `code` appears in [`codes`] for `src`/`dialect`.
pub fn count(src: &str, dialect: &str, code: &str) -> usize {
    codes(src, dialect)
        .iter()
        .filter(|c| c.as_str() == code)
        .count()
}
