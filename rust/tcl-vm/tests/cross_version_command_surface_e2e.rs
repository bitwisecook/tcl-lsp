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

//! **The emulated release governs the grammar and the command surface, not
//! just the runtime semantics.**
//!
//! Issues #1462/#1463: a version-pinned VM must reject what its emulated
//! release rejects. Two facts move together with the release:
//!
//! - the **lexing grammar** (#1462) — `{*}` expansion is TIP 157 (8.5+), so
//!   under an 8.4 pin it is a hard `extra characters after close-brace`
//!   exactly as `tclsh8.4` reports it;
//! - the **builtin command surface** (#1463) — `lassign` is 8.5+, `lpop` is
//!   9.0+, so under an older pin they resolve to `invalid command name`,
//!   while user-defined procs (the polyfill pattern) always stay callable.
//!
//! The host wiring below mirrors `tclvm`: one resolved `DialectProfile`
//! drives the compiler's `LexerConfig` + registry AND the VM's runtime pin.
//! Vectors are pinned against tclsh 8.4.20 / 8.6.16 / 9.0.4 behaviour and
//! byte-compared against a real interpreter whenever one is on `PATH`
//! (`TCLSH84` / `TCLSH86` / `TCLSH90` override the binary names).

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_vm::{CompileError, CompileService, Vm};

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
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
}

/// Run `src` on a VM pinned to `profile`, compiling for that profile exactly
/// as `tclvm --tcl-version` does. Returns the `puts` output; a compile
/// rejection or an uncaught runtime error becomes the error's message, so a
/// vector can pin `extra characters after close-brace` and `invalid command
/// name "lassign"` the same way it pins a value.
fn profile_output(src: &str, profile: &'static DialectProfile) -> String {
    let svc = CompilerSvc::for_profile(profile);
    let asm = match svc.compile(src) {
        Ok(asm) => asm,
        Err(e) => return e.0,
    };
    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(CompilerSvc::for_profile(profile)));
    let completion = vm.run_module(&asm);
    let out = String::from_utf8_lossy(&cap.0.borrow()).trim().to_string();
    if !completion.code.is_ok() || out.is_empty() {
        // An uncaught error's message, or — for a profile with no `puts`
        // (the iRules sandbox bans it) — the script's final result.
        return completion.result.to_str().trim().to_string();
    }
    out
}

/// [`profile_output`] for a plain release pin (the `--tcl-version` path).
fn vm_output(src: &str, version: TclVersion) -> String {
    profile_output(src, DialectProfile::by_name(version.dialect_name()))
}

/// One script and what each release must produce (`puts` output, or the
/// compile/runtime error message).
struct Vector {
    name: &'static str,
    script: &'static str,
    want_84: &'static str,
    want_85: &'static str,
    want_86: &'static str,
    want_90: &'static str,
}

const VECTORS: &[Vector] = &[
    // -- #1462: the {*} expansion grammar is a compile fact --
    Vector {
        name: "{*} expansion is a hard parse error before 8.5",
        script: "puts [llength [list {*}{a b}]]\n",
        want_84: "extra characters after close-brace",
        want_85: "2",
        want_86: "2",
        want_90: "2",
    },
    // -- #1463: the builtin surface follows the release --
    Vector {
        name: "lassign arrives in 8.5",
        script: "puts [lassign {a b} x]\n",
        want_84: "invalid command name \"lassign\"",
        want_85: "b",
        want_86: "b",
        want_90: "b",
    },
    Vector {
        name: "dict arrives in 8.5",
        script: "puts [dict get {a 1} a]\n",
        want_84: "invalid command name \"dict\"",
        want_85: "1",
        want_86: "1",
        want_90: "1",
    },
    Vector {
        name: "try arrives in 8.6",
        script: "puts [catch {try { } } m]$m\n",
        want_84: "1invalid command name \"try\"",
        want_85: "1invalid command name \"try\"",
        want_86: "0",
        want_90: "0",
    },
    Vector {
        name: "lmap arrives in 8.6",
        script: "puts [catch {lmap v {1 2} {expr {$v * 2}}} m]$m\n",
        want_84: "1invalid command name \"lmap\"",
        want_85: "1invalid command name \"lmap\"",
        want_86: "02 4",
        want_90: "02 4",
    },
    Vector {
        name: "lpop arrives in 9.0",
        script: "set l {a b c}\nputs [catch {lpop l} m]$m\n",
        want_84: "1invalid command name \"lpop\"",
        want_85: "1invalid command name \"lpop\"",
        want_86: "1invalid command name \"lpop\"",
        want_90: "0c",
    },
    Vector {
        name: "an unavailable builtin is absent from info commands",
        script: "puts [llength [info commands lassign]]\n",
        want_84: "0",
        want_85: "1",
        want_86: "1",
        want_90: "1",
    },
    // -- the polyfill pattern: a user proc always wins over a hidden builtin --
    Vector {
        name: "a user-defined proc is callable at every release",
        script: "proc lassign {l args} { return polyfill }\nputs [lassign {a b} x]\n",
        want_84: "polyfill",
        want_85: "polyfill",
        want_86: "polyfill",
        want_90: "polyfill",
    },
];

