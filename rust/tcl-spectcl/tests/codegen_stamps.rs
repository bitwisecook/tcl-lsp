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
//! computed claims the pack's facts too (rung 1). A command a pack declares as
//! a Tcl body has its definition inlined into the code that calls it and
//! records the claim beside the procedure binding that holds the live command
//! to it (rung 3). The VM's compile service counts every plain-dispatch
//! compile it is asked for, which is what a refused site costs: an admitted
//! module runs with none, so a refusal cannot hide behind a correct result.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use tcl_compiler::codegen::ModuleAsm;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::DialectProfile;
use tcl_dialect::model::DependencyTier;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::{
    BackingKind, CommandBindingIdentity, CompileError, PackFactStamp, ProcedureBindingIdentity,
    ProcedureCompileTarget, ProcedureDispatch, ScriptCommandPlan, ScriptCompileTarget, SiteClaim,
};
use tcl_spectcl::PackSet;
use tcl_vm::{Code, CompileService, Vm};

/// The default compile service, counting the plain-dispatch compiles the VM
/// asks of it.
struct PlainCounting {
    inner: BytecodeCompileService,
    plain: Rc<Cell<usize>>,
    /// What every reference-body claim of an optimised compile is rewritten to
    /// state, for a service that stands in for a compiler that made a claim
    /// the artefact contradicts.
    forged: Option<Forgery>,
}

/// A reference-body claim the compiler did not make.
#[derive(Debug, Clone, Copy)]
enum Forgery {
    /// A claim of a body for a command the pack does not back with one.
    Backing(BackingKind),
    /// A claim of a body no procedure binding of the function holds the live
    /// command to.
    Procedure,
}

impl PlainCounting {
    fn installed_on(vm: &mut Vm) -> Rc<Cell<usize>> {
        Self::installed_with(vm, BytecodeCompileService::default(), None)
    }

    /// `inner` as `vm`'s compile service.
    fn installed_with(
        vm: &mut Vm,
        inner: BytecodeCompileService,
        forged: Option<Forgery>,
    ) -> Rc<Cell<usize>> {
        let plain = Rc::new(Cell::new(0));
        vm.set_compiler(Box::new(Self {
            inner,
            plain: Rc::clone(&plain),
            forged,
        }));
        plain
    }

    fn count(&self) {
        self.plain.set(self.plain.get() + 1);
    }

    /// `module` with every reference-body claim restated as forged.
    fn forge(&self, mut module: ModuleAsm) -> ModuleAsm {
        let Some(forged) = self.forged else {
            return module;
        };
        let restate = |function: &mut tcl_compiler::codegen::FunctionAsm| {
            for claim in &mut function.site_claims {
                if let SiteClaim::ReferenceBody {
                    procedure, backing, ..
                } = claim
                {
                    match forged {
                        Forgery::Backing(kind) => *backing = kind,
                        Forgery::Procedure => procedure.body.push_str(" + 0"),
                    }
                }
            }
        };
        restate(&mut module.top_level);
        restate(&mut module.top_level_body);
        module.procedures.values_mut().for_each(restate);
        module
    }
}

impl CompileService for PlainCounting {
    type Module = ModuleAsm;

    fn compile(&self, src: &str) -> Result<ModuleAsm, CompileError> {
        self.inner.compile(src).map(|module| self.forge(module))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_for_profile(src, profile)
            .map(|module| self.forge(module))
    }

    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_script_for_profile(target, profile)
            .map(|module| self.forge(module))
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
            .map(|module| self.forge(module))
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
            dependency_tier: None,
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

/// The seam `tcl-engine-tclvm`'s `with_registry` compiles through: an
/// embedder's registry is an owned value handed to
/// `BytecodeCompileService::new`, and the service compiles for a profile
/// against `project_for_profile`'s view of it. A site resting on a pack
/// there claims the pack as it does on the registry the pack was
/// installed into, and stamps that registry's overlay generation, the number
/// a VM holding `PackSet::fact_stamps` compares. A projection that dropped
/// the pack origins would leave the site unclaimed, and one that dropped the
/// generation would stamp `0`, which no held fact matches. Both rungs: an
/// alias site and a pack's fold over an overridden builtin.
#[test]
fn a_site_compiled_through_the_service_stamps_the_bases_overlay_generation() {
    let profile = tcl_spectcl::environment::profile_for_dialect("tcl9.0");
    let through_the_service = |registry: &CommandRegistry, source: &str| {
        // The cache shares its registries; an owned value to hand the service
        // is the base's projection, which is what the service then projects
        // again for the profile it compiles.
        BytecodeCompileService::new(registry.project_for_profile(profile))
            .compile_for_profile(source, profile)
            .expect("compiles")
    };

    let (set, registry) = bundled("service-alias", "lassign");
    assert_ne!(set.key, 0);
    assert_eq!(registry.overlay_generation(), Some(set.key));
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert_eq!(held[0].overlay_generation, set.key);
    let module = through_the_service(&registry, USE);
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::BuiltinAlias {
            binding: CommandBindingIdentity::new("vendor::unpack", "lassign"),
            facts: held[0].clone(),
        }],
        "{:#?}",
        module.top_level
    );

    let (set, registry) = bundled_source("service-fold", FOLD_PACK);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert_eq!(held[0].overlay_generation, set.key);
    let module = through_the_service(&registry, "set n [llength {a b c}]\nset n\n");
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::PackFacts(held[0].clone())],
        "{:#?}",
        module.top_level
    );
}

/// The seam the workspace's overlay reaches the compiler through: a service
/// built for the profile and the key the packs were installed under compiles
/// against that very generation, so a site resting on a pack claims it and
/// stamps the same overlay generation the owned projection above does — both
/// rungs, an alias site and a pack's fold over an overridden builtin. A key
/// nothing installed builds no service, and a service does not compile once
/// its generation is not installed for the profile it is asked for: it does
/// not fall back to the registry the pack is missing from.
#[test]
fn a_service_for_the_packs_overlay_compiles_against_the_generation_they_installed() {
    let profile = tcl_spectcl::environment::profile_for_dialect("tcl9.0");
    let through_the_overlay = |set: &PackSet, source: &str| {
        BytecodeCompileService::for_profile_with_overlay(profile, set.key)
            .expect("the packs' overlay is installed")
            .compile_for_profile(source, profile)
            .expect("compiles")
    };

    let (set, _registry) = bundled("overlay-alias", "lassign");
    assert_ne!(set.key, 0);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    let module = through_the_overlay(&set, USE);
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::BuiltinAlias {
            binding: CommandBindingIdentity::new("vendor::unpack", "lassign"),
            facts: held[0].clone(),
        }],
        "{:#?}",
        module.top_level
    );

    let (set, _registry) = bundled_source("overlay-fold", FOLD_PACK);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    let module = through_the_overlay(&set, "set n [llength {a b c}]\nset n\n");
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::PackFacts(held[0].clone())],
        "{:#?}",
        module.top_level
    );

    // Nothing installed this key: no service, and the miss names it.
    let missing = 0x0BAD_0BAD;
    let miss = BytecodeCompileService::for_profile_with_overlay(profile, missing)
        .err()
        .expect("nothing installed the overlay");
    assert_eq!(miss.overlay, missing);
    assert_eq!(miss.environment, "tcl9.0");

    // The pack is installed for `tcl9.0` and not for `tcl8.6`, so the service
    // does not compile for 8.6 either, rather than compile it plain.
    let service = BytecodeCompileService::for_profile_with_overlay(profile, set.key)
        .expect("installed for 9.0");
    let older = tcl_spectcl::environment::profile_for_dialect("tcl8.6");
    let refused = service
        .compile_for_profile(USE, older)
        .expect_err("no generation for 8.6 under this overlay");
    assert!(
        refused.0.contains("tcl8.6") && refused.0.contains("not installed"),
        "{refused:?}"
    );
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

