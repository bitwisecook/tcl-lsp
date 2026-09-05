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

//! `namespace` command-surface regression vectors (issues #1442, #1446,
//! #1451, #1453, #1463, #1583, #1584).
//!
//! Every expectation is pinned byte-for-byte against tclsh 8.6.16 *and*
//! tclsh 9.0.4 unless a `9.0:` comment records a deliberate release
//! difference. The governing C functions are named per group.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
use tcl_dialect::DialectProfile;
use tcl_vm::{CompileError, CompileService, Vm};

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The `tclvm` compile service: registry, lexer grammar, and expression
/// dialect all resolved once from the emulated release's profile.
struct CompilerSvc {
    registry: &'static tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static DialectProfile>,
}

impl CompilerSvc {
    fn for_profile(profile: &'static DialectProfile) -> Self {
        Self {
            registry: tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            // `Some(profile)` rather than the profile's *name*: this harness is
            // always constructed from a resolved profile, so there is no
            // unstated-dialect case to represent (`None`) and no name to
            // re-resolve.
            dialect: Some(profile),
        }
    }
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) =
            tcl_compiler::lowering::first_fatal_parse_error_with_config(src, self.config)
        {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, self.registry, self.config, self.dialect);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, self.registry))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        Self::for_profile(profile).compile(src)
    }
}

/// Compile and run `src` on a VM pinned to `release`; return the result
/// string (an uncaught error leaves its message there), or the compile
/// rejection.
fn run_at(src: &str, release: &str) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
    let svc = CompilerSvc::for_profile(profile);
    let asm = match svc.compile(src) {
        Ok(asm) => asm,
        Err(e) => return e.0,
    };
    let mut vm = Vm::with_output(Box::new(Capture::default()));
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(CompilerSvc::for_profile(profile)));
    vm.run_module(&asm).result.to_str().to_string()
}

/// [`run_at`] at the VM's default release (9.0), for the vectors tclsh 8.6.16
/// and 9.0.4 agree on.
fn run(src: &str) -> String {
    run_at(src, "tcl9.0")
}

/// Run each `(release, script)` step on **one** VM, re-pinning its profile
/// between steps; the last step's result is returned. Real tclsh cannot change
/// release mid-session, so this exists only for invariants about what the
/// emulated surface must do when an embedder re-pins it.
fn run_flipping(steps: &[(&str, &str)]) -> String {
    let mut vm = Vm::with_output(Box::new(Capture::default()));
    let mut last = String::new();
    for (release, src) in steps {
        let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(CompilerSvc::for_profile(profile)));
        match CompilerSvc::for_profile(profile).compile(src) {
            Ok(asm) => last = vm.run_module(&asm).result.to_str().to_string(),
            Err(e) => return e.0,
        }
    }
    last
}

// #1442 — `namespace which -variable` never answers with a call frame
// (`NamespaceWhichCmd` → `Tcl_FindNamespaceVar`, tclNamesp.c:4657)

#[test]
fn which_variable_ignores_proc_locals() {
    // tclsh 8.6.16 / 9.0.4: {} — a proc local is not a namespace variable.
    assert_eq!(
        run("proc t {} {set loc 1; namespace which -variable loc}\nt"),
        ""
    );
    // …including inside a namespace, where the VM used to answer ::ns::loc.
    assert_eq!(
        run("namespace eval ns {proc q {} {set loc 1; namespace which -variable loc}}\nns::q"),
        ""
    );
    // A local that *shadows* a real namespace variable still reports the
    // namespace one (tclsh: ::ns2::shadow).
    assert_eq!(
        run("namespace eval ns2 {variable shadow 5\n\
             proc q {} {set shadow 9; namespace which -variable shadow}}\n\
             ns2::q"),
        "::ns2::shadow"
    );
}

#[test]
fn which_variable_resolves_real_namespace_variables() {
    // tclsh 8.6.16 / 9.0.4: the namespace's own table, qualified or not.
    assert_eq!(
        run("namespace eval nv {variable nsv 1}\nnamespace which -variable ::nv::nsv"),
        "::nv::nsv"
    );
    assert_eq!(
        run("namespace eval nv {variable nsv 1\nnamespace which -variable nsv}"),
        "::nv::nsv"
    );
    assert_eq!(
        run("namespace eval nv {variable nsv 1}\nnamespace which -variable ::nv::nope"),
        ""
    );
    // An array is a namespace variable too.
    assert_eq!(
        run("namespace eval ar {variable arr; set arr(x) 1}\nnamespace which -variable ::ar::arr"),
        "::ar::arr"
    );
}

#[test]
fn which_variable_global_fallback_is_a_release_axis() {
    // `ObjFindNamespaceVar` hands `TclGetNamespaceForQualName` two candidate
    // namespaces; Tcl 9.0 added `flags |= TCL_NAMESPACE_ONLY`
    // (tcl9.0.4 tclVar.c:5951), blanking the second.
    let src = "set ::gv 1\nnamespace eval n {}\nnamespace eval n {namespace which -variable gv}";
    // tclsh 8.6.16: ::gv
    assert_eq!(run_at(src, "tcl8.6"), "::gv");
    // tclsh 9.0.4: {}
    assert_eq!(run_at(src, "tcl9.0"), "");
}

#[test]
fn declared_but_unset_namespace_variables_are_introspectable() {
    // Tcl 9.0.4: `variable only` materialises an unset varTable cell. Both
    // namespace introspection surfaces see it; `info exists` still says 0.
    assert_eq!(
        run("namespace eval declared {variable only
             list [namespace which -variable only] [info vars] [info exists only]}"),
        "::declared::only only 0"
    );
}

#[test]
fn namespace_which_validates_its_positional_option_shape() {
    let usage = "wrong # args: should be \"namespace which ?-command? ?-variable? name\"";
    assert_eq!(run("catch {namespace which -zork puts} m; set m"), usage);
    assert_eq!(
        run("catch {namespace which -command puts extra} m; set m"),
        usage
    );
    // With one word there is no option position: even an option-looking word
    // is the command name, and an unresolved command produces the empty value.
    assert_eq!(run("namespace which -command"), "");
}

#[test]
fn origin_follows_import_chains() {
    // tclsh 8.6.16 / 9.0.4 (`NamespaceOriginCmd` → `TclGetOriginalCommand`).
    assert_eq!(
        run(
            "namespace eval src {namespace export p; proc p {} {return P}}\n\
             namespace eval dst {namespace import ::src::p}\n\
             namespace origin ::dst::p"
        ),
        "::src::p"
    );
    // A non-imported command is its own origin, resolved — not merely
    // prefixed with the current namespace.
    assert_eq!(
        run("namespace eval foo {}\nnamespace eval foo {namespace origin set}"),
        "::set"
    );
    assert_eq!(
        run("catch {namespace origin nosuchcommand} m; set m"),
        "invalid command name \"nosuchcommand\""
    );
}

/// The `Namespaces::command_origin` contract: `None` means "not an imported
/// command" (C's `cmdPtr->deleteProc == DeleteImportedCmd`), so a shared core
/// can tell the two apart. The VM used to answer `Some(self)` for every
/// command, which happened to be invisible through `namespace origin` — it
/// folds both onto the same answer — but left the trait unusable for anything
/// that needs the distinction, and disagreed with the WASM runtime's impl.
#[test]
fn command_origin_reports_none_for_a_command_that_was_not_imported() {
    use tcl_runtime_api::Namespaces;

    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let svc = CompilerSvc::for_profile(profile);
    let asm = svc
        .compile(
            "namespace eval src {namespace export p; proc p {} {return P}}\n\
             namespace eval dst {namespace import ::src::p}",
        )
        .expect("setup compiles");
    let mut vm = Vm::with_output(Box::new(Capture::default()));
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(CompilerSvc::for_profile(profile)));
    assert!(vm.run_module(&asm).code.is_ok());

    let cur = Namespaces::current(&vm);
    let imported = vm.find_command(cur, "::dst::p").expect("the import exists");
    let source = vm.find_command(cur, "::src::p").expect("the source exists");
    let builtin = vm.find_command(cur, "set").expect("a builtin exists");

    let origin = vm
        .command_origin(imported)
        .expect("an import has an origin");
    assert_eq!(vm.command_name(origin).as_deref(), Some("::src::p"));
    assert_eq!(
        vm.command_origin(source),
        None,
        "a plain proc is not an import"
    );
    assert_eq!(
        vm.command_origin(builtin),
        None,
        "a builtin is not an import"
    );
}

// #1446 — `namespace export` / `import` leading-option handling
// (`NamespaceExportCmd`/`Tcl_Export`, `NamespaceImportCmd`/`Tcl_Import`)

#[test]
fn export_query_form_reports_the_pattern_list() {
    // tclsh 8.6.16 / 9.0.4: `a b` (the regression that masked the -clear bug).
    assert_eq!(
        run("namespace eval e {namespace export a b}\nnamespace eval e {namespace export}"),
        "a b"
    );
    // C's `Tcl_Export` skips a pattern already present, so the list is a set.
    assert_eq!(
        run(
            "namespace eval e {namespace export a; namespace export a b}\n\
             namespace eval e {namespace export}"
        ),
        "a b"
    );
}

#[test]
fn export_clear_really_clears() {
    // tclsh 8.6.16 / 9.0.4: {}
    assert_eq!(
        run(
            "namespace eval e {namespace export a b; namespace export -clear}\n\
             namespace eval e {namespace export}"
        ),
        ""
    );
    // The behavioural vector, independent of the query form: after -clear
    // nothing is importable any more.
    assert_eq!(
        run("namespace eval d1 {proc foo {} {}; proc bar {} {}\n\
             namespace export foo bar; namespace export -clear}\n\
             namespace eval d2 {namespace import ::d1::*}\n\
             info commands ::d2::*"),
        ""
    );
    // …and a -clear followed by patterns sets exactly those.
    assert_eq!(
        run(
            "namespace eval e {namespace export a b; namespace export -clear c}\n\
             namespace eval e {namespace export}"
        ),
        "c"
    );
}

