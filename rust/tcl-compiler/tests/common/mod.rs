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

//! Shared integration-test helpers.
//!
//! [`wasm_link`] carries the WASM whole-program real-link toolchain — the
//! gate, the reserved-runtime build, and the scratch paths — shared by
//! `wasm_real_link.rs` and `wasm_tiers.rs`.
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

pub mod wasm_link;

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

/// Describe one diverging `HashMap` field: keys only on one side, plus keys
/// whose values differ.  Module-scope so both halves of the field sweep can
/// use it without repeating the body (and to keep each function within the
/// clippy line budget).
macro_rules! map_field {
    ($parts:ident, $got:ident, $want:ident, $f:ident) => {
        if $got.$f != $want.$f {
            let gk: std::collections::BTreeSet<_> = $got.$f.keys().cloned().collect();
            let wk: std::collections::BTreeSet<_> = $want.$f.keys().cloned().collect();
            let only_pi: Vec<_> = gk.difference(&wk).take(6).cloned().collect();
            let only_full: Vec<_> = wk.difference(&gk).take(6).cloned().collect();
            let mut valdiff: Vec<String> = Vec::new();
            for k in gk.intersection(&wk) {
                if $got.$f[k] != $want.$f[k] {
                    valdiff.push(format!("{k:?}"));
                    if valdiff.len() >= 6 {
                        break;
                    }
                }
            }
            $parts.push(format!(
                "{}[+per_item={only_pi:?} -full={only_full:?} valdiff={valdiff:?}]",
                stringify!($f)
            ));
        }
    };
}

/// Describe one diverging sequence/set field: lengths plus a bounded sample
/// of entries present on only one side.
macro_rules! vec_field {
    ($parts:ident, $got:ident, $want:ident, $f:ident) => {
        if $got.$f != $want.$f {
            let g: Vec<String> = $got.$f.iter().map(|x| format!("{x:?}")).collect();
            let w: Vec<String> = $want.$f.iter().map(|x| format!("{x:?}")).collect();
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
            $parts.push(format!(
                "{}[len {}->{} +per_item={only_pi:?} -full={only_full:?}]",
                stringify!($f),
                w.len(),
                g.len()
            ));
        }
    };
}

/// Describe one diverging scalar field verbatim.
macro_rules! scalar_field {
    ($parts:ident, $got:ident, $want:ident, $f:ident) => {
        if $got.$f != $want.$f {
            $parts.push(format!(
                "{}[per_item={:?} full={:?}]",
                stringify!($f),
                $got.$f,
                $want.$f
            ));
        }
    };
}

/// A compact but **actionable** description of how `got` (per-item) diverges from
/// `want` (full) — which fields differ, with sample keys/entries — so a finding
/// written to the progress log is enough to start debugging without re-running.
pub fn describe_analysis_divergence(
    name: &str,
    got: &AnalysisResult,
    want: &AnalysisResult,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Every field of `AnalysisResult`, in declaration order, so a divergence
    // can never hide behind "a field not covered by the describer".
    if got.global_scope != want.global_scope {
        parts.push(format!(
            "global_scope[{}]",
            first_scope_diff(&got.global_scope, &want.global_scope, "<global>")
        ));
    }
    map_field!(parts, got, want, all_procs);
    vec_field!(parts, got, want, proc_declaration_sites);
    map_field!(parts, got, want, superseded_procs);
    map_field!(parts, got, want, all_classes);
    vec_field!(parts, got, want, class_body_spans);
    map_field!(parts, got, want, all_variables);
    vec_field!(parts, got, want, all_defined_symbols);
    vec_field!(parts, got, want, diagnostics);
    vec_field!(parts, got, want, command_invocations);
    vec_field!(parts, got, want, package_requires);
    vec_field!(parts, got, want, package_provides);
    vec_field!(parts, got, want, package_ifneededs);
    scalar_field!(parts, got, want, has_dynamic_providers);
    vec_field!(parts, got, want, source_targets);
    map_field!(parts, got, want, command_aliases);
    map_field!(parts, got, want, alias_offsets);
    map_field!(parts, got, want, renamed_commands);
    map_field!(parts, got, want, rename_offsets);
    map_field!(parts, got, want, ensemble_subcommand_targets);
    vec_field!(parts, got, want, namespace_imports);
    vec_field!(parts, got, want, namespace_forgets);
    map_field!(parts, got, want, destroyed_commands);
    vec_field!(parts, got, want, namespace_exports);
    map_field!(parts, got, want, namespace_paths);
    vec_field!(parts, got, want, auto_path_entries);
    vec_field!(parts, got, want, qualified_var_refs);
    vec_field!(parts, got, want, namespace_refs);
    vec_field!(parts, got, want, namespace_overrides);
    vec_field!(parts, got, want, stub_commands);
    vec_field!(parts, got, want, stub_expr_defs);
    vec_field!(parts, got, want, regex_patterns);
    map_field!(parts, got, want, suppressed_lines);
    scalar_field!(parts, got, want, unknown_proc_info);
    map_field!(parts, got, want, instance_classes);
    if got.object_handle_facts != want.object_handle_facts {
        parts.push("object_handle_facts[differ]".to_owned());
    }
    vec_field!(parts, got, want, created_instance_commands);
    vec_field!(parts, got, want, instance_command_bindings);
    vec_field!(parts, got, want, ambiguous_instance_names);
    map_field!(parts, got, want, object_methods);
    vec_field!(parts, got, want, unresolved_command_sites);
    vec_field!(parts, got, want, scoped_command_regions);
    map_field!(parts, got, want, scoped_sibling_defs);
    // `hierarchy_cache` is skipped: a derived cache, its `PartialEq` is
    // constitutionally `true` (see `types.rs`), so it can never diverge.
    scalar_field!(parts, got, want, dialect);
    if got.library_versions != want.library_versions {
        parts.push("library_versions[differ]".to_owned());
    }

    if parts.is_empty() {
        parts.push("!= but no field pinpointed (a field not covered by the describer)".to_owned());
    }
    format!("{name}: {}", parts.join(" ; "))
}

