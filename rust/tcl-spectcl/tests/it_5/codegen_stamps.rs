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

use tcl_codegen::codegen::ModuleAsm;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_dialect::DialectProfile;
use tcl_dialect::model::DependencyTier;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::native_substitution::{
    NativeSubstitutionCompilationEntry, NativeSubstitutionTarget,
};
use tcl_runtime_api::{
    BackingKind, CommandBindingIdentity, CompileError, CompileFailure, CompileUnit,
    NativeCompilationEntry, PackFactStamp, ProcedureBindingIdentity, ProcedureCompileTarget,
    ProcedureCompileTargetBytes, ProcedureDispatch, ScriptCommandPlan, ScriptCompileTarget,
    ScriptCompileTargetBytes, SiteClaim, SourceImage,
};
use tcl_spectcl::PackSet;
use tcl_vm::{Code, CompileService, Vm};

mod live_differential;
mod reference_bodies;
mod splice_matrix;
mod spliced_bodies;
mod stamp_admission;

// Select one original compiled fixture body through its retained provenance.
fn fixture_procedure<'a>(
    module: &'a ModuleAsm,
    label: &str,
) -> &'a tcl_codegen::codegen::FunctionAsm {
    let mut candidates = module
        .procedure_provenance
        .iter()
        .filter(|(_, source)| source.name == label);
    let (id, provenance) = candidates
        .next()
        .expect("original compiled fixture procedure");
    assert!(
        candidates.next().is_none(),
        "ambiguous compiled fixture {label}"
    );
    let body = module
        .procedures
        .get(id)
        .expect("same retained compiled body ID");
    assert_eq!(body.name, provenance.name);
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&module.profile.grammar);
    assert_eq!(
        body.proc_body_src.as_deref(),
        Some(rules.collapse_braced_word(&provenance.body).as_ref())
    );
    body
}