#[test]
fn only_the_first_word_is_the_export_flag() {
    // The registry pins `max_leading_option_words: Some(1)`; C tests only
    // objv[1] (tclNamesp.c:3552). tclsh 8.6.16 / 9.0.4: `-clear x`.
    assert_eq!(
        run("namespace eval e {namespace export -clear -clear x}\n\
             namespace eval e {namespace export}"),
        "-clear x"
    );
    // A pattern may not carry a namespace qualifier (`Tcl_Export`).
    assert_eq!(
        run("catch {namespace eval e {namespace export ::foo}} m; set m"),
        "invalid export pattern \"::foo\": pattern can't specify a namespace"
    );
}

#[test]
fn only_the_first_word_is_the_import_flag() {
    let setup = "namespace eval s3 {namespace export q; proc q {} {return Q}}\n";
    // A trailing `-force` is an ordinary pattern, and an unqualified pattern
    // names no source namespace (tclsh 8.6.16 / 9.0.4).
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t3 {{catch {{namespace import ::s3::q -force}} m; set m}}"
        )),
        "no namespace specified in import pattern \"-force\""
    );
    // The import that preceded the bad pattern still happened, as in C.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t3 {{catch {{namespace import ::s3::q -force}}}}\n\
             info commands ::t3::*"
        )),
        "::t3::q"
    );
    // A *leading* -force is the flag.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t4 {{namespace import -force ::s3::q}}\n\
             info commands ::t4::*"
        )),
        "::t4::q"
    );
    // The introspection form lists the current namespace's imports.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t5 {{namespace import ::s3::q; namespace import}}"
        )),
        "q"
    );
}

#[test]
fn namespace_import_rejects_self_and_unknown_sources() {
    assert_eq!(
        run("namespace eval self {catch {namespace import ::self::*} m; set m}"),
        "import pattern \"::self::*\" tries to import from namespace \"self\" into itself"
    );
    assert_eq!(
        run("namespace eval dest {catch {namespace import ::nosuch::*} m; set m}"),
        "unknown namespace in import pattern \"::nosuch::*\""
    );
}

// #1451 — namespace teardown (`TclTeardownNamespace`, tclNamesp.c:1084)

#[test]
fn deleting_a_namespace_clears_paths_in_both_directions() {
    // (a) another namespace's path loses the deleted entry, and a recreated
    // namespace does not resurrect it. tclsh 8.6.16 / 9.0.4: {} then the
    // `invalid command name` the empty path implies.
    assert_eq!(
        run("namespace eval a {proc pp {} {return A1}}\n\
             namespace eval b {namespace path ::a}\n\
             namespace delete ::a\n\
             namespace eval b {namespace path}"),
        ""
    );
    assert_eq!(
        run("namespace eval a {proc pp {} {return A1}}\n\
             namespace eval b {namespace path ::a}\n\
             namespace delete ::a\n\
             namespace eval a {proc pp {} {return A2}}\n\
             namespace eval b {catch {pp} m; set m}"),
        "invalid command name \"pp\""
    );
}

#[test]
fn deleting_a_namespace_clears_its_own_path_and_unknown_handler() {
    // (b) a recreated namespace starts clean. tclsh 8.6.16 / 9.0.4: {} / {}.
    assert_eq!(
        run("namespace eval a {}\n\
             namespace eval z {namespace path ::a; namespace unknown {puts UNK}}\n\
             namespace delete ::z\n\
             namespace eval z {}\n\
             namespace eval z {namespace path}"),
        ""
    );
    assert_eq!(
        run("namespace eval a {}\n\
             namespace eval z {namespace path ::a; namespace unknown {puts UNK}}\n\
             namespace delete ::z\n\
             namespace eval z {}\n\
             namespace eval z {namespace unknown}"),
        ""
    );
}

#[test]
fn an_unset_namespace_unknown_is_empty_outside_the_global_namespace() {
    // Only the global namespace carries `::unknown` by default.
    assert_eq!(run("namespace unknown"), "::unknown");
    assert_eq!(
        run("namespace eval q {}\nnamespace eval q {namespace unknown}"),
        ""
    );
    // Resetting a namespace's handler goes back to empty, not to ::unknown.
    assert_eq!(
        run(
            "namespace eval q {namespace unknown {puts A}; namespace unknown {}}\n\
             namespace eval q {namespace unknown}"
        ),
        ""
    );
}

#[test]
fn deleting_a_namespace_deletes_its_ensembles() {
    // (c) `Tcl_DeleteNamespace`'s `while (nsPtr->ensembles != NULL)` loop —
    // the command lives in the global table but belongs to ::ens1.
    assert_eq!(
        run(
            "namespace eval ens1 {namespace export *; proc sub {} {return S}\n\
             namespace ensemble create -command ::myens}\n\
             info commands ::myens"
        ),
        "::myens"
    );
    assert_eq!(
        run(
            "namespace eval ens1 {namespace export *; proc sub {} {return S}\n\
             namespace ensemble create -command ::myens}\n\
             namespace delete ::ens1\n\
             info commands ::myens"
        ),
        ""
    );
}

#[test]
fn namespace_owned_ensemble_trace_precedes_the_dying_marker() {
    // Tcl_DeleteNamespace drains nsPtr->ensembles before setting NS_DYING.
    // A global command owned by ::N therefore remains visible as an ensemble
    // while its delete trace also sees the namespace token as live; both are
    // absent once namespace deletion returns. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc observe {old new op} {
                 lappend ::seen [list [namespace exists ::N] $old \
                     [info commands $old] [namespace ensemble exists $old]]
             }
             namespace eval N {namespace ensemble create -command ::E}
             trace add command ::E delete observe
             namespace delete ::N
             list $seen [namespace exists ::N] [info commands ::E]"),
        "{{1 ::E ::E 1}} 0 {}"
    );
}

#[test]
fn namespace_owned_ensembles_retire_live_heads_in_creation_order() {
    // Tcl_CreateEnsemble pushes each token at the head of nsPtr->ensembles and
    // Tcl_DeleteNamespace removes one live head at a time. A delete callback's
    // newly-created ensemble is consequently the next head, ahead of the older
    // token that was already waiting. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 lappend ::seen [namespace tail $old]
                 if {[namespace tail $old] eq \"E2\"} {
                     namespace eval ::N {
                         namespace ensemble create -command ::E3
                     }
                     trace add command ::E3 delete deleted
                 }
             }
             namespace eval ::N {
                 namespace ensemble create -command ::E1
                 namespace ensemble create -command ::E2
             }
             trace add command ::E1 delete deleted
             trace add command ::E2 delete deleted
             namespace delete ::N
             set seen"),
        "E2 E3 E1"
    );
}

#[test]
fn parent_teardown_reaches_each_child_after_parent_commands() {
    // Tcl drains a parent's owned ensembles while both namespace tokens are
    // live, then marks only the parent dying and retires its ordinary command
    // table. The child remains live until its own recursive ensemble phase.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc ensdeleted {tag old new op} {
                 lappend ::seen [list $tag [namespace exists ::P] \
                                      [namespace exists ::P::C]]
             }
             proc cmddeleted {old new op} {
                 lappend ::seen [list PC [namespace exists ::P] \
                                      [namespace exists ::P::C] \
                                      [info commands ::CE]]
             }
             namespace eval ::P {
                 namespace ensemble create -command ::PE
                 proc p {} {}
                 namespace eval C {
                     namespace ensemble create -command ::CE
                 }
             }
             trace add command ::PE delete [list ensdeleted P]
             trace add command ::P::p delete cmddeleted
             trace add command ::CE delete [list ensdeleted C]
             namespace delete ::P
             set seen"),
        "{P 1 1} {PC 0 1 ::CE} {C 0 1}"
    );
}

#[test]
fn child_namespace_teardown_uses_tcl_string_hash_order() {
    // TclDeleteNamespaceChildren snapshots the parent's childTable with
    // Tcl_FirstHashEntry/Tcl_NextHashEntry, not namespace creation order.
    // Each child's owned ensemble makes that recursive order observable. Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {tag old new op} {lappend ::seen $tag}
             foreach n {one two three four five six seven eight nine ten} {
                 namespace eval ::P::$n \
                     [list namespace ensemble create -command ::E_$n]
                 trace add command ::E_$n delete [list deleted $n]
             }
             namespace delete ::P
             set seen"),
        "six four three eight seven nine five two one ten"
    );
}

#[test]
fn namespace_teardown_uses_command_table_tcl_string_hash_order() {
    // TclTeardownNamespace snapshots nsPtr->cmdTable with
    // Tcl_FirstHashEntry/Tcl_NextHashEntry before deleting each token, so the
    // delete traces fire in the retained TCL_STRING_KEYS bucket order rather
    // than definition or lexical order. Exact Tcl 9.0.4 oracle result (issue
    // #1752's own vector).
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             namespace delete ::N
             set log"),
        "six four three eight seven nine five two one ten"
    );
}

#[test]
fn command_table_hash_order_records_table_growth() {
    // RebuildTable quadruples the bucket array once numEntries reaches three
    // times numBuckets and re-pushes each chain head-first, reversing it.
    // Thirteen commands cross the first threshold. Exact Tcl 9.0.4 oracle
    // result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             for {set i 0} {$i < 13} {incr i} {
                 proc ::N::c$i {} {}
                 trace add command ::N::c$i delete rec
             }
             namespace delete ::N
             set log"),
        "c5 c6 c7 c8 c9 c0 c1 c10 c2 c11 c12 c3 c4"
    );
}

#[test]
fn command_table_hash_order_retains_capacity_across_deletions() {
    // Tcl_DeleteHashEntry never shrinks the bucket array, so commands created
    // after a bulk deletion land in the grown table and precede the survivors.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {proc ::N::$n {} {}}
             foreach i {1 2 4 5 6 7 8 9 10 11} {rename ::N::a$i {}}
             foreach n {b1 b2 b3} {proc ::N::$n {} {}}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "b1 b2 b3 a0 a3"
    );
}

#[test]
fn command_table_hash_order_moves_a_redefined_command() {
    // TclCreateObjCommandInNs deletes the existing hash entry and creates a
    // fresh one, so redefining `one` moves it to its bucket head. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {proc ::N::$n {} {}}
             proc ::N::one {} {return again}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "six four three eight seven one nine five two ten"
    );
    // TclRenameCommand creates the destination entry the same way, before it
    // deletes the source's. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {proc ::N::$n {} {}}
             rename ::N::one ::N::uno
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "six four three eight seven uno nine five two ten"
    );
}

