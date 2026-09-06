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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_vm::{Code, CompileError, CompileService, ProcedureCompileTarget, ProcedureDispatch, Vm};

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

struct CountingCompilerSvc {
    calls: Rc<Cell<usize>>,
}

impl CompileService for CountingCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.compile_for_profile(src, DialectProfile::plain_tcl())
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.calls.set(self.calls.get() + 1);
        BytecodeCompileService::for_profile(profile).compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        self.calls.set(self.calls.get() + 1);
        BytecodeCompileService::for_profile(profile)
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

struct FixedFallbackCompilerSvc;

impl CompileService for FixedFallbackCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        BytecodeCompileService::default().compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            BytecodeCompileService::default()
                .compile_procedure_for_profile(target, profile, dispatch)
        } else {
            Err(CompileError(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }
}

struct WrongProfileCompilerSvc;

impl CompileService for WrongProfileCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        BytecodeCompileService::for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        )
        .compile(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        _profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        _profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        BytecodeCompileService::for_profile(wrong)
            .compile_procedure_for_profile(target, wrong, dispatch)
    }
}

#[test]
fn named_profiles_reject_fixed_or_mislabelled_compile_services() {
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let mut fixed = Vm::new();
    fixed.set_dialect_profile(v84);
    fixed.set_compiler(Box::new(FixedFallbackCompilerSvc));
    assert_eq!(
        fixed.eval_source("set x 1").unwrap_err().message,
        "CompileService does not support dialect profile tcl8.4"
    );

    let mut mislabelled = Vm::new();
    mislabelled.set_dialect_profile(v84);
    mislabelled.set_compiler(Box::new(WrongProfileCompilerSvc));
    assert_eq!(
        mislabelled
            .compile_function("lassign {a b} x")
            .unwrap_err()
            .message,
        "bytecode compiled for dialect profile tcl8.5 cannot run under tcl8.4"
    );
}

#[test]
fn command_surface_override_accepts_the_universal_fallback_from_tcl91() {
    let v91 = tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
    let fallback = DialectProfile::plain_tcl();
    let mut vm = Vm::new();
    vm.set_dialect_profile(v91);

    assert_eq!(fallback.vm_runtime_version, TclVersion::V9_0);
    assert!(
        vm.set_command_surface_profile(fallback),
        "the permissive fallback must cover every Tcl 9.1 builtin despite its inert 9.0 base"
    );
    assert!(std::ptr::eq(vm.command_surface_profile(), fallback));
}

#[test]
fn named_command_surface_override_cannot_hide_compiled_release_commands() {
    let v91 = tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(v91).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(v91.grammar);
    let ir = lower_to_ir("lassign {a b} x; set x", registry, config, Some(v91));
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, registry);

    let mut vm = Vm::new();
    vm.set_dialect_profile(v91);
    assert!(
        !vm.set_command_surface_profile(v90),
        "a named Tcl 9.0 surface cannot host Tcl 9.1 bytecode"
    );
    assert!(std::ptr::eq(vm.command_surface_profile(), v91));
    let completion = vm.run_module(&module);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(&*completion.result.to_str(), "a");
}

#[test]
fn command_surface_override_rejects_a_same_release_narrower_profile() {
    let bpf = tcl_registry::model::ingress::resolve_environment("bpf").analyser_profile();
    let bigip = tcl_registry::model::ingress::resolve_environment("f5-bigip").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(bpf).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(bpf.grammar);
    // `lassign` becomes list/variable opcodes, so it proves that module
    // profile validation alone cannot enforce a later command-surface change.
    let ir = lower_to_ir("lassign {a b} x; set x", registry, config, Some(bpf));
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, registry);

    let mut vm = Vm::new();
    vm.set_dialect_profile(bpf);
    assert!(
        !vm.set_command_surface_profile(bigip),
        "the config-only BIG-IP surface hides BPF's Tcl bytecode commands"
    );
    assert!(std::ptr::eq(vm.command_surface_profile(), bpf));
    let completion = vm.run_module(&module);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(&*completion.result.to_str(), "a");
}

