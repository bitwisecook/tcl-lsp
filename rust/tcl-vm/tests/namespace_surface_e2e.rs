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
//! #1451, #1453, #1463).
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
    dialect: &'static str,
}

impl CompilerSvc {
    fn for_profile(profile: &'static DialectProfile) -> Self {
        Self {
            registry: tcl_registry::registry_for_profile(profile),
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: profile.name,
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
    let profile = DialectProfile::by_name(release);
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

// ===========================================================================
// #1442 — `namespace which -variable` never answers with a call frame
// (`NamespaceWhichCmd` → `Tcl_FindNamespaceVar`, tclNamesp.c:4657)
// ===========================================================================

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

// ===========================================================================
// #1446 — `namespace export` / `import` leading-option handling
// (`NamespaceExportCmd`/`Tcl_Export`, `NamespaceImportCmd`/`Tcl_Import`)
// ===========================================================================

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

// ===========================================================================
// #1451 — namespace teardown (`TclTeardownNamespace`, tclNamesp.c:1084)
// ===========================================================================

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

// ===========================================================================
// #1453 — ensemble option tables and subcommand resolution
// (`TclNamespaceEnsembleCmd` + `NsEnsembleImplementationCmd`, tclEnsemble.c)
// ===========================================================================

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
            "{setup}catch {{namespace ensemble configure ::ab5 -namespace ::e5}} m; set m"
        )),
        "option -namespace is read-only"
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

// ===========================================================================
// #1463 — the availability gate covers the TclOO root object commands
// ===========================================================================

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