#[test]
fn namespace_teardown_visits_a_callback_created_command_in_the_next_pass() {
    // TclTeardownNamespace loops while cmdTable is non-empty, so a command a
    // delete callback creates is torn down by a second snapshot, after every
    // entry of the first one. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             proc mk {old new op} {
                 lappend ::log [namespace tail $old]
                 proc ::N::zz {} {}
                 trace add command ::N::zz delete rec
             }
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             trace add command ::N::six delete mk
             namespace delete ::N
             set log"),
        "six six four three eight seven nine five two one ten zz"
    );
}

#[test]
fn namespace_teardown_defers_a_callback_recreated_command_to_the_next_pass() {
    // A redefinition inside the delete callback unlinks the dying token's own
    // entry (Tcl_DeleteCommandFromToken's CMD_DYING branch), so the
    // replacement is a distinct token the current snapshot no longer names and
    // the next snapshot picks up. 9.0: exact tclsh 9.0.4 oracle result; 8.6.16
    // segfaults on this script.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             proc remake {old new op} {
                 proc ::N::two {} {}
                 trace add command ::N::two delete rec
                 lappend ::log remade
             }
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             trace add command ::N::two delete remake
             namespace delete ::N
             set log"),
        "six four three eight seven nine five remade two one ten two"
    );
}

#[test]
fn namespace_teardown_skips_a_command_a_callback_already_deleted() {
    // CMD_TRACE_ACTIVE is per Command, not interpreter-wide: `one` fires its
    // own delete trace nested inside six's callback, and the snapshot entry
    // for it is gone by the time the loop reaches it. Exact Tcl 9.0.4 oracle
    // result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             proc killone {old new op} {rename ::N::one {}; lappend ::log killed-one}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             trace add command ::N::six delete killone
             namespace delete ::N
             set log"),
        "one killed-one six four three eight seven nine five two ten"
    );
}

#[test]
fn namespace_teardown_places_imports_at_their_hash_positions() {
    // An imported redirect occupies an ordinary cmdTable entry, so it retires
    // at its own hash position rather than in a separate pass over the
    // namespace's imports. Exact Tcl 9.0.4 oracle result.
    let imports = "set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
             namespace eval N {namespace import ::S::s1 ::S::s2}
             foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log";
    assert_eq!(
        run(imports),
        "six four three s1 eight seven s2 five two one"
    );
    // `namespace forget` deletes the entry and the re-import creates a new one
    // at the same bucket head, so the order is unchanged.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
             namespace eval N {
                 namespace import ::S::s1 ::S::s2
                 namespace forget ::S::s1
                 namespace import ::S::s1
             }
             foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "six four three s1 eight seven s2 five two one"
    );
}

#[test]
fn namespace_teardown_retires_each_sources_import_tree_before_the_next_entry() {
    // Tcl_DeleteCommandFromToken walks the deleted command's ImportRef list
    // depth-first straight after its own delete trace, so a source's whole
    // import tree fires before the next cmdTable entry. Exact Tcl 9.0.4 oracle
    // result.
    assert_eq!(
        run("set log {}
             namespace eval N {proc one {} {}; proc two {} {}; proc six {} {}
                               namespace export *}
             namespace eval I {namespace import ::N::one; namespace export *}
             namespace eval J {namespace import ::I::one}
             foreach c {::N::one ::N::two ::N::six ::I::one ::J::one} {
                 trace add command $c delete [list apply {{c old new op} {
                     lappend ::log $c
                 }} $c]
             }
             namespace delete ::N
             set log"),
        "::N::six ::N::two ::N::one ::I::one ::J::one"
    );
}

#[test]
fn a_delete_callbacks_own_trace_dies_with_the_dying_token() {
    // `Tcl_DeleteCommandFromToken` frees the whole `cmdPtr->tracePtr` list
    // after `CallCommandTraces`. A trace the callback registers on the command
    // being deleted attaches to that same dying token, so it never fires — not
    // in this walk (`CallCommandTraces` follows `active.nextTracePtr`), and not
    // for a later command that takes the vacated name. Exact Tcl 9.0.4 oracle
    // results (identical on 8.6.16).
    let recorders = "set log {}
             proc inner {old new op} {lappend ::log inner:$old}
             proc outer {old new op} {lappend ::log outer:$old
                 trace add command $old delete inner}\n";
    // `rename cmd {}`.
    assert_eq!(
        run(&format!(
            "{recorders}
             proc p {{}} {{}}
             trace add command ::p delete outer
             rename ::p {{}}
             set a $log
             proc p {{}} {{}}
             rename ::p {{}}
             list $a $log"
        )),
        "outer:::p outer:::p"
    );
    // Redefinition (`TclCreateObjCommandInNs` deletes the old token first).
    assert_eq!(
        run(&format!(
            "{recorders}
             proc p {{}} {{}}
             trace add command ::p delete outer
             proc p {{}} {{}}
             set a $log
             proc p {{}} {{}}
             set b $log
             rename ::p {{}}
             list $a $b $log"
        )),
        "outer:::p outer:::p outer:::p"
    );
    // Namespace teardown.
    assert_eq!(
        run(&format!(
            "{recorders}
             namespace eval N {{proc q {{}} {{}}}}
             trace add command ::N::q delete outer
             namespace delete ::N
             set a $log
             namespace eval N {{proc q {{}} {{}}}}
             namespace delete ::N
             list $a $log"
        )),
        "outer:::N::q outer:::N::q"
    );
}

#[test]
fn a_replacements_trace_survives_the_deletion_that_created_it() {
    // The other half of the same rule: a callback that *binds* a replacement
    // at the vacated name registers on that new token, whose own trace list C
    // never touches. Only the dying token's list is freed. Exact Tcl 9.0.4
    // oracle results (identical on 8.6.16).
    let recorders = "set log {}
             proc inner {old new op} {lappend ::log inner:$old}
             proc mk {old new op} {lappend ::log mk:$old
                 proc ::p {} {}
                 trace add command ::p delete inner}\n";
    assert_eq!(
        run(&format!(
            "{recorders}
             proc p {{}} {{}}
             trace add command ::p delete mk
             rename ::p {{}}
             set a [list $log [llength [info commands ::p]]]
             rename ::p {{}}
             list $a $log [llength [info commands ::p]]"
        )),
        "{mk:::p 1} {mk:::p inner:::p} 0"
    );
    // A trace the callback adds *before* the replacement still belongs to the
    // dying token and goes with it; only the one added after survives.
    assert_eq!(
        run("set log {}
             proc early {old new op} {lappend ::log early:$old}
             proc late {old new op} {lappend ::log late:$old}
             proc mk {old new op} {lappend ::log mk:$old
                 trace add command $old delete early
                 proc ::p {} {}
                 trace add command ::p delete late}
             proc p {} {}
             trace add command ::p delete mk
             rename ::p {}
             set a $log
             rename ::p {}
             list $a $log"),
        "mk:::p {mk:::p late:::p}"
    );
}

#[test]
fn a_refused_alias_rename_keeps_the_command_tables_growth() {
    // `TclRenameCommand` creates the destination hash entry *before*
    // `TclPreventAliasLoop` and deletes it again on a refusal, so the transient
    // twelfth entry rebuilds the eleven-command table from 4 buckets to 16 and
    // the grown array outlives the rejected rename. Exact Tcl 9.0.4 oracle
    // results (identical on 8.6.16).
    let eleven = "set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten eleven} {
                 proc ::N::$n {} {}
             }\n";
    let trace_and_delete = "foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N\n";
    assert_eq!(
        run(&format!("{eleven}{trace_and_delete} set log")),
        "six four three eight seven nine five two one eleven ten"
    );
    assert_eq!(
        run(&format!(
            "{eleven}
             interp alias {{}} ::a {{}} ::N::b
             set c [catch {{rename ::a ::N::b}} m]
             {trace_and_delete}
             list $c $m [llength [info commands ::a]] $log"
        )),
        "1 {cannot define or rename alias \"b\": would create a loop} 1 \
         {three seven one two four eleven eight five nine six ten}"
    );
}

#[test]
fn deleting_a_namespace_fully_retires_its_visible_ensemble_once() {
    // The ensemble lives in the global command table but is owned by ::N.
    // Namespace teardown fires and removes its delete trace. A later, distinct
    // ::E lifecycle must not inherit that trace. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set hits {}
             proc deleted {old new op} {lappend ::hits [list $old $new $op]}
             namespace eval N {
                 proc x args {return OLD}
                 namespace ensemble create -command ::E -map {x ::N::x}
             }
             trace add command ::E delete deleted
             namespace delete ::N
             set first $hits
             namespace eval N {
                 proc x args {return NEW}
                 namespace ensemble create -command ::E -map {x ::N::x}
             }
             rename ::E {}
             list $first $hits"),
        "{{::E {} delete}} {{::E {} delete}}"
    );
}

#[test]
fn deleting_a_namespace_fully_retires_ordinary_commands_once() {
    // Ordinary namespace members use the same command-token deletion
    // lifecycle as explicitly renamed commands. The trace fires once during
    // teardown and cannot attach to a later command at the vacated name.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set hits {}
             proc deleted {old new op} {lappend ::hits [list $old $new $op]}
             namespace eval N {proc p {} {}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set first $hits
             namespace eval N {proc p {} {}}
             rename ::N::p {}
             list $first $hits"),
        "{{::N::p {} delete}} {{::N::p {} delete}}"
    );
}