#[test]
fn compatible_irules_host_surface_override_remains_accepted() {
    let irules = DialectProfile::irules();
    let host = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let mut vm = Vm::new();
    vm.set_dialect_profile(irules);
    assert!(vm.set_command_surface_profile(host));
    assert!(std::ptr::eq(vm.command_surface_profile(), host));
}

/// Run `src` on a VM pinned to `profile`, compiling for that profile exactly
/// as `tclvm --tcl-version` does. Returns the `puts` output; a compile
/// rejection or an uncaught runtime error becomes the error's message, so a
/// vector can pin `extra characters after close-brace` and `invalid command
/// name "lassign"` the same way it pins a value.
fn profile_output(src: &str, profile: &'static DialectProfile) -> String {
    let svc = BytecodeCompileService::for_profile(profile);
    let asm = match svc.compile(src) {
        Ok(asm) => asm,
        Err(e) => return e.0,
    };
    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
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
    profile_output(
        src,
        tcl_registry::model::ingress::resolve_environment(version.dialect_name())
            .analyser_profile(),
    )
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
    // The vector above uses `dict get`, whose argument is a runtime value, so
    // it never reaches the constant folder. An **all-literal** `dict create` in
    // a value position *is* folded in codegen — and a fold is a rewrite that
    // bypasses the runtime's availability gate entirely, so the folded form
    // used to succeed under 8.4 while every unfoldable spelling correctly
    // raised `invalid command name "dict"` (issue #1427).
    Vector {
        name: "a *foldable* dict create is still absent before 8.5",
        script: "puts [dict create a 1 a 2]\n",
        want_84: "invalid command name \"dict\"",
        want_85: "a 2",
        want_86: "a 2",
        want_90: "a 2",
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
    // -- #1607: `package`'s option table follows the release --
    // `prefer` is TIP 268 (8.5); `files` is 9.0's. Byte-checked against
    // tclsh8.4.20 / 8.5.19 / 8.6.16 / 9.0.4, which this suite also re-runs
    // against any installed tclsh.
    Vector {
        name: "package's option list follows the release",
        script: "puts [catch {package zz} m]$m\n",
        want_84: "1bad option \"zz\": must be forget, ifneeded, names, present, \
                  provide, require, unknown, vcompare, versions, or vsatisfies",
        want_85: "1bad option \"zz\": must be forget, ifneeded, names, prefer, present, \
                  provide, require, unknown, vcompare, versions, or vsatisfies",
        want_86: "1bad option \"zz\": must be forget, ifneeded, names, prefer, present, \
                  provide, require, unknown, vcompare, versions, or vsatisfies",
        want_90: "1bad option \"zz\": must be files, forget, ifneeded, names, prefer, \
                  present, provide, require, unknown, vcompare, versions, or vsatisfies",
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
            "hide=1;unknown command \"lassign\"\n",
            "expose=1;unknown hidden command \"lassign\"\n",
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
            "hide=1;unknown command \"lassign\"\n",
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
            "same-hide=1;unknown command \"lassign\"\n",
            "same-hidden=\n",
            "same-expose=1;unknown hidden command \"la\"\n",
            "same-call=1;invalid command name \"lassign2\"\n",
            "child-hide=1;unknown command \"lassign\"\n",
            "child-hidden=\n",
            "child-expose=1;unknown hidden command \"la\"\n",
            "child-call=1;invalid command name \"lassign2\"\n",
            "child-command-hide=1;unknown command \"lassign\"\n",
            "child-command-hidden=\n",
            "child-command-expose=1;unknown hidden command \"lassign\"\n",
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
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    )));
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
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    let probe = vm
        .eval_source(
            "catch {namespace eval imported {set cmd imported_; append cmd lass ign; $cmd {a b} x}} imported; \\
             catch {set cmd es; append cmd caped; $cmd {a b} x} escaped; \\
             list $imported $escaped\n",
        )
        .expect("8.4 probe compiles");
    assert_eq!(
        probe.result.to_str().as_ref(),
        "{invalid command name \"imported_lassign\"} {invalid command name \"escaped\"}"
    );
}