#[test]
fn command_surface_follows_the_emulated_release() {
    for v in VECTORS {
        assert_eq!(
            vm_output(v.script, TclVersion::V8_4),
            v.want_84,
            "[8.4] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V8_5),
            v.want_85,
            "[8.5] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V8_6),
            v.want_86,
            "[8.6] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_0),
            v.want_90,
            "[9.0] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_1),
            v.want_90,
            "[9.1] {}",
            v.name
        );
    }
}

/// A release-hidden builtin must stay hidden when Tcl code tries to move it
/// out of the registry's namespace.  `rename` and `interp hide` both remove a
/// command table entry before restoring it under another spelling; that
/// removal must consult the same surface gate as ordinary dispatch.
#[test]
fn hidden_builtin_cannot_escape_through_rename_or_hide() {
    let src = concat!(
        "puts \"direct=[catch {lassign {a b} x y} msg];$msg\"\n",
        "puts \"rename=[catch {rename lassign la} msg];$msg\"\n",
        "puts \"alias=[catch {la {a b} x y} msg];$msg\"\n",
        "puts \"hide=[catch {interp hide {} lassign} msg];$msg\"\n",
        "puts \"expose=[catch {interp expose {} lassign la} msg];$msg\"\n",
        "puts \"after-hide=[catch {la {a b} x y} msg];$msg\"\n",
        "puts \"vars=[info exists x],[info exists y]\"\n",
    );
    assert_eq!(
        vm_output(src, TclVersion::V8_4),
        concat!(
            "direct=1;invalid command name \"lassign\"\n",
            "rename=1;can't rename \"lassign\": command doesn't exist\n",
            "alias=1;invalid command name \"la\"\n",
            "hide=0;\n",
            "expose=0;\n",
            "after-hide=1;invalid command name \"la\"\n",
            "vars=0,0",
        )
    );
    assert_eq!(
        vm_output(src, TclVersion::V9_0),
        concat!(
            "direct=0;\n",
            "rename=0;\n",
            "alias=0;\n",
            "hide=0;\n",
            "expose=1;exposed command \"la\" already exists\n",
            "after-hide=0;\n",
            "vars=1,1",
        )
    );
}