#[test]
fn namespace_delete_marks_the_namespace_dying_before_command_traces() {
    // The namespace token is already non-existent, but the dying command is
    // still visible during its delete trace. A callback may replace and invoke
    // that command; the replacement remains in the dying table only until
    // teardown finishes. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 lappend ::seen [list before [namespace exists ::N] \
                                      [info commands $old]]
                 proc $old {} {return NEW}
                 lappend ::seen [list after [namespace exists ::N] \
                                      [info commands $old] \
                                      [namespace exists ::N] [$old]]
             }
             namespace eval N {proc p {} {return OLD}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set seen"),
        "{before 0 ::N::p} {after 0 ::N::p 0 NEW}"
    );
    assert_eq!(
        run("set hits 0
             proc deleted {old new op} {incr ::hits; proc $old {} {return NEW}}
             namespace eval N {proc p {} {return OLD}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set after [info commands ::N::p]
             namespace eval N {proc p {} {return LATER}}
             rename ::N::p {}
             list $after $hits"),
        "{} 1"
    );

    // Re-entering the exact dying namespace is refused: `namespace eval ::N`
    // no longer finds the token (it is `NS_DYING`) and `Tcl_CreateNamespace`
    // then trips over the child entry `TclTeardownNamespace` unlinks only
    // after the command loop, so the callback errors out and appends nothing.
    // Exact Tcl 9.0.4 oracle result (identical on 8.6.16).
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 namespace eval ::N {lappend ::seen [namespace exists ::N]}
             }
             namespace eval N {proc p {} {}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             list $seen [namespace exists ::N]"),
        "{} 0"
    );
}

#[test]
fn namespace_exists_uses_namespace_name_canonicalisation() {
    // A trailing separator run belongs to namespace-name grammar and drops;
    // it must not be preserved as the empty command tail. Exact Tcl 9.0.4
    // oracle result (and a regression for namespace_colon_runs_e2e).
    assert_eq!(
        run("namespace eval c9::: {}
             list [namespace exists ::c9] [namespace exists c9:::]"),
        "1 1"
    );
}

#[test]
fn namespace_delete_retires_callback_created_hidden_ensemble_imports() {
    // The callback creates a namespace-owned ensemble, imports it, and moves
    // that import to the hidden table. Namespace teardown must revisit the
    // live command/ensemble frontier and retire the hidden import through its
    // source token's normal import graph. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("namespace export E
             proc target {} {return TARGET}
             proc late {old new op} {
                 namespace eval ::N {
                     namespace ensemble create -command ::E -map {x ::target}
                 }
                 namespace eval ::I {namespace import ::E}
                 interp hide {} ::I::E held
             }
             namespace eval ::N {proc p {} {}}
             trace add command ::N::p delete late
             namespace delete ::N
             set c [catch {interp invokehidden {} held x} m o]
             list [info commands ::E] [interp hidden {}] \
                  [info commands ::I::E] $c $m [dict get $o -errorcode]"),
        "{} {} {} 1 {invalid hidden command name \"held\"} \
         {TCL LOOKUP HIDDENTOKEN held}"
    );
}

#[test]
fn namespace_path_checks_that_every_entry_exists() {
    // (d) `NamespacePathCmd` resolves each entry with
    // `TclGetNamespaceFromObj` before installing the path.
    assert_eq!(
        run("namespace eval outer2 {catch {namespace path inner} m; set m}"),
        "namespace \"inner\" not found in \"::outer2\""
    );
    assert_eq!(
        run("namespace eval outer3 {catch {namespace path ::inner} m; set m}"),
        "namespace \"::inner\" not found"
    );
    // A rejected path leaves the previous one installed.
    assert_eq!(
        run("namespace eval a {}\n\
             namespace eval o {namespace path ::a; catch {namespace path {::a ::nope}}\n\
             namespace path}"),
        "::a"
    );
}

#[test]
fn namespace_delete_accepts_the_global_root() {
    // Tcl 9.0.4 accepts the command and tears down the tree.
    assert_eq!(run("namespace delete ::"), "");
    assert_eq!(
        run("namespace delete ::
             puts hi"),
        "invalid command name \"puts\""
    );
}

#[test]
fn namespace_origin_reports_its_own_usage_for_both_bad_arities() {
    let usage = "wrong # args: should be \"namespace origin name\"";
    assert_eq!(run("catch {namespace origin} m; set m"), usage);
    assert_eq!(run("catch {namespace origin set extra} m; set m"), usage);
}

#[test]
fn namespace_children_matches_tcls_string_hash_iteration_order() {
    let setup = "namespace eval order {}
                 foreach n {one two three four five six seven eight nine ten} {
                     namespace eval ::order::$n {}
                 }\n";
    assert_eq!(
        run(&format!("{setup}namespace children ::order")),
        "::order::six ::order::four ::order::three ::order::eight \
         ::order::seven ::order::nine ::order::five ::order::two \
         ::order::one ::order::ten"
    );
    assert_eq!(
        run(&format!("{setup}namespace children ::order ::order::t*")),
        "::order::three ::order::two ::order::ten"
    );
}

#[test]
fn namespace_children_retains_tcls_hash_capacity_after_deletion() {
    assert_eq!(
        run("namespace eval p {}
             foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {
                 namespace eval ::p::$n {}
             }
             foreach i {1 2 4 5 6 7 8 9 10 11} {namespace delete ::p::a$i}
             namespace children ::p"),
        "::p::a0 ::p::a3"
    );
}

// #1453 — ensemble option tables and subcommand resolution
// (`TclNamespaceEnsembleCmd` + `NsEnsembleImplementationCmd`, tclEnsemble.c)

#[test]
fn ensemble_create_options_abbreviate() {
    // `Tcl_GetIndexFromObj` flags 0 (tclEnsemble.c:211). tclsh: ::ab5.
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}}\n\
             namespace eval e5 {namespace ensemble create -comm ::ab5 -sub go}"
        ),
        "::ab5"
    );
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}\n\
             namespace ensemble create -comm ::ab5 -sub go}\n\
             ::ab5 go"
        ),
        "G"
    );
    // The `namespace ensemble` subcommand table abbreviates too.
    assert_eq!(
        run("namespace eval e5 {proc go {} {}}\n\
             namespace eval e5 {namespace ensemble cr -command ::ab7 -subcommands go}"),
        "::ab7"
    );
    assert_eq!(
        run("namespace eval e5 {proc go {} {}\n\
             namespace ensemble create -command ::ab7 -subcommands go}\n\
             namespace ensemble ex ::ab7"),
        "1"
    );
    assert_eq!(
        run("catch {namespace ensemble frobnicate} m; set m"),
        "bad subcommand \"frobnicate\": must be configure, create, or exists"
    );
}

#[test]
fn ensemble_create_has_no_namespace_option() {
    // `ensembleCreateOptions` (tclEnsemble.c:57-60) omits -namespace.
    assert_eq!(
        run("catch {namespace ensemble create -namespace ::x} m; set m"),
        "bad option \"-namespace\": must be -command, -map, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
    assert_eq!(
        run("catch {namespace ensemble create -bogus v} m; set m"),
        "bad option \"-bogus\": must be -command, -map, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
    // `-p` prefixes both -parameters and -prefixes.
    assert_eq!(
        run("catch {namespace ensemble create -p 1} m; set m"),
        "ambiguous option \"-p\": must be -command, -map, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
}

#[test]
fn ensemble_create_accepts_parameters_and_threads_them() {
    assert_eq!(
        run("namespace eval e6 {namespace ensemble create -command ::ab6 -parameters {q}}"),
        "::ab6"
    );
    // `ens p sub args…` → `target p args…` (C's numParameters threading).
    assert_eq!(
        run("namespace eval e6 {namespace export *\n\
             proc go {p a} {return \"$p/$a\"}\n\
             namespace ensemble create -command ::ab6 -parameters {q}}\n\
             ::ab6 VAL go ARG"),
        "VAL/ARG"
    );
}

#[test]
fn ensemble_wrong_args_list_quotes_the_command_and_parameters() {
    // The usage prefix is a Tcl list of command/parameter words, not a string
    // joined with spaces. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("namespace eval {E space} {
                 namespace ensemble create -command {::E space} \
                     -parameters {{a b}}
             }
             catch {{::E space}} m o
             list $m [dict get $o -errorcode]"),
        "{wrong # args: should be \"{::E space} {a b} subcommand ?arg ...?\"} \
         {TCL WRONGARGS}"
    );
}

#[test]
fn ensemble_create_checks_pair_arity_before_option_words() {
    // C: `if (objc & 1)` fires before any `Tcl_GetIndexFromObj`
    // (tclEnsemble.c:192), so an odd tail is `wrong # args`, never `bad option`.
    let usage = "wrong # args: should be \"namespace ensemble create ?option value ...?\"";
    assert_eq!(
        run("catch {namespace ensemble create -command} m; set m"),
        usage
    );
    assert_eq!(
        run("catch {namespace ensemble create -command ::zz -sub} m; set m"),
        usage
    );
    assert_eq!(
        run("catch {namespace ensemble create -bogus} m; set m"),
        usage
    );
}

/// Configuring an ensemble through a `namespace import` alias configures the
/// ORIGIN: both spellings then observe the one config, and the alias stays an
/// alias so `namespace origin` still answers the source (tclsh 9.0.4-pinned).
#[test]
fn configuring_an_imported_ensemble_updates_the_origin() {
    let setup = "namespace eval S {namespace export ens\n\
                 proc impl {} {return ORIG}\n\
                 proc impl2 {} {return NEW}\n\
                 namespace ensemble create -command ::S::ens -map {go impl}}\n\
                 namespace eval T {namespace import ::S::ens}\n\
                 namespace eval S {namespace ensemble configure ::T::ens -map {go impl2}}\n";
    // Both spellings dispatch the new target.
    assert_eq!(run(&format!("{setup}::T::ens go")), "NEW");
    assert_eq!(run(&format!("{setup}::S::ens go")), "NEW");
    // Both spellings read back the new config.
    assert_eq!(
        run(&format!(
            "{setup}namespace ensemble configure ::T::ens -map"
        )),
        "go ::S::impl2"
    );
    assert_eq!(
        run(&format!(
            "{setup}namespace ensemble configure ::S::ens -map"
        )),
        "go ::S::impl2"
    );
    // The alias is still an alias — configuring it did not fork a new ensemble.
    assert_eq!(
        run(&format!("{setup}namespace origin ::T::ens")),
        "::S::ens"
    );
}

#[test]
fn imported_ensemble_tracks_atomic_replacement_but_not_true_deletion() {
    // Tcl_CreateObjCommand-style replacement keeps the source table token, so
    // an existing import observes the new implementation and retains its
    // provenance. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc tgt_old args {return OLD}
             proc tgt_new args {return NEW}
             namespace eval S {
                 namespace export E
                 namespace ensemble create -command ::S::E -map {x ::tgt_old}
             }
             namespace eval I {namespace import ::S::E}
             namespace eval S {
                 namespace ensemble create -command ::S::E -map {x ::tgt_new}
             }
             list [::I::E x] [namespace origin ::I::E] \
                  [namespace ensemble configure ::I::E -map]"),
        "NEW ::S::E {x ::tgt_new}"
    );

    // A true source deletion removes the import. Recreating an unrelated
    // command at the old source name does not resurrect it; configure and
    // invocation retain their distinct Tcl LOOKUP messages. Exact 9.0.4.
    assert_eq!(
        run("proc tgt_old args {return OLD}
             proc tgt_new args {return NEW}
             namespace eval S {
                 namespace export E
                 namespace ensemble create -command ::S::E -map {x ::tgt_old}
             }
             namespace eval I {namespace import ::S::E}
             rename ::S::E {}
             set before [list [info commands ::I::E] \
                              [namespace ensemble exists ::I::E]]
             set cc [catch {namespace ensemble configure ::I::E} cm co]
             set ic [catch {::I::E x} im io]
             namespace eval S {
                 namespace ensemble create -command ::S::E -map {x ::tgt_new}
             }
             list $before $cc $cm [dict get $co -errorcode] \
                  $ic $im [dict get $io -errorcode] \
                  [info commands ::I::E] [namespace ensemble exists ::I::E]"),
        "{{} 0} 1 {unknown command \"::I::E\"} {TCL LOOKUP COMMAND ::I::E} \
         1 {invalid command name \"::I::E\"} {TCL LOOKUP COMMAND ::I::E} {} 0"
    );
}

#[test]
fn transitive_import_tracks_an_intermediate_replacement_and_deletion() {
    // Replacing an imported binding turns that name into a real command token.
    // Its downstream imports immediately observe the new implementation and
    // origin; deleting that new source token then removes them. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             proc ::A::p {} {return A}
             set before [list [::B::p] [namespace origin ::B::p]]
             rename ::A::p {}
             list $before [info commands ::B::p]"),
        "{A ::A::p} {}"
    );
}

