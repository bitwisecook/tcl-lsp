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

//! Rename and import provenance across releases: nested imports, hidden sources, refused alias renames, and deletion through an empty destination.

use super::*;

/// Renaming an import moves its local key, not its source provenance.  A
/// nested import must follow that renamed local key while both `namespace
/// origin` calls still reach the original builtin, exactly as C Tcl's command
/// token links do.
#[test]
fn renamed_imports_preserve_origin_through_nested_imports() {
    let src = concat!(
        "namespace export lassign\n",
        "namespace eval one {namespace import ::lassign; namespace export lassign}\n",
        "namespace eval two {namespace import ::one::lassign}\n",
        "namespace eval one {rename lassign moved}\n",
        "puts \"one=[namespace eval one {namespace origin moved}]\"\n",
        "puts \"two=[namespace eval two {namespace origin lassign}]\"\n",
        "puts \"call=[namespace eval two {lassign {a b} x}]\"\n",
    );
    let want = "one=::lassign\ntwo=::lassign\ncall=b";
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] renamed import"
        );
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] real Tcl oracle");
        }
    }
}

/// Hiding an imported command moves its provenance into the hidden-token
/// sidecar.  Renaming the source while that import is hidden must retarget the
/// hidden provenance too, so exposing it restores the same live source-token
/// relationship that C Tcl preserves.  Imports from a named child namespace
/// and through two nested consumers follow the restored root import, and
/// qualified `namespace forget` recognises the source's new name through every
/// import layer.  The equal visible `held` command pins the typed
/// visible/hidden key domains: it must neither capture nor be captured by the
/// hidden token.
#[test]
fn source_rename_retargets_hidden_import_provenance() {
    let src = concat!(
        "namespace eval outer::src {proc a {} {return imported}; namespace export a}\n",
        "proc held {} {return visible}\n",
        "namespace import ::outer::src::a; namespace export a\n",
        "namespace eval one {namespace import ::a; namespace export a}\n",
        "namespace eval two {namespace import ::one::a}\n",
        "interp hide {} a held\n",
        "rename ::outer::src::a ::outer::src::b\n",
        "interp expose {} held a\n",
        "puts \"root=[namespace origin a];[a]\"\n",
        "puts \"one=[namespace origin ::one::a];[::one::a]\"\n",
        "puts \"two=[namespace origin ::two::a];[::two::a]\"\n",
        "puts \"collision=[held]\"\n",
        "namespace eval two {namespace forget ::outer::src::b}\n",
        "namespace eval one {namespace forget ::outer::src::b}\n",
        "namespace forget ::outer::src::b\n",
        "puts \"forgot=[info commands a],[info commands ::one::a],[info commands ::two::a]\"\n",
    );
    let want = concat!(
        "root=::outer::src::b;imported\n",
        "one=::outer::src::b;imported\n",
        "two=::outer::src::b;imported\n",
        "collision=visible\n",
        "forgot=,,",
    );
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] hidden imported provenance retarget"
        );
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] real Tcl oracle");
        }
    }
}

/// A refused alias rename must undo the retargeting of every import that had
/// followed the provisional new name.  This covers direct and nested
/// `namespace origin` lookups, and proves qualified `namespace forget` still
/// recognises the original source.
#[test]
fn refused_alias_rename_restores_import_provenance() {
    let src = concat!(
        "namespace eval src {interp alias {} ::src::a {} b; namespace export a}\n",
        "namespace eval one {namespace import ::src::a; namespace export a}\n",
        "namespace eval two {namespace import ::one::a}\n",
        "puts \"rename=[catch {rename ::src::a b} m];$m\"\n",
        "puts \"one=[namespace origin ::one::a]\"\n",
        "puts \"two=[namespace origin ::two::a]\"\n",
        "namespace eval one {namespace forget ::src::a}\n",
        "puts \"forgot=[info commands ::one::a]\"\n",
    );
    let want = concat!(
        "rename=1;cannot define or rename alias \"b\": would create a loop\n",
        "one=::src::a\n",
        "two=::src::a\n",
        "forgot=",
    );
    // C 8.4 names the alias by the name it was created with
    // (`aliasPtr->namePtr`; tclsh 8.4.20).
    assert_eq!(
        vm_output(src, TclVersion::V8_4),
        want.replace("alias \"b\"", "alias \"::src::a\""),
        "[V8_4] refused alias rename"
    );
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] refused alias rename"
        );
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] real Tcl oracle");
        }
    }
}

/// An alias-loop check must not provisionally overwrite an engine-registered
/// command hidden by the current release.  That overwrite would invoke its
/// delete callback and discard command/execution traces before rollback could
/// restore the command; a later release change must reveal the original
/// builtin with both trace registrations intact.
#[test]
fn refused_alias_rename_preserves_a_hidden_destination_for_later_releases() {
    let mut vm = tcl_vm::test_support::native_vm_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        Box::new(std::io::stdout()),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    )));
    let trace_setup = vm
        .eval_source(
            "proc on_delete args {lappend ::events delete}\n\
             proc on_enter args {lappend ::events enter}\n\
             trace add command lassign delete on_delete\n\
             trace add execution lassign enter on_enter\n",
        )
        .expect("8.5 trace setup compiles");
    assert!(trace_setup.code.is_ok());
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    )));
    let setup = vm
        .eval_source(
            "namespace eval src {interp alias {} ::src::a {} lassign; namespace export a}\n\
             namespace eval imported {namespace import ::src::a}\n\
             catch {rename ::src::a lassign} message\n\
             list $message [namespace origin ::imported::a] \\
                  [catch {trace info command lassign}] \\
                  [catch {trace info execution lassign}] [info exists ::events]\n",
        )
        .expect("8.4 setup compiles");
    assert!(setup.code.is_ok());
    // `lassign` is unavailable at 8.4, so introspecting its traces errors
    // rather than answering an empty list — C resolves the name with
    // `TCL_LEAVE_ERR_MSG` first (`TraceCommandObjCmd`, `tclTrace.c`
    // 9.0.4:661), measured on tclsh 8.4.20 through 9.0.4. The point of the
    // pair is unchanged: no registration is reachable here, and none fired.
    assert_eq!(
        setup.result.to_str().as_ref(),
        "{cannot define or rename alias \"lassign\": would create a loop} ::src::a 1 1 0"
    );

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    )));
    let probe = vm
        .eval_source("list [namespace origin ::imported::a] [lassign {a b} value] $::events\n")
        .expect("8.5 probe compiles");
    assert!(probe.code.is_ok());
    assert_eq!(probe.result.to_str().as_ref(), "::src::a b enter");
}