/// Every public command-table mutation and indirection preserves the release
/// surface.  In particular, child-interpreter hide used to mutate the parked
/// table directly, and import used a cloned builtin without its final source
/// identity, so both could expose `lassign` to an emulated 8.4 VM.
#[test]
fn hidden_builtin_stays_hidden_through_children_imports_and_aliases() {
    let src = concat!(
        "interp create child\n",
        "namespace export lassign\n",
        "namespace eval imported {namespace import ::lassign}\n",
        "puts \"same-hide=[catch {interp hide {} lassign la} m];$m\"\n",
        "puts \"same-hidden=[info commands la]\"\n",
        "puts \"same-expose=[catch {interp expose {} la lassign2} m];$m\"\n",
        "puts \"same-call=[catch {lassign2 {a b} x} m];$m\"\n",
        "puts \"child-hide=[catch {interp hide child lassign la} m];$m\"\n",
        "puts \"child-hidden=[child hidden]\"\n",
        "puts \"child-expose=[catch {interp expose child la lassign2} m];$m\"\n",
        "puts \"child-call=[catch {child eval {lassign2 {a b} x}} m];$m\"\n",
        "interp create child_command\n",
        "puts \"child-command-hide=[catch {child_command hide lassign} m];$m\"\n",
        "puts \"child-command-hidden=[child_command hidden]\"\n",
        "puts \"child-command-expose=[catch {child_command expose lassign} m];$m\"\n",
        "puts \"child-command-call=[catch {child_command eval {lassign {a b} x}} m];$m\"\n",
        "puts \"import-visible=[namespace eval imported {info commands lassign}]\"\n",
        "puts \"import-origin=[catch {namespace eval imported {namespace origin lassign}} m];$m\"\n",
        "puts \"import-call=[catch {namespace eval imported {lassign {a b} x}} m];$m\"\n",
        "interp alias {} alias {} lassign\n",
        "puts \"alias-call=[catch {alias {a b} x} m];$m\"\n",
    );
    assert_eq!(
        vm_output(src, TclVersion::V8_4),
        concat!(
            "same-hide=0;\n",
            "same-hidden=\n",
            "same-expose=0;\n",
            "same-call=1;invalid command name \"lassign2\"\n",
            "child-hide=0;\n",
            "child-hidden=\n",
            "child-expose=0;\n",
            "child-call=1;invalid command name \"lassign2\"\n",
            "child-command-hide=0;\n",
            "child-command-hidden=\n",
            "child-command-expose=0;\n",
            "child-command-call=1;invalid command name \"lassign\"\n",
            "import-visible=\n",
            "import-origin=1;invalid command name \"lassign\"\n",
            "import-call=1;invalid command name \"lassign\"\n",
            "alias-call=1;invalid command name \"lassign\"",
        )
    );
    let want_newer = concat!(
        "same-hide=0;\n",
        "same-hidden=\n",
        "same-expose=0;\n",
        "same-call=0;b\n",
        "child-hide=0;\n",
        "child-hidden=la\n",
        "child-expose=0;\n",
        "child-call=0;b\n",
        "child-command-hide=0;\n",
        "child-command-hidden=lassign\n",
        "child-command-expose=0;\n",
        "child-command-call=0;b\n",
        "import-visible=lassign\n",
        "import-origin=0;::lassign2\n",
        "import-call=0;b\n",
        "alias-call=1;invalid command name \"lassign\"",
    );
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(vm_output(src, version), want_newer, "[{version:?}] surface");
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want_newer, "[{env}] real Tcl oracle");
        }
    }
}