#[test]
fn deleting_an_imported_intermediate_retires_its_downstream_imports() {
    // A::p is still an imported command token here. Deleting it retires B::p,
    // which imports that immediate token, without retiring the upstream S::p
    // implementation. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             rename ::A::p {}
             list [info commands ::A::p] [info commands ::B::p] \
                  [::S::p] [namespace origin ::S::p]"),
        "{} {} S ::S::p"
    );
}

#[test]
fn import_delete_trace_can_reimport_the_same_origin_at_the_same_name() {
    // The callback creates a new imported command token whose name and origin
    // equal the dying token's. Stable binding identity, rather than provenance
    // equality alone, keeps that replacement alive. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p}
             proc reimport {old new op} {
                 namespace eval ::A {namespace import -force ::S::p}
             }
             trace add command ::A::p delete reimport
             rename ::A::p {}
             list [info commands ::A::p] [::A::p] \
                  [namespace origin ::A::p]"),
        "::A::p S ::S::p"
    );
}

#[test]
fn import_delete_retires_the_exact_generation_after_trace_relocation() {
    // The delete callback can move the dying imported token. Retirement
    // follows its stable generation, including relocated trace sidecars, and
    // leaves neither spelling callable. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc move {old new op} {rename $old ::I::q}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete move
             rename ::S::p {}
             namespace eval I {
                 set code [catch {q} message options]
                 list [info commands ::I::p] [info commands ::I::q] \
                      $code $message [dict get $options -errorcode]
             }"),
        "{} {} 1 {invalid command name \"q\"} {TCL LOOKUP COMMAND q}"
    );

    // Hidden and exposed domains carry that same generation. The callback's
    // double relocation leaves neither a hidden token nor the exposed global
    // command behind. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc move {old new op} {
                 interp hide {} $old held
                 interp expose {} held q
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete move
             rename ::S::p {}
             set code [catch {q} message options]
             list [info commands ::I::p] [info commands ::q] \
                  [interp hidden {}] $code $message \
                  [dict get $options -errorcode]"),
        "{} {} {} 1 {invalid command name \"q\"} {TCL LOOKUP COMMAND q}"
    );
}

#[test]
fn interp_hide_and_expose_failures_keep_typed_tcl_identities() {
    // Exercise the current interpreter, a named child, and the child's own
    // command entry point. Names with spaces also prove that lookup codes are
    // encoded as Tcl lists rather than joined diagnostic strings. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("proc outcome script {
                 set code [catch {uplevel 1 $script} message options]
                 list $code $message [dict get $options -errorcode]
             }
             set out {}
             lappend out [outcome {interp hide {} {not here}}]
             lappend out [outcome {interp expose {} {not hidden}}]
             proc p {} {return P}
             interp hide {} p held
             proc q {} {return Q}
             lappend out [outcome {interp hide {} q held}]
             lappend out [outcome {interp expose {} held q}]
             interp create kid
             lappend out [outcome {interp hide kid {not here}}]
             lappend out [outcome {interp expose kid {not hidden}}]
             kid eval {
                 proc p {} {return P}
                 interp hide {} p held
                 proc q {} {return Q}
             }
             lappend out [outcome {interp hide kid q held}]
             lappend out [outcome {interp expose kid held q}]
             lappend out [outcome {kid hide {not here}}]
             lappend out [outcome {kid expose {not hidden}}]
             kid eval {
                 proc a {} {return A}
                 interp hide {} a
                 proc a {} {return NEW}
             }
             lappend out [outcome {kid hide a}]
             lappend out [outcome {kid expose a}]
             set out"),
        "{1 {unknown command \"not here\"} {TCL LOOKUP COMMAND {not here}}} \
         {1 {unknown hidden command \"not hidden\"} {TCL LOOKUP HIDDENTOKEN {not hidden}}} \
         {1 {hidden command named \"held\" already exists} {TCL HIDE ALREADY_HIDDEN}} \
         {1 {exposed command \"q\" already exists} {TCL EXPOSE COMMAND_EXISTS}} \
         {1 {unknown command \"not here\"} {TCL LOOKUP COMMAND {not here}}} \
         {1 {unknown hidden command \"not hidden\"} {TCL LOOKUP HIDDENTOKEN {not hidden}}} \
         {1 {hidden command named \"held\" already exists} {TCL HIDE ALREADY_HIDDEN}} \
         {1 {exposed command \"q\" already exists} {TCL EXPOSE COMMAND_EXISTS}} \
         {1 {unknown command \"not here\"} {TCL LOOKUP COMMAND {not here}}} \
         {1 {unknown hidden command \"not hidden\"} {TCL LOOKUP HIDDENTOKEN {not hidden}}} \
         {1 {hidden command named \"a\" already exists} {TCL HIDE ALREADY_HIDDEN}} \
         {1 {exposed command \"a\" already exists} {TCL EXPOSE COMMAND_EXISTS}}"
    );
}

#[test]
fn interp_visibility_qualifier_validation_is_shared_by_every_entry_form() {
    // The one-word `$child` shorthand uses that word for both roles. Hide
    // diagnoses the hidden-token role first; expose diagnoses its visible
    // destination first. Explicit forms also validate each distinct role.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc outcome script {
                 set code [catch {uplevel 1 $script} message options]
                 list $code $message [dict get $options -errorcode]
             }
             proc p {} {return P}
             namespace eval N {proc p {} {return NP}}
             interp create kid
             kid eval {
                 proc p {} {return P}
                 namespace eval N {proc p {} {return NP}}
             }
             list \
                 [outcome {interp hide {} N::p}] \
                 [outcome {interp expose {} N::p}] \
                 [outcome {interp hide kid N::p}] \
                 [outcome {interp expose kid N::p}] \
                 [outcome {kid hide N::p}] \
                 [outcome {kid expose N::p}] \
                 [outcome {interp hide kid p N::held}] \
                 [outcome {interp expose kid N::held q}] \
                 [outcome {interp hide kid N::p held}] \
                 [outcome {interp expose kid held N::q}]"),
        "{1 {cannot use namespace qualifiers in hidden command token (rename)} \
             {TCL VALUE HIDDENTOKEN}} \
         {1 {cannot expose to a namespace (use rename then expose)} \
             {TCL EXPOSE NON_GLOBAL}} \
         {1 {cannot use namespace qualifiers in hidden command token (rename)} \
             {TCL VALUE HIDDENTOKEN}} \
         {1 {cannot expose to a namespace (use rename then expose)} \
             {TCL EXPOSE NON_GLOBAL}} \
         {1 {cannot use namespace qualifiers in hidden command token (rename)} \
             {TCL VALUE HIDDENTOKEN}} \
         {1 {cannot expose to a namespace (use rename then expose)} \
             {TCL EXPOSE NON_GLOBAL}} \
         {1 {cannot use namespace qualifiers in hidden command token (rename)} \
             {TCL VALUE HIDDENTOKEN}} \
         {1 {cannot use namespace qualifiers in hidden command token (rename)} \
             {TCL VALUE HIDDENTOKEN}} \
         {1 {can only hide global namespace commands (use rename then hide)} \
             {TCL HIDE NON_GLOBAL}} \
         {1 {cannot expose to a namespace (use rename then expose)} \
             {TCL EXPOSE NON_GLOBAL}}"
    );
}

#[test]
fn imported_binding_delete_trace_runs_before_unlink_and_is_reentrant() {
    // The imported binding remains in its namespace table while its delete
    // trace runs. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {lappend ::seen [info commands $old]}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete observed
             rename ::S::p {}
             set seen"),
        "::I::p"
    );

    // A delete trace may replace the imported binding. The outer source
    // deletion must not unlink that newly-created real command. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("proc replace {old new op} {proc $old {} {return REPLACED}}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete replace
             rename ::S::p {}
             list [::I::p] [namespace origin ::I::p]"),
        "REPLACED ::I::p"
    );
}