/// `vdouble N` — a command a pack says a Tcl body defines, its definition the
/// text of a `proc`.
fn reference_pack(backing: &str) -> String {
    format!(
        "speclib vendor 2.0 {{\n    \
             command vdouble {{\n        \
                 arity 1\n        \
                 arg 0 -role Value\n        \
                 runtime_backing {backing}\n    \
             }}\n\
         }}\n"
    )
}

/// The definition the pack carries, as the backing spells it.
const DOUBLE_IN_THE_PACK: &str = "tcl-body {-pack-text {proc vdouble {x} {expr {$x * 2}}}}";

/// A procedure that calls the command: the frame inlining needs, since a
/// script's global level has none of its own.
const CALLER: &str = "proc caller {n} {vdouble $n}";

/// What a site that inlined the pack's body records: the binding that holds
/// the live command to the definition.
fn double_binding() -> ProcedureBindingIdentity {
    ProcedureBindingIdentity::new("vdouble", "::vdouble", "x", "expr {$x * 2}")
}

fn tcl9() -> &'static DialectProfile {
    tcl_spectcl::environment::profile_for_dialect("tcl9.0")
}

/// A VM pinned to the profile a packs' overlay is installed for, which is the
/// one a service for that overlay compiles under.
fn vm_for_overlay() -> Vm {
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl9());
    vm
}

/// The service for the packs' overlay — what the editor's queries compile
/// through.
fn service_for(set: &PackSet) -> BytecodeCompileService {
    BytecodeCompileService::for_profile_with_overlay(tcl9(), set.key)
        .expect("the packs' overlay is installed")
}

fn calls_a_command(function: &tcl_compiler::codegen::FunctionAsm) -> bool {
    function.instructions.iter().any(|instruction| {
        matches!(
            instruction.op,
            tcl_compiler::codegen::Op::INVOKE_STK1 | tcl_compiler::codegen::Op::INVOKE_STK4
        )
    })
}

/// Define the live command as a procedure, and the caller, on `vm`.
fn define(vm: &mut Vm, double: &str) {
    for script in [double, CALLER] {
        let completion = vm.eval_source(script).expect("compiles");
        assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    }
}

fn call_caller(vm: &mut Vm, n: &str) -> String {
    let completion = vm.eval_source(&format!("caller {n}")).expect("compiles");
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    completion.result.to_str().to_string()
}

/// Rung 3: a procedure calling a command a pack declares as a Tcl body has
/// the pack's definition inlined, with the procedure binding that holds the
/// live command to it and the claim on the pack that declared the backing.
/// The VM holding that pack's facts, with a live procedure of exactly the
/// claimed text, runs the inlined function as compiled — no plain recompile.
#[test]
fn a_tcl_body_backed_command_is_inlined_and_admitted() {
    let (set, _registry) =
        bundled_source("reference-admitted", &reference_pack(DOUBLE_IN_THE_PACK));
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");

    let module = service_for(&set)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    let caller = &module.procedures["::caller"];
    assert_eq!(caller.procedure_bindings, vec![double_binding()]);
    assert_eq!(
        caller.site_claims,
        vec![SiteClaim::ReferenceBody {
            procedure: double_binding(),
            backing: BackingKind::TclBody,
            facts: held[0].clone(),
        }]
    );
    assert!(!calls_a_command(caller), "{:#?}", caller.instructions);
    assert!(
        caller
            .instructions
            .iter()
            .any(|instruction| instruction.source_cmd_text == "expr {$x * 2}"),
        "the inlined commands keep the text of the definition they came from: {:#?}",
        caller.instructions
    );
    assert_eq!(
        module.source, CALLER,
        "the artefact's source is the module's own"
    );
    assert_eq!(module.claimed_packs(), held);

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&set), None);
    vm.set_pack_facts(held.clone());
    define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert_eq!(
        plain.get(),
        0,
        "admitted: the inlined function ran as compiled"
    );

    // The same VM without the pack's facts refuses the claim: the call still
    // answers, through the live procedure, after a plain recompile.
    vm.set_pack_facts(Vec::new());
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert!(plain.get() > 0, "no facts held: refused, recompiled plain");
}

/// The negative: the same definition, from a command the pack backs with the
/// host, is neither inlined nor activated. Compiled against that pack nothing
/// is inlined, no binding is recorded and the call stays a call. And a unit
/// that claims the body for such a command — here the inlining pack's own
/// compile, restated as host-native — is refused though the live procedure's
/// text matches exactly, because an exact match says nothing about whether the
/// command *is* that procedure.
#[test]
fn a_host_native_backing_never_defines_a_proc() {
    let (native, _registry) = bundled_source("reference-native", &reference_pack("host-native"));
    let module = service_for(&native)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    let caller = &module.procedures["::caller"];
    assert!(caller.procedure_bindings.is_empty());
    assert!(caller.site_claims.is_empty());
    assert!(calls_a_command(caller), "{:#?}", caller.instructions);

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&native), None);
    vm.set_pack_facts(facts(&native));
    define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert_eq!(
        plain.get(),
        0,
        "generic dispatch to the live command needs no recompile"
    );

    // A compiler that claimed the body for that command anyway.
    let (inlining, _registry) =
        bundled_source("reference-forged", &reference_pack(DOUBLE_IN_THE_PACK));
    for (forged, refused) in [
        (None, false),
        (Some(Forgery::Backing(BackingKind::HostNative)), true),
        (Some(Forgery::Backing(BackingKind::None)), true),
        (Some(Forgery::Backing(BackingKind::ShippedBuiltin)), true),
        (Some(Forgery::Backing(BackingKind::TclBody)), false),
        // The backing is right and the claim names a procedure the function
        // does not hold the live command to.
        (Some(Forgery::Procedure), true),
    ] {
        let mut vm = vm_for_overlay();
        let plain = PlainCounting::installed_with(&mut vm, service_for(&inlining), forged);
        vm.set_pack_facts(facts(&inlining));
        define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
        assert_eq!(call_caller(&mut vm, "21"), "42", "{forged:?}");
        assert_eq!(plain.get() > 0, refused, "{forged:?}");
    }
}

