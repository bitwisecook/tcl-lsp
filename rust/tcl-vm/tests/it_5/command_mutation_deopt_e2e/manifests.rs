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

//! Identity manifests and spec-pack sets: which rungs a changed pack set, another world, another intrinsic table or other package floors refuse.

use super::*;

/// A service whose optimised modules all state a manifest listing two packs,
/// and whose modules from a source marked `# rests-on-pack` have a top level
/// with a pack-fact site on the first of them: the shape of code that rests on
/// a pack the VM may or may not hold. Plain-dispatch modules state none.
struct PackRestingCompilerSvc {
    pub(super) inner: BytecodeCompileService,
    claimed: tcl_runtime_api::PackFactStamp,
    also_listed: tcl_runtime_api::PackFactStamp,
    pub(super) plain_calls: Rc<Cell<usize>>,
}

impl CompileService for PackRestingCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        let mut module = self.inner.compile(src)?;
        if src.starts_with("# rests-on-pack") {
            module
                .top_level
                .site_claims
                .push(tcl_runtime_api::SiteClaim::PackFacts(self.claimed.clone()));
        }
        let manifest =
            std::sync::Arc::make_mut(module.manifest.as_mut().expect("the compiler states one"));
        manifest.packs = vec![self.claimed.clone(), self.also_listed.clone()];
        manifest.packs.sort();
        Ok(module)
    }

    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.plain_calls.set(self.plain_calls.get() + 1);
        self.inner.compile_traced(src)
    }
}

struct NativeOverride;

/// Pins the VM to a context part-way through the script that calls it.
struct Repin(tcl_runtime_api::RuntimeContext);

impl NativeCommand for NativeOverride {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        Completion::new(Code::Ok, Value::string("NATIVE_OVERRIDE"), Value::empty())
    }
}

impl NativeCommand for Repin {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.pin_context(&self.0).expect("the ingress agrees");
        Completion::new(Code::Ok, Value::empty(), Value::empty())
    }
}

/// A module compiled with no spec pack claims nothing, so the pack facts a
/// VM holds cannot refuse it: it is admitted as compiled under no facts,
/// under one pack set's, and under a changed set's — the rung-1 check never
/// turns a rung-0 module plain.
#[test]
fn a_rung_zero_module_is_admitted_under_a_changed_pack_set() {
    let module = BytecodeCompileService::default()
        .compile("set l {a b c}\nset n [llength $l]\nlindex $l $n")
        .expect("module compiles");
    assert_eq!(module.top_level.site_claims, []);
    assert!(
        !module.top_level.command_bindings.is_empty(),
        "specialised code, whose bindings admission checks"
    );
    let stamp = |content_hash| tcl_runtime_api::PackFactStamp {
        pack: "vendor".to_owned(),
        content_hash,
        vocabulary_version: "2".to_owned(),
        overlay_generation: 9,
        evaluator_revision: 0,
    };

    let mut vm = tcl_vm::test_support::native_vm();
    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    for facts in [Vec::new(), vec![stamp(1)], vec![stamp(2)]] {
        vm.set_pack_facts(facts);
        let completion = vm.run_module(&module);
        assert_eq!(completion.code, Code::Ok, "{completion:?}");
        assert_eq!(completion.result.to_str().as_ref(), "");
    }
    assert_eq!(plain_calls.get(), 0, "admitted as compiled every time");
}

type ManifestEdit = fn(&mut tcl_runtime_api::ArtefactIdentityManifest);

fn pack_stamp(pack: &str, content_hash: u64) -> tcl_runtime_api::PackFactStamp {
    tcl_runtime_api::PackFactStamp {
        pack: pack.to_owned(),
        content_hash,
        vocabulary_version: "2".to_owned(),
        overlay_generation: 9,
        evaluator_revision: 0,
    }
}