/// Changing a VM's profile is not merely a lookup-cache invalidation: bodies
/// compiled while a newer surface was active can contain direct bytecode for a
/// command that the older profile would reject.  Keep the compiler installed
/// at its original 8.5 construction profile to prove the VM selects the new
/// profile for every recompile itself.
#[test]
#[allow(clippy::too_many_lines)] // One stateful profile lifecycle must stay in one causal vector.
fn profile_mutation_recompiles_cached_bodies_and_rejects_live_continuations() {
    let v85 = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let v86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let mut vm = Vm::new();
    // TclOO is an 8.6 command surface (issue #1463), so the class factory is
    // only reachable there; the resulting object command is script-created and
    // stays callable on every release, which is what the flips below exercise.
    vm.set_dialect_profile(v86);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v85)));
    let oo_setup = vm
        .eval_source(
            "oo::class create C {method m {} {lassign {a b} x; return $x}}\n\
             C create o",
        )
        .expect("8.6 TclOO setup compiles");
    assert!(oo_setup.code.is_ok(), "{}", oo_setup.result.to_str());
    vm.set_dialect_profile(v85);

    // Prime every cache/body owner that retains executable bytecode: an
    // ordinary proc, TclOO method, child proc reached through an alias,
    // reusable host FunctionHandle, eval cache, lexer/parser cache, and a
    // parked coroutine continuation.
    let setup = vm
        .eval_source(
            "proc p {} {lassign {a b} x; return $x}\n\
             expr {0b10 + 1}",
        )
        .expect("8.5 setup compiles");
    assert!(setup.code.is_ok(), "{}", setup.result.to_str());
    let handle = vm
        .compile_function("lassign {a b} handle_x; set handle_x")
        .expect("8.5 handle compiles");
    // A reusable handle is VM-affine: another VM can have the same profile
    // generation but must never adopt its bytecode.
    let mut foreign_vm = Vm::new();
    foreign_vm.set_dialect_profile(v85);
    foreign_vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v85)));
    assert_eq!(
        foreign_vm.invoke_function(&handle).result.to_str().as_ref(),
        "FunctionHandle belongs to a different Vm"
    );
    // AOT modules carry the profile they were lowered for and are rejected at
    // the entry boundary instead of being stamped with the VM's generation.
    let aot = BytecodeCompileService::for_profile(v85)
        .compile("set profile_aot 1")
        .expect("AOT module compiles");
    foreign_vm.set_dialect_profile(v84);
    assert_eq!(
        foreign_vm.run_module(&aot).result.to_str().as_ref(),
        "bytecode compiled for dialect profile tcl8.5 cannot run under tcl8.4"
    );
    let generic_registry = tcl_registry::CommandRegistry::build_default();
    let generic_ir =
        tcl_compiler::lowering::lower_to_ir_for_bytecode("lassign {a b} x", &generic_registry);
    let generic_cfg = build_cfg_codegen(&generic_ir, false);
    let generic_aot = codegen_module(&generic_cfg, &generic_ir, &generic_registry);
    assert_eq!(
        foreign_vm.run_module(&generic_aot).result.to_str().as_ref(),
        "bytecode compiled for dialect profile tcl cannot run under tcl8.4"
    );
    let traced_registry = tcl_registry::model::ingress::static_context_for_profile(v85).commands();
    let traced_config = tcl_lexer::LexerConfig::from_grammar(v85.grammar);
    let traced_ir = tcl_compiler::lowering::lower_to_ir_traced_with_dialect(
        "lassign {a b} x",
        traced_registry,
        traced_config,
        Some(v85),
    );
    let traced_cfg = build_cfg_codegen(&traced_ir, false);
    let traced_aot = codegen_module(&traced_cfg, &traced_ir, traced_registry);
    assert!(std::ptr::eq(traced_aot.profile, v85));
    assert_eq!(
        foreign_vm.run_module(&traced_aot).result.to_str().as_ref(),
        "bytecode compiled for dialect profile tcl8.5 cannot run under tcl8.4"
    );

    // The reported escape, exactly: a body compiled under 8.5 must not retain
    // its inline lassign lowering after a direct 8.4 flip, and the reverse
    // flip must compile it back onto the newer surface.
    vm.set_dialect_profile(v84);
    let exact_84 = vm
        .eval_source("catch {p} message; set message")
        .expect("8.4 exact repro compiles");
    assert_eq!(
        exact_84.result.to_str().as_ref(),
        "invalid command name \"lassign\""
    );
    assert_eq!(
        vm.eval_source("expr {0b10 + 1}")
            .expect("8.4 expression repro compiles")
            .code,
        Code::Error
    );
    assert!(
        vm.eval_source("{*}[list set ::profile_mutation_lexer 1]")
            .is_err()
    );
    vm.set_dialect_profile(v85);
    assert_eq!(
        vm.eval_source("list [p] [expr {0b10 + 1}]")
            .expect("8.5 reverse repro compiles")
            .result
            .to_str()
            .as_ref(),
        "a 3"
    );

    // `coroutine` begins in 8.6. Retain the 8.5-built proc/handle above, then
    // add a live continuation and child-owned profile while the newer surface
    // is selected.
    vm.set_dialect_profile(v86);
    let newer_setup = vm
        .eval_source(
            "coroutine c apply {{} {yield ready; lassign {a b} x; return $x}}\n\
             interp create child\n\
             child eval {proc p {} {lassign {a b} x; return $x}}\n\
             interp alias {} child_p child p",
        )
        .expect("8.6 coroutine setup compiles");
    assert!(newer_setup.code.is_ok(), "{}", newer_setup.result.to_str());
    // This source is lexically accepted on 8.5+ (its eventual runtime
    // command error is immaterial); retaining its compiled module exercises
    // the lexer-sensitive eval cache that 8.4 must discard and re-lex.
    let lexer_86 = vm
        .eval_source("{*}[list set ::profile_mutation_lexer 1]")
        .expect("8.5 lexer source compiles");
    assert_eq!(lexer_86.code, Code::Error);

    assert!(vm.set_child_dialect_profile("child", v84));
    vm.set_dialect_profile(v84);

    let stale = vm
        .eval_source(
            "catch {p} proc_msg; catch {o m} method_msg; catch {child_p} alias_msg; \\
             list $proc_msg $method_msg $alias_msg",
        )
        .expect("8.4 probes compile");
    assert_eq!(
        stale.result.to_str().as_ref(),
        "{invalid command name \"lassign\"} {invalid command name \"lassign\"} {invalid command name \"lassign\"}"
    );
    let handle_result = vm.invoke_function(&handle);
    assert_eq!(handle_result.code, Code::Error);
    assert_eq!(
        handle_result.result.to_str().as_ref(),
        "invalid command name \"lassign\""
    );

    // The exact same source must not be served from its newer-profile eval cache:
    // expression grammar and lexer grammar changed with the profile too.
    assert_eq!(
        vm.eval_source("expr {0b10 + 1}")
            .expect("8.4 expr compile")
            .code,
        Code::Error
    );
    assert!(
        vm.eval_source("{*}[list set ::profile_mutation_lexer 2]")
            .is_err()
    );

    // A coroutine cannot be recompiled in place after it yielded: it owns a
    // live PC/operand stack. The frame generation guard therefore rejects the
    // stale continuation before it reaches its old inline lassign opcode.
    let continuation = vm
        .eval_source("catch {c} continuation_msg; set continuation_msg")
        .expect("8.4 continuation probe compiles");
    assert_eq!(
        continuation.code,
        Code::Ok,
        "catch must absorb the stale frame"
    );
    assert_eq!(
        continuation.result.to_str().as_ref(),
        "cannot continue bytecode after dialect profile changed"
    );

    // The reverse mutation gets fresh 8.5 bodies (including the child and
    // alias) rather than retaining the failed 8.4 recompilation.
    assert!(vm.set_child_dialect_profile("child", v85));
    vm.set_dialect_profile(v85);
    let restored = vm
        .eval_source(
            "proc q {} {lassign {a b} x; return $x}; \\
             list [p] [o m] [child_p] [q] [expr {0b10 + 1}]",
        )
        .expect("8.5 restoration compiles");
    assert_eq!(restored.result.to_str().as_ref(), "a a a a 3");

    // And the live-coroutine surface returns normally once a newly-created
    // continuation is compiled under a release that supports it.
    vm.set_dialect_profile(v86);
    let resumed = vm
        .eval_source("coroutine c2 apply {{} {yield ready; lassign {a b} x; return $x}}; c2")
        .expect("8.6 coroutine restoration compiles");
    assert_eq!(resumed.result.to_str().as_ref(), "a");
}