/// A body the pack carries is text that goes stale without anyone touching the
/// pack: the library it models moves on. The load says so, and the site that
/// rests on it turns plain on the first mismatch — the live procedure answers,
/// where the inlined text would have said otherwise.
#[test]
fn a_pack_text_body_that_diverges_turns_the_site_plain() {
    let (set, _registry) =
        bundled_source("reference-diverged", &reference_pack(DOUBLE_IN_THE_PACK));
    assert!(
        set.notices
            .iter()
            .any(|notice| notice.message.contains("diverges from it silently")),
        "{:#?}",
        set.notices
    );

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&set), None);
    vm.set_pack_facts(facts(&set));
    define(&mut vm, "proc vdouble {x} {expr {$x * 2 + 1}}");
    assert_eq!(
        call_caller(&mut vm, "21"),
        "43",
        "the live library's answer, not the pack's"
    );
    assert!(
        plain.get() > 0,
        "the live text differs: refused, recompiled plain"
    );

    // The library catches up with the pack, and the site is admitted again.
    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&set), None);
    vm.set_pack_facts(facts(&set));
    define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert_eq!(plain.get(), 0);
}

/// A package on disk — a manifest, a pack in `specs/` whose backing points at
/// a file of the package, and that file — and its directory.
fn package_on_disk(name: &str, backing_path: &str, definition: Option<&str>) -> PathBuf {
    package_on_disk_saying(name, backing_path, definition, "")
}

/// The same, with the author asserting that the body may be evaluated.
fn package_on_disk_evaluable(name: &str, backing_path: &str, definition: Option<&str>) -> PathBuf {
    package_on_disk_saying(name, backing_path, definition, " -evaluate")
}

fn package_on_disk_saying(
    name: &str,
    backing_path: &str,
    definition: Option<&str>,
    flag: &str,
) -> PathBuf {
    let dir = scratch(name);
    std::fs::create_dir_all(dir.join("specs")).expect("specs dir");
    std::fs::create_dir_all(dir.join("lib")).expect("lib dir");
    std::fs::write(dir.join("tclpkg.tcl"), "package vendor 1.0\n").expect("manifest");
    let backing = format!("tcl-body {{-package-source {backing_path}{flag}}}");
    std::fs::write(dir.join("specs/vendor.tclspec"), reference_pack(&backing)).expect("pack");
    if let Some(definition) = definition {
        std::fs::write(dir.join("lib/double.tcl"), definition).expect("library file");
    }
    dir
}

fn load_package_pack(dir: &std::path::Path, tier: Option<DependencyTier>) -> PackSet {
    tcl_spectcl::pack::load(&[tcl_spectcl::PackFile {
        tier: tcl_spectcl::Tier::Workspace,
        path: dir.join("specs/vendor.tclspec"),
        origin: tcl_spectcl::discovery::Origin::DotDir,
        dependency_tier: tier,
    }])
}

fn warnings(set: &PackSet) -> Vec<String> {
    set.notices
        .iter()
        .filter(|notice| notice.severity == tcl_spectcl::pack::Severity::Warning)
        .map(|notice| notice.message.clone())
        .collect()
}

/// A body a pack names as a file of its package is read at load, through the
/// store that read the pack, and reaches the compiler as text: the command
/// carries it, the registry holds it, a compile inlines it, and nothing that
/// compiles reads a file. What was read moves the set's key, so a registry
/// built before the file changed is not the one built after.
#[test]
fn a_package_source_body_is_read_at_load_through_the_store() {
    let definition = "proc vdouble {x} {expr {$x * 2}}";
    let dir = package_on_disk("package-source", "lib/double.tcl", Some(definition));
    let set = load_package_pack(&dir, Some(DependencyTier::Root));
    let command = set.packs[0].command("vdouble").expect("declared");
    assert_eq!(command.reference_text.as_deref(), Some(definition));
    assert!(warnings(&set).is_empty(), "{:#?}", set.notices);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
    let spec = registry.get("vdouble").expect("installed");
    assert_eq!(registry.reference_body(spec), Some(definition));

    let module = service_for(&set)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    let caller = &module.procedures["::caller"];
    assert_eq!(caller.procedure_bindings, vec![double_binding()]);
    assert!(!calls_a_command(caller));

    // The same files give the same key; another library gives another, and
    // the registry for it inlines the library as it now is.
    assert_eq!(
        load_package_pack(&dir, Some(DependencyTier::Root)).key,
        set.key
    );
    std::fs::write(
        dir.join("lib/double.tcl"),
        "proc vdouble {x} {expr {$x * 4}}",
    )
    .expect("library file");
    let moved = load_package_pack(&dir, Some(DependencyTier::Root));
    assert_ne!(moved.key, set.key);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &moved);
    let module = service_for(&moved)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    assert_eq!(
        module.procedures["::caller"].procedure_bindings,
        vec![ProcedureBindingIdentity::new(
            "vdouble",
            "::vdouble",
            "x",
            "expr {$x * 4}"
        )]
    );
}

/// A file that cannot be read, a path that leaves the package and a pack no
/// package ships are each a warning on the command's row; the command keeps
/// its declaration and is not inlined. A load with no store reads nothing and
/// says nothing.
#[test]
fn a_package_source_that_cannot_be_read_is_said_and_not_inlined() {
    let definition = "proc vdouble {x} {expr {$x * 2}}";
    for (name, path, written, expected) in [
        ("package-missing", "lib/double.tcl", None, "No such file"),
        (
            "package-escaping",
            "../double.tcl",
            Some(definition),
            "relative path inside the package",
        ),
        (
            "package-absolute",
            "/etc/hostname",
            Some(definition),
            "relative path inside the package",
        ),
    ] {
        let dir = package_on_disk(name, path, written);
        let set = load_package_pack(&dir, Some(DependencyTier::Root));
        let command = set.packs[0].command("vdouble").expect("declared");
        assert_eq!(command.reference_text, None, "{name}");
        assert!(
            matches!(
                command.spec.runtime_backing,
                tcl_registry::RuntimeBacking::TclBody { .. }
            ),
            "{name}: the declaration stands"
        );
        let said = warnings(&set);
        assert_eq!(said.len(), 1, "{name}: {:#?}", set.notices);
        assert!(said[0].contains(expected), "{name}: {}", said[0]);
        assert!(
            said[0].contains("a call to `vdouble` is not inlined"),
            "{name}: {}",
            said[0]
        );
        let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
        let module = service_for(&set)
            .compile_for_profile(CALLER, tcl9())
            .expect("compiles");
        assert!(
            module.procedures["::caller"].procedure_bindings.is_empty(),
            "{name}"
        );
    }

    // A pack no package ships has no directory to read from.
    let dir = package_on_disk("package-unshipped", "lib/double.tcl", Some(definition));
    std::fs::remove_file(dir.join("tclpkg.tcl")).expect("remove the manifest");
    let unshipped = load_package_pack(&dir, None);
    let said = warnings(&unshipped);
    assert_eq!(said.len(), 1, "{said:#?}");
    assert!(said[0].contains("no `tclpkg.tcl` above the pack ships it"));

    // A load with no store reads nothing and says nothing.
    let in_memory = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: dir.join("specs/vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        reference_pack("tcl-body {-package-source lib/double.tcl}"),
    )]);
    assert!(warnings(&in_memory).is_empty(), "{:#?}", in_memory.notices);
    let command = in_memory.packs[0].command("vdouble").expect("declared");
    assert_eq!(command.reference_text, None);
    // A load that read nothing keeps the key its sources gave it, whether it had
    // a store or not.
    assert_eq!(unshipped.key, in_memory.key);
}