/// Candidate selection, rather than only final dispatch, owns release
/// visibility. An unavailable namespace-local builtin must not shadow a later
/// global command with the same tail.
#[test]
fn unavailable_local_candidate_falls_through_to_global_command() {
    let mut vm = tcl_vm::test_support::native_vm_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        Box::new(std::io::stdout()),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    )));
    let setup = vm
        .eval_source("namespace eval n {}; rename lassign ::n::x; proc ::x {} {return global}\n")
        .expect("8.5 setup compiles");
    assert!(setup.code.is_ok());

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    )));
    let probe = vm
        .eval_source("namespace eval n {x}\n")
        .expect("8.4 probe compiles");
    assert!(probe.code.is_ok());
    assert_eq!(probe.result.to_str().as_ref(), "global");
}

#[test]
fn equal_hidden_token_does_not_capture_visible_import_provenance() {
    // Tcl 9.0.4 keeps the imported clone's original lineage when its source is
    // hidden; it must not chase the unrelated visible `b` import merely
    // because the hidden token is also `b`.
    assert_eq!(
        profile_output(
            "namespace eval src {proc a {} {return hidden}; namespace export a}\n\
             namespace eval other {proc b {} {return visible}; namespace export b}\n\
             namespace import ::src::a; namespace import ::other::b; namespace export a b\n\
             namespace eval consumer {namespace import ::a}\n\
             interp hide {} a b\n\
             list [namespace origin ::consumer::a] [namespace origin ::b] \
                  [consumer::a] [b] [interp hidden {}]\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        ),
        "::src::a ::other::b hidden visible b"
    );
}

#[test]
fn retiring_hidden_cross_alias_does_not_delete_equal_visible_command() {
    assert_eq!(
        profile_output(
            "interp create target; target eval {proc p {} {return target}}\n\
             interp alias {} a target p; proc b {} {return visible}\n\
             interp hide {} a b; interp delete target\n\
             list [b] [interp hidden {}] [info commands b]\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        ),
        "visible {} b"
    );
}

/// `rename command {}` is deletion, not the failed-rename transaction path:
/// it drops the command's provenance and traces, leaves alias targets and
/// import sources alone, and deletes a visible builtin on every release that
/// exposes it.
#[test]
fn empty_rename_destination_deletes_commands_across_indirections() {
    let src = concat!(
        "proc tracer args {puts \"trace:[join $args |]\"}\n",
        "proc owned {} {}\n",
        "trace add command owned delete tracer\n",
        "interp alias {} alias {} set\n",
        "namespace eval source {proc imported {} {}; namespace export imported}\n",
        "namespace eval imported {namespace import ::source::imported}\n",
        "rename owned {}\n",
        "rename alias {}\n",
        "rename ::imported::imported {}\n",
        "puts \"owned=[info commands owned] alias=[info commands alias] target=[info commands set] imported=[info commands ::imported::imported] source=[info commands ::source::imported]\"\n",
        "rename lassign {}\n",
        // `append`, not `string cat`: the name is still built at runtime (so
        // the call cannot be folded), but `string cat` is 8.6+ and this vector
        // runs from 8.5.
        "set builtin las\n",
        "append builtin sign\n",
        "puts \"builtin=[catch {$builtin {a b} x} message];$message\"\n",
    );
    let want = concat!(
        "trace:::owned||delete\n",
        "owned= alias= target=set imported= source=::source::imported\n",
        "builtin=1;invalid command name \"lassign\"",
    );
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] empty rename destination"
        );
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] real Tcl oracle");
        }
    }
}

/// Coroutine state follows the command key chosen by Tcl's full lookup rule,
/// rather than the caller namespace spelling.  A `namespace path` source is
/// therefore renamed into the caller namespace, while an empty destination
/// tears down the continuation at the resolved source key.
#[test]
fn namespace_path_resolved_coroutine_rename_and_delete_keep_state_in_sync() {
    let src = concat!(
        "namespace eval ::pr {proc g {} {yield 1; yield 2}; coroutine c g}\n",
        "namespace eval ::pc {namespace path ::pr}\n",
        "namespace eval ::pc {rename c d}\n",
        "puts \"renamed=[::pc::d]\"\n",
        "namespace eval ::pr {coroutine c g}\n",
        "namespace eval ::pc {rename c {}}\n",
        "puts \"deleted=[catch {::pr::c} message];$message\"\n",
    );
    let want = "renamed=2\ndeleted=1;invalid command name \"::pr::c\"";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] path-resolved coroutine"
        );
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] real Tcl oracle");
        }
    }
}
