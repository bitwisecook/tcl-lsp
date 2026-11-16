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

//! Hidden commands in safe interpreters: `hide`, `expose` and the W129 reports through indirection, deferred bodies and ensembles.

use super::*;

#[test]
fn safe_interp_hides_unsafe_commands_and_expose_restores_945() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // tclsh 9.0.4: a `-safe` child hides `source` (and the rest of the
    // non-CMD_IS_SAFE set) — calling it raises `invalid command name`.
    // The source warning retains its explicit child/source assumptions.
    // It does not establish an actual selected hidden command, so the
    // possible source edge remains available to workspace consumers.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `source` in a safe interp warns: {:?}",
        r.diagnostics
    );
    assert_eq!(
        r.source_targets.len(),
        1,
        "conditional visibility does not discard the possible source edge: {:?}",
        r.source_targets
    );
    // A safe command (`puts`) draws no W129; a non-safe interp draws
    // none either.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "interp create -safe s\ninterp eval s { puts hi }\n\
             interp create n\ninterp eval n { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r2.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "safe commands and non-safe interps draw no W129: {:?}",
        r2.diagnostics
    );
    // `interp expose s source` restores the command.
    let mut a3 = Analyser::new();
    let r3 = a3.analyse(
        "interp create -safe s\ninterp expose s source\n\
             interp eval s { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r3.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an exposed command is callable again: {:?}",
        r3.diagnostics
    );
    // `interp hide n source` hides it in a *normal* interp too.
    let mut a4 = Analyser::new();
    let r4 = a4.analyse(
        "interp create n\ninterp hide n source\n\
             interp eval n { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        r4.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "an explicitly hidden command warns in a normal interp: {:?}",
        r4.diagnostics
    );
}

#[test]
fn safe_interp_child_redefinition_of_a_hidden_builtin_is_callable_945() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // tclsh 9.0.4: `proc source {} {…}` inside a safe interp creates a
    // real command in the ordinary command table — a table entirely
    // separate from the hidden set — so calling `source` afterwards
    // runs the user's proc, never `invalid command name`.  The gate
    // must not draw W129 for the call, and the call must still be
    // fully analysed (resolving to the local proc), not skipped as an
    // effect-free hidden call.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { proc source {} { return ok }; source }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a locally-redefined name is callable, not hidden: {:?}",
        r.diagnostics
    );
    assert!(
        r.all_procs.contains_key("::@interp@s::source"),
        "the local proc is recorded, not skipped: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    // A call *preceding* the redefinition is unaffected by this file's
    // whole-body approximation in the other direction — still hidden
    // until the walk has seen a local definition — is not asserted
    // here (the analyser's per-body model is not that finely ordered);
    // what matters is that a real, common redefinition idiom never
    // produces a false-positive W129 that also drops the call's facts.
}

#[test]
fn interp_expose_with_a_new_name_tracks_the_new_name_945() {
    // tclsh 9.0.4: `interp expose s source src` restores the hidden
    // `source` implementation under the name `src` — `source` itself
    // stays absent from ordinary lookup, and `src` becomes callable.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp expose s source src\n\
             interp eval s { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "the hidden name itself stays hidden after an exposed rename: {:?}",
        r.diagnostics
    );
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "interp create -safe s\ninterp expose s source src\n\
             interp eval s { src b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r2.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "the new exposed name is callable: {:?}",
        r2.diagnostics
    );
}

