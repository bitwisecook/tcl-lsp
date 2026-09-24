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

//! A pack's codegen-axis stamp, from the pack file to the VM's admission —
//! `docs/design/compiler/registry-consumer-contracts.md` § *The loader's
//! stamp rejection rule* and rungs 1 and 2 of § *Four rungs of codegen
//! meeting `.tclspec`*.
//!
//! A bundled pack declares `vendor::unpack` as `alias_of lassign` and
//! carries `lassign`'s own `codegen_hook`. The load admits the stamp;
//! codegen specialises a call of `vendor::unpack` and records the
//! **target's** identity, `lassign`, at the site; and the VM admits the
//! module only where the pack name still resolves — through its alias hop —
//! to the `lassign` builtin and it holds the pack's facts, which the site
//! claims beside the binding. From the workspace tier the same stamp is
//! refused and the call compiles generic; a constant a pack's `const_fold`
//! computed claims the pack's facts too (rung 1). The VM's compile service
//! counts every plain-dispatch compile it is asked for, which is what a
//! refused site costs: an admitted module runs with none, so a refusal
//! cannot hide behind a correct result.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use tcl_compiler::codegen::ModuleAsm;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::DialectProfile;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::{
    CommandBindingIdentity, CompileError, PackFactStamp, ProcedureCompileTarget, ProcedureDispatch,
    ScriptCommandPlan, ScriptCompileTarget, SiteClaim,
};
use tcl_spectcl::PackSet;
use tcl_vm::{Code, CompileService, Vm};

/// The default compile service, counting the plain-dispatch compiles the VM
/// asks of it.
struct PlainCounting {
    inner: BytecodeCompileService,
    plain: Rc<Cell<usize>>,
}

impl PlainCounting {
    fn installed_on(vm: &mut Vm) -> Rc<Cell<usize>> {
        let plain = Rc::new(Cell::new(0));
        vm.set_compiler(Box::new(Self {
            inner: BytecodeCompileService::default(),
            plain: Rc::clone(&plain),
        }));
        plain
    }

    fn count(&self) {
        self.plain.set(self.plain.get() + 1);
    }
}

impl CompileService for PlainCounting {
    type Module = ModuleAsm;

    fn compile(&self, src: &str) -> Result<ModuleAsm, CompileError> {
        self.inner.compile(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner.compile_for_profile(src, profile)
    }

    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner.compile_script_for_profile(target, profile)
    }