/// The text a package source names is run as the command's declared
/// implementation too, where its author asserted the body may be evaluated: the
/// load reads the file and derives the hook from it as it does from a pack-text
/// body, and a file the load could not read, or a direct dependency's body, which
/// the gate has dropped, supplies none to derive from. The same file without the
/// author's word derives nothing.
#[test]
fn a_package_source_body_is_a_declared_implementation_too() {
    use tcl_registry::value_transfer::SemanticsDeclaration;

    let definition = "proc vdouble {x} {expr {$x * 2}}";
    let dir =
        package_on_disk_evaluable("package-implementation", "lib/double.tcl", Some(definition));
    let own = load_package_pack(&dir, Some(DependencyTier::Root));
    let command = own.packs[0].command("vdouble").expect("declared");
    assert!(
        matches!(command.spec.semantics, SemanticsDeclaration::Declared(_)),
        "{:?}",
        command.spec.semantics
    );
    assert_eq!(command.hooks.len(), 1, "{:?}", command.hooks);

    let silent = package_on_disk(
        "package-implementation-unasserted",
        "lib/double.tcl",
        Some(definition),
    );
    let set = load_package_pack(&silent, Some(DependencyTier::Root));
    let command = set.packs[0].command("vdouble").expect("declared");
    assert_eq!(command.reference_text.as_deref(), Some(definition));
    assert!(matches!(
        command.spec.semantics,
        SemanticsDeclaration::Inherited
    ));
    assert!(command.hooks.is_empty());
    assert!(warnings(&set).is_empty(), "{:#?}", set.notices);

    let unread = package_on_disk_evaluable("package-implementation-unread", "lib/double.tcl", None);
    let set = load_package_pack(&unread, Some(DependencyTier::Root));
    let command = set.packs[0].command("vdouble").expect("declared");
    assert!(matches!(
        command.spec.semantics,
        SemanticsDeclaration::Inherited
    ));
    assert!(command.hooks.is_empty());

    let dependency = load_package_pack(&dir, Some(DependencyTier::Direct));
    let command = dependency.packs[0].command("vdouble").expect("declared");
    assert!(matches!(
        command.spec.semantics,
        SemanticsDeclaration::Inherited
    ));
    assert!(command.hooks.is_empty());
}

/// The capability matrix's reference-body row at the load: the workspace's own
/// package's body is inlined; a direct dependency's pack loses the backing, with
/// a warning that says why, and nothing of it is inlined; a pack no package
/// ships is not narrowed.
#[test]
fn a_direct_dependencys_reference_body_is_dropped_at_load() {
    let compiled = |tier: Option<DependencyTier>, name: &str| {
        let set = tcl_spectcl::pack::load_in_memory(vec![(
            tcl_spectcl::PackFile {
                tier: tcl_spectcl::Tier::Workspace,
                path: scratch(name).join("vendor.tclspec"),
                origin: tcl_spectcl::discovery::Origin::DotDir,
                dependency_tier: tier,
            },
            reference_pack(DOUBLE_IN_THE_PACK),
        )]);
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
        let spec = registry.get("vdouble").expect("installed");
        let module = service_for(&set)
            .compile_for_profile(CALLER, tcl9())
            .expect("compiles");
        let inlined = !module.procedures["::caller"].procedure_bindings.is_empty();
        let refusal = warnings(&set)
            .into_iter()
            .find(|message| message.contains("may not declare a reference body"));
        (spec.runtime_backing, inlined, refusal)
    };

    let (backing, inlined, refusal) = compiled(Some(DependencyTier::Root), "reference-root");
    assert!(matches!(
        backing,
        tcl_registry::RuntimeBacking::TclBody { .. }
    ));
    assert!(inlined);
    assert_eq!(refusal, None);

    let (backing, inlined, refusal) = compiled(Some(DependencyTier::Direct), "reference-direct");
    assert_eq!(backing, tcl_registry::RuntimeBacking::None);
    assert!(!inlined);
    assert!(
        refusal
            .expect("said on the pack file")
            .contains("a direct dependency's pack")
    );

    let (_, inlined, refusal) = compiled(None, "reference-unshipped");
    assert!(inlined);
    assert_eq!(refusal, None);
}

// The splice, run.
//
// The tests above prove the claim a site makes. These prove the code the claim
// admits: each runs a caller the compile service inlined a pack's bodies into, in a
// VM that holds the pack's facts and the live definitions, and holds it to what the
// definitions answer when called as procedures.

/// A pack whose commands each say a Tcl body defines them: `(name, arity,
/// definition)`.
fn bodies_pack(commands: &[(&str, usize, &str)]) -> String {
    use std::fmt::Write as _;

    let mut pack = String::from("speclib vendor 2.0 {\n");
    for (name, arity, definition) in commands {
        let _ = write!(pack, "    command {name} {{\n        arity {arity}\n");
        for index in 0..*arity {
            let _ = writeln!(pack, "        arg {index} -role Value");
        }
        let _ = write!(
            pack,
            "        runtime_backing tcl-body {{-pack-text {{{definition}}}}}\n    }}\n"
        );
    }
    pack.push_str("}\n");
    pack
}

/// What a script answered, and how many plain recompiles the VM asked for: none
/// when the inlined function ran as compiled.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Code,
    answer: String,
    plain: usize,
}

/// A pack of procedure-backed commands, loaded.
struct Spliced {
    set: PackSet,
    /// What runs before the definitions: the namespaces they are made in.
    prelude: Vec<String>,
    definitions: Vec<String>,
}

impl Spliced {
    fn new(name: &str, commands: &[(&str, usize, &str)]) -> Self {
        let (set, _registry) = bundled_source(name, &bodies_pack(commands));
        Self {
            set,
            prelude: Vec::new(),
            definitions: commands
                .iter()
                .map(|(_, _, definition)| (*definition).to_owned())
                .collect(),
        }
    }

    /// The same pack with `namespace` made before its definitions are.
    fn in_namespace(mut self, namespace: &str) -> Self {
        self.prelude
            .push(format!("namespace eval {namespace} {{}}"));
        self
    }

    /// `script` after `setup`, run by a VM whose compile service inlines the pack's
    /// bodies and which holds the pack's facts.
    fn inlined(&self, setup: &[&str], script: &str) -> Outcome {
        let mut vm = vm_for_overlay();
        let plain = PlainCounting::installed_with(&mut vm, service_for(&self.set), None);
        vm.set_pack_facts(facts(&self.set));
        self.drive(&mut vm, &plain, setup, script)
    }