/// The default compile service, counting the plain-dispatch compiles the VM
/// asks of it.
struct PlainCounting {
    inner: BytecodeCompileService,
    plain: Rc<Cell<usize>>,
    /// The pack definitions the VM's own optimised compiles of procedure
    /// bodies spliced in: the procedure bindings those functions record.
    spliced: Rc<Cell<usize>>,
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
        Self::installed_counting(vm, inner, forged).0
    }

    /// [`Self::installed_with`], with the count of definitions the VM's own
    /// compiles spliced in as well.
    fn installed_counting(
        vm: &mut Vm,
        inner: BytecodeCompileService,
        forged: Option<Forgery>,
    ) -> (Rc<Cell<usize>>, Rc<Cell<usize>>) {
        let plain = Rc::new(Cell::new(0));
        let spliced = Rc::new(Cell::new(0));
        vm.set_compiler(Box::new(Self {
            inner,
            plain: Rc::clone(&plain),
            spliced: Rc::clone(&spliced),
            forged,
        }));
        (plain, spliced)
    }

    fn count(&self) {
        self.plain.set(self.plain.get() + 1);
    }

    /// `module` with every reference-body claim restated as forged.
    fn forge(&self, mut module: ModuleAsm) -> ModuleAsm {
        let Some(forged) = self.forged else {
            return module;
        };
        let restate = |function: &mut tcl_codegen::codegen::FunctionAsm| {
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
    ) -> Result<ScriptCommandPlan, CompileError> {
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

    // The entry-aware and byte-ingress compiles the VM asks for: forwarded to
    // the inner service exactly, so the wrapper changes nothing but the count
    // (a defaulted method here would refuse, or silently compile plain).

    fn compile_script_with_entry(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_script_with_entry(target, profile, entry)
            .map(|module| self.forge(module))
    }

    fn compile_procedure_with_entry(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<ModuleAsm, CompileError> {
        if dispatch == ProcedureDispatch::Plain {
            self.count();
        }
        self.inner
            .compile_procedure_with_entry(target, profile, entry, dispatch)
            .map(|module| self.forge(module))
    }

    fn compile_substitution_with_entry(
        &self,
        target: NativeSubstitutionTarget<'_>,
        profile: &'static DialectProfile,
        entry: NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_substitution_with_entry(target, profile, entry)
            .map(|module| self.forge(module))
    }

    fn compile_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_script_bytes_for_profile(target, profile)
            .map(|module| self.forge(module))
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_script_bytes_with_entry(target, profile, entry)
            .map(|module| self.forge(module))
    }

    fn compile_plain_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
    ) -> Result<ModuleAsm, CompileError> {
        self.count();
        self.inner
            .compile_plain_script_bytes_for_profile(target, profile)
    }

    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        self.count();
        self.inner
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }

    fn compile_procedure_bytes_for_profile(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<ModuleAsm, CompileError> {
        if dispatch == ProcedureDispatch::Plain {
            self.count();
        }
        self.inner
            .compile_procedure_bytes_for_profile(target, profile, dispatch)
            .map(|module| self.forge(module))
    }

    fn compile_procedure_bytes_with_entry(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<ModuleAsm, CompileError> {
        if dispatch == ProcedureDispatch::Plain {
            self.count();
        }
        self.inner
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
            .inspect(|module| {
                self.spliced
                    .set(self.spliced.get() + module.top_level_body.procedure_bindings.len());
            })
            .map(|module| self.forge(module))
    }

    fn script_command_plan_bytes_for_profile(
        &self,
        source: &SourceImage,
        profile: &'static DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        self.inner
            .script_command_plan_bytes_for_profile(source, profile)
    }

    fn script_command_plan_bytes_with_entry(
        &self,
        source: &SourceImage,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ScriptCommandPlan, CompileError> {
        self.inner
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_failure_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
        unit: CompileUnit,
    ) -> Option<CompileFailure> {
        self.inner.compile_failure_for_profile(src, profile, unit)
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
/// The unscoped override must beat the shipped command's release-scoped row
/// under profile-aware lookup, as the pack collision policy promises.
const FOLD_PACK: &str = "speclib vendor 2.0 {\n    \
     command llength -override {\n        \
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

/// Compile this whole-document fixture through the production service, which
/// retains its supplied-source entry and the pack registry's overlay provenance.
fn compile(source: &str, registry: &CommandRegistry) -> ModuleAsm {
    let profile = tcl9();
    BytecodeCompileService::new(registry.project_for_profile(profile))
        .compile_for_profile(source, profile)
        .expect("fixture whole document compiles")
}

/// Run a setup script on `vm`, compiled for the Tcl 9.0 profile the fixtures
/// and their VM are pinned to.
fn prepare(vm: &mut Vm, script: &str) {
    let module = BytecodeCompileService::default()
        .compile_for_profile(script, tcl9())
        .expect("setup compiles");
    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
}

const USE: &str = "set l {1 2 3}\nvendor::unpack $l a b\nlist $a $b\n";

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
    tcl_vm::test_support::native_vm_for_profile(tcl9(), Box::new(std::io::stdout()))
}

/// The service for the packs' overlay — what the editor's queries compile
/// through.
fn service_for(set: &PackSet) -> BytecodeCompileService {
    BytecodeCompileService::for_profile_with_overlay(tcl9(), set.key)
        .expect("the packs' overlay is installed")
}

fn calls_a_command(function: &tcl_codegen::codegen::FunctionAsm) -> bool {
    function.instructions.iter().any(|instruction| {
        matches!(
            instruction.op,
            tcl_codegen::codegen::Op::INVOKE_STK1 | tcl_codegen::codegen::Op::INVOKE_STK4
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

/// `n` splices where the VM's own compile splices pack bodies, and none while
/// it does not ([`tcl_codegen::compile_service::LIVE_REFERENCE_INLINING`]).
const fn live_splices(n: usize) -> usize {
    if tcl_codegen::compile_service::LIVE_REFERENCE_INLINING {
        n
    } else {
        0
    }
}

/// What a script answered, how many plain recompiles the VM asked for (none
/// when the inlined function ran as compiled), and how many pack definitions
/// the VM's own compiles of procedure bodies spliced in.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Code,
    answer: String,
    plain: usize,
    spliced: usize,
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
        let (plain, spliced) =
            PlainCounting::installed_counting(&mut vm, service_for(&self.set), None);
        vm.set_pack_facts(facts(&self.set));
        self.drive(&mut vm, &plain, &spliced, setup, script)
    }

    /// The same run with nothing inlined: what the call means.
    fn reference(&self, setup: &[&str], script: &str) -> Outcome {
        let mut vm = vm_for_overlay();
        let (plain, spliced) =
            PlainCounting::installed_counting(&mut vm, BytecodeCompileService::default(), None);
        self.drive(&mut vm, &plain, &spliced, setup, script)
    }

    fn drive(
        &self,
        vm: &mut Vm,
        plain: &Rc<Cell<usize>>,
        spliced: &Rc<Cell<usize>>,
        setup: &[&str],
        script: &str,
    ) -> Outcome {
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
            spliced: spliced.get(),
        }
    }

    /// How many pack commands `source` compiled with a definition spliced in:
    /// the procedure bindings of the procedure `name`.
    fn bindings(&self, source: &str, name: &str) -> usize {
        let module = service_for(&self.set)
            .compile_for_profile(source, tcl9())
            .expect("compiles");
        fixture_procedure(&module, name).procedure_bindings.len()
    }
}