    fn compile_traced(&self, src: &str) -> Result<ModuleAsm, CompileError> {
        self.count();
        self.inner.compile_traced(src)
    }

    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.count();
        self.inner.compile_traced_for_profile(src, profile)
    }

    fn compile_plain_dispatch_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.count();
        self.inner.compile_plain_dispatch_for_profile(src, profile)
    }

    fn compile_plain_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.count();
        self.inner.compile_plain_script_for_profile(target, profile)
    }

    fn script_command_plan_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> ScriptCommandPlan {
        self.inner.script_command_plan_for_profile(src, profile)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<ModuleAsm, CompileError> {
        if dispatch == ProcedureDispatch::Plain {
            self.count();
        }
        self.inner
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

/// `vendor::unpack LIST VAR…` — `lassign` by another name, with `lassign`'s
/// own codegen hook when `alias_of` names it.
fn unpack_pack(alias_of: &str) -> String {
    format!(
        "speclib vendor 2.0 {{\n    \
             command vendor::unpack {{\n        \
                 arity 1..\n        \
                 arg 0 -role Value\n        \
                 arg 1 -role VarWrite\n        \
                 arg 2 -role VarWrite\n        \
                 alias_of {alias_of}\n        \
                 codegen_hook -native Lassign\n    \
             }}\n\
         }}\n"
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-codegen-stamps-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// `llength` overridden by a bundled pack whose `const_fold` names the
/// shipped folder: a constant computed at compile time from the pack's facts.
/// The `dialects` row scopes the override to the release the test compiles
/// for: a profile's resolution prefers a scoped spec over a catch-all one,
/// so an unscoped override would lose to the shipped `llength`'s own scope.
const FOLD_PACK: &str = "speclib vendor 2.0 {\n    \
     command llength -override {\n        \
         dialects tcl9.0\n        \
         arity 1\n        \
         arg 0 -role Value\n        \
         const_fold -native llength::const_fold\n    \
     }\n\
 }\n";

/// The pack loaded as the bundled tier from a `specs/` directory, and the
/// registry it installs into.
fn bundled(name: &str, alias_of: &str) -> (PackSet, Arc<CommandRegistry>) {
    bundled_source(name, &unpack_pack(alias_of))
}

/// [`bundled`] for any pack source.
fn bundled_source(name: &str, source: &str) -> (PackSet, Arc<CommandRegistry>) {
    let specs = scratch(name).join("specs");
    std::fs::create_dir_all(&specs).expect("specs dir");
    std::fs::write(specs.join("vendor.tclspec"), source).expect("write pack");
    let set = tcl_spectcl::bundled::load_from(&specs);
    let registry = tcl_spectcl::bundled::registry_for_dialect_from("tcl9.0", &set);
    (set, registry)
}

/// The facts a VM holds for `set`, under this thread's evaluator revision —
/// the one the compiling thread's sites recorded.
fn facts(set: &PackSet) -> Vec<PackFactStamp> {
    set.fact_stamps(tcl_compiler::site_claims::evaluator_revision())
}

/// Compile `source` against `registry` — the pipeline a compile service runs,
/// over a registry that carries the pack.
fn compile(source: &str, registry: &CommandRegistry) -> ModuleAsm {
    let config = tcl_lexer::LexerConfig::default();
    let ir = tcl_compiler::lowering::lower_script_module_for_bytecode(
        source, "", registry, config, None, false,
    );
    let cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry_and_config(
        &ir, false, registry, config,
    );
    tcl_compiler::codegen::codegen_module(&cfg, &ir, registry)
}

/// Run a setup script through the default compile service on `vm`.
fn prepare(vm: &mut Vm, script: &str) {
    let module = BytecodeCompileService::default()
        .compile(script)
        .expect("setup compiles");
    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
}

const USE: &str = "set l {1 2 3}\nvendor::unpack $l a b\nlist $a $b\n";

/// The tier gate at the emitter: the same pack loaded from the workspace
/// tier keeps `alias_of` but loses the stamp, so the call compiles to
/// generic dispatch — no `lassign` identity at the site and no claim — and
/// the VM runs the module as compiled, the alias answering at run time.
#[test]
fn a_workspace_stamp_is_refused_and_specialises_nothing() {
    let set = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: scratch("workspace").join("vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
        },
        unpack_pack("lassign"),
    )]);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
    let unpack = registry.get("vendor::unpack").expect("installed");
    assert_eq!(unpack.alias_of, Some("lassign"));
    assert_eq!(unpack.codegen_hook, None, "the tier gate drops the stamp");

    let module = compile(USE, &registry);
    let bindings = &module.top_level.command_bindings;
    assert!(
        bindings.iter().all(|b| b.identity != "lassign"),
        "{bindings:#?}"
    );
    assert!(module.top_level.site_claims.is_empty());

    let mut vm = Vm::new();
    let plain = PlainCounting::installed_on(&mut vm);
    prepare(
        &mut vm,
        "namespace eval vendor {}\ninterp alias {} vendor::unpack {} lassign",
    );
    let before = plain.get();
    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "1 2");
    assert_eq!(plain.get(), before, "generic dispatch needs no recompile");
}

/// The site records `lassign`'s identity, never the pack command's own name:
/// the binding names the spelling the source wrote and the builtin the
/// stamp is the own of. Beside it the site claims the pack facts that made
/// the target admissible — exactly one of the facts a VM holds for the set.
#[test]
fn an_admitted_alias_stamp_records_the_targets_identity() {
    let (set, registry) = bundled("identity", "lassign");
    assert_eq!(
        registry
            .get("vendor::unpack")
            .expect("installed")
            .codegen_hook,
        Some(tcl_registry::hooks::CodegenHookId::Lassign),
        "the bundled stamp is admitted"
    );
    let module = compile(USE, &registry);
    let bindings = &module.top_level.command_bindings;
    let binding = CommandBindingIdentity::new("vendor::unpack", "lassign");
    assert!(bindings.contains(&binding), "{bindings:#?}");
    assert!(
        bindings.iter().all(|b| b.identity != "vendor::unpack"),
        "{bindings:#?}"
    );

    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert_eq!(held[0].pack, "vendor");
    assert_eq!(held[0].overlay_generation, set.key);
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::BuiltinAlias {
            binding,
            facts: held[0].clone(),
        }]
    );
}

