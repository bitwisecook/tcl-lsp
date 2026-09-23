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

//! Standalone analysis and the memoised editor path answer alike
//! (`docs/design/compiler/value-transfers.md` § *One invocation, one
//! context*): the per-procedure lattice the memoised [`function_lattice`]
//! query builds is the lattice a direct [`CompilationUnit`] build computes,
//! under the same command trust, and an edit that changes a whole-module
//! fact the lattice reads re-keys it. The lightweight token tier never
//! reaches the lattice at all.

use std::sync::Mutex;

use tcl_compiler::analyses::{ConstValue, LatticeValue};

use super::*;

/// The memoised unit and a direct build of `file` under its own dialect,
/// both from one database.
fn both_paths(db: &TclDatabase, file: SourceFile) -> (Arc<CompilationUnit>, CompilationUnit) {
    let dialect = file.dialect(db).clone();
    let cfg_key = lexer_cfg_key(db, &dialect);
    let memoised = compilation_unit(db, file, cfg_key);
    let direct = CompilationUnit::build_with_options(
        file.text(db),
        unit_build_options(
            db,
            file,
            cfg_key,
            db.registry(&dialect),
            None,
            &declared_command_surface(db, file),
        ),
    );
    (memoised, direct)
}

/// `qname`'s lattice, value by value: `(variable, version, value)`,
/// sorted.
fn lattice_of(unit: &CompilationUnit, qname: &str) -> Vec<(String, u32, String)> {
    let fu = unit.procedures.get(qname).expect(qname);
    let mut values: Vec<(String, u32, String)> = fu
        .sccp
        .values
        .iter()
        .map(|((sym, ver), value)| (fu.ssa.var_name(*sym).to_owned(), *ver, format!("{value:?}")))
        .collect();
    values.sort();
    values
}

/// The value `name`'s version `version` holds in `qname`.
fn value_at(unit: &CompilationUnit, qname: &str, name: &str, version: u32) -> Option<LatticeValue> {
    let fu = unit.procedures.get(qname).expect(qname);
    let sym = fu.ssa.var_symbol(name).expect(name);
    fu.sccp.values.get(&(sym, version)).cloned()
}

/// The answers recorded for `command`'s statements in `qname`.
fn answers_for(unit: &CompilationUnit, qname: &str, command: &str) -> Vec<String> {
    unit.procedures
        .get(qname)
        .expect(qname)
        .sccp
        .explanations
        .iter()
        .filter(|explanation| explanation.command == command)
        .map(|explanation| explanation.answer.clone())
        .collect()
}

/// On program (3) of the value-transfer contract and the `incr` /
/// `append` / `lappend` / `string range` witnesses, the memoised lattice
/// is the direct one, value for value, under a release that names its
/// grammar and under a profile that does not.
#[test]
fn direct_and_memoised_lattices_agree_on_the_cell_update_witnesses() {
    const SRC: &str = "proc p {} {\n    set n 1\n    incr n\n    incr n 2\n    set s hello\n    append s { world}\n    set l {}\n    lappend l a b\n    lappend l {c d}\n    set z 010\n    incr z\n    set big 9223372036854775807\n    incr big\n    set r [string range abcdefghijkl 010 end]\n    set x 10\n    foreach a {1 2} { incr x $a }\n    return $n\n}\n\
                       proc q {} {\n    set acc {}\n    foreach {a b} {1 10 2 20} { incr acc [expr {$b / $a}] }\n    return $acc\n}\n";
    for dialect in ["tcl8.6", "tcl9.0", "f5-irules"] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), dialect.to_owned(), None);
        let (memoised, direct) = both_paths(&db, file);
        for qname in ["::p", "::q"] {
            let want = lattice_of(&direct, qname);
            assert_eq!(lattice_of(&memoised, qname), want, "{dialect}: {qname}");
            // `n` and `z` are `p`'s; `q` pins the parity alone.
            if qname == "::p" {
                assert!(
                    want.iter()
                        .any(|(name, _, value)| name == "n" && value.contains("Int(4)")),
                    "{dialect}: program (3) folds to 4 on both paths: {want:?}"
                );
                let z = want
                    .iter()
                    .filter(|(name, _, _)| name == "z")
                    .map(|(_, _, value)| value.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                match dialect {
                    "tcl8.6" => assert!(z.contains("Int(9)"), "{dialect}: {z}"),
                    "tcl9.0" => assert!(z.contains("Int(11)"), "{dialect}: {z}"),
                    _ => assert!(
                        !z.contains("Int(9)") && !z.contains("Int(11)"),
                        "{dialect}: {z}"
                    ),
                }
            }
        }
    }
}

/// The memoised lattice takes the module's command trust as the direct
/// one does: with `proc incr` defined at the top level, `incr n` calls
/// that procedure, so neither path folds `n#2` and both say why (#2164).
#[test]
fn the_memoised_lattice_declines_a_renamed_head() {
    let body = "proc p {} {set n 1; incr n; return $n}\n";
    let shadowed = format!("proc incr {{v args}} {{return 99}}\n{body}");
    for dialect in ["tcl8.6", "tcl9.0"] {
        for (src, want, answer) in [
            (
                shadowed.as_str(),
                LatticeValue::Overdefined,
                "declined: rebinding-suspected",
            ),
            (body, LatticeValue::Const(ConstValue::Int(2)), "evaluated"),
        ] {
            let db = TclDatabase::default();
            let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
            let (memoised, direct) = both_paths(&db, file);
            for (path, unit) in [("memoised", &*memoised), ("direct", &direct)] {
                assert_eq!(
                    value_at(unit, "::p", "n", 2),
                    Some(want.clone()),
                    "{dialect} {path}: {src}"
                );
                assert_eq!(
                    answers_for(unit, "::p", "incr"),
                    [answer],
                    "{dialect} {path}: {src}"
                );
            }
        }
    }
}