#[test]
fn tcloo_method_profile_refresh_is_persisted_after_the_first_call() {
    let calls = Rc::new(Cell::new(0));
    let mut vm = Vm::new();
    // TclOO's class factory is 8.6+ (issue #1463); the object it makes is
    // script-created and survives the flip to 8.4 below.
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    vm.set_compiler(Box::new(CountingCompilerSvc {
        calls: Rc::clone(&calls),
    }));
    let setup = vm
        .eval_source("oo::class create C {method m {} {lassign {a b} x; return $x}}; C create o")
        .expect("class setup compiles");
    assert!(setup.code.is_ok(), "{}", setup.result.to_str());

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    let before = calls.get();
    let first = vm.invoke_command("o", &[tcl_vm::Value::string("m")]);
    let after_first = calls.get();
    let second = vm.invoke_command("o", &[tcl_vm::Value::string("m")]);
    assert_eq!(first.code, Code::Error);
    assert_eq!(second.code, Code::Error);
    assert_eq!(
        after_first,
        before + 1,
        "first call recompiles the method once"
    );
    assert_eq!(
        calls.get(),
        after_first,
        "second call reuses the refreshed method"
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
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
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
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
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

/// `interp expose`'s destination is always the exact global command-table
/// key.  A caller may have a same-spelled local command, which Tcl leaves
/// untouched while exposing the hidden binding globally.  Exercise the root,
/// named-child, and child-command (by-id) entry points so each routes through
/// the same collision owner rather than contextual command lookup.
#[test]
fn expose_uses_exact_global_destination_in_every_entry_point() {
    let src = concat!(
        "proc a {} {return root-global}; interp hide {} a held\n",
        "namespace eval n {proc b {} {return root-local}; interp expose {} held b; puts \"root=[b],[::b]\"}\n",
        "interp create named; named eval {proc a {} {return named-global}; interp hide {} a held; namespace eval n {proc b {} {return named-local}; interp expose {} held b; puts \"named=[b],[::b]\"}}\n",
        "interp create byid; byid eval {proc a {} {return byid-global}; interp hide {} a; namespace eval n {proc a {} {return byid-local}}}\n",
        "byid expose a; puts \"byid=[byid eval {namespace eval n {join [list [a] [::a]] ,}}]\"\n",
    );
    let want =
        "root=root-local,root-global\nnamed=named-local,named-global\nbyid=byid-local,byid-global";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] exact expose destination"
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
fn root_invokehidden_dispatches_without_exposing_the_command() {
    let src = concat!(
        "proc a {} {return A}\n",
        "interp hide {} a held\n",
        "puts \"hidden=[interp invokehidden {} held]\"\n",
        "puts \"visible=[llength [info commands held]] listed=[interp hidden {}]\"\n",
    );
    let want = "hidden=A\nvisible=0 listed=held";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] root invokehidden"
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
    let out = profile_output(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    assert!(
        !out.contains("invalid command name"),
        "plain tcl8.4 has source (it merely fails to read the file): {out}"
    );
}

const SELF_HIDING_COROUTINE: &str = r"
set first [coroutine c apply {{} {
    interp hide {} [info coroutine] held
    yield first
    interp expose {} held d
    yield second
    return done
}}]
puts [list $first [interp invokehidden {} held] [d] [interp hidden {}]]
";

// A deleted coroutine binding and a later command at the same spelling are
// distinct lifecycles.  The original driver is still unwinding after it
// deletes `c`; the replacement is then created at `c` and renamed to `d`.
// Its completion must not retire `d` through the stale active sidecar handle.
const SELF_DELETING_COROUTINE_REUSES_AND_RENAMES_BINDING: &str = r"
proc fresh {} {yield fresh-one; yield fresh-two; return fresh-done}
proc stale {} {
    rename [info coroutine] {}
    coroutine c fresh
    rename c d
    return stale-done
}
set start [coroutine c stale]
puts [list $start [d] [d] [catch {d} message] $message]
";

const SELF_HIDING_TRACED_COMMAND: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {interp hide {} p held; return ok}
trace add execution p {enter leave} tr
set result [p]
puts [list $result $log [interp hidden {}]]
";

const SELF_EXPOSING_TRACED_COMMAND: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {interp expose {} held q; return ok}
trace add execution p {enter leave} tr
interp hide {} p held
set result [interp invokehidden {} held]
puts [list $result $log [info commands q] [interp hidden {}]]
";

const SELF_RENAMING_TRACED_COMMAND: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {rename p q; return ok}
trace add execution p {enter leave} tr
set result [p]
puts [list $result $log [info commands q]]
";

const SELF_HIDING_TRACED_ERROR: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {interp hide {} p held; error boom}
trace add execution p {enter leave} tr
set code [catch {p} result]
puts [list $code $result $log [interp hidden {}]]
";

// A leave callback can delete the running binding and install a replacement.
// Tcl stops the cloned old trace list at that lifecycle boundary: later old
// callbacks must not run against the replacement generation.
const LEAVE_TRACE_DELETE_RECREATE_STOPS_OLD_LIST: &str = r"
set log {}
proc a {} {return old}
proc first args {lappend ::log A; rename a {}; proc a {} {return new}}
proc second args {lappend ::log B}
trace add execution a leave first
trace add execution a leave second
set old [a]
set new [a]
puts [list $old $log $new]
";

// Tcl runs enter callbacks newest-first, then leave callbacks oldest-first.
// Each successful leave result feeds the next callback, but never replaces
// the traced command's completion.
const EXECUTION_TRACE_ORDER_AND_LEAVE_RESULT_CHAIN: &str = r"
set log {}
proc p {} {return ORIG}
proc a args {lappend ::log [list A {*}$args]; return R1}
proc b args {lappend ::log [list B {*}$args]; return R2}
trace add execution p {enter leave} a
trace add execution p {enter leave} b
set result [p]
puts [list $result $log]
";

// Removing one pending registration skips only that entry; later entries in
// the captured Tcl trace list still fire.
const EXECUTION_TRACE_REMOVAL_CONTINUES_SNAPSHOT: &str = r"
set log {}
proc p {} {return ok}
proc a args {lappend ::log A}
proc b args {lappend ::log B}
proc c args {lappend ::log C; trace remove execution p enter b}
trace add execution p enter a
trace add execution p enter b
trace add execution p enter c
p
puts $log
";

#[test]
fn active_coroutine_and_execution_trace_follow_self_hide() {
    for profile in [
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    ] {
        assert_eq!(
            profile_output(SELF_HIDING_COROUTINE, profile),
            "first second done {}"
        );
        assert_eq!(
            profile_output(SELF_DELETING_COROUTINE_REUSES_AND_RENAMES_BINDING, profile),
            "stale-done fresh-two fresh-done 1 {invalid command name \"d\"}"
        );
        assert_eq!(
            profile_output(SELF_HIDING_TRACED_COMMAND, profile),
            "ok {{p enter} {p 0 ok leave}} held"
        );
        assert_eq!(
            profile_output(SELF_EXPOSING_TRACED_COMMAND, profile),
            "ok {{held enter} {held 0 ok leave}} q {}"
        );
        assert_eq!(
            profile_output(SELF_RENAMING_TRACED_COMMAND, profile),
            "ok {{p enter} {p 0 ok leave}} q"
        );
        assert_eq!(
            profile_output(SELF_HIDING_TRACED_ERROR, profile),
            "1 boom {{p enter} {p 1 boom leave}} held"
        );
        assert_eq!(
            profile_output(LEAVE_TRACE_DELETE_RECREATE_STOPS_OLD_LIST, profile),
            "old A new"
        );
        assert_eq!(
            profile_output(EXECUTION_TRACE_ORDER_AND_LEAVE_RESULT_CHAIN, profile),
            "ORIG {{B p enter} {A p enter} {A p 0 ORIG leave} {B p 0 R1 leave}}"
        );
        assert_eq!(
            profile_output(EXECUTION_TRACE_REMOVAL_CONTINUES_SNAPSHOT, profile),
            "C A"
        );
    }
}

#[test]
fn self_hide_vectors_match_real_tcl_when_available() {
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6", "tclsh"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        for (script, expected) in [
            (SELF_HIDING_COROUTINE, "first second done {}"),
            (
                SELF_DELETING_COROUTINE_REUSES_AND_RENAMES_BINDING,
                "stale-done fresh-two fresh-done 1 {invalid command name \"d\"}",
            ),
            (
                SELF_HIDING_TRACED_COMMAND,
                "ok {{p enter} {p 0 ok leave}} held",
            ),
            (
                SELF_EXPOSING_TRACED_COMMAND,
                "ok {{held enter} {held 0 ok leave}} q {}",
            ),
            (
                SELF_RENAMING_TRACED_COMMAND,
                "ok {{p enter} {p 0 ok leave}} q",
            ),
            (
                SELF_HIDING_TRACED_ERROR,
                "1 boom {{p enter} {p 1 boom leave}} held",
            ),
            (LEAVE_TRACE_DELETE_RECREATE_STOPS_OLD_LIST, "old A new"),
            (
                EXECUTION_TRACE_ORDER_AND_LEAVE_RESULT_CHAIN,
                "ORIG {{B p enter} {A p enter} {A p 0 ORIG leave} {B p 0 R1 leave}}",
            ),
            (EXECUTION_TRACE_REMOVAL_CONTINUES_SNAPSHOT, "C A"),
        ] {
            if let Some(actual) = tclsh_output(env, names, script) {
                assert_eq!(actual, expected, "{env}");
            }
        }
    }
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

/// An enter trace may delete the command currently being invoked while also
/// installing a replacement under its former visible name.  Tcl treats the
/// in-flight command token as deleted; it neither invokes nor resurrects it,
/// and the callback-created replacement remains authoritative.
#[test]
fn deleted_hidden_enter_trace_matches_real_tclsh() {
    let src = concat!(
        "proc p {} {return hidden}\n",
        "proc replace args {\n",
        "  rename p {}\n",
        "  interp expose {} held p\n",
        "  rename p {}\n",
        "  proc p {} {return replacement}\n",
        "}\n",
        "trace add execution p enter replace\n",
        "interp hide {} p held\n",
        "proc p {} {return initial}\n",
        "set rc [catch {interp invokehidden {} held} msg]\n",
        "puts [list $rc $msg [p] [interp hidden {}]]\n",
    );
    let want = "1 {attempt to invoke a deleted command} replacement {}";
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] hidden enter-trace lifecycle");
        }
    }
}

/// An imported proc owns its local binding even after its source has been
/// renamed. Refreshing trace-stale bytecode must update that exact resolved
/// import, never the `ProcDef` source name, which may now name a replacement.
#[test]
fn imported_proc_refresh_preserves_renamed_source_replacement() {
    let src = concat!(
        "namespace eval src {proc p {} {return original}; namespace export p}\n",
        "namespace eval dst {namespace import ::src::p}\n",
        "rename ::src::p ::src::q\n",
        "proc ::src::p {} {return replacement}\n",
        "proc noop args {}\n",
        "trace add execution ::dst::p enterstep noop\n",
        "puts [list [::dst::p] [::src::p] [::src::q] [namespace origin ::dst::p]]\n",
    );
    let want = "original replacement original ::src::q";
    assert_eq!(
        profile_output(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        ),
        want
    );
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] imported proc refresh");
        }
    }
}