/// The VM admits the module through the alias hop: `vendor::unpack` is an
/// alias of `lassign`, whose builtin identity is the one recorded, and the
/// VM holds the pack's facts, so the module runs with no plain recompile.
/// Before the alias exists the same module is refused — recompiled plain,
/// where the pack name is unknown.
#[test]
fn the_vm_admits_it_through_the_alias_hop() {
    let (set, registry) = bundled("admitted", "lassign");
    let module = compile(USE, &registry);

    let mut vm = Vm::new();
    let plain = PlainCounting::installed_on(&mut vm);
    vm.set_pack_facts(facts(&set));
    let refused = vm.run_module(&module);
    assert_eq!(refused.code, Code::Error, "{}", refused.result.to_str());
    assert!(
        plain.get() > 0,
        "nothing at the pack name: refused, recompiled plain"
    );

    // The namespace first: Tcl 8.4 to 9.1 create it for a qualified alias
    // themselves, and this VM does not yet, so without it the pack name
    // would not resolve at all and the test would prove nothing about the
    // hop.
    prepare(
        &mut vm,
        "namespace eval vendor {}\ninterp alias {} vendor::unpack {} lassign",
    );
    let before = plain.get();
    let admitted = vm.run_module(&module);
    assert_eq!(admitted.code, Code::Ok, "{}", admitted.result.to_str());
    assert_eq!(admitted.result.to_str().as_ref(), "1 2");
    assert_eq!(
        plain.get(),
        before,
        "admitted: the specialised module ran as compiled"
    );
}

/// Rung 1's check on a rung-2 site: the VM holds facts for a pack whose
/// content hash differs — the pack changed after the module was compiled —
/// so the site is refused and the module recompiled plain, the alias
/// answering through ordinary dispatch. The negative control: the same VM
/// holding the set's own facts admits it.
#[test]
fn a_changed_pack_invalidates_the_site() {
    let (set, registry) = bundled("changed", "lassign");
    let module = compile(USE, &registry);

    let mut vm = Vm::new();
    let plain = PlainCounting::installed_on(&mut vm);
    prepare(
        &mut vm,
        "namespace eval vendor {}\ninterp alias {} vendor::unpack {} lassign",
    );

    let changed: Vec<PackFactStamp> = facts(&set)
        .into_iter()
        .map(|stamp| PackFactStamp {
            content_hash: stamp.content_hash ^ 1,
            ..stamp
        })
        .collect();
    vm.set_pack_facts(changed);
    let before = plain.get();
    let invalidated = vm.run_module(&module);
    assert_eq!(
        invalidated.code,
        Code::Ok,
        "{}",
        invalidated.result.to_str()
    );
    assert_eq!(invalidated.result.to_str().as_ref(), "1 2");
    assert!(
        plain.get() > before,
        "a changed pack: refused, recompiled plain"
    );

    vm.set_pack_facts(facts(&set));
    let before = plain.get();
    let admitted = vm.run_module(&module);
    assert_eq!(admitted.code, Code::Ok, "{}", admitted.result.to_str());
    assert_eq!(plain.get(), before, "the set's own facts admit it");
}

/// Rung 1 on its own: a constant the pack's `const_fold` computed at compile
/// time claims the pack's facts beside the binding, which is the builtin's
/// own. A VM holding no facts refuses the unit though the binding matches —
/// recompiled plain, the builtin answering at run time — and the same VM
/// holding the set's facts runs the folded constant as compiled.
#[test]
fn a_pack_fold_is_admitted_only_under_its_pack_s_facts() {
    let (set, registry) = bundled_source("fold", FOLD_PACK);
    let module = compile("set n [llength {a b c}]\nset n\n", &registry);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert!(
        module
            .top_level
            .command_bindings
            .contains(&CommandBindingIdentity::new("llength", "llength")),
        "{:#?}",
        module.top_level.command_bindings
    );
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::PackFacts(held[0].clone())],
        "{:#?}",
        module.top_level
    );

    let mut vm = Vm::new();
    let plain = PlainCounting::installed_on(&mut vm);
    let refused = vm.run_module(&module);
    assert_eq!(refused.code, Code::Ok, "{}", refused.result.to_str());
    assert_eq!(refused.result.to_str().as_ref(), "3");
    assert!(plain.get() > 0, "no facts held: refused, recompiled plain");

    vm.set_pack_facts(held);
    let before = plain.get();
    let admitted = vm.run_module(&module);
    assert_eq!(admitted.code, Code::Ok, "{}", admitted.result.to_str());
    assert_eq!(admitted.result.to_str().as_ref(), "3");
    assert_eq!(plain.get(), before, "the set's facts admit the fold");
}

/// The negative: a proc at the pack name is not the builtin, so the VM
/// refuses the specialised site and recompiles the module plain — the proc
/// runs, where the specialised `lassign` code would have assigned `1 2` —
/// even though it holds the pack's facts.
#[test]
fn a_proc_at_the_pack_name_recompiles_plain() {
    let (set, registry) = bundled("proc", "lassign");
    let module = compile(USE, &registry);

    let mut vm = Vm::new();
    let plain = PlainCounting::installed_on(&mut vm);
    vm.set_pack_facts(facts(&set));
    prepare(
        &mut vm,
        "namespace eval vendor {}\n\
         proc vendor::unpack {l a b} {upvar 1 $a x $b y; set x P; set y Q}",
    );
    let before = plain.get();
    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "P Q");
    assert!(plain.get() > before, "refused: recompiled plain");
}
