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

//! FP precision catalogue → Rust regression tests.
//!
//! Mirrors `docs/design/compiler/FP.md` (the false-positive / true-positive
//! catalogue), giving the analyser a standalone precision net. Each
//! test pins a *must-stay-silent* (FP) arm and, where the catalogue has one, a
//! *must-fire* (TP) control.
//!
//! Reproducers are copied verbatim from the FP.md catalogue entries. Where the
//! analyser legitimately diverges from the catalogue verdict — a different
//! structure, or a feature not yet implemented — the test captures the *actual*
//! behaviour and the divergence is called out in a comment (and, when it is a
//! genuine residual false positive, marked `#[ignore]` with the FP id so it is
//! tracked, not silently green). See `docs/design/rust/fp-rust-port-plan.md`.

use crate::analyser::Analyser;
use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;

/// Default dialect for catalogue reproducers that are not dialect-sensitive.
/// Entries that vary by Tcl version override this per-call.
pub(super) const D: &str = "tcl8.6";

/// Every diagnostic code the full pipeline surfaces for `src` under `dialect`,
/// mirroring the user-facing `tcl diag` path (`collect_rows`): the analyser
/// pass plus the `run_all_checks` compiler-checks pass (shimmer / taint /
/// dead-store), with optimisation codes excluded exactly as `diag` excludes
/// them.
pub(super) fn codes(src: &str, dialect: &str) -> Vec<String> {
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

/// True if `code` appears anywhere in the full diagnostic set for `src`.
pub(super) fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

#[cfg(test)]
mod sanity {
    use super::{D, codes, fires};

    #[test]
    fn baseline_read_before_set_fires_w210() {
        assert!(fires("proc f {} { puts $u }", D, "W210"));
    }

    #[test]
    fn guarded_read_does_not_fire_w210() {
        assert!(!fires(
            "proc f {} { if {[info exists u]} { puts $u } }",
            D,
            "W210"
        ));
    }

    #[test]
    fn clean_proc_is_silent() {
        assert!(
            codes("proc f {} { return ok }", D).is_empty(),
            "clean proc emitted: {:?}",
            codes("proc f {} { return ok }", D)
        );
    }
}

mod bnd;
mod ds;
mod inj;
mod nab;
mod obj;
mod opt;
mod rbs;
mod rch;
mod sh;
mod sty;
mod tnt;
