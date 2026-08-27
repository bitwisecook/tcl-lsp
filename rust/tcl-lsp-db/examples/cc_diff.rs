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

//! Repro for the pre-existing `function_lattice` memo divergence: dumps the
//! per-file difference between the memoised `compiler_check_diagnostics` and a
//! fresh whole-module `compiler_check_diagnostics_uncached` build (checks +
//! optimisations).  Usage:
//! `FILE=tmp/tcl8.6.16/library/safe.tcl cargo run -p tcl-lsp-db --example cc_diff`.
//! See `docs/design/rust/incremental-analysis.md` and the `compiler_check_corpus`
//! known-failing guard.

use std::collections::BTreeSet;

use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb, compiler_check_diagnostics,
    compiler_check_diagnostics_uncached,
};

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rel = std::env::var("FILE").expect("set FILE (relative to repo root)");
    let dialect = std::env::var("DIALECT").unwrap_or_else(|_| "tcl8.6".to_owned());
    let src = std::fs::read_to_string(root.join(&rel)).expect("read FILE");

    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.clone(), None);
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
    );
    let memo = compiler_check_diagnostics(&db, file, cfg);
    let reg = db.registry(&dialect);
    let fresh = compiler_check_diagnostics_uncached(&src, reg, &dialect, None, None);

    let ck = |d: &tcl_compiler::compiler_checks::Diagnostic| (d.span.start(), d.span.end(), d.code);
    let ms: BTreeSet<_> = memo.checks.iter().map(ck).collect();
    let fs: BTreeSet<_> = fresh.checks.iter().map(ck).collect();
    println!(
        "CHECKS: {} memo-only, {} fresh-only",
        ms.difference(&fs).count(),
        fs.difference(&ms).count()
    );
    for k in ms.difference(&fs).take(6) {
        println!("  memo-only  {k:?}");
    }
    for k in fs.difference(&ms).take(6) {
        println!("  fresh-only {k:?}");
    }

    let ok = |o: &tcl_compiler::optimiser::Optimisation| (o.span.start(), o.span.end(), o.code);
    let mo: BTreeSet<_> = memo.optimisations.iter().map(ok).collect();
    let fo: BTreeSet<_> = fresh.optimisations.iter().map(ok).collect();
    println!(
        "OPTS:   {} memo-only, {} fresh-only",
        mo.difference(&fo).count(),
        fo.difference(&mo).count()
    );
}