    /// The same run with nothing inlined: what the call means.
    fn reference(&self, setup: &[&str], script: &str) -> Outcome {
        let mut vm = vm_for_overlay();
        let plain = PlainCounting::installed_on(&mut vm);
        self.drive(&mut vm, &plain, setup, script)
    }

    fn drive(&self, vm: &mut Vm, plain: &Rc<Cell<usize>>, setup: &[&str], script: &str) -> Outcome {
        for source in self
            .prelude
            .iter()
            .chain(&self.definitions)
            .map(String::as_str)
            .chain(setup.iter().copied())
        {
            let completion = vm.eval_source(source).expect("compiles");
            assert_eq!(
                completion.code,
                Code::Ok,
                "{source}: {}",
                completion.result.to_str()
            );
        }
        let completion = vm.eval_source(script).expect("compiles");
        Outcome {
            code: completion.code,
            answer: completion.result.to_str().to_string(),
            plain: plain.get(),
        }
    }

    /// How many pack commands `source` compiled with a definition spliced in:
    /// the procedure bindings of the procedure `name`.
    fn bindings(&self, source: &str, name: &str) -> usize {
        service_for(&self.set)
            .compile_for_profile(source, tcl9())
            .expect("compiles")
            .procedures[name]
            .procedure_bindings
            .len()
    }
}

/// A body with a `return` before its end, spliced where its value is the
/// procedure's own, answers the value it returned.
#[test]
fn a_body_with_a_return_before_its_end_answers_through_the_splice() {
    let pack = Spliced::new(
        "splice-early-return",
        &[(
            "vabs",
            1,
            "proc vabs {x} {if {$x < 0} {return [expr {-$x}]}; return $x}",
        )],
    );
    let caller = "proc caller {n} {vabs $n}";
    let branch = "proc caller3 {n} {if {$n != 0} {vabs $n} else {return zero}}";
    assert_eq!(pack.bindings(caller, "::caller"), 1, "inlined");
    assert_eq!(pack.bindings(branch, "::caller3"), 1, "inlined");
    for (script, answer) in [
        ("caller -5", "5"),
        ("caller 7", "7"),
        ("caller3 -5", "5"),
        ("caller3 0", "zero"),
    ] {
        assert_eq!(
            pack.inlined(&[caller, branch], script),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{script}"
        );
    }
}

/// A body that ends in a `return`, spliced into a static `eval` that is the
/// procedure's last command, still hands its value out: the block's value is its
/// last command's, and the procedure's is the block's.
#[test]
fn an_inlined_body_inside_a_static_eval_keeps_its_value() {
    let pack = Spliced::new(
        "splice-eval",
        &[("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}")],
    );
    let caller = "proc caller {n} {eval {vdouble $n}}";
    let after = "proc after {n} {eval {vdouble $n}; return tail}";
    assert_eq!(pack.bindings(caller, "::caller"), 1, "inlined");
    for (script, answer) in [("caller 21", "42"), ("after 21", "tail")] {
        assert_eq!(
            pack.inlined(&[caller, after], script),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{script}"
        );
    }
}

/// An unqualified command in an inlined body is the one the definition's own
/// namespace resolves it to, not whatever the caller's namespace makes of the
/// name: a caller in `app` that defines its own `string` does not turn the pack's
/// `string length` into that.
#[test]
fn an_inlined_body_resolves_the_commands_it_calls_as_its_definition_did() {
    let pack = Spliced::new(
        "splice-namespace",
        &[("vlen", 1, "proc vlen {s} {string length $s}")],
    );
    let setup = [
        "namespace eval app {}",
        "proc app::string {args} {return shadowed}",
        "proc app::caller {n} {vlen $n}",
    ];
    assert_eq!(
        pack.inlined(&setup, "app::caller abc"),
        Outcome {
            code: Code::Ok,
            answer: "3".to_owned(),
            plain: 0
        }
    );
    // The shadow is made after the caller was compiled, which no compile can see.
    let late = [
        "namespace eval app {}",
        "proc app::caller {n} {vlen $n}",
        "app::caller abc",
        "proc app::string {args} {return shadowed}",
    ];
    let outcome = pack.inlined(&late, "app::caller abc");
    assert_eq!(outcome.answer, "3", "{outcome:?}");
}