#[test]
fn import_delete_traces_are_depth_first_with_visible_ancestors() {
    // Deleting imported A::p traces A first, then recursively B, while both
    // bindings remain visible through B's callback. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {
                 lappend ::seen [list $old [info commands ::A::p] \
                                      [info commands ::B::p]]
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             trace add command ::A::p delete observed
             trace add command ::B::p delete observed
             rename ::A::p {}
             set seen"),
        "{::A::p ::A::p ::B::p} {::B::p ::A::p ::B::p}"
    );

    // A real source trace runs before its import cascade, with source/import
    // bindings still visible. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {
                 lappend ::seen [list $old [info commands ::S::p] \
                                      [info commands ::A::p]]
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p}
             trace add command ::S::p delete observed
             trace add command ::A::p delete observed
             rename ::S::p {}
             set seen"),
        "{::S::p ::S::p ::A::p} {::A::p ::S::p ::A::p}"
    );

    // Direct ImportRefs are linked newest-first, independent of map iteration.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {lappend ::seen $old}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p}
             namespace eval B {namespace import ::S::p}
             trace add command ::A::p delete observed
             trace add command ::B::p delete observed
             rename ::S::p {}
             set seen"),
        "::B::p ::A::p"
    );
}

#[test]
fn namespace_import_rejects_a_cycle_before_mutating_the_graph() {
    // Without -force, Tcl diagnoses the occupied destination before walking
    // the prospective source chain. The import graph is unchanged.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p; namespace export p}
             set code [catch {
                 namespace eval S {namespace import ::B::p}
             } message options]
             list $code $message [dict get $options -errorcode] \
                  [namespace origin ::S::p] [namespace origin ::A::p] \
                  [namespace origin ::B::p]"),
        "1 {can't import command \"p\": already exists} \
         {TCL IMPORT OVERWRITE} ::S::p ::S::p ::S::p"
    );

    // Tcl_Import walks the source import chain before a forced overwrite and
    // rejects an edge back to the destination token. Exact Tcl 9.0.4 oracle:
    // the error is structured and all three pre-existing bindings survive.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p; namespace export p}
             set code [catch {
                 namespace eval S {namespace import -force ::B::p}
             } message options]
             list $code $message [dict get $options -errorcode] \
                  [namespace origin ::S::p] [namespace origin ::A::p] \
                  [namespace origin ::B::p]"),
        "1 {import pattern \"::B::p\" would create a loop containing command \"::S::p\"} \
         {TCL IMPORT LOOP} ::S::p ::S::p ::S::p"
    );

    // Retirement is consequently finite and removes only the selected import.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             rename ::B::p {}
             list [info commands ::S::p] [info commands ::A::p] \
                  [info commands ::B::p]"),
        "::S::p ::A::p {}"
    );
}

#[test]
fn dying_namespace_handles_are_rejected_during_command_delete_traces() {
    // Tcl marks an ordinary namespace token dead before retiring member
    // commands. Parent/children must therefore use the same lifecycle-aware
    // handle lookup as exists, while the command token remains observable.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 set pc [catch {namespace parent ::N} pm po]
                 set cc [catch {namespace children ::N} cm co]
                 lappend ::seen [list [namespace exists ::N] \
                     [info commands $old] $pc $pm [dict get $po -errorcode] \
                     $cc $cm [dict get $co -errorcode]]
             }
             namespace eval N {proc p {} {}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set seen"),
        "{0 ::N::p 1 {namespace \"::N\" not found} {TCL LOOKUP NAMESPACE ::N} 1 \
         {namespace \"::N\" not found} {TCL LOOKUP NAMESPACE ::N}}"
    );
}

#[test]
fn retained_namespace_handle_remains_dead_after_deletion_returns() {
    // Deletion unlinks the namespace name but an active proc retains the old
    // token. Its display name survives; name-based lookup through that token
    // must remain dead after teardown has finished. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("namespace eval N {
                 proc p {} {
                     namespace delete ::N
                     set code [catch {namespace parent {}} message options]
                     list [namespace current] [namespace exists {}] $code \
                          $message [dict get $options -errorcode]
                 }
                 p
             }"),
        "::N 0 1 {namespace \"\" not found in \"::N\"} {TCL LOOKUP NAMESPACE {}}"
    );
}

#[test]
fn recreated_namespace_does_not_revive_a_retained_dead_token() {
    // A recreated spelling is a fresh namespace token. The old activation
    // continues to name (and reject lookup through) its deleted identity while
    // absolute lookup reaches the new live token. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("namespace eval N {
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {}
                     set old [list [namespace current] [namespace exists {}] \
                                  [namespace exists ::N]]
                     set old_code [catch {namespace parent {}} old_message old_options]
                     set new_code [catch {namespace parent ::N} new_message]
                     list $old $old_code $old_message \
                          [dict get $old_options -errorcode] $new_code $new_message
                 }
                 p
             }"),
        "{::N 0 1} 1 {namespace \"\" not found in \"::N\"} \
         {TCL LOOKUP NAMESPACE {}} 0 ::"
    );
}

#[test]
fn import_created_by_a_source_delete_trace_joins_the_old_token_cascade() {
    // The source remains importable during its delete trace. The resulting
    // alias references the dying token and is recursively retired afterwards.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc create_import {old new op} {
                 namespace eval ::A {namespace import ::S::p}
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             trace add command ::S::p delete create_import
             rename ::S::p {}
             list [info commands ::S::p] [info commands ::A::p]"),
        "{} {}"
    );
}

#[test]
fn imported_ensemble_keeps_source_identity_across_hide_and_expose() {
    let setup = "proc tgt_old args {return OLD}
                 namespace eval S {
                     namespace ensemble create -command ::E -map {x ::tgt_old}
                 }
                 namespace export E
                 namespace eval I {namespace import ::E}
                 interp hide {} ::E held
                 proc ::E args {return REPLACEMENT}\n";
    // A visible same-name replacement must not capture the import whose real
    // source is the hidden token. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run(&format!(
            "{setup}list [::I::E x] [namespace origin ::I::E] \
             [namespace ensemble configure ::I::E -map] \
             [namespace ensemble exists ::I::E]"
        )),
        "OLD ::held {x ::tgt_old} 1"
    );
    assert_eq!(
        run(&format!(
            "{setup}interp expose {{}} held E2
             list [::I::E x] [namespace origin ::I::E] \
                  [namespace ensemble configure ::I::E -map] [::E2 x]"
        )),
        "OLD ::E2 {x ::tgt_old} OLD"
    );
}

#[test]
fn command_lookup_codes_are_attached_only_by_real_lookup_failures() {
    // Message text is not identity: a user-generated error that happens to use
    // Tcl's lookup wording keeps NONE, while the bootstrap `unknown` path for a
    // true miss carries the command name in TCL LOOKUP COMMAND. Exact 9.0.4.
    assert_eq!(
        run(
            "set fc [catch {error {invalid command name \"fabricated\"}} fm fo]
             set mc [catch {definitely_missing} mm mo]
             list $fc $fm [dict get $fo -errorcode] \
                  $mc $mm [dict get $mo -errorcode]"
        ),
        "1 {invalid command name \"fabricated\"} NONE \
         1 {invalid command name \"definitely_missing\"} \
         {TCL LOOKUP COMMAND definitely_missing}"
    );
}

#[test]
fn lookup_error_codes_quote_dynamic_names_as_list_elements() {
    // Error codes are four-element Tcl lists even when the looked-up name is
    // not itself a valid bare list element. Exact Tcl 9.0.4 oracle results.
    assert_eq!(
        run("catch {namespace origin {not here}} m o
             list $m [dict get $o -errorcode] \
                  [llength [dict get $o -errorcode]]"),
        "{invalid command name \"not here\"} \
         {TCL LOOKUP COMMAND {not here}} 4"
    );
    assert_eq!(
        run("namespace eval E {
                 proc go {} {}
                 namespace export go
                 namespace ensemble create -command ::ens
             }
             catch {::ens {not here}} m o
             list $m [dict get $o -errorcode] \
                  [llength [dict get $o -errorcode]]"),
        "{unknown or ambiguous subcommand \"not here\": must be go} \
         {TCL LOOKUP SUBCOMMAND {not here}} 4"
    );
    assert_eq!(
        run("namespace eval A {
                 catch {namespace parent {not here}} pm po
                 catch {namespace children {also not here}} cm co
                 list $pm [dict get $po -errorcode] \
                      $cm [dict get $co -errorcode]
             }"),
        "{namespace \"not here\" not found in \"::A\"} \
         {TCL LOOKUP NAMESPACE {not here}} \
         {namespace \"also not here\" not found in \"::A\"} \
         {TCL LOOKUP NAMESPACE {also not here}}"
    );
}

/// Reconfiguring an ensemble reaches every importer in the chain, not just the
/// direct ones. C hands each import the *source's* command token, so a
/// two-hop chain (`::S::e` imported and re-exported by `::A`, imported again
/// by `::B`) observes one shared definition; the VM clones the dispatcher, so
/// the refresh has to follow provenance to its ultimate origin. Every
/// expectation below is tclsh 9.0.4- and 8.6.16-pinned.
#[test]
fn reconfiguring_an_ensemble_reaches_transitive_importers() {
    let setup = "namespace eval S {\n\
                     namespace export e\n\
                     proc impl1 {args} {return OLD}\n\
                     proc impl2 {args} {return NEW}\n\
                     namespace ensemble create -command ::S::e -map {go ::S::impl1}}\n\
                 namespace eval A {namespace import ::S::e; namespace export e}\n\
                 namespace eval B {namespace import ::A::e}\n\
                 namespace ensemble configure ::S::e -map {go ::S::impl2}\n";
    // Dispatch: all three spellings run the new target.
    assert_eq!(run(&format!("{setup}::S::e go")), "NEW");
    assert_eq!(run(&format!("{setup}::A::e go")), "NEW");
    assert_eq!(run(&format!("{setup}::B::e go")), "NEW");
    // Config query agrees with dispatch at every spelling — the pair that
    // diverged while only direct edges were refreshed.
    for spelling in ["::S::e", "::A::e", "::B::e"] {
        assert_eq!(
            run(&format!(
                "{setup}namespace ensemble configure {spelling} -map"
            )),
            "go ::S::impl2",
            "{spelling}"
        );
    }
    // Provenance is untouched: both imports still answer the original source.
    assert_eq!(run(&format!("{setup}namespace origin ::A::e")), "::S::e");
    assert_eq!(run(&format!("{setup}namespace origin ::B::e")), "::S::e");
}

/// A namespace-scoped link (`namespace upvar`, or an `upvar` at the global
/// level) is a real cell in the namespace's table, so `namespace which
/// -variable` and `info vars` both see it. A *proc*-local alias is a
/// `CompiledLocal` in C and stays invisible. Both directions tclsh-pinned.
#[test]
fn namespace_scoped_links_are_namespace_variables() {
    assert_eq!(
        run("set ::x 42\nnamespace eval n {namespace upvar :: x y}\n\
             namespace eval n {namespace which -variable y}"),
        "::n::y"
    );
    assert_eq!(
        run("set ::x 42\nnamespace eval n {namespace upvar :: x y}\n\
             info vars ::n::*"),
        "::n::y"
    );
    // A link made at the global level is a global-namespace cell too.
    assert_eq!(
        run("set ::src 5\nupvar #0 ::src galias\nnamespace which -variable galias"),
        "::galias"
    );
    // A proc-local alias is not: `z` names no cell in the namespace.
    assert_eq!(
        run("set ::x 1\nproc p {} {upvar #0 ::x z; return [namespace which -variable z]}\np"),
        ""
    );
    // An ordinary namespace variable is unaffected.
    assert_eq!(
        run("namespace eval n2 {variable v 1}\nnamespace eval n2 {namespace which -variable v}"),
        "::n2::v"
    );
}

/// `-map` is a dict, so the read-back preserves the order the pairs were given
/// (never sorted) and a repeated key keeps its first position while taking the
/// last value. All three expectations are tclsh 9.0.4-pinned.
#[test]
fn ensemble_map_preserves_dict_order_and_collapses_repeats() {
    let setup = "namespace eval M {namespace export *\n\
                 proc zeta {} {return Z}\n\
                 proc alpha {} {return A}\n\
                 proc mid {} {return M}\n\
                 namespace ensemble create -command ::E -map {zz zeta aa alpha}}\n";
    // Insertion order, not sorted — `aa` would sort first.
    assert_eq!(
        run(&format!("{setup}namespace ensemble configure ::E -map")),
        "zz ::M::zeta aa ::M::alpha"
    );
    // A later `configure` replaces the map wholesale, in its own order.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval M {{namespace ensemble configure ::E \
             -map {{mm mid zz zeta aa alpha}}}}\n\
             namespace ensemble configure ::E -map"
        )),
        "mm ::M::mid zz ::M::zeta aa ::M::alpha"
    );
    // A repeated key: last value wins, first position kept.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval M {{namespace ensemble configure ::E \
             -map {{zz zeta aa alpha zz mid}}}}\n\
             namespace ensemble configure ::E -map"
        )),
        "zz ::M::mid aa ::M::alpha"
    );
    // Dispatch follows the collapsed entry, not the stale first one.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval M {{namespace ensemble configure ::E \
             -map {{zz zeta aa alpha zz mid}}}}\n\
             ::E zz"
        )),
        "M"
    );
}