/// Reproduce the stateful form of the import escape: importing while the 8.5
/// surface is active, then pinning that same interpreter to 8.4.  The local
/// clone and provenance remain in its table, but resolution must now reject
/// the final builtin identity.
#[test]
fn version_mutation_rechecks_an_existing_imports_source_identity() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
    vm.set_compiler(Box::new(CompilerSvc::for_profile(DialectProfile::by_name(
        "tcl8.5",
    ))));
    let setup = vm
        .eval_source(
            "namespace export lassign\n\
             namespace eval imported {namespace import ::lassign}\n\
             interp hide {} lassign hidden_lassign\n\
             interp expose {} hidden_lassign escaped\n\
             namespace eval imported {rename lassign imported_lassign}\n",
        )
        .expect("8.5 import setup compiles");
    assert!(setup.code.is_ok());
    vm.set_dialect_profile(DialectProfile::by_name("tcl8.4"));
    let probe = vm
        .eval_source(
            "catch {namespace eval imported {set cmd [string cat imported_ lass ign]; $cmd {a b} x}} imported; \\
             catch {set cmd [string cat es caped]; $cmd {a b} x} escaped; \\
             list $imported $escaped\n",
        )
        .expect("8.4 probe compiles");
    assert_eq!(
        probe.result.to_str().as_ref(),
        "{invalid command name \"imported_lassign\"} {invalid command name \"escaped\"}"
    );
}

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
    for version in [
        TclVersion::V8_4,
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
    let mut vm = Vm::new();
    vm.set_dialect_profile(DialectProfile::by_name("tcl8.4"));
    vm.set_compiler(Box::new(CompilerSvc::for_profile(DialectProfile::by_name(
        "tcl8.4",
    ))));
    let setup = vm
        .eval_source(
            "proc on_delete args {lappend ::events delete}\n\
             proc on_enter args {lappend ::events enter}\n\
             trace add command lassign delete on_delete\n\
             trace add execution lassign enter on_enter\n\
             namespace eval src {interp alias {} ::src::a {} lassign; namespace export a}\n\
             namespace eval imported {namespace import ::src::a}\n\
             catch {rename ::src::a lassign} message\n\
             list $message [namespace origin ::imported::a] \\
                  [llength [trace info command lassign]] \\
                  [llength [trace info execution lassign]] [info exists ::events]\n",
        )
        .expect("8.4 setup compiles");
    assert!(setup.code.is_ok());
    assert_eq!(
        setup.result.to_str().as_ref(),
        "{cannot define or rename alias \"lassign\": would create a loop} ::src::a 1 1 0"
    );

    vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
    vm.set_compiler(Box::new(CompilerSvc::for_profile(DialectProfile::by_name(
        "tcl8.5",
    ))));
    let probe = vm
        .eval_source("list [namespace origin ::imported::a] [lassign {a b} value] $::events\n")
        .expect("8.5 probe compiles");
    assert!(probe.code.is_ok());
    assert_eq!(probe.result.to_str().as_ref(), "::src::a b enter");
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
        "set builtin [string cat las sign]\n",
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

/// Hide/expose collisions reject before moving either binding.  The visible
/// destination keeps both command and execution traces, and the hidden token
/// remains invocable, matching Tcl's non-destructive failure contract.
#[test]
fn hide_and_expose_collisions_preserve_bindings_and_traces() {
    let src = concat!(
        "proc a {} {return A}; proc b {} {return B}\n",
        "proc cb args {lappend ::events [lindex $args end]}\n",
        "trace add command b delete cb; trace add execution b enter cb\n",
        "interp hide {} a token\n",
        "puts \"hide=[catch {interp hide {} b token} m];$m\"\n",
        "puts \"expose=[catch {interp expose {} token b} m];$m\"\n",
        "interp expose {} token restored\n",
        "puts \"visible=[b] hidden=[restored] traces=[llength [trace info command b]],[llength [trace info execution b]] events=$::events\"\n",
    );
    let want = concat!(
        "hide=1;hidden command named \"token\" already exists\n",
        "expose=1;exposed command \"b\" already exists\n",
        "visible=B hidden=A traces=1,1 events=enter",
    );
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] hide collision"
        );
    }
    for (env, names) in [("TCLSH90", &["tclsh9.0"][..])] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] real Tcl oracle");
        }
    }
}

#[test]
fn child_command_hide_and_expose_collisions_propagate_without_mutation() {
    let src = concat!(
        "interp create kid\n",
        "kid eval {proc a {} {return A}; proc cb args {lappend ::events [lindex $args end]}}\n",
        "kid hide a\n",
        "kid eval {proc a {} {return B}; trace add command a delete cb; trace add execution a enter cb}\n",
        "puts \"hide=[catch {kid hide a} m];$m\"\n",
        "puts \"expose=[catch {kid expose a} m];$m\"\n",
        "puts \"state=[kid eval {list [a] [llength [trace info command a]] [llength [trace info execution a]]}]\"\n",
    );
    let want = concat!(
        "hide=1;hidden command named \"a\" already exists\n",
        "expose=1;exposed command \"a\" already exists\n",
        "state=B 1 1",
    );
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] child hide collision"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}

#[test]
fn hide_expose_moves_traces_without_callbacks() {
    let src = concat!(
        "proc a {} {return A}; proc cb args {lappend ::events [lindex $args end]}\n",
        "trace add command a {rename delete} cb; trace add execution a enter cb\n",
        "interp hide {} a token\n",
        "interp expose {} token moved\n",
        "puts \"state=[moved] [llength [trace info command moved]] [llength [trace info execution moved]] $::events\"\n",
    );
    let want = "state=A 1 1 enter";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] hide trace move"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}

