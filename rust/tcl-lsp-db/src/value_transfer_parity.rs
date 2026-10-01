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
    let memoised = compilation_unit(db, file, cfg_key, 0).expect("no overlay always builds");
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

/// The document's unit under `config`'s pack overlay, which these tests
/// install before they ask.
fn installed_unit(
    db: &TclDatabase,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<CompilationUnit> {
    document_compilation_unit_for(db, file, config).expect("the pack overlay is installed")
}

/// `qname`'s lattice, value by value: `(variable, version, value)`,
/// sorted.
fn lattice_of(unit: &CompilationUnit, qname: &str) -> Vec<(String, u32, String)> {
    lattice_values(unit.procedures.get(qname).expect(qname))
}

/// The top level's lattice, listed as [`lattice_of`] lists a procedure's.
fn top_lattice_of(unit: &CompilationUnit) -> Vec<(String, u32, String)> {
    lattice_values(&unit.top_level)
}

fn lattice_values(fu: &tcl_compiler::compilation_unit::FunctionUnit) -> Vec<(String, u32, String)> {
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

/// The existence fact `name`'s version `version` holds in `qname`.
fn existence_at(
    unit: &CompilationUnit,
    qname: &str,
    name: &str,
    version: u32,
) -> Option<tcl_registry::value_transfer::Existence> {
    let fu = unit.procedures.get(qname).expect(qname);
    let sym = fu.ssa.var_symbol(name).expect(name);
    fu.sccp.existence.get(&(sym, version)).copied()
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
/// grammar, under a vendor dialect that declares its base (`f5-irules`,
/// 8.4's: ruling 8), and under a profile that declares none (the lenient
/// `tcl`).
#[test]
fn direct_and_memoised_lattices_agree_on_the_cell_update_witnesses() {
    const SRC: &str = "proc p {} {\n    set n 1\n    incr n\n    incr n 2\n    set s hello\n    append s { world}\n    set l {}\n    lappend l a b\n    lappend l {c d}\n    set z 010\n    incr z\n    set big 9223372036854775807\n    incr big\n    set r [string range abcdefghijkl 010 end]\n    set x 10\n    foreach a {1 2} { incr x $a }\n    return $n\n}\n\
                       proc q {} {\n    set acc {}\n    foreach {a b} {1 10 2 20} { incr acc [expr {$b / $a}] }\n    return $acc\n}\n";
    for dialect in ["tcl8.6", "tcl9.0", "f5-irules", "tcl"] {
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
                // tclsh 8.4 to 8.6: `set z 010; incr z` is 9; 9.0 and 9.1: 11.
                match dialect {
                    "tcl8.6" | "f5-irules" => assert!(z.contains("Int(9)"), "{dialect}: {z}"),
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
/// `dict incr` over an empty dictionary gives `a 3`. The existence rung
/// proves a local no store has bound absent, and every keyed update creates
/// its dictionary, so `dict set d a 1` over one is `a 1` — on both paths, as
/// tclsh 8.5 to 9.1 print.
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
            (
                unbound,
                1,
                LatticeValue::Const(ConstValue::String("a 1".to_owned())),
            ),
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

/// Serialises the tests that publish a pack set: the published plan is the
/// process's, and every worker builds its host from it.
static PUBLISHED_PACKS: Mutex<()> = Mutex::new(());

/// The workspace pack declaring `tenant::label` (`value-evaluation.md`
/// § *A private command*), its body folding `PREFIX` before the name, or
/// without its `evaluate` row.
fn tenant_pack(evaluate: Option<&str>) -> tcl_spectcl::PackSet {
    tenant_pack_with_body(
        evaluate
            .map(|prefix| format!("fold [string cat \"{prefix}\" $name]"))
            .as_deref(),
    )
}

/// The same pack with `body` as the implementation's body, or without the
/// `evaluate` row.
fn tenant_pack_with_body(body: Option<&str>) -> tcl_spectcl::PackSet {
    let evaluate = body.map_or(String::new(), |body| {
        format!(
            "        evaluate -implementation tenant.label.v1 -host bounded_tcl {{\n\
             \x20           inputs {{arg 0 exact}}\n\
             \x20           depends {{tcl_profile implementation_identity}}\n\
             \x20           budget {{-commands 2000 -wall-clock 20 -value-bytes 65536}}\n\
             \x20           body {{name}} {{ {body} }}\n\
             \x20       }}\n"
        )
    });
    let source = format!(
        "speclib tenant 2.2 {{\n\
         \x20   command tenant::label {{\n\
         \x20       arity 1\n\
         \x20       semantics {{\n\
         \x20           effects {{no_store_writes no_external_io}}\n\
         \x20           result -semantic string\n\
         \x20       }}\n\
         {evaluate}\
         \x20   }}\n\
         }}\n"
    );
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/tenant.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        source,
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    packs
}

/// Make `packs` the workspace's, as the server does on a load: the
/// overlaid registry for `dialect`, the published hook plan, and this
/// worker's host built from it. The pack's content key is the overlay.
fn install_workspace_packs(packs: &tcl_spectcl::PackSet, dialect: &str) -> u64 {
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, packs);
    tcl_spectcl::hooks::publish(packs);
    tcl_spectcl::hooks::ensure_thread_host();
    packs.key
}

/// A config whose only setting is the workspace's pack overlay.
fn overlay_config(db: &TclDatabase, overlay: u64) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        overlay,
        Vec::new(),
        Vec::new(),
    )
}

/// A workspace pack's declared evaluator reaches the memoised editor path:
/// with `tenant::label` declared by the pack, `if {[tenant::label x] eq
/// "tenant:x"} …` is a constant condition the database's analysis reports
/// as I230 — at the top level and in a procedure body, whose lattice the
/// memoised `function_lattice` builds — and once the pack's `evaluate` row
/// is removed, it is not.
#[test]
fn a_workspace_pack_evaluator_reaches_i230_on_the_memoised_path() {
    use salsa::Setter as _;
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let src = "if {[tenant::label x] eq \"tenant:x\"} {puts yes} else {puts no}\n\
               proc p {} {\n    if {[tenant::label y] eq \"tenant:y\"} {puts yes} else {puts no}\n}\n";
    let constant_branches = |db: &TclDatabase, file: SourceFile, config: AnalyserConfig| {
        file_analysis_incremental(db, file, config)
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::I230)
            .count()
    };
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let config = overlay_config(
        &db,
        install_workspace_packs(&tenant_pack(Some("tenant:")), "tcl9.0"),
    );
    assert_eq!(
        constant_branches(&db, file, config),
        2,
        "both conditions fold through the pack's evaluator"
    );
    let unit = installed_unit(&db, file, config);
    let tally = unit.procedures.get("::p").expect("::p").sccp.route_tally;
    assert!(
        tally.implementation > 0,
        "the procedure's memoised lattice ran the declared implementation: {tally:?}"
    );

    let without = install_workspace_packs(&tenant_pack(None), "tcl9.0");
    config.set_spec_pack_key(&mut db).to(without);
    assert_eq!(
        constant_branches(&db, file, config),
        0,
        "without the `evaluate` row nothing evaluates `tenant::label`"
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A pack edit invalidates every lattice of the file: the memoised
/// procedure lattice folds `[tenant::label acme]` through the pack's body,
/// and after the body changes — a new pack content, a new overlay — the
/// same query answers from the new body, never from the old lattice.
#[test]
fn a_pack_edit_invalidates_the_lattice() {
    use salsa::Setter as _;
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let src = "proc p {} {set r [tenant::label acme]; return $r}\n";
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let config = overlay_config(
        &db,
        install_workspace_packs(&tenant_pack(Some("tenant:")), "tcl9.0"),
    );
    let unit = installed_unit(&db, file, config);
    assert_eq!(value_at(&unit, "::p", "r", 1), Some(text("tenant:acme")));

    let edited = install_workspace_packs(&tenant_pack(Some("t:")), "tcl9.0");
    config.set_spec_pack_key(&mut db).to(edited);
    let unit = installed_unit(&db, file, config);
    assert_eq!(value_at(&unit, "::p", "r", 1), Some(text("t:acme")));
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A pool thread whose host was built from a superseded plan answers with
/// the new plan. The server builds a thread's host where a worker closure
/// asks for it (`with_pack_hooks`), but several queries reaching the unit —
/// the semantic-token and handle queries among them — run on a pool thread
/// without that call. Here this thread builds its host under the first
/// plan, a reload elsewhere publishes an edited pack without touching this
/// thread, and the next query re-keyed to the new pack must fold through
/// the new body: a thread that kept its old host computed the re-keyed
/// lattice through a host that no longer serves the published plan, and
/// memoised that answer under the new key and epoch.
#[test]
fn a_pool_thread_with_a_stale_host_answers_with_the_new_plan() {
    use salsa::Setter as _;
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let src = "proc p {} {set r [tenant::label acme]; return $r}\n";
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let config = overlay_config(
        &db,
        install_workspace_packs(&tenant_pack(Some("tenant:")), "tcl9.0"),
    );
    let unit = installed_unit(&db, file, config);
    assert_eq!(value_at(&unit, "::p", "r", 1), Some(text("tenant:acme")));

    // The reload publishes the edited pack; this thread's host is still the
    // first plan's, and nothing on this thread asks for the new one.
    let edited = tenant_pack(Some("t:"));
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &edited);
    tcl_spectcl::hooks::publish(&edited);
    config.set_spec_pack_key(&mut db).to(edited.key);
    let unit = installed_unit(&db, file, config);
    assert_eq!(
        value_at(&unit, "::p", "r", 1),
        Some(text("t:acme")),
        "the query folds through the published plan's body"
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// The evaluator epoch re-keys the memoised lattices. `p` folds
/// `[tenant::label acme]` and its lattice is memoised; `q`'s argument makes
/// the body spin past its `-commands` budget, so this worker's host
/// quarantines the body and the process's epoch moves. The database cannot
/// see a host: `p`'s unit is still the memo that folded. Once the epoch is
/// taken as the input — what the server does after the diagnostics pass
/// that saw the quarantine — `p` is recomputed under the host it now has,
/// and declines `transient` rather than serve an answer the body can no
/// longer give.
#[test]
fn an_evaluator_epoch_re_keys_the_memoised_lattices() {
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let packs = tenant_pack_with_body(Some(
        "if {$name eq \"spin\"} {while 1 {format %d 1}}; fold [string cat \"tenant:\" $name]",
    ));
    let mut db = TclDatabase::default();
    let config = overlay_config(&db, install_workspace_packs(&packs, "tcl9.0"));
    let folding = SourceFile::new(
        &db,
        "proc p {} {set r [tenant::label acme]; return $r}\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let spinning = SourceFile::new(
        &db,
        "proc q {} {set s [tenant::label spin]; return $s}\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let unit = installed_unit(&db, folding, config);
    assert_eq!(value_at(&unit, "::p", "r", 1), Some(text("tenant:acme")));

    let epoch = tcl_registry::pack_hooks::evaluator_epoch();
    let unit = installed_unit(&db, spinning, config);
    assert_eq!(
        value_at(&unit, "::q", "s", 1),
        Some(LatticeValue::Overdefined)
    );
    assert!(
        tcl_registry::pack_hooks::evaluator_epoch() > epoch,
        "the quarantine moved the process's epoch"
    );
    let unit = installed_unit(&db, folding, config);
    assert_eq!(
        value_at(&unit, "::p", "r", 1),
        Some(text("tenant:acme")),
        "the memo cannot see the host"
    );

    let now = tcl_registry::pack_hooks::evaluator_epoch();
    assert!(set_evaluator_epoch(&mut db, now), "the epoch moved");
    assert!(
        !set_evaluator_epoch(&mut db, now),
        "and a second sync is a no-op"
    );
    let unit = installed_unit(&db, folding, config);
    assert_eq!(
        value_at(&unit, "::p", "r", 1),
        Some(LatticeValue::Overdefined)
    );
    assert_eq!(
        answers_for(&unit, "::p", "tenant::label"),
        ["declined: transient"]
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// The destructuring writers — `regexp`'s no-match preserve,
/// `scan`'s partial conversion, and `lassign` — answer alike on the
/// memoised editor path and a direct build, value for value, under a
/// release (`tcl8.6`) and a release with its own numeral grammar
/// (`tcl9.0`). `array set arr {k v}` runs beside them so the element-write
/// outcome the container harvesters read is part of the parity
/// too, even though `arr` itself carries no scalar lattice value.
#[test]
fn destructuring_agrees_on_both_paths() {
    const SRC: &str = "proc p {} {\n    set a before\n    set b before\n    \
                       regexp {(x)(y)} zz a b\n    set c before\n    set d before\n    \
                       scan \"12\" \"%d %d\" c d\n    lassign {1 2} e f\n    \
                       array set arr {k v}\n    return $a\n}\n";
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    for dialect in ["tcl8.6", "tcl9.0"] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), dialect.to_owned(), None);
        let (memoised, direct) = both_paths(&db, file);
        assert_eq!(
            lattice_of(&memoised, "::p"),
            lattice_of(&direct, "::p"),
            "{dialect}"
        );
        for (path, unit) in [("memoised", &*memoised), ("direct", &direct)] {
            // `regexp`'s no-match preserves both match variables.
            assert_eq!(
                value_at(unit, "::p", "a", 2),
                Some(text("before")),
                "{dialect} {path}: a no-match preserves $a"
            );
            assert_eq!(
                value_at(unit, "::p", "b", 2),
                Some(text("before")),
                "{dialect} {path}: a no-match preserves $b"
            );
            // `scan` converts the field the input reaches and preserves the
            // one past it.
            assert_eq!(
                value_at(unit, "::p", "c", 2),
                Some(LatticeValue::Const(ConstValue::Int(12))),
                "{dialect} {path}: %d converts $c"
            );
            assert_eq!(
                value_at(unit, "::p", "d", 2),
                Some(text("before")),
                "{dialect} {path}: the exhausted field preserves $d"
            );
            // `lassign` writes each named target its position takes.
            assert_eq!(
                value_at(unit, "::p", "e", 1),
                Some(LatticeValue::Const(ConstValue::Int(1))),
                "{dialect} {path}: lassign writes $e"
            );
            assert_eq!(
                value_at(unit, "::p", "f", 1),
                Some(LatticeValue::Const(ConstValue::Int(2))),
                "{dialect} {path}: lassign writes $f"
            );
        }
    }
}

/// The existence rung agrees on both paths: `x` decides
/// `Unbound` after `unset x`, and `n` binds on `incr n`'s normal path,
/// value for value with the direct build, under a release, a vendor
/// dialect declaring its base (`f5-irules`, 8.4's), and the lenient `tcl`
/// profile that spans them all.
#[test]
fn existence_agrees_on_both_paths() {
    use tcl_registry::value_transfer::{BindingKind, Existence};

    const SRC: &str = "proc p {} {\n    set x 1\n    unset x\n    \
                       if {[info exists x]} {puts yes}\n    \
                       incr n\n    if {[info exists n]} {puts also}\n}\n";
    for dialect in ["tcl8.6", "tcl9.0", "f5-irules", "tcl"] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), dialect.to_owned(), None);
        let (memoised, direct) = both_paths(&db, file);
        for (path, unit) in [("memoised", &*memoised), ("direct", &direct)] {
            assert_eq!(
                existence_at(unit, "::p", "x", 2),
                Some(Existence::Unbound),
                "{dialect} {path}: unset leaves x unbound"
            );
            assert_eq!(
                existence_at(unit, "::p", "n", 1),
                Some(Existence::Bound(BindingKind::Scalar)),
                "{dialect} {path}: incr binds n on its normal path"
            );
        }
    }
}

/// The seven ordered-state programs of the value-transfer contract answer
/// alike on both paths, value for value, in a procedure and at the top level,
/// under a release, a release with its own numeral grammar, a vendor dialect
/// declaring its base and the lenient `tcl` profile. Over `x` = 1 the first six
/// leave the expression's value in `r#1` and the last write it made in `x#2` —
/// 5 and 2, 0 and 1, 21 and 10, 5 and 3, 2 and 2, 3 and 2 — and the seventh,
/// whose expression stops at an error inside a `catch`, leaves `x#2` unknown,
/// so no read after it is forwarded the 1 it held before.
#[test]
fn the_seven_ordered_state_witnesses() {
    const EXPRESSIONS: [(&str, i64, i64); 6] = [
        ("{$x + [incr x] + $x}", 5, 2),
        ("{0 && [incr x]}", 0, 1),
        ("{$x + [set x 10] + $x}", 21, 10),
        ("{[incr x] + [incr x]}", 5, 3),
        ("{$x ? [incr x] : [incr x 10]}", 2, 2),
        ("\"$x + [incr x]\"", 3, 2),
    ];
    const CAUGHT: &str = "catch {expr {[incr x] + [error mid]}} msg\nputs $msg\nputs $x\n";
    let int = |value: i64| Some(LatticeValue::Const(ConstValue::Int(value)));
    // Each program as a procedure's body and as the top level, with what its
    // `r#1` and `x#2` hold, or `None` where the expression has no value.
    let mut programs: Vec<(String, String, Option<i64>, Option<i64>)> = EXPRESSIONS
        .iter()
        .map(|&(expression, value, after)| {
            let body = format!("set x 1\nset r [expr {expression}]\nputs $r\nputs $x\n");
            (expression.to_owned(), body, Some(value), Some(after))
        })
        .collect();
    programs.push((
        "an error in the middle".to_owned(),
        format!("set x 1\n{CAUGHT}"),
        None,
        None,
    ));
    for dialect in ["tcl8.6", "tcl9.0", "f5-irules", "tcl"] {
        for (name, body, value, after) in &programs {
            let indented = format!("    {}\n", body.trim_end().replace('\n', "\n    "));
            for (form, src) in [
                ("procedure", format!("proc p {{}} {{\n{indented}}}\n")),
                ("top level", body.clone()),
            ] {
                let db = TclDatabase::default();
                let file = SourceFile::new(&db, src, dialect.to_owned(), None);
                let (memoised, direct) = both_paths(&db, file);
                let at = format!("{dialect} {form}: {name}");
                if form == "procedure" {
                    assert_eq!(
                        lattice_of(&memoised, "::p"),
                        lattice_of(&direct, "::p"),
                        "{at}"
                    );
                } else {
                    assert_eq!(top_lattice_of(&memoised), top_lattice_of(&direct), "{at}");
                }
                let lookup = |unit: &CompilationUnit, name: &str, version: u32| {
                    let function = if form == "procedure" {
                        unit.procedures.get("::p")?
                    } else {
                        &unit.top_level
                    };
                    let sym = function.ssa.var_symbol(name)?;
                    function.sccp.values.get(&(sym, version)).cloned()
                };
                for (path, unit) in [("memoised", &*memoised), ("direct", &direct)] {
                    match (value, after) {
                        (Some(value), Some(after)) => {
                            assert_eq!(lookup(unit, "r", 1), int(*value), "{at} {path}: r");
                            assert_eq!(lookup(unit, "x", 2), int(*after), "{at} {path}: x");
                        }
                        _ => assert_eq!(
                            lookup(unit, "x", 2),
                            Some(LatticeValue::Overdefined),
                            "{at} {path}: the error path forwards no 1"
                        ),
                    }
                }
            }
        }
    }
}
