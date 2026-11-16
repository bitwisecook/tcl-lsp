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

//! The value-transfer witnesses, program by program
//! (`docs/design/compiler/value-transfers-examples.md`).
//!
//! Each test drives the compiler the way a user reaches it — the shared
//! lattice through `CompilationUnit::build`, the rewrites through the
//! multipass optimiser — and, where the witness is a program's output,
//! runs the original and the optimised program under every `tclsh` release
//! found on `PATH`. A release that is not installed is skipped and the
//! test says which ran; none at all is reported, never skipped silently.

use crate::body_identity;
use body_identity::fixture_id;

use std::io::Write as _;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::interprocedural::{ConstantReturn, ProcSummary};
use tcl_compiler::intervals::{Interval, compute_intervals_with, numbers_for_dialect};
use tcl_compiler::ir::Statement;
use tcl_compiler::lowering::lower_to_ir_with_dialect;
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::optimiser::manager::{
    optimise_raw, optimise_source_multipass, optimise_unit, optimise_with_dialect,
};
use tcl_compiler::static_loops::{
    DEFAULT_MAX_STATIC_LOOP_ITERS, LoopSemantics, StaticEnv, StaticValue, summarise_for_statement,
};
use tcl_compiler::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;
use tcl_registry::model::ingress::{resolve_environment, static_context_for};
use tcl_registry::value_transfer::{BindingKind, Existence};

mod branch_selection;
mod catch;
mod cell_updates;
mod dead_stores;
mod declared_routes;
mod existence;
mod expressions;
mod loops;
mod ordered_state;
mod preserving_outcomes;
mod procedure_summaries;
mod refinements;
mod try_and_errors;
mod typed_statements;
mod unseen_code;

/// The releases the oracle runs, oldest first.
const RELEASES: [&str; 5] = ["8.4", "8.5", "8.6", "9.0", "9.1"];

/// The multipass optimiser's iteration ceiling for a witness program.
const PASSES: usize = 8;

/// The dialects the exit witnesses analyse under: a release per numeral
/// grammar and integer tower; `f5-irules`, a vendor dialect that evaluates
/// under the 8.4 base it declares (ruling 8); and the lenient `tcl` profile,
/// which declares no release, so every axis answers by unanimity (ruling 7).
const DIALECTS: [&str; 5] = ["tcl8.4", "tcl8.6", "tcl9.0", "f5-irules", "tcl"];

/// Run `script` from a file under `tclsh`: whether it exited cleanly and
/// what it printed. A script run from a file exits non-zero on an error,
/// where one read from stdin does not.
fn run_script(tclsh: &str, script: &str) -> Option<(bool, String)> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "value-transfer-witness-{}-{}.tcl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path)
        .and_then(|mut file| file.write_all(script.as_bytes()))
        .ok()?;
    let output = Command::new(tclsh).arg(&path).output();
    let _ = std::fs::remove_file(&path);
    let output = output.ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

/// Every release with its reference interpreter, oldest first, as the shared
/// oracle lookup finds it ([`tcl_test_support::witness_tclsh`]): a release
/// with none fails the test where `TCL_LSP_REQUIRE_TCLSH` requires it, and
/// is reported as skipped otherwise.
fn releases_on_path() -> Vec<(&'static str, String)> {
    RELEASES
        .iter()
        .filter_map(|&series| {
            let version = tcl_dialect::TclVersion::from_version_string(series)?;
            let tclsh = tcl_test_support::witness_tclsh(version)?;
            Some((series, tclsh.path.to_string_lossy().into_owned()))
        })
        .collect()
}

/// The dialect profile name a release is analysed under.
fn dialect_of(series: &str) -> String {
    format!("tcl{series}")
}

/// `source` rewritten by the multipass optimiser under `dialect`, with
/// every rewrite it applied.
fn optimised(source: &str, dialect: &str) -> (String, Vec<Optimisation>) {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    optimise_source_multipass(source, registry, Some(profile), PASSES)
}

/// The single-pass rewrites of `source` under `dialect`, each span an
/// offset into `source` itself.
fn rewrites_of(source: &str, dialect: &str) -> Vec<Optimisation> {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    optimise_with_dialect(source, registry, Some(profile))
}

/// The compilation unit `CompilationUnit::build` makes of `source` under
/// `dialect` — the direct path every consumer reads the shared lattice
/// from.
fn unit_of(source: &str, dialect: &str) -> CompilationUnit {
    let registry = static_context_for(dialect).commands();
    CompilationUnit::build_for_dialect(source, registry, false, dialect)
}

/// The compilation unit of `source` under `dialect` with its procedure
/// summaries, as the optimiser and the explorer build it.
fn summarised_unit(source: &str, dialect: &str) -> CompilationUnit {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    unit_of(source, dialect).with_interprocedural(registry, Some(profile))
}

/// The summary of `proc` in `source` under `dialect`.
fn summary_of(source: &str, dialect: &str, proc: &str) -> ProcSummary {
    let unit = summarised_unit(source, dialect);
    let id = fixture_id(&unit.ir_module.procedures, proc);
    unit.interproc
        .expect("the summaries")
        .procedures
        .remove(&id)
        .expect("the procedure's summary")
}

/// The shared lattice's value for `var`'s version `version` in `proc`.
fn value_at(unit: &CompilationUnit, proc: &str, var: &str, version: u32) -> Option<LatticeValue> {
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.values.get(&(symbol, version)).cloned()
}

