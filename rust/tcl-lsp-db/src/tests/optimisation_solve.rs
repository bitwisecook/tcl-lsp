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

//! The per-procedure optimisation memo and the callee projection it reads.

use super::*;

/// The per-procedure optimiser memo (`function_optimisations`) must
/// skip an unrelated procedure across edits: a body edit that does not change a
/// proc's *opt-projection* (`OptDepsKey`) leaves every other proc's optimise a
/// cache hit, while the edited proc's own re-keys (its `FnLatticeKey` changed).
/// This is the per-edit incrementality win — the whole-module `optimise_unit`
/// re-optimised all procs on any edit.
#[test]
fn function_optimisations_reused_on_unrelated_edit() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let runs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("function_optimisations"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    // Three independent procs (no foldable cross-proc calls, no pure-non-const
    // proc → the memo path, not the whole-module fallback).
    let file = SourceFile::new(
        &db,
        "proc a {} { puts 11111 }\n\
             proc b {} { puts 22222 }\n\
             proc c {} { puts 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        3,
        "cold build: every procedure's optimise runs once"
    );

    // Edit only `b`'s body (a literal change that does not alter b's
    // opt-projection) — `a`/`c` are cache hits; only `b`'s optimise re-runs.
    file.set_text(&mut db).to("proc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        1,
        "unrelated body edit -> exactly ONE function_optimisations recomputes"
    );
}

/// The opt projection a memoised procedure reads its callees through keeps
/// what the dead-store passes ask of them: the completion, and each
/// parameter's default, which the call-by-name reads take for an omitted
/// `Name` argument.
#[test]
fn the_opt_projection_keeps_completion_and_defaults() {
    let mut summary = ProcSummary::unknown("::bumpd");
    summary.params = vec!["name".to_owned()];
    summary
        .param_defaults
        .insert("name".to_owned(), "n".to_owned());
    summary.completes = true;
    let projected = opt_callee_to_summary(&opt_callee_from_summary(&summary));
    assert_eq!(projected.param_defaults, summary.param_defaults);
    assert!(projected.completes);
}

/// A callee that completes whatever its arguments hold makes the unused
/// store of its call dead in the per-procedure optimiser memo as in the
/// whole-module build: the memo reads a callee's summary through its
/// opt projection, which carries the completion with the purity.
#[test]
fn a_completing_callee_reaches_the_optimiser_memo() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    // `k` is pure with an argument-independent constant return and `f`
    // prints, so no procedure is an argument-sensitive fold target and the
    // module keeps the memo path rather than the whole-module fallback.
    let text = "proc k {} { return 1 }\nproc f {} {\n    set u [k]\n    puts done\n}\n";
    let file = SourceFile::new(&db, text.to_owned(), "tcl8.6".to_owned(), None);
    let memoised = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|key| key.contains("function_optimisations"))
            .count(),
        2,
        "each procedure's optimisations come from the memo"
    );
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let whole = compiler_check_diagnostics_uncached(text, registry, "tcl8.6", None, None);
    let codes = |optimisations: &[Optimisation]| -> Vec<(String, u32, u32)> {
        optimisations
            .iter()
            .map(|o| (o.code.as_str().to_owned(), o.span.start(), o.span.end()))
            .collect()
    };
    assert_eq!(codes(&memoised.optimisations), codes(&whole.optimisations));
    assert!(
        memoised
            .optimisations
            .iter()
            .any(|o| o.code.as_str() == "O126"),
        "the unused store of a completing call goes; got {:?}",
        memoised.optimisations
    );
}