#[test]
fn ensemble_configure_reads_and_writes_the_config_table() {
    let setup = "namespace eval e5 {namespace export *; proc go {} {return G}\n\
                 namespace ensemble create -command ::ab5 -subcommands go}\n";
    // The query dict, in C's option order.
    assert_eq!(
        run(&format!("{setup}namespace ensemble configure ::ab5")),
        "-map {} -namespace ::e5 -parameters {} -prefixes 1 -subcommands go -unknown {}"
    );
    // A single (abbreviated) option reads its value.
    assert_eq!(
        run(&format!("{setup}namespace ensemble configure ::ab5 -sub")),
        "go"
    );
    // `-namespace` is readable but never writable.
    assert_eq!(
        run(&format!(
            "{setup}catch {{namespace ensemble configure ::ab5 -namespace ::e5}} m o
             list $m [dict get $o -errorcode]"
        )),
        "{option -namespace is read-only} {TCL ENSEMBLE READ_ONLY}"
    );
    // `-command` is create-only, so it is not in the configure table.
    assert_eq!(
        run(&format!(
            "{setup}catch {{namespace ensemble configure ::ab5 -command ::zz}} m; set m"
        )),
        "bad option \"-command\": must be -map, -namespace, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
    // An odd tail longer than one word is `wrong # args`.
    assert_eq!(
        run(&format!(
            "{setup}catch {{namespace ensemble configure ::ab5 -subcommands go -prefixes}} m; set m"
        )),
        "wrong # args: should be \"namespace ensemble configure cmdname \
         ?-option value ...? ?arg ...?\""
    );
    // A write takes effect.
    assert_eq!(
        run(&format!(
            "{setup}namespace ensemble configure ::ab5 -prefixes 0\n\
             namespace ensemble configure ::ab5 -prefixes"
        )),
        "0"
    );
}

#[test]
fn ensemble_dispatch_messages_match_c() {
    // `NsEnsembleImplementationCmd`'s three miss shapes. Note the ensemble
    // enumeration keeps its comma before `or` for two entries.
    let two = "namespace eval e7 {namespace export *; proc bar {} {}; proc baz {} {}\n\
               namespace ensemble create -command ::ab8}\n";
    assert_eq!(
        run(&format!("{two}catch {{::ab8 ba}} m; set m")),
        "unknown or ambiguous subcommand \"ba\": must be bar, or baz"
    );
    assert_eq!(
        run(&format!("{two}catch {{::ab8 zz}} m; set m")),
        "unknown or ambiguous subcommand \"zz\": must be bar, or baz"
    );
    assert_eq!(
        run(&format!(
            "{two}catch {{::ab8 zz}} m o; list $m [dict get $o -errorcode]"
        )),
        "{unknown or ambiguous subcommand \"zz\": must be bar, or baz} \
         {TCL LOOKUP SUBCOMMAND zz}"
    );
    // `-prefixes 0` drops the "or ambiguous" half.
    let exact = "namespace eval e9 {namespace export *; proc bar {} {}; proc baz {} {}\n\
                 namespace ensemble create -command ::ab10 -prefixes 0}\n";
    assert_eq!(
        run(&format!("{exact}catch {{::ab10 ba}} m; set m")),
        "unknown subcommand \"ba\": must be bar, or baz"
    );
    // An ensemble over a namespace that exports nothing has its own message.
    assert_eq!(
        run(
            "namespace eval e8 {namespace ensemble create -command ::ab9}\n\
             catch {::ab9 zz} m; set m"
        ),
        "unknown subcommand \"zz\": namespace ::e8 does not export any commands"
    );
}

#[test]
fn ensemble_subcommand_scan_matches_c_on_the_empty_word() {
    // C's prefix scan is a `strncmp` over the word's length, so an empty
    // subcommand prefixes every entry: a one-entry table resolves outright
    // (unlike `Tcl_GetIndexFromObj`, which forces the error path for an empty
    // key), a two-entry one is ambiguous. tclsh 8.6.16 / 9.0.4: G.
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}}\n\
             namespace eval e5 {namespace ensemble create -command ::ab5}\n\
             ::ab5 {}"
        ),
        "G"
    );
    assert_eq!(
        run(
            "namespace eval e7 {namespace export *; proc bar {} {}; proc baz {} {}\n\
             namespace ensemble create -command ::ab8}\n\
             catch {::ab8 {}} m; set m"
        ),
        "unknown or ambiguous subcommand \"\": must be bar, or baz"
    );
    // `-prefixes 0` turns the empty word into a plain miss.
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}}\n\
             namespace eval e5 {namespace ensemble create -command ::ab5 -prefixes 0}\n\
             catch {::ab5 {}} m; set m"
        ),
        "unknown subcommand \"\": must be go"
    );
}

#[test]
fn ensemble_map_validation_matches_tcl_dict_and_target_errors() {
    assert_eq!(
        run("catch {namespace ensemble create -map {go}} m; set m"),
        "missing value to go with key"
    );
    assert_eq!(
        run("catch {namespace ensemble create -map {go {}}} m; set m"),
        "ensemble subcommand implementations must be non-empty lists"
    );
    assert_eq!(
        run("set badmap \"go \\{\"
             catch {namespace ensemble create -map $badmap} m; set m"),
        "unmatched open brace in dict"
    );
}

#[test]
fn ensemble_unknown_result_is_redispatched_as_a_command_prefix() {
    assert_eq!(
        run("proc uh {ens args} {return [list list REPLACED $ens]}
             namespace eval se5 {
                 namespace ensemble create -command ::se5 -subcommands {} -unknown ::uh
             }
             ::se5 nope 1 2"),
        "REPLACED ::se5 1 2"
    );
    // An empty successful result requests one reparse, so a command exported
    // by the callback becomes visible to the same ensemble invocation.
    assert_eq!(
        run("proc define {ens args} {
                 namespace eval se6 {proc nope args {return DEFINED}; namespace export nope}
                 return {}
             }
             namespace eval se6 {namespace ensemble create -command ::se6 -unknown ::define}
             ::se6 nope"),
        "DEFINED"
    );
    // Reparse reads the live ensemble configuration, not the dispatch-time
    // snapshot. Tcl 9.0.4 makes a map installed by the callback visible now.
    assert_eq!(
        run("proc target args {return TARGET}
             proc repair {ens args} {
                 namespace ensemble configure $ens -map {nope ::target}; return {}
             }
             namespace eval se7 {
                 namespace ensemble create -command ::se7 -unknown ::repair
             }
             ::se7 nope"),
        "TARGET"
    );
}

#[test]
fn ensemble_unknown_nonempty_prefix_uses_live_parameter_count() {
    // A non-empty replacement prefix uses the post-callback `-parameters`
    // layout too. `missing` becomes the live parameter and `ARG` the live
    // subcommand, so Tcl splices `missing` rather than the old trailing ARG.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc reroute {ens args} {
                 namespace ensemble configure $ens -parameters {p}
                 return [list list PREFIX]
             }
             namespace eval LP {
                 namespace ensemble create -command ::LP -subcommands {} \
                     -unknown ::reroute
             }
             ::LP missing ARG"),
        "PREFIX missing"
    );
}

#[test]
fn ensemble_unknown_exceptional_codes_and_malformed_results_match_c() {
    // TCL_ERROR propagates unchanged. The other exceptional callback codes
    // are invalid ensemble results and are converted to UNKNOWN_RESULT using
    // Tcl's completion-code names (or the integer for a custom code). Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set bad_code error
             proc exceptional {ens args} {return -code $::bad_code VALUE}
             namespace eval UC {
                 namespace ensemble create -command ::UC -subcommands {} \
                     -unknown ::exceptional
             }
             set results {}
             foreach code {error break continue return 7} {
                 set bad_code $code
                 set c [catch {::UC nope ARG} m o]
                 lappend results [list $c $m [dict get $o -errorcode]]
             }
             set results"),
        "{1 VALUE NONE} \
         {1 {unknown subcommand handler returned bad code: break} \
            {TCL ENSEMBLE UNKNOWN_RESULT}} \
         {1 {unknown subcommand handler returned bad code: continue} \
            {TCL ENSEMBLE UNKNOWN_RESULT}} \
         {1 {unknown subcommand handler returned bad code: return} \
            {TCL ENSEMBLE UNKNOWN_RESULT}} \
         {1 {unknown subcommand handler returned bad code: 7} \
            {TCL ENSEMBLE UNKNOWN_RESULT}}"
    );

    // A successful callback result is parsed as a Tcl list. Preserve the list
    // parser's VALUE code and the ensemble-specific errorInfo frame. Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc malformed {ens args} {return [string index {{}} 0]}
             namespace eval UML {
                 namespace ensemble create -command ::UML -subcommands {} \
                     -unknown ::malformed
             }
             catch {::UML nope} m o
             list $m [dict get $o -errorcode] \
                  [expr {[string first \
                      {while parsing result of ensemble unknown subcommand handler} \
                      [dict get $o -errorinfo]] >= 0}]"),
        "{unmatched open brace in list} {TCL VALUE LIST BRACE} 1"
    );
}