/// The shared lattice's value for the last version of `var` in `proc`
/// under `dialect`.
fn last_value(source: &str, dialect: &str, proc: &str, var: &str) -> LatticeValue {
    let unit = unit_of(source, dialect);
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function
        .sccp
        .values
        .iter()
        .filter(|((sym, _), _)| *sym == symbol)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, value)| value.clone())
        .expect("a definition")
}

/// The answers the run recorded for `command`'s statements in `proc`.
fn answers_for(unit: &CompilationUnit, proc: &str, command: &str) -> Vec<String> {
    unit.procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure")
        .sccp
        .explanations
        .iter()
        .filter(|explanation| explanation.command == command)
        .map(|explanation| explanation.answer.clone())
        .collect()
}

fn text(value: &str) -> LatticeValue {
    LatticeValue::Const(ConstValue::String(value.to_owned()))
}

/// Every release on `PATH` prints `expected` for `source` and for its
/// optimised form under that release's dialect.
fn prints_under_every_release(source: &str, expected: &str) {
    prints_under_releases_from(source, expected, "8.4");
}

/// [`prints_under_every_release`] for the releases from `first` on: a
/// program using an option a release lacks runs only where it exists.
fn prints_under_releases_from(source: &str, expected: &str, first: &str) {
    for (series, tclsh) in releases_on_path() {
        if series < first {
            continue;
        }
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, expected.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A model's answer for `x` after the statement: the text of its value, or
/// `None` where it declines.
fn lattice_text(value: LatticeValue) -> Option<String> {
    match value {
        LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
        LatticeValue::Const(ConstValue::String(s)) => Some(s),
        _ => None,
    }
}

/// A seed as the literal ingress reads it: a canonical integer is the
/// integer, and any other text is its text.
fn seeded(text: &str) -> StaticValue {
    tcl_registry::value_transfer::ExactValue::from_literal(text)
        .as_int()
        .map_or_else(|| StaticValue::Str(text.to_owned()), StaticValue::Int)
}

/// Serialises the tests that publish a hook plan: the published plan is the
/// process's, and a thread whose host was built from a plan follows it.
static PUBLISHED_PACKS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A workspace pack of `source` loaded as the language server and the CLI
/// load one: the pack set, its hook plan published and this thread's host
/// built from it.
fn pack_workspace(name: &str, source: &str) -> tcl_spectcl::PackSet {
    let packs = pack_workspace_noting(name, source);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    packs
}

/// [`pack_workspace`] for a pack whose load notices the caller accepts.
fn pack_workspace_noting(name: &str, source: &str) -> tcl_spectcl::PackSet {
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from(format!("/workspace/.tcl-lsp/{name}.tclspec")),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        source.to_owned(),
    )]);
    tcl_spectcl::hooks::publish(&packs);
    tcl_spectcl::hooks::ensure_thread_host();
    packs
}

/// The folded type of `var`'s version `version` in `proc`: the semantic
/// type and representation evidence the evaluation that produced it stated.
fn folded_at(
    unit: &CompilationUnit,
    proc: &str,
    var: &str,
    version: u32,
) -> Option<tcl_compiler::value_transfer::FoldedType> {
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.folded_types.get(&(symbol, version)).cloned()
}

/// The existence fact `var`'s last version holds in `proc`: the fact the
/// solver established where the version was defined.
fn last_existence(unit: &CompilationUnit, proc: &str, var: &str) -> Option<Existence> {
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function
        .sccp
        .existence
        .iter()
        .filter(|((sym, _), _)| *sym == symbol)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, fact)| *fact)
}

const SCALAR: Option<Existence> = Some(Existence::Bound(BindingKind::Scalar));
const ARRAY: Option<Existence> = Some(Existence::Bound(BindingKind::Array));
const UNBOUND: Option<Existence> = Some(Existence::Unbound);

/// The dialects the release table is analysed under: each release, and
/// the `tcl` profile that spans them all.
const RELEASE_DIALECTS: [&str; 6] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "tcl"];

/// Whether one pass of the optimiser under `dialect` deletes the statement
/// spelled `store` in `source` (O108, O109 or O126).
fn removes_store(source: &str, dialect: &str, store: &str) -> bool {
    let at = u32::try_from(source.find(store).expect("the store")).expect("an offset");
    rewrites_of(source, dialect).iter().any(|rewrite| {
        matches!(
            rewrite.code,
            DiagCode::O108 | DiagCode::O109 | DiagCode::O126
        ) && rewrite.span.start() <= at
            && at < rewrite.span.end()
    })
}

/// Whether the analyser reports `code` anywhere in `source` under
/// `dialect`.
fn reports(source: &str, dialect: &str, code: DiagCode) -> bool {
    tcl_compiler::analyser::Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

/// The truths the diagnostics claim for the conditions of `source` under
/// `dialect`: `true` for an `if` that is always true, `false` for one that
/// is always false.
fn condition_claims(source: &str, dialect: &str) -> Vec<bool> {
    tcl_compiler::analyser::Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::I230)
        .filter_map(|diagnostic| {
            if diagnostic.message.contains("is always true") {
                Some(true)
            } else if diagnostic.message.contains("is always false") {
                Some(false)
            } else {
                None
            }
        })
        .collect()
}

/// The shared lattice's value for `var`'s version `version` at the top level.
fn top_value_at(unit: &CompilationUnit, var: &str, version: u32) -> Option<LatticeValue> {
    let function = &unit.top_level;
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.values.get(&(symbol, version)).cloned()
}
