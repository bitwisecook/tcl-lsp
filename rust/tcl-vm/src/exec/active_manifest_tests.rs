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

//! VM artifact admission and command-currency controls, independent of Tcl oracles.

use std::rc::Rc;
use std::sync::Arc;

use tcl_bytecode::{FunctionAsm, Instruction, Op, SourceCommandBoundary};
use tcl_runtime_api::{Code, PackFactStamp, SiteClaim};

use super::{Frame, Tick};
use crate::compiled::CompiledUnit;
use crate::interp::Vm;

fn vm() -> Vm {
    crate::native_fixture::core(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    )
}

fn facts() -> PackFactStamp {
    PackFactStamp {
        pack: "active-manifest-control".into(),
        content_hash: 17,
        vocabulary_version: "2".into(),
        overlay_generation: 29,
        evaluator_revision: 0,
    }
}

fn claimed_assembly() -> FunctionAsm {
    FunctionAsm {
        site_claims: vec![SiteClaim::PackFacts(facts())],
        ..FunctionAsm::default()
    }
}

fn retained_unit(vm: &mut Vm, asm: FunctionAsm) -> CompiledUnit {
    let manifest = Arc::new(vm.held_identity().clone());
    let namespace = vm.source_namespace_path();
    vm.compiled_unit(Rc::new(asm), namespace)
        .with_manifest(Some(manifest))
}

#[test]
fn compiled_entry_checks_retained_manifest_and_actual_pack_claims() {
    // naming.compiler.active-manifest-revalidation
    // docs/design/analysis/name-resolution-proofs/compiler-active-manifest-revalidation.md
    // Software artifact identity only; no Native provider behavior is asserted.
    let mut current = vm();
    current.set_pack_facts(vec![facts()]);
    let unit = retained_unit(&mut current, claimed_assembly());
    assert_eq!(current.run_compiled_unit(unit).code, Code::Ok);
    assert!(current.execution_refusal.is_none());

    let mut changed = vm();
    changed.set_pack_facts(vec![facts()]);
    let unit = retained_unit(&mut changed, claimed_assembly());
    changed.set_pack_facts(Vec::new());
    let _ = changed.run_compiled_unit(unit);
    assert!(changed.execution_refusal.is_some());

    // Absence of a manifest supplies no missing pack stamp.
    let mut missing = vm();
    let namespace = missing.source_namespace_path();
    let unit = missing.compiled_unit(Rc::new(claimed_assembly()), namespace);
    assert!(unit.manifest.is_none());
    let _ = missing.run_compiled_unit(unit);
    assert!(missing.execution_refusal.is_some());
}

#[test]
fn active_currency_rechecks_pack_claims_and_manifest_package_floors() {
    // naming.compiler.active-manifest-revalidation
    // docs/design/analysis/name-resolution-proofs/compiler-active-manifest-revalidation.md
    // Software artifact identity only; no Native provider behavior is asserted.
    let mut vm = vm();
    vm.set_pack_facts(vec![facts()]);
    let claimed = Frame::new(retained_unit(&mut vm, claimed_assembly()), false);
    let generic = Frame::new(retained_unit(&mut vm, FunctionAsm::default()), false);
    assert!(vm.active_native_bindings_match(&claimed, &claimed.asm));
    assert!(vm.active_native_bindings_match(&generic, &generic.asm));

    vm.set_pack_facts(Vec::new());
    assert_ne!(claimed.command_epoch, vm.trace_deopt_epoch());
    assert!(!vm.active_native_bindings_match(&claimed, &claimed.asm));
    assert!(vm.active_native_bindings_match(&generic, &generic.asm));

    vm.set_pack_facts(vec![facts()]);
    assert!(vm.active_native_bindings_match(&claimed, &claimed.asm));
    let profile_generation = vm.profile_generation();
    let mut changed = vm.runtime_context().clone();
    changed.packages = vec![("active-manifest-package".into(), "1.0".into())];
    vm.pin_context(&changed)
        .expect("genuine changed package floors");
    assert_eq!(vm.profile_generation(), profile_generation);
    // The pack stamp is present: this refusal comes from the retained manifest.
    assert!(vm.held_identity().packs.contains(&facts()));
    assert!(!vm.active_native_bindings_match(&claimed, &claimed.asm));
    assert!(vm.active_native_bindings_match(&generic, &generic.asm));
}

#[test]
fn stale_pack_command_boundary_does_not_acknowledge_the_changed_epoch() {
    // naming.compiler.active-manifest-revalidation
    // docs/design/analysis/name-resolution-proofs/compiler-active-manifest-revalidation.md
    // Software artifact identity only; no Native provider behavior is asserted.
    let mut vm = vm();
    vm.set_pack_facts(vec![facts()]);
    let mut instruction = Instruction::new(Op::NOP, Vec::new());
    instruction.source_command_boundary = SourceCommandBoundary::Start;
    instruction.source_cmd_text = tcl_lexer::SourceImage::document("list OK");
    let mut asm = claimed_assembly();
    asm.instructions.push(instruction);
    let mut frame = Frame::new(retained_unit(&mut vm, asm), false);
    let admitted_epoch = frame.command_epoch;
    vm.set_pack_facts(Vec::new());
    // No CompileService is attached. Exact-source replay must fail rather
    // than execute or acknowledge the stale pack-specialised instruction.
    assert!(matches!(vm.tick(&mut frame), Tick::Return(_)));
    assert_eq!(frame.command_epoch, admitted_epoch);
    assert_eq!(frame.pc, 0);
}

#[test]
fn refused_overlay_pin_preserves_the_active_artifact_identity() {
    // naming.compiler.active-manifest-revalidation
    // docs/design/analysis/name-resolution-proofs/compiler-active-manifest-revalidation.md
    // Software artifact identity only; no Native provider behavior is asserted.
    let mut vm = vm();
    vm.set_pack_facts(vec![facts()]);
    let frame = Frame::new(retained_unit(&mut vm, claimed_assembly()), false);
    let context = vm.runtime_context().clone();
    let identity = vm.held_identity().clone();
    let epoch = vm.trace_deopt_epoch();
    let mut missing = context.clone();
    missing.overlay_generation = 0x0AC7_1327;
    assert!(matches!(
        vm.pin_context(&missing),
        Err(tcl_registry::model::PinError::OverlayMiss(_))
    ));
    assert_eq!(vm.runtime_context(), &context);
    assert_eq!(vm.held_identity(), &identity);
    assert_eq!(vm.trace_deopt_epoch(), epoch);
    assert!(vm.active_native_bindings_match(&frame, &frame.asm));
}