/// The manifest is checked as a whole, and what a disagreeing field refuses
/// is the rungs that rest on it: a unit states a manifest listing a pack the
/// VM does not hold, so the unit with a pack-fact site is recompiled plain,
/// and the unit in the same compile with only generic-dispatch sites — which
/// carries the same disagreeing manifest — is admitted as compiled.
///
/// The pack-fact unit's own claim is a fact the VM holds, so it is the
/// manifest, not the claim, that refuses it.
#[test]
fn a_manifest_disagreeing_on_packs_refuses_only_rung_one_sites() {
    let held = pack_stamp("held", 1);
    let missing = pack_stamp("missing", 2);
    let plain_calls = Rc::new(Cell::new(0));
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(PackRestingCompilerSvc {
        inner: BytecodeCompileService::default(),
        claimed: held.clone(),
        also_listed: missing.clone(),
        plain_calls: Rc::clone(&plain_calls),
    }));
    vm.set_pack_facts(vec![held.clone()]);

    let rung_one = vm
        .compile_function("# rests-on-pack\nset x 1")
        .expect("compiles");
    let rung_zero = vm.compile_function("set y 2").expect("compiles");

    assert_eq!(invoke(&mut vm, &rung_one), "1");
    assert_eq!(
        plain_calls.get(),
        1,
        "the manifest lists a pack the VM does not hold, so the pack-fact unit is plain"
    );
    assert_eq!(invoke(&mut vm, &rung_zero), "2");
    assert_eq!(
        plain_calls.get(),
        1,
        "the generic-dispatch unit rests on no pack, so the disagreement does not reach it"
    );

    // Holding both packs, the same compile is admitted as it stands.
    vm.set_pack_facts(vec![held, missing]);
    let admitted = vm
        .compile_function("# rests-on-pack\nset x 1")
        .expect("compiles");
    assert_eq!(invoke(&mut vm, &admitted), "1");
    assert_eq!(plain_calls.get(), 1, "agreeing manifests are admitted");
}

/// A unit that rests on no pack states none, so no pack set the VM holds can
/// refuse it: it is admitted under no facts, one pack set's, and a changed
/// set's, and never turns plain.
#[test]
fn a_rung_zero_unit_is_admitted_under_a_changed_pack_set() {
    let plain_calls = Rc::new(Cell::new(0));
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::new(Cell::new(0)),
        plain_calls: Rc::clone(&plain_calls),
    }));
    let unit = vm
        .compile_function("set l {a b c}\nllength $l")
        .expect("compiles");
    for facts in [
        Vec::new(),
        vec![pack_stamp("vendor", 1)],
        vec![pack_stamp("vendor", 2), pack_stamp("other", 3)],
    ] {
        vm.set_pack_facts(facts);
        assert_eq!(invoke(&mut vm, &unit), "3");
    }
    assert_eq!(plain_calls.get(), 0, "admitted as compiled every time");
}

/// What every rung rests on — the ABI and the world a unit was lexed and
/// specialised for — refuses the whole module when it disagrees, and the
/// refusal names the field and both values. The unedited module is the
/// control.
#[test]
fn a_manifest_for_another_world_refuses_the_whole_module() {
    let module = BytecodeCompileService::default()
        .compile("set x 1")
        .expect("module compiles");
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(vm.run_module(&module).code, Code::Ok);

    let held = vm.held_identity().clone();
    let abi = tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION;
    let edits: [(&str, ManifestEdit, String, String); 4] = [
        (
            "abi_version",
            |m| m.abi_version ^= 1,
            (abi ^ 1).to_string(),
            abi.to_string(),
        ),
        (
            "environment",
            |m| m.environment = "tcl9.0".to_owned(),
            "tcl9.0".to_owned(),
            held.environment.clone(),
        ),
        (
            "release",
            |m| m.release = "8.6".to_owned(),
            "8.6".to_owned(),
            held.release.clone(),
        ),
        (
            "build",
            |m| {
                m.build = tcl_dialect::model::BuildProfileId::JimFull;
            },
            "JimFull".to_owned(),
            format!("{:?}", held.build),
        ),
    ];
    for (field, edit, compiled, holds) in edits {
        let mut foreign = module.clone();
        edit(std::sync::Arc::make_mut(
            foreign.manifest.as_mut().expect("the compiler states one"),
        ));
        let completion = vm.run_module(&foreign);
        assert_eq!(completion.code, Code::Error, "{field}");
        let message = completion.result.to_str();
        assert_eq!(
            &*message,
            format!(
                "bytecode manifest disagrees with the runtime on {field}: \
                 compiled for {compiled}, runtime holds {holds}"
            ),
            "{field}"
        );
    }
}

