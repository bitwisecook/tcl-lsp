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

//! A workspace pack's declared evaluator on the database's memoised path
//! (`docs/design/compiler/value-transfers.md` § *One invocation, one
//! context*): the pack is loaded from `SpecTcl` text, installed and its hook
//! plan published as the server does, so the pack's body runs; a pack edit,
//! a reload on another thread and the evaluator epoch each re-key the
//! memoised lattice. The parity tests that need no pack are
//! `tcl-lsp-db`'s own `value_transfer_parity`.

use std::sync::{Arc, Mutex};

use tcl_compiler::analyser::NonAsciiMode;
use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, document_compilation_unit_for,
    file_analysis_incremental, set_evaluator_epoch,
};

fn fixture_function<'a>(
    unit: &'a CompilationUnit,
    display: &str,
) -> &'a tcl_compiler::compilation_unit::FunctionUnit {
    let mut declarations = unit.ir_module.procedures.iter().filter(|(_, procedure)| {
        procedure.qualified_name == display
            && procedure
                .declaration_site
                .as_ref()
                .is_some_and(|site| site.source.source_image().bytes() == unit.source.as_bytes())
    });
    let (&body, _) = declarations
        .next()
        .expect("fixture retains one original declaration for this display");
    assert!(declarations.next().is_none());
    assert!(unit.ir_module.procedure_ids.owns(body));
    let function = unit.function_for_body(Some(body)).expect(display);
    assert_eq!(function.body_id, Some(body));
    function
}

/// The value `name`'s version `version` holds in `qname`.
fn value_at(unit: &CompilationUnit, qname: &str, name: &str, version: u32) -> Option<LatticeValue> {
    let fu = fixture_function(unit, qname);
    let sym = fu.ssa.var_symbol(name).expect(name);
    fu.sccp.values.get(&(sym, version)).cloned()
}

/// The answers recorded for `command`'s statements in `qname`.
fn answers_for(unit: &CompilationUnit, qname: &str, command: &str) -> Vec<String> {
    fixture_function(unit, qname)
        .sccp
        .explanations
        .iter()
        .filter(|explanation| explanation.command == command)
        .map(|explanation| explanation.answer.clone())
        .collect()
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

/// The document's unit under `config`'s pack overlay, which these tests
/// install before they ask.
fn installed_unit(
    db: &TclDatabase,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<CompilationUnit> {
    document_compilation_unit_for(db, file, config).expect("the pack overlay is installed")
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
    let tally = fixture_function(&unit, "::p").sccp.route_tally;
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
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.into()));
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
/// the new plan. A thread builds its host on its first hook dispatch, and
/// queries reaching the unit — the semantic-token and handle queries among
/// them — run on whichever pool thread is free. Here this thread builds its host under the first
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
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.into()));
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
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.into()));
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