// W129 through `[...]` bracket-substitution indirection.

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_lambda_body_reports_hidden_source_1001() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded myPackage 1.0 [list apply {dir {\n\
                     source [file join $dir font.tcl]\n\
                 }} $dir]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `source` nested inside a list-quoted apply lambda body \
             (reached only via `package ifneeded`'s deferred script) warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_in_command_prefix_position_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 trace add variable x write [list apply {{a b c} {\n\
                     exec ls\n\
                 }} $x]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `exec` nested inside a list-quoted apply lambda body \
             reached via a CommandPrefix (`trace add … command`) position warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_after_idle_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 after idle [list apply {x {\n\
                     file delete $x\n\
                 }} val]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `file` nested inside a list-quoted apply lambda body \
             reached via `after idle` warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_hidden_command_directly_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { after idle [list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a bare `[list source …]` in a deferred-call position warns \
             directly, with no `apply` indirection needed: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_in_plain_data_is_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 set data [list apply {x { source $x }} value]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a `[list apply …]` value that is only ever stored, never \
             invoked, must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn list_quoted_apply_lambda_outside_any_safe_interp_is_untouched_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "package ifneeded myPackage 1.0 [list apply {dir {source [file join $dir x]}} $dir]\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "no safe interpreter is involved, so no W129 can ever fire: {:?}",
        r.diagnostics
    );
    assert_eq!(
        r.all_procs.len(),
        0,
        "this fix must not widen the general analyser's scope — \
             the list-quoted lambda body stays un-analysed outside a \
             safe-interpreter context, exactly as before #1001: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_direct_nested_bracket_substitution_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { set x [source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a directly nested `[source …]` substitution warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_direct_nested_bracket_substitution_deep_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { if {[catch {set y [exec ls]} err]} { puts $err } }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a deeply nested `[exec …]` substitution still warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_expand_list_quoted_head_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { {*}[list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "`{{*}}[list source …]` used as the whole statement warns \
             on the effective (expanded) head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_expand_dynamic_var_head_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { set cmdList [list source b.tcl]; {*}$cmdList }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an opaque `{{*}}$var` expansion is not statically resolvable \
             and must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_dynamic_variable_dispatch_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { set cmd source; $cmd b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "dynamic dispatch through a variable is not statically \
             resolvable and must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_eval_list_quoted_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { eval [list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "`eval [list source …]` warns on the resolved head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_uplevel_list_quoted_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { uplevel [list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "`uplevel [list source …]` warns on the resolved head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_safe_command_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { after idle [list apply {x { puts $x }} val] }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a safe command (`puts`) reached via list-quoted apply must \
             not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_redefined_command_not_flagged_through_indirection_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 proc source {} { return ok }\n\
                 after idle [list apply {{} { source }}]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a locally-redefined name reached via list-quoted apply is \
             callable, not hidden: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_alias_bridged_command_not_flagged_through_indirection_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp alias s foo {} source\n\
             interp eval s { after idle [list apply {{} { foo }}] }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an alias bridging a capability in from the trusted parent is \
             not itself a hidden registry name, so it must not warn: {:?}",
        r.diagnostics
    );
}

// Safe-interp visibility survives `analyse_per_item`.

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_reaches_deferred_proc_body_under_per_item_1001() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\ninterp eval s { proc f {} { source foo }; f }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a hidden call inside a deferred proc body warns under \
             incremental (per-item) analysis, matching `Analyser::analyse`: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_reaches_deferred_apply_body_under_per_item_1001() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\ninterp eval s { apply {{} { source foo }} }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a hidden call inside a deferred apply-lambda body warns under \
             incremental (per-item) analysis: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_reaches_deferred_method_body_under_per_item_1001() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\n\
             interp eval s {\n\
                 oo::class create C { method m {} { source foo } }\n\
                 [C new] m\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a hidden call inside a deferred TclOO method body warns under \
             incremental (per-item) analysis: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn deferred_proc_body_outside_any_safe_interp_is_untouched_under_per_item_1001() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item("proc f {} { source foo }\nf\n", "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "no safe interpreter is involved, so no W129 can ever fire: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_deferred_body_redefinition_updates_its_source_visibility() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\n\
             interp eval s {\n\
                 proc f {} { proc source {} { return ok }; source }\n\
                 f\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "the deferred body retains its own original source definition before the later call: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_expand_list_quoted_apply_lambda_body_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { {*}[list apply {dir {source $dir/evil.tcl}} $env(HOME)] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `source` nested inside a `{{*}}[list apply ...]`-invoked \
             lambda body warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_package_ifneeded_then_require_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded evil 1.0 [list apply {dir {source $dir/evil.tcl}} $env(HOME)]\n\
                 package require evil\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "the deferred script's hidden `source` warns once `package \
             require` is present too: {:?}",
        r.diagnostics
    );
}

// Namespace ensemble -map redirection.

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_create_map_redirect_to_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a reporting map does not supply an original mapped-target receipt: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_configure_map_redirect_to_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens\n\
                 namespace ensemble configure myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a reporting map does not supply an original mapped-target receipt: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_default_name_map_redirect_to_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace eval myns { namespace ensemble create -map {go source} }\n\
                 myns go pkg.tcl\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a reporting map does not supply an original mapped-target receipt: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_map_redirect_to_safe_command_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go puts}\n\
                 myens go hi\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an ensemble redirect to a safe command must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn ensemble_map_redirect_outside_any_safe_interp_is_untouched_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace ensemble create -command myens -map {go source}\n\
             myens go pkg.tcl\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "no safe interpreter is involved, so no W129 can ever fire: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_map_redirect_multiword_target_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go {source b.tcl}}\n\
                 myens go\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a multiword reporting prefix does not own its effective head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_configure_map_replaces_not_merges_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens \
                     -map {bad source ok puts}\n\
                 namespace ensemble configure myens -map {ok puts}\n\
                 myens bad\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a subcommand dropped by a later `-map` must not keep warning \
             through its stale mapping: {:?}",
        r.diagnostics
    );
}