/// A command a word substitutes is text to the compiler, which cannot spell it
/// from the global namespace as it does a call: a body that runs one stays a call
/// in another namespace and answers what its definition does, and is spliced
/// where nothing needs spelling.
#[test]
fn a_body_that_substitutes_a_command_is_not_given_the_callers_namespace_to_resolve_it_in() {
    let pack = Spliced::new(
        "splice-namespace-substitution",
        &[
            ("vjoin", 1, "proc vjoin {l} {return [join $l ,]}"),
            (
                "vquoted",
                1,
                "proc vquoted {s} {set n \"[string length $s]x\"; return $n}",
            ),
            (
                "vformat",
                1,
                "proc vformat {x} {return [format %d-%s $x $x]}",
            ),
            ("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}"),
        ],
    );
    for (name, shadow, argument, answer) in [
        ("vjoin", "join", "{a b}", "a,b"),
        ("vquoted", "string", "abc", "3x"),
        ("vformat", "format", "5", "5-5"),
        // The `expr` it substitutes is the statement's own, bound to the live command.
        ("vdouble", "string", "4", "8"),
    ] {
        let setup = [
            "namespace eval app {}".to_owned(),
            format!("proc app::{shadow} {{args}} {{return shadowed}}"),
            format!("proc app::caller {{n}} {{{name} $n}}"),
        ];
        let setup: Vec<&str> = setup.iter().map(String::as_str).collect();
        assert_eq!(
            pack.inlined(&setup, &format!("app::caller {argument}")),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{name}"
        );
        let global = format!("proc caller {{n}} {{{name} $n}}");
        let namespaced = format!("namespace eval app {{}}\nproc app::caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&global, "::caller"), 1, "{name}: global");
        assert_eq!(
            pack.bindings(&namespaced, "::app::caller"),
            usize::from(name == "vdouble"),
            "{name}: another namespace"
        );
    }
}

/// A body reads what its definition binds, and nothing of the caller's: a name it
/// never sets is unset in the definition's frame, and a name it sets only on one
/// path is unset on the other, whatever the caller holds of the same spelling.
#[test]
fn an_inlined_body_reads_only_what_its_definition_binds() {
    let pack = Spliced::new(
        "splice-frame",
        &[
            ("vgreet", 0, "proc vgreet {} {return \"hello $name\"}"),
            (
                "vpick",
                1,
                "proc vpick {flag} {if {$flag} {set v 1}; return $v}",
            ),
        ],
    );
    let greet = "proc greet {n} {set name bob; vgreet}";
    let choose = "proc choose {n} {vpick $n}";
    for (script, code, answer) in [
        (
            "greet 1",
            Code::Error,
            "can't read \"name\": no such variable",
        ),
        (
            "choose 0",
            Code::Error,
            "can't read \"v\": no such variable",
        ),
        ("choose 1", Code::Ok, "1"),
    ] {
        assert_eq!(
            pack.inlined(&[greet, choose], script),
            Outcome {
                code,
                answer: answer.to_owned(),
                plain: 0
            },
            "{script}"
        );
    }
}

/// A braced word is literal: `$x` in one is the two characters, and the splice
/// leaves it as the definition has it, in a call's word, a return's value and the
/// text of a command a word substitutes.
#[test]
fn a_braced_word_in_an_inlined_body_stays_literal() {
    let pack = Spliced::new(
        "splice-braced",
        &[
            ("vlit", 1, "proc vlit {x} {string length {$x}}"),
            ("vbrace", 1, "proc vbrace {x} {return {$x}}"),
            (
                "vnested",
                1,
                "proc vnested {x} {return [string length {$x}]}",
            ),
        ],
    );
    for (name, answer, inlined) in [("vlit", "2", 1), ("vbrace", "$x", 1), ("vnested", "2", 0)] {
        let caller = format!("proc caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&caller, "::caller"), inlined, "{name}");
        assert_eq!(
            pack.inlined(&[&caller], "caller abc"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{name}"
        );
    }
}

/// A body that reads a parameter through an operand the compiler keeps as text — a
/// command an expression substitutes, a quoted operand, a loop condition that is a
/// command — is not rewritten for the caller's frame, so it stays a call and
/// answers what its definition does.
#[test]
fn a_body_that_reads_through_an_operand_kept_as_text_stays_a_call() {
    let pack = Spliced::new(
        "splice-text-operands",
        &[
            (
                "vcommand",
                1,
                "proc vcommand {x} {expr {[string length $x] + 1}}",
            ),
            ("vquote", 1, "proc vquote {x} {expr {\"$x\" eq \"abc\"}}"),
            (
                "vwhile",
                1,
                "proc vwhile {x} {set i 0; while {[expr {$i < $x}]} {incr i}; return $i}",
            ),
        ],
    );
    for (name, argument, answer) in [
        ("vcommand", "abc", "4"),
        ("vquote", "abc", "1"),
        ("vwhile", "4", "4"),
    ] {
        let caller = format!("proc caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&caller, "::caller"), 0, "{name}");
        assert_eq!(
            pack.inlined(&[&caller], &format!("caller {argument}")),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{name}"
        );
    }
}

/// A function claims the procedure it inlined and no other: where codegen emits a
/// body from the source text of the command that carries it — a `catch`, a `try`,
/// an `uplevel` — a splice made in the lowered copy is never emitted, so the
/// function keeps its call and records no binding for it.
#[test]
fn a_function_claims_a_splice_only_where_it_emits_it() {
    let pack = Spliced::new(
        "splice-claims",
        &[
            ("vdouble", 1, "proc vdouble {x} {expr {$x * 2}}"),
            (
                "vearly",
                1,
                "proc vearly {x} {if {$x < 0} {return neg}; expr {$x * 2}}",
            ),
            (
                "vlocal",
                1,
                "proc vlocal {x} {set y [expr {$x + 1}]; return $y}",
            ),
        ],
    );
    let mut emitted = 0;
    for call in ["vdouble $n", "vearly $n", "vlocal $n"] {
        for (name, template) in [
            ("plain", "proc caller {n} {@CALL@}"),
            ("catch", "proc caller {n} {catch {@CALL@} r; return $r}"),
            (
                "catch twice",
                "proc caller {n} {catch {@CALL@; @CALL@} r; return $r}",
            ),
            (
                "try",
                "proc caller {n} {try {@CALL@} on error {e} {return err}}",
            ),
            (
                "try handler",
                "proc caller {n} {try {error x} on error {e} {@CALL@}}",
            ),
            (
                "try finally",
                "proc caller {n} {try {set k 1} finally {@CALL@}}",
            ),
            ("uplevel", "proc caller {n} {uplevel 0 {@CALL@}}"),
            (
                "catch in a loop",
                "proc caller {n} {foreach i {1 2} {catch {@CALL@}}; return done}",
            ),
        ] {
            let source = template.replace("@CALL@", call);
            let module = service_for(&pack.set)
                .compile_for_profile(&source, tcl9())
                .expect("compiles");
            let caller = &module.procedures["::caller"];
            assert_eq!(
                !caller.procedure_bindings.is_empty(),
                !calls_a_command(caller),
                "{call} in {name}: bindings {:?}, instructions {:#?}",
                caller.procedure_bindings,
                caller.instructions
            );
            assert_eq!(
                caller.site_claims.is_empty(),
                caller.procedure_bindings.is_empty(),
                "{call} in {name}: a binding is claimed with the pack's facts, and only then"
            );
            emitted += caller.procedure_bindings.len();
        }
    }
    assert!(emitted > 0, "nothing was spliced, so nothing was compared");
}

/// A body whose value somebody reads is not given a wrap: each iteration of an
/// `lmap` gives its body's value to the list it builds, so a body with a `return`
/// stays a call there, a body that is a branch is not one the collector gathers
/// from, and one with neither is spliced as the last command's value.
#[test]
fn an_inlined_body_in_an_lmap_body_gives_the_list_each_iterations_value() {
    let pack = Spliced::new(
        "splice-lmap",
        &[
            ("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}"),
            ("vtail", 1, "proc vtail {x} {expr {$x * 2}}"),
            (
                "vif",
                1,
                "proc vif {x} {if {$x < 0} {set r neg} else {set r pos}}",
            ),
            ("vnoop", 0, "proc vnoop {} {}"),
        ],
    );
    for (label, source, answer, bindings) in [
        (
            "a body with a return",
            "proc caller {n} {lmap i {1 2 3} {vdouble $n}}",
            "14 14 14",
            0,
        ),
        (
            "a body with none",
            "proc caller {n} {lmap i {1 2 3} {vtail $n}}",
            "14 14 14",
            1,
        ),
        (
            "a branch",
            "proc caller {n} {lmap i {1 2} {vif $n}}",
            "pos pos",
            0,
        ),
        (
            "an empty body alone",
            "proc caller {n} {lmap i {1 2} {vnoop}}",
            "{} {}",
            1,
        ),
        (
            "an empty body after a command",
            "proc caller {n} {lmap i {1 2} {set a 5; vnoop}}",
            "{} {}",
            0,
        ),
        (
            "a dict map",
            "proc caller {n} {dict map {k v} {a 1 b 2} {vdouble $n}}",
            "a 14 b 14",
            0,
        ),
    ] {
        assert_eq!(pack.bindings(source, "::caller"), bindings, "{label}");
        assert_eq!(
            pack.inlined(&[source], "caller 7"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{label}"
        );
    }
}

/// A braced word is literal where the wrap stores a value as well: `{a\tb}` and
/// `{[string length $x]}` are the characters they are, whether the value is the
/// last command's or an early `return`'s. The answers are Tcl's, written down: the
/// VM's own compile of `return {[…]}` evaluates the brackets, so the matrix, which
/// holds a splice to the definition run as a procedure, has no reference for it.
#[test]
fn a_braced_literal_stays_literal_through_the_wrap() {
    let pack = Spliced::new(
        "splice-braced-wrap",
        &[
            (
                "vsetbs",
                1,
                "proc vsetbs {x} {if {$x < 0} {return neg}; set r {a\\tb}}",
            ),
            (
                "vretbs",
                1,
                "proc vretbs {x} {if {$x > 0} {return {a\\tb}}; return neg}",
            ),
            (
                "vsetbr",
                1,
                "proc vsetbr {x} {if {$x < 0} {return neg}; set r {[string length $x]}}",
            ),
            (
                "vretbr",
                1,
                "proc vretbr {x} {if {$x > 0} {return {[string length $x]}}; return neg}",
            ),
        ],
    );
    for (name, answer) in [
        ("vsetbs", r"a\tb"),
        ("vretbs", r"a\tb"),
        ("vsetbr", "[string length $x]"),
        ("vretbr", "[string length $x]"),
    ] {
        let caller = format!("proc caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&caller, "::caller"), 1, "{name}");
        assert_eq!(
            pack.inlined(&[&caller], "caller 7"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{name}"
        );
    }
}

/// A definition in a namespace of its own resolves each command it names there
/// first and in the global namespace after it, so a `vendor::string` the package
/// defines answers for `vendor::vlen`'s `string`, made before the caller was
/// compiled or after it ran. No spelling says that to a caller elsewhere, so the
/// body is spliced only where the caller looks names up as the definition did.
#[test]
fn a_definition_in_a_namespace_resolves_its_commands_as_it_did() {
    let pack = Spliced::new(
        "splice-definition-namespace",
        &[
            (
                "vendor::vlen",
                1,
                "proc vendor::vlen {s} {string length $s}",
            ),
            (
                "vendor::vinc",
                1,
                "proc vendor::vinc {x} {return [expr {$x + 1}]}",
            ),
        ],
    )
    .in_namespace("vendor");
    let shadow = "proc vendor::string {args} {return shadowed}";
    for (source, name, call) in [
        (
            "proc caller {n} {vendor::vlen $n}",
            "::caller",
            "caller abc",
        ),
        (
            "namespace eval app {}\nproc app::caller {n} {vendor::vlen $n}",
            "::app::caller",
            "app::caller abc",
        ),
    ] {
        assert_eq!(pack.bindings(source, name), 0, "{call}");
        for (setup, answer) in [
            (vec![source], "3"),
            (vec![shadow, source], "shadowed"),
            (vec![source, call, shadow], "shadowed"),
        ] {
            assert_eq!(
                pack.inlined(&setup, call),
                Outcome {
                    code: Code::Ok,
                    answer: answer.to_owned(),
                    plain: 0
                },
                "{call} with {setup:?}"
            );
        }
    }
    // A body that names no command by a word the compiler keeps is spliced anywhere,
    // and what lowering consumed of it is held to the live command in the
    // definition's namespace: a `vendor::expr` made later is the one that answers.
    let increment = "proc caller {n} {vendor::vinc $n}";
    assert_eq!(pack.bindings(increment, "::caller"), 1, "names no command");
    let expr_shadow = "proc vendor::expr {args} {return shadowed}";
    for (setup, answer) in [
        (vec![increment], "8"),
        (vec![expr_shadow, increment], "shadowed"),
        (vec![increment, "caller 7", expr_shadow], "shadowed"),
    ] {
        let outcome = pack.inlined(&setup, "caller 7");
        assert_eq!(
            (outcome.code, outcome.answer.as_str()),
            (Code::Ok, answer),
            "{setup:?}"
        );
    }
    let own = "proc vendor::caller {n} {vlen $n}";
    assert_eq!(
        pack.bindings(own, "::vendor::caller"),
        1,
        "its own namespace"
    );
    assert_eq!(
        pack.inlined(&[own], "vendor::caller abc"),
        Outcome {
            code: Code::Ok,
            answer: "3".to_owned(),
            plain: 0
        }
    );
}

/// A variable a command a word substitutes is handed by name is the definition's
/// own, and the splice has no `$` there to rewrite: `[set y]` would read the
/// caller's `y`, and `[incr y]` would write it.
#[test]
fn a_variable_a_substituted_command_names_is_the_definitions_own() {
    let pack = Spliced::new(
        "splice-named-variables",
        &[
            ("vsetcmd", 1, "proc vsetcmd {x} {set y $x; return [set y]}"),
            (
                "vincrcmd",
                1,
                "proc vincrcmd {x} {set y $x; return [incr y]}",
            ),
            (
                "vfmtcmd",
                1,
                "proc vfmtcmd {x} {return [format %s [set x]]}",
            ),
            ("vexprcmd", 1, "proc vexprcmd {x} {expr {[set x] + 1}}"),
        ],
    );
    for (name, answer) in [
        ("vsetcmd", "1"),
        ("vincrcmd", "2"),
        ("vfmtcmd", "1"),
        ("vexprcmd", "2"),
    ] {
        let source = format!("proc caller {{n}} {{set x 42; set y 43; {name} $n}}");
        assert_eq!(pack.bindings(&source, "::caller"), 0, "{name}");
        assert_eq!(
            pack.inlined(&[&source], "caller 1"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0
            },
            "{name}"
        );
    }
}

/// The code generator answers the empty string for the value of an `if` that ends
/// an arm the procedure answers, so a call whose body is one is spliced where its
/// value is the procedure's own and stays a call in an arm.
#[test]
fn a_call_whose_body_is_a_branch_is_not_given_the_value_of_an_arm() {
    let pack = Spliced::new(
        "splice-arm",
        &[(
            "vif",
            1,
            "proc vif {x} {if {$x < 0} {set r neg} else {set r pos}}",
        )],
    );
    for (label, source, bindings) in [
        ("terminal", "proc caller {n} {vif $n}", 1),
        (
            "then arm",
            "proc caller {n} {if {$n != 0} {vif $n} else {return zero}}",
            0,
        ),
        (
            "else arm",
            "proc caller {n} {if {$n == 0} {return zero} else {vif $n}}",
            0,
        ),
        (
            "switch arm",
            "proc caller {n} {switch -- $n {0 {return zero} default {vif $n}}}",
            0,
        ),
    ] {
        assert_eq!(pack.bindings(source, "::caller"), bindings, "{label}");
        assert_eq!(
            pack.inlined(&[source], "caller 7"),
            Outcome {
                code: Code::Ok,
                answer: "pos".to_owned(),
                plain: 0
            },
            "{label}"
        );
    }
}

/// The bodies a pack gives its commands, one for each shape the splice has a path
/// for: `(name, arity, definition)`.
const SPLICED_SHAPES: &[(&str, usize, &str)] = &[
    // A trailing `return`.
    ("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}"),
    // No `return`: the last command's value.
    ("vtail", 1, "proc vtail {x} {expr {$x * 2}}"),
    // A `return` before the end, and a trailing one.
    (
        "vabs",
        1,
        "proc vabs {x} {if {$x < 0} {return [expr {-$x}]}; return $x}",
    ),
    // A `return` before the end, and a last command's value.
    (
        "vsign",
        1,
        "proc vsign {x} {if {$x < 0} {return neg}; set r pos}",
    ),
    ("vconst", 0, "proc vconst {} {return fixed}"),
    (
        "vlocal",
        1,
        "proc vlocal {x} {set y [expr {$x + 1}]; return $y}",
    ),
    ("vinc", 1, "proc vinc {x} {set y $x; incr y; return $y}"),
    ("vnoop", 0, "proc vnoop {} {}"),
    ("vpair", 2, "proc vpair {a b} {return \"$a-$b\"}"),
    // A braced word, which is literal.
    ("vlit", 1, "proc vlit {x} {string length {$x}}"),
    ("vbrace", 1, "proc vbrace {x} {return {$x}}"),
    // A command a word substitutes.
    (
        "vsubst",
        1,
        "proc vsubst {x} {set n [string length $x]; return \"$n-$x\"}",
    ),
    // A value from a branch that has no `return`, and a loop the caller's own
    // loop would nest.
    (
        "vif",
        1,
        "proc vif {x} {if {$x < 0} {set r neg} else {set r pos}}",
    ),
    (
        "vloop",
        1,
        "proc vloop {x} {set t 0; foreach i $x {incr t $i}; set t}",
    ),
    // A braced word with a backslash, and one with brackets, which are literal
    // where the value is the last command's and where it is a `return`'s; the
    // `return {[…]}` form is `a_braced_literal_stays_literal_through_the_wrap`'s.
    (
        "vsetbs",
        1,
        "proc vsetbs {x} {if {$x < 0} {return neg}; set r {a\\tb}}",
    ),
    (
        "vretbs",
        1,
        "proc vretbs {x} {if {$x > 0} {return {a\\tb}}; return neg}",
    ),
    (
        "vsetbr",
        1,
        "proc vsetbr {x} {if {$x < 0} {return neg}; set r {[string length $x]}}",
    ),
    // A variable a command a word substitutes names: no `$` shows the read.
    ("vsetcmd", 1, "proc vsetcmd {x} {set y $x; return [set y]}"),
    (
        "vincrcmd",
        1,
        "proc vincrcmd {x} {set y $x; return [incr y]}",
    ),
    (
        "vfmtcmd",
        1,
        "proc vfmtcmd {x} {return [format %s [set x]]}",
    ),
    ("vexprcmd", 1, "proc vexprcmd {x} {expr {[set x] + 1}}"),
];

/// The places a call can stand, `@CALL@` the call: where its value is the
/// procedure's, where nothing reads it, and where a command around it does.
///
/// An `if` inside an `if` is not among them: the plain compile of a procedure
/// whose value comes from the inner one answers the empty string where Tcl
/// answers the value, so it is no reference for the splice.
const SPLICE_SITES: &[(&str, &str)] = &[
    ("terminal", "proc caller {n} {@CALL@}"),
    ("after a set", "proc caller {n} {set a 5; @CALL@}"),
    (
        "dropped before a return",
        "proc caller {n} {@CALL@; set k 1; return $k}",
    ),
    (
        "dropped before a last set",
        "proc caller {n} {@CALL@; set k 1}",
    ),
    (
        "then arm",
        "proc caller {n} {if {$n != 0} {@CALL@} else {return zero}}",
    ),
    (
        "else arm",
        "proc caller {n} {if {$n == 0} {return zero} else {@CALL@}}",
    ),
    (
        "switch arm",
        "proc caller {n} {switch -- $n {0 {return zero} default {@CALL@}}}",
    ),
    ("static eval", "proc caller {n} {eval {@CALL@}}"),
    (
        "static eval, then more",
        "proc caller {n} {eval {@CALL@}; return after}",
    ),
    (
        "catch result",
        "proc caller {n} {catch {@CALL@} r; return $r}",
    ),
    (
        "catch code",
        "proc caller {n} {set c [catch {@CALL@}]; return $c}",
    ),
    (
        "try ok",
        "proc caller {n} {try {@CALL@} on ok {r} {return $r}}",
    ),
    (
        "try terminal",
        "proc caller {n} {try {@CALL@} on error {e} {return err}}",
    ),
    (
        "try handler",
        "proc caller {n} {try {error x} on error {e} {@CALL@}}",
    ),
    ("uplevel", "proc caller {n} {uplevel 0 {@CALL@}}"),
    (
        "loop body",
        "proc caller {n} {foreach i {1 2} {@CALL@}; return done}",
    ),
    (
        "while body",
        "proc caller {n} {while {1} {@CALL@; break}; return done}",
    ),
    ("twice", "proc caller {n} {@CALL@; @CALL@}"),
    (
        "in a substitution",
        "proc caller {n} {set y [@CALL@]; return $y}",
    ),
    // A body whose value somebody reads: each iteration of an `lmap` and of a
    // `dict map` gives its body's value to the list it builds.
    ("lmap body", "proc caller {n} {lmap i {1 2 3} {@CALL@}}"),
    (
        "lmap body after a set",
        "proc caller {n} {lmap i {1 2} {set a 5; @CALL@}}",
    ),
    (
        "dict map body",
        "proc caller {n} {dict map {k v} {a 1 b 2} {@CALL@}}",
    ),
    // A caller that holds a variable of each spelling the bodies use.
    (
        "caller with the bodies' names",
        "proc caller {n} {set x 42; set y 43; set r 44; @CALL@}",
    ),
];

/// Every body, at every site, answers what its definition answers when it is
/// called as a procedure — for a value, a zero and a negative — and the VM takes
/// the function as compiled. The matrix is a comparison, so it needs no list of
/// expected answers; it asks only that the splice is the call it replaced, and
/// that the plain shapes are spliced at all, so the comparison is not of a
/// call with itself.
#[test]
fn every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site() {
    let pack = Spliced::new("splice-matrix", SPLICED_SHAPES);
    let mut disagreements = Vec::new();
    let mut spliced = 0usize;
    for (name, arity, _) in SPLICED_SHAPES {
        let call = if *arity == 0 {
            (*name).to_owned()
        } else {
            format!("{name} {}", vec!["$n"; *arity].join(" "))
        };
        for (site, template) in SPLICE_SITES {
            let source = template.replace("@CALL@", &call);
            spliced += pack.bindings(&source, "::caller");
            for n in ["-5", "0", "7"] {
                let script = format!("caller {n}");
                let want = pack.reference(&[&source], &script);
                let got = pack.inlined(&[&source], &script);
                if got.code != want.code || got.answer != want.answer || got.plain != 0 {
                    disagreements.push(format!(
                        "{name} at `{site}` ({script}): the definition answers {:?} {:?}, the \
                         splice {:?} {:?} after {} plain recompile(s)\n    {source}",
                        want.code, want.answer, got.code, got.answer, got.plain
                    ));
                }
            }
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} disagreement(s):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
    assert!(
        spliced >= SPLICED_SHAPES.len() * 6,
        "the comparison is of a call with itself: only {spliced} splice(s) were made"
    );
}