/// Deterministic first-difference pinpoint for the `Scope` tree: walks the two
/// trees in lock-step (map fields through sorted key views, children in
/// order) and names the path + field of the first divergence.  Replaces a
/// `Debug`-line diff whose "first differing line" sampled whichever `HashMap`
/// entry the hasher enumerated first — different every run.
fn first_scope_diff(
    got: &tcl_compiler::analyser::types::Scope,
    want: &tcl_compiler::analyser::types::Scope,
    path: &str,
) -> String {
    use std::collections::BTreeMap;
    macro_rules! scope_map_diff {
        ($f:ident) => {
            if got.$f != want.$f {
                let g: BTreeMap<_, _> = got.$f.iter().collect();
                let w: BTreeMap<_, _> = want.$f.iter().collect();
                for (k, gv) in &g {
                    match w.get(k) {
                        None => {
                            return format!(
                                "{path}.{}[{k:?}]: per_item-only={gv:?}",
                                stringify!($f)
                            );
                        }
                        Some(wv) if gv != wv => {
                            return format!(
                                "{path}.{}[{k:?}]: per_item={gv:?} full={wv:?}",
                                stringify!($f)
                            );
                        }
                        Some(_) => {}
                    }
                }
                if let Some((k, wv)) = w.iter().find(|(k, _)| !g.contains_key(*k)) {
                    return format!("{path}.{}[{k:?}]: full-only={wv:?}", stringify!($f));
                }
            }
        };
    }
    if got.kind != want.kind || got.name != want.name || got.body_span != want.body_span {
        return format!(
            "{path}: header per_item=({:?}, {:?}, {:?}) full=({:?}, {:?}, {:?})",
            got.kind, got.name, got.body_span, want.kind, want.name, want.body_span
        );
    }
    scope_map_diff!(variables);
    scope_map_diff!(procs);
    scope_map_diff!(classes);
    if got.defined_symbols != want.defined_symbols {
        return format!("{path}.defined_symbols differ");
    }
    if got.oo_global_resolution != want.oo_global_resolution
        || got.oo_method_frame != want.oo_method_frame
        || got.oo_defining_class != want.oo_defining_class
    {
        return format!("{path}: oo flags differ");
    }
    if got.children.len() != want.children.len() {
        let gn: Vec<_> = got.children.iter().map(|c| c.name.as_str()).collect();
        let wn: Vec<_> = want.children.iter().map(|c| c.name.as_str()).collect();
        return format!("{path}.children: per_item={gn:?} full={wn:?}");
    }
    for (i, (gc, wc)) in got.children.iter().zip(&want.children).enumerate() {
        if gc != wc {
            let child = format!("{path}/{}#{i}", gc.name);
            return first_scope_diff(gc, wc, &child);
        }
    }
    format!("{path}: no difference found (walk out of sync with Scope fields?)")
}
