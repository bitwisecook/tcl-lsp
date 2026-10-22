// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected framework source keeps the genuine core's complete manifest context.

use super::{Code, Vm};
use std::sync::Arc;
use tcl_bytecode::ModuleAsm;
use tcl_runtime_api::ManifestField;

fn host() -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
}

fn vendor_vm() -> Vm {
    let mut vm = crate::native_fixture::interpreter(host());
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_command_surface_profile(host()));
    vm
}

fn host_module(vm: &mut Vm) -> ModuleAsm {
    let saved = vm.active_native_profile.replace(host());
    let module = vm.compile_cached("set framework_value 42").unwrap();
    vm.active_native_profile = saved;
    module.as_ref().clone()
}

#[test]
fn genuine_framework_source_uses_core_manifest_and_restores_vendor_pin() {
    let mut vm = vendor_vm();
    let vendor = vm.held_identity().clone();
    let completion = vm
        .try_eval_native_host_source("set framework_value 42")
        .unwrap();
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.string_bytes().as_ref(), b"42");
    assert!(vm.active_native_profile.is_none());
    assert_eq!(vm.held_identity(), &vendor);
    assert_eq!(
        vm.dialect_profile().cache_key(),
        tcl_dialect::DialectProfile::irules().cache_key()
    );
}

#[test]
fn framework_manifest_is_admitted_only_under_its_original_selection() {
    let mut vm = vendor_vm();
    let module = host_module(&mut vm);
    assert!(vm.validate_module_profile(&module).is_err());
    vm.active_native_profile = Some(host());
    vm.validate_module_profile(&module).unwrap();
    assert!(vm.manifest_admits(&module.top_level, module.manifest.as_deref()));
    vm.active_native_profile = None;
    assert!(vm.validate_module_profile(&module).is_err());
    assert!(!vm.manifest_admits(&module.top_level, module.manifest.as_deref()));
}

#[test]
fn selecting_profile_without_core_never_issues_host_context() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_native_engine_profile(host()));
    assert!(vm.native_core_pin.is_none());
    vm.active_native_profile = Some(host());
    assert!(vm.selected_source_identity().is_err());
    let mut producer = vendor_vm();
    let module = host_module(&mut producer);
    assert!(vm.validate_module_profile(&module).is_err());
    assert!(!vm.manifest_admits(&module.top_level, module.manifest.as_deref()));
}

#[test]
fn changed_engine_and_returned_label_do_not_renew_core_context() {
    let mut vm = vendor_vm();
    let foreign = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    assert!(vm.set_native_engine_profile(foreign));
    assert!(vm.native_core_pin.is_none());
    assert!(vm.set_native_engine_profile(host()));
    assert!(vm.native_core_pin.is_none());
    vm.active_native_profile = Some(host());
    assert!(vm.selected_source_identity().is_err());
}

#[test]
fn foreign_active_selection_cannot_borrow_original_core_context() {
    let mut vm = vendor_vm();
    vm.active_native_profile =
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile());
    assert!(vm.selected_source_identity().is_err());
    vm.active_native_profile = Some(host());
    assert_eq!(vm.selected_source_identity().unwrap().environment, "tcl9.0");
    let mut child = vm.fork_child_state();
    child.active_native_profile = Some(host());
    assert_eq!(
        child.native_core_pin.as_ref().unwrap().identity(),
        vm.native_core_pin.as_ref().unwrap().identity()
    );
}

#[test]
fn complete_installed_core_input_survives_vendor_repins() {
    let mut vm = crate::native_fixture::interpreter(host());
    let mut context = vm.runtime_context().clone();
    context
        .packages
        .push(("host_contract".into(), "1.0".into()));
    vm.pin_context(&context).unwrap();
    let expected = vm.held_identity().clone();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    vm.active_native_profile = Some(host());
    assert_eq!(vm.selected_source_identity().unwrap(), &expected);
    assert_eq!(vm.native_core_pin.as_ref().unwrap().context, context);
    let mut invalid = context.clone();
    invalid.build = tcl_dialect::model::BuildProfileId::Unknown;
    assert!(vm.pin_context(&invalid).is_err());
    assert_eq!(vm.selected_source_identity().unwrap(), &expected);
}

#[test]
fn host_manifest_keeps_exact_generic_identity_checks() {
    let mut vm = vendor_vm();
    vm.active_native_profile = Some(host());
    let original = host_module(&mut vm);
    let original_manifest = original.manifest.as_ref().unwrap();
    for field in [
        ManifestField::AbiVersion,
        ManifestField::Environment,
        ManifestField::Release,
        ManifestField::Build,
    ] {
        let mut module = original.clone();
        let mut manifest = original_manifest.as_ref().clone();
        match field {
            ManifestField::AbiVersion => manifest.abi_version ^= 1,
            ManifestField::Environment => manifest.environment.push_str("-foreign"),
            ManifestField::Release => manifest.release.push_str("-foreign"),
            ManifestField::Build => manifest.build = tcl_dialect::model::BuildProfileId::Unknown,
            _ => unreachable!(),
        }
        assert_eq!(
            manifest.disagreements(vm.selected_source_identity().unwrap()),
            vec![field]
        );
        module.manifest = Some(Arc::new(manifest));
        assert!(vm.validate_module_profile(&module).is_err(), "{field:?}");
        assert!(
            !vm.manifest_admits(&module.top_level, module.manifest.as_deref()),
            "{field:?}"
        );
    }
    vm.validate_module_profile(&original).unwrap();
}

#[test]
fn host_context_keeps_all_manifest_fields_and_current_pack_facts() {
    let mut vm = vendor_vm();
    vm.active_native_profile = Some(host());
    let before = vm.selected_source_identity().unwrap().clone();
    let stamp = tcl_runtime_api::PackFactStamp {
        pack: "host-proof".into(),
        content_hash: 71,
        vocabulary_version: "2".into(),
        overlay_generation: 72,
        evaluator_revision: 73,
    };
    vm.set_pack_facts(vec![stamp.clone()]);
    let held = vm.selected_source_identity().unwrap().clone();
    assert_eq!(held.packs, vec![stamp]);
    assert_eq!(held.abi_version, before.abi_version);
    assert_eq!(held.environment, before.environment);
    assert_eq!(held.release, before.release);
    assert_eq!(held.build, before.build);
    assert_eq!(held.packages, before.packages);
    assert_eq!(held.intrinsic_table_hash, before.intrinsic_table_hash);
    assert_eq!(
        held.embedded_stdlib_revision,
        before.embedded_stdlib_revision
    );
    vm.set_pack_facts(Vec::new());
    assert_eq!(vm.selected_source_identity().unwrap(), &before);
    assert_eq!(vm.held_identity().environment, "f5-irules");
}