#[test]
fn ensemble_unknown_retains_the_live_command_token() {
    assert_eq!(
        run("proc target args {return TARGET}
             namespace eval ER {
                 proc repair {ens args} {
                     namespace ensemble configure $ens -map {nope ::target}
                     rename $ens ::ER2
                     return {}
                 }
                 namespace ensemble create -command ::ER -unknown ::ER::repair
             }
             list [::ER nope] [namespace ensemble configure ::ER2 -map] [::ER2 nope]"),
        "TARGET {nope ::target} TARGET"
    );

    assert_eq!(
        run("set seen {}
             proc target args {return TARGET}
             proc repair {ens args} {
                 set ::seen $ens
                 namespace ensemble configure $ens -map {nope ::target}
                 return {}
             }
             namespace eval S {
                 namespace export E
                 namespace ensemble create -command E -unknown ::repair
             }
             namespace eval I {namespace import ::S::E}
             list [::I::E nope] $seen [namespace origin ::I::E]"),
        "TARGET ::S::E ::S::E"
    );

    assert_eq!(
        run("proc target args {return TARGET}
             proc replace {ens args} {
                 rename $ens {}
                 namespace ensemble create -command $ens -map {nope ::target}
                 return \\{
             }
             namespace eval D {
                 namespace ensemble create -command E -unknown ::replace
             }
             set c [catch {::D::E nope} m o]
             list $c $m [dict get $o -errorcode] [::D::E nope]"),
        "1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} TARGET"
    );

    assert_eq!(
        run("proc target args {return TARGET}
             set seen {}
             proc hide_repair {ens args} {
                 namespace ensemble configure $ens -map {nope ::target} -unknown ::hidden_repair
                 interp hide {} $ens heldE
                 return {}
             }
             proc hidden_repair {ens args} {
                 set ::seen $ens
                 return [list ::target]
             }
             namespace ensemble create -command ::EH -unknown ::hide_repair
             set first [::EH nope]
             set seen {}
             set second [interp invokehidden {} heldE other]
             list $first $second $seen [info commands ::EH] [interp hidden {}]"),
        "TARGET TARGET ::heldE {} heldE"
    );

    assert_eq!(
        run("proc zap {ens args} {namespace delete ::ND; return {}}
             namespace eval ND {
                 namespace ensemble create -command ::NDE -unknown ::zap
             }
             set c [catch {::NDE nope} m o]
             list $c $m [dict get $o -errorcode] [info commands ::NDE]"),
        "1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} {}"
    );

    // A hidden real ensemble still belongs to its implementation namespace.
    // Deleting that subtree retires the hidden token and every import of it;
    // the active unknown callback observes the token's deleted state. Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run(
            "proc hidden_zap {ens args} {namespace delete ::HD; return {}}
             namespace eval HD {
                 namespace ensemble create -command ::HDE -unknown ::hidden_zap
             }
             namespace export HDE
             namespace eval HI {namespace import ::HDE}
             interp hide {} ::HDE heldHDE
             set c [catch {interp invokehidden {} heldHDE nope} m o]
             list $c $m [dict get $o -errorcode] [interp hidden {}] \
                  [info commands ::HI::HDE] \
                  [namespace ensemble exists ::HI::HDE]"
        ),
        "1 {unknown subcommand handler deleted its ensemble} \
         {TCL ENSEMBLE UNKNOWN_DELETED} {} {} 0"
    );
}

#[test]
fn ensemble_unknown_deleted_seeds_the_handler_errorinfo_frame() {
    // Tcl seeds this special context before the ordinary command-invocation
    // machinery appends its enclosing frame. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc target args {return TARGET}
             proc replace {ens args} {
                 rename $ens {}
                 namespace ensemble create -command $ens -map {nope ::target}
                 return \\{
             }
             namespace eval D {
                 namespace ensemble create -command E -unknown ::replace
             }
             catch {::D::E nope} message options
             split [dict get $options -errorinfo] [format %c 10]"),
        "{unknown subcommand handler deleted its ensemble} \
         {    (ensemble unknown subcommand handler)} \
         {    invoked from within} {\"::D::E nope\"}"
    );
}

#[test]
fn ensemble_default_target_miss_names_the_rewritten_subcommand() {
    assert_eq!(
        run(
            "namespace eval k1 {namespace ensemble create -subcommands ghost}
             catch {::k1 ghost} m; set m"
        ),
        "invalid command name \"ghost\""
    );

    // The ensemble rewrites only the default target's result spelling. A
    // custom `unknown` command owns all completion options, even if it chooses
    // Tcl's ordinary invalid-command message. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc ::unknown {cmd args} {
                 return -code error -errorcode {CUSTOM USER} \
                     \"invalid command name \\\"$cmd\\\"\"
             }
             namespace eval E {
                 namespace ensemble create -command ::E -subcommands x
             }
             catch {::E x} m o
             list $m [dict get $o -errorcode]"),
        "{invalid command name \"x\"} {CUSTOM USER}"
    );
}

// #1463 — the availability gate covers the TclOO root object commands

#[test]
fn tcloo_roots_follow_their_introducing_release() {
    // Real tclsh 8.4/8.5 have no TclOO at all.
    for release in ["tcl8.4", "tcl8.5"] {
        assert_eq!(
            run_at("catch {oo::class create C {}} m; set m", release),
            "invalid command name \"oo::class\"",
            "{release}"
        );
        assert_eq!(
            run_at("catch {oo::object new} m; set m", release),
            "invalid command name \"oo::object\"",
            "{release}"
        );
    }
    // 8.6 has oo::class but not TIP 558's oo::configurable.
    assert_eq!(
        run_at("catch {oo::configurable create C {}} m; set m", "tcl8.6"),
        "invalid command name \"oo::configurable\""
    );
    assert_eq!(run_at("oo::class create D {}", "tcl8.6"), "::D");
    assert_eq!(run_at("oo::configurable create C {}", "tcl9.0"), "::C");
}

#[test]
fn a_script_created_object_is_release_invariant() {
    // The gate must consult the registry for the engine-installed roots only:
    // an object command that happens to share a registry name is user code.
    assert_eq!(
        run_at(
            "oo::class create lpop {method m {} {return M}}\n[lpop new] m",
            "tcl8.6"
        ),
        "M"
    );
}

#[test]
fn a_hidden_tcloo_root_cannot_be_renamed_back_into_view() {
    // The removal seam is gated too (#1463's earlier reopen), so the gate
    // cannot be walked around.
    assert_eq!(
        run_at("catch {rename oo::class fresh} m; set m", "tcl8.4"),
        "can't rename \"oo::class\": command doesn't exist"
    );
}

/// A renamed engine-installed root keeps its **registry identity**, so it is
/// still dated by the name the registry knows rather than by the name it now
/// answers to. Real tclsh cannot switch release mid-session, so this pins the
/// invariant the gate exists to enforce rather than a tclsh transcript: the
/// same command, re-pinned to a release that has no such builtin, must
/// disappear exactly as it would have under its original name.
#[test]
fn a_renamed_tcloo_root_still_gates_by_its_registry_identity() {
    // `oo::configurable` is TIP 558 (9.0); renaming it does not make it an
    // 8.6 command.
    assert_eq!(
        run_flipping(&[
            (
                "tcl9.0",
                "rename oo::configurable myconf\nmyconf create C {}"
            ),
            ("tcl8.6", "catch {myconf create D {}} m; set m"),
        ]),
        "invalid command name \"myconf\""
    );
    // The rename really did work on the release that has the command…
    assert_eq!(
        run_flipping(&[(
            "tcl9.0",
            "rename oo::configurable myconf\nmyconf create C {}"
        )]),
        "::C"
    );
    // …and the vacated name is not left gated: it is the user's to take.
    assert_eq!(
        run_flipping(&[
            ("tcl9.0", "rename oo::configurable myconf"),
            (
                "tcl9.0",
                "oo::class create oo::configurable {method m {} {return MINE}}\n\
                 [oo::configurable new] m"
            ),
        ]),
        "MINE"
    );
    // A script-created object stays release-invariant across the same flip —
    // the control that says the gate is keyed on identity, not on being an
    // object command.
    assert_eq!(
        run_flipping(&[
            (
                "tcl9.0",
                "oo::class create keeper {method m {} {return KEPT}}\nkeeper create k"
            ),
            ("tcl8.6", "k m"),
        ]),
        "KEPT"
    );
}
