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

//! `namespace which` and `namespace origin`: variable and command lookup, import chains, lookup error codes and namespace-scoped links.

use super::*;

// `namespace which -variable` never answers with a call frame
// (`NamespaceWhichCmd` → `Tcl_FindNamespaceVar`, tclNamesp.c:4657)

#[test]
fn which_variable_ignores_proc_locals() {
    // tclsh 8.6.16 / 9.0.4: {} — a proc local is not a namespace variable.
    assert_eq!(
        run("proc t {} {set loc 1; namespace which -variable loc}\nt"),
        ""
    );
    // …including inside a namespace: answering `::ns::loc` here would be wrong.
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
/// can tell the two apart. Answering `Some(self)` for every command would be
/// invisible through `namespace origin` — it folds both onto the same
/// answer — leaving the trait unusable for anything that needs the
/// distinction, and disagreeing with the WASM runtime's implementation.
#[test]
fn command_origin_reports_none_for_a_command_that_was_not_imported() {
    use tcl_runtime_api::Namespaces;

    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let svc = BytecodeCompileService::for_profile(profile);
    let asm = svc
        .compile(
            "namespace eval src {namespace export p; proc p {} {return P}}\n\
             namespace eval dst {namespace import ::src::p}",
        )
        .expect("setup compiles");
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(Capture::default()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
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