/// The keyed updates of `dict` answer alike on both paths: `dict set` then
/// `dict incr` over an empty dictionary gives `a 3`. A local no store has
/// bound is not proven absent below the existence rung (slice 8), so a
/// keyed update over one declines — on both paths.
#[test]
fn keyed_updates_agree_on_both_paths() {
    let bound = "proc p {} {set d {}; dict set d a 1; dict incr d a 2; return $d}\n";
    let unbound = "proc p {} {dict set d a 1; return $d}\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        for (src, version, want) in [
            (
                bound,
                3,
                LatticeValue::Const(ConstValue::String("a 3".to_owned())),
            ),
            (unbound, 1, LatticeValue::Overdefined),
        ] {
            let db = TclDatabase::default();
            let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
            let (memoised, direct) = both_paths(&db, file);
            assert_eq!(
                lattice_of(&memoised, "::p"),
                lattice_of(&direct, "::p"),
                "{dialect}: {src}"
            );
            assert_eq!(
                value_at(&direct, "::p", "d", version),
                Some(want.clone()),
                "{dialect}: {src}"
            );
        }
    }
}

/// The value-position routes answer alike on both paths: `[set x]` reads
/// the variable, `list`, `llength` and `string length` run the shared
/// cores, and the `::tcl::dict::` spelling of a keyed update shares its
/// declaration with `dict set` — where the profile has `dict`: the iRules
/// profile has none, so `d` holds no value there.
#[test]
fn value_position_routes_agree_on_both_paths() {
    let src = "proc p {} {\n    set x hello\n    set r [set x]\n    set l [list a {b c}]\n    set n [llength $l]\n    set m [string length $x]\n    set d {}\n    ::tcl::dict::set d k v\n    return $d\n}\n";
    for dialect in ["tcl8.6", "tcl9.0", "f5-irules"] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        let (memoised, direct) = both_paths(&db, file);
        assert_eq!(
            lattice_of(&memoised, "::p"),
            lattice_of(&direct, "::p"),
            "{dialect}"
        );
        let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
        let keyed = if dialect == "f5-irules" {
            LatticeValue::Overdefined
        } else {
            text("k v")
        };
        for (path, unit) in [("memoised", &*memoised), ("direct", &direct)] {
            for (name, version, want) in [
                ("r", 1, text("hello")),
                ("l", 1, text("a {b c}")),
                ("n", 1, LatticeValue::Const(ConstValue::Int(2))),
                ("m", 1, LatticeValue::Const(ConstValue::Int(5))),
                ("d", 2, keyed.clone()),
            ] {
                assert_eq!(
                    value_at(unit, "::p", name, version),
                    Some(want),
                    "{dialect} {path}: {name}#{version}"
                );
            }
        }
    }
}

/// A variable trace installed anywhere in the module is a whole-module
/// fact the lattice reads: adding one for `n` re-keys `p`'s memoised
/// lattice, which drops `n`'s constant as the direct build does.
#[test]
fn a_trace_installation_invalidates_the_lattice() {
    use salsa::Setter as _;
    let body = "proc p {} {set n 1; incr n; return $n}\n";
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, body.to_owned(), "tcl8.6".to_owned(), None);
    let (memoised, direct) = both_paths(&db, file);
    for unit in [&*memoised, &direct] {
        assert_eq!(
            value_at(unit, "::p", "n", 2),
            Some(LatticeValue::Const(ConstValue::Int(2)))
        );
    }
    file.set_text(&mut db)
        .to(format!("{body}trace add variable n write cb\n"));
    let (memoised, direct) = both_paths(&db, file);
    assert_eq!(lattice_of(&memoised, "::p"), lattice_of(&direct, "::p"));
    for (path, unit) in [("memoised", &*memoised), ("direct", &direct)] {
        assert!(
            !matches!(value_at(unit, "::p", "n", 2), Some(LatticeValue::Const(_))),
            "{path}: a traced `n` is no constant: {:?}",
            lattice_of(unit, "::p")
        );
    }
}

/// The lightweight token tier stays structure-only: answering
/// `file_token_facts` runs no query that builds a lattice, so no route is
/// evaluated or recorded behind a token request.
#[test]
fn file_token_facts_never_evaluates() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let db = TclDatabase::with_event_logger(move |key| sink.lock().unwrap().push(key));
    let file = SourceFile::new(
        &db,
        "proc p {} {set n 1; incr n; dict set d a 1; return $n}\np\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_token_facts(&db, file);
    let executed = log.lock().unwrap().clone();
    assert!(
        executed.iter().any(|key| key.contains("file_token_facts")),
        "{executed:?}"
    );
    for deep in [
        "compilation_unit",
        "function_lattice",
        "file_analysis_incremental",
        "item_body_analysis",
    ] {
        assert!(
            !executed.iter().any(|key| key.contains(deep)),
            "`file_token_facts` ran `{deep}`: {executed:?}"
        );
    }
}