/// A manifest for another intrinsic table refuses the rung that rests on a
/// shipped implementation's identity and no other: a module whose top level
/// carries command bindings is recompiled plain, and one that carries none is
/// admitted as compiled.
#[test]
fn a_manifest_for_another_intrinsic_table_refuses_only_shipped_backing_sites() {
    let service = BytecodeCompileService::default();
    let specialised = service
        .compile("set l {a b c}\nlindex $l [llength $l]")
        .expect("compiles");
    let generic = service.compile("native_probe").expect("compiles");
    assert_ne!(specialised.top_level.command_bindings, []);
    assert_eq!(generic.top_level.command_bindings, []);

    let run = |module: &tcl_bytecode::ModuleAsm| {
        let plain_calls = Rc::new(Cell::new(0));
        let mut vm = tcl_vm::test_support::native_vm();
        vm.register_native_command("native_probe", Rc::new(NativeOverride));
        vm.set_compiler(Box::new(CountingCompilerSvc {
            inner: BytecodeCompileService::default(),
            fast_calls: Rc::new(Cell::new(0)),
            plain_calls: Rc::clone(&plain_calls),
        }));
        let completion = vm.run_module(module);
        assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
        plain_calls.get()
    };
    for module in [&specialised, &generic] {
        assert_eq!(
            run(module),
            0,
            "the honest manifest is admitted as compiled"
        );
    }
    let foreign = |module: &tcl_bytecode::ModuleAsm| {
        let mut foreign = module.clone();
        std::sync::Arc::make_mut(foreign.manifest.as_mut().expect("the compiler states one"))
            .intrinsic_table_hash[0] ^= 1;
        foreign
    };
    assert_eq!(
        run(&foreign(&specialised)),
        1,
        "the shipped-backing unit is plain"
    );
    assert_eq!(
        run(&foreign(&generic)),
        0,
        "the generic unit rests on no table"
    );
}

/// Pinning another context under the same profile is a change of world a unit
/// admitted under the old one is checked against again: package floors the
/// unit's manifest does not state refuse the rungs that rest on packs — the
/// unit with a pack-fact site is recompiled plain — and no other.
#[test]
fn a_pin_to_other_package_floors_refuses_the_units_that_rest_on_packs() {
    let held = pack_stamp("held", 1);
    let other = pack_stamp("other", 2);
    let plain_calls = Rc::new(Cell::new(0));
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(PackRestingCompilerSvc {
        inner: BytecodeCompileService::default(),
        claimed: held.clone(),
        also_listed: other.clone(),
        plain_calls: Rc::clone(&plain_calls),
    }));
    vm.set_pack_facts(vec![held, other]);
    let rung_one = vm
        .compile_function("# rests-on-pack\nset x 1")
        .expect("compiles");
    let rung_zero = vm.compile_function("set y 2").expect("compiles");
    assert_eq!(invoke(&mut vm, &rung_one), "1");
    assert_eq!(invoke(&mut vm, &rung_zero), "2");
    assert_eq!(
        plain_calls.get(),
        0,
        "the floors agree, so both are admitted"
    );

    let mut context = vm.runtime_context().clone();
    context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
    vm.pin_context(&context).expect("the ingress agrees");
    assert_eq!(invoke(&mut vm, &rung_zero), "2");
    assert_eq!(
        plain_calls.get(),
        0,
        "the generic-dispatch unit rests on no package floor"
    );
    assert_eq!(invoke(&mut vm, &rung_one), "1");
    assert_eq!(
        plain_calls.get(),
        1,
        "the pack-fact unit's manifest states other floors, so it is plain"
    );
}

/// A running function carries its module's manifest: a pin to other package
/// floors part-way through is seen at its next command, and the function that
/// rests on a pack is recompiled plain and finishes, where the same function
/// pinned to the context it already holds is not touched.
#[test]
fn a_running_function_is_checked_against_its_manifest_when_the_pin_changes() {
    let held = pack_stamp("held", 1);
    let other = pack_stamp("other", 2);
    let run = |pin_other_floors: bool| {
        let plain_calls = Rc::new(Cell::new(0));
        let mut vm = tcl_vm::test_support::native_vm();
        vm.set_compiler(Box::new(PackRestingCompilerSvc {
            inner: BytecodeCompileService::default(),
            claimed: held.clone(),
            also_listed: other.clone(),
            plain_calls: Rc::clone(&plain_calls),
        }));
        vm.set_pack_facts(vec![held.clone(), other.clone()]);
        let mut context = vm.runtime_context().clone();
        if pin_other_floors {
            context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
        }
        vm.register_native_command("repin", Rc::new(Repin(context)));
        let handle = vm
            .compile_function("# rests-on-pack\nset x 1\nrepin\nset y 2")
            .expect("compiles");
        let result = invoke(&mut vm, &handle);
        (result, plain_calls.get())
    };

    assert_eq!(
        run(false),
        ("2".to_owned(), 0),
        "the same context: admitted as compiled"
    );
    assert_eq!(
        run(true),
        ("2".to_owned(), 1),
        "other floors: plain from the next command"
    );
}