#[test]
fn child_hide_expose_moves_traces_for_named_and_by_id_forms() {
    let src = concat!(
        "interp create named; interp create byid\n",
        "named eval {proc a {} {return N}; proc cb args {lappend ::events [lindex $args end]}; trace add execution a enter cb}\n",
        "interp hide named a held; puts \"named-hidden=[interp invokehidden named held]\"; interp expose named held moved\n",
        "puts \"named=[named eval {list [moved] [llength [trace info execution moved]] $::events}]\"\n",
        "byid eval {proc a {} {return I}; proc cb args {lappend ::events [lindex $args end]}; trace add execution a enter cb}\n",
        "byid hide a; puts \"byid-hidden=[byid invokehidden a]\"; byid expose a\n",
        "puts \"byid=[byid eval {list [a] [llength [trace info execution a]] $::events}]\"\n",
    );
    let want = "named-hidden=N\nnamed=N 1 {enter enter}\nbyid-hidden=I\nbyid=I 1 {enter enter}";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] child trace move"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}

/// A `catch` body is compiled at run time through the VM's compile service,
/// so an 8.5+-only spelling inside it is a *catchable* error under an 8.4
/// pin — matching C Tcl, which defers a compile-time parse error to a
/// bytecode instruction that raises it at runtime.
#[test]
fn expansion_parse_error_inside_catch_is_catchable_at_8_4() {
    let src = "puts [catch {llength [list {*}{a b}]} m]\nputs $m\n";
    assert_eq!(
        vm_output(src, TclVersion::V8_4),
        "1\nextra characters after close-brace"
    );
    assert_eq!(vm_output(src, TclVersion::V9_0), "0\n2");
}

/// #1463's precision point about vendor profiles: an iRules-pinned VM
/// validates against the vendor availability mask, not plain tcl8.4. Both
/// run a Tcl 8.4 core, but the TMM sandbox (K36322151) additionally bans
/// `source` (and `puts`, which is why the probe reports through the script
/// result rather than an output channel) — under plain tcl8.4 both stay
/// callable.
#[test]
fn vendor_profile_masks_differ_from_the_plain_release() {
    let src = "catch {source /no/such/file.tcl} m\nset m\n";
    assert_eq!(
        profile_output(src, DialectProfile::irules()),
        "invalid command name \"source\"",
        "the TMM sandbox has no source at all"
    );
    let out = profile_output(src, DialectProfile::by_name("tcl8.4"));
    assert!(
        !out.contains("invalid command name"),
        "plain tcl8.4 has source (it merely fails to read the file): {out}"
    );
}

/// Run `src` under a real tclsh, or `None` when that binary isn't available.
fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    use std::io::Write as _;
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var(bin_env) {
        candidates.push(explicit);
    }
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(&name)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            continue;
        };
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(src.as_bytes());
        }
        let Ok(out) = child.wait_with_output() else {
            continue;
        };
        return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    None
}

/// Byte-compare the value-producing vectors against a real interpreter when
/// one is installed; skips silently otherwise. Error-message vectors are
/// excluded — tclsh reports those on stderr with a traceback, so only the
/// `catch`-shaped vectors (whose divergence is on stdout) are comparable.
#[test]
fn vectors_match_real_tclsh_when_available() {
    let mut checked = 0usize;
    for v in VECTORS {
        if !v.script.contains("catch") && !v.script.contains("info commands") {
            continue;
        }
        for (env, names, want) in [
            ("TCLSH84", &["tclsh8.4"][..], v.want_84),
            ("TCLSH86", &["tclsh8.6"][..], v.want_86),
            ("TCLSH90", &["tclsh9.0"][..], v.want_90),
        ] {
            if let Some(out) = tclsh_output(env, names, v.script) {
                assert_eq!(out, want, "[{env}] {}", v.name);
                checked += 1;
            }
        }
    }
    if checked == 0 {
        eprintln!("no system tclsh found — pinned expectations still verified");
    }
}
