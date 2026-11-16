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

//! The context a VM is pinned to: vendor profile masks, the trace gate, the identity a module states for its pin, and contexts the ingress refuses.

use super::*;

/// A precision point about vendor profiles: an iRules-pinned VM
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

/// The `trace` option table is the pinned profile's, not the plain release
/// its runtime version names: an iRules VM emulates 8.4 but the TMM's Tcl has
/// only the three legacy forms, so `trace add` — which `tcl8.4` has — is a bad
/// option there and `trace variable` is not. The permissive fallback states
/// no table of its own, so a VM pinned to nothing keeps answering for the
/// release it emulates, Tcl 9, which has lost the legacy forms.
#[test]
fn the_trace_gate_reads_the_pinned_profile_not_the_release_name() {
    let probe = |profile: &'static DialectProfile, form: &str| {
        profile_output(
            &format!("set r \"[catch {{trace {form}}} m]:$m\"\nset r\n"),
            profile,
        )
    };
    let add = "add variable x write cb";
    let legacy = "variable x w cb";
    let resolve =
        |name: &str| tcl_registry::model::ingress::resolve_environment(name).analyser_profile();

    assert_eq!(
        probe(DialectProfile::irules(), add),
        "1:bad option \"add\": must be variable, vdelete, or vinfo"
    );
    assert_eq!(probe(DialectProfile::irules(), legacy), "0:");

    assert_eq!(probe(resolve("tcl8.4"), add), "0:");
    assert_eq!(probe(resolve("tcl8.4"), legacy), "0:");

    for profile in [resolve("tcl9.0"), DialectProfile::plain_tcl()] {
        assert_eq!(probe(profile, add), "0:", "{}", profile.name);
        assert_eq!(
            probe(profile, legacy),
            "1:bad option \"variable\": must be add, info, or remove",
            "{}",
            profile.name
        );
    }
}

/// `set_dialect_profile` is the profile form of `pin_context`: the context a
/// profile names, resolved through the ingress, pins the same profile, the
/// same release and the same identity.
#[test]
fn the_profile_form_of_a_pin_is_the_context_the_profile_names() {
    for profile in DialectProfile::all()
        .iter()
        .chain([DialectProfile::plain_tcl(), DialectProfile::tk()])
    {
        // Vendor profiles have no native core of their own; they run on the
        // default engine.
        let mut by_profile = tcl_vm::test_support::native_vm();
        by_profile.set_dialect_profile(profile);
        let mut by_context = tcl_vm::test_support::native_vm();
        by_context
            .pin_context(&tcl_registry::model::runtime_context_for_profile(profile))
            .unwrap_or_else(|error| panic!("{}: {error}", profile.name));

        assert!(
            std::ptr::eq(by_context.dialect_profile(), by_profile.dialect_profile()),
            "{}",
            profile.name
        );
        assert_eq!(by_context.runtime_version(), by_profile.runtime_version());
        assert_eq!(by_context.runtime_context(), by_profile.runtime_context());
        assert_eq!(by_context.held_identity(), by_profile.held_identity());
    }
}

/// A VM states the identity it holds — the context it is pinned to, the pack
/// facts it was given, this build's tables — in the shape a compiled module
/// states its own, so the two compare field for field.
#[test]
fn a_vm_states_the_identity_a_module_compiled_for_its_pin_states() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    let compiled = BytecodeCompileService::for_profile(profile)
        .compile("set x 1")
        .expect("compiles")
        .manifest
        .expect("a compiler states a manifest");
    assert_eq!(vm.held_identity(), &*compiled);
    assert_eq!(compiled.environment, "tcl8.6");
    assert_eq!(compiled.release, "8.6");

    let stamp = |pack: &str| tcl_runtime_api::PackFactStamp {
        pack: pack.to_owned(),
        content_hash: 1,
        vocabulary_version: "2".to_owned(),
        overlay_generation: 9,
        evaluator_revision: 0,
    };
    vm.set_pack_facts(vec![stamp("b"), stamp("a"), stamp("b")]);
    assert_eq!(vm.held_identity().packs, vec![stamp("a"), stamp("b")]);
    assert_eq!(
        vm.held_identity().environment,
        compiled.environment,
        "the facts do not move the world"
    );
    vm.set_dialect_profile(profile);
    assert_eq!(
        vm.held_identity().packs,
        vec![stamp("a"), stamp("b")],
        "a pin states the facts the VM already holds"
    );
    vm.set_pack_facts(Vec::new());
    assert_eq!(vm.held_identity(), &*compiled);
}

/// A context the ingress does not agree with is an error and leaves the pin
/// as it was; an overlay nothing has installed is one of them, and is never
/// the un-overlaid generation under another name.
#[test]
fn a_context_the_ingress_refuses_leaves_the_pin_unchanged() {
    use tcl_registry::model::PinError;

    const OVERLAY: u64 = 0x0C0_1703;
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let context = tcl_registry::model::runtime_context_for_profile(profile);
    let mut vm = tcl_vm::test_support::native_vm();
    vm.pin_context(&context).expect("the profile's own context");

    let mut unknown = context.clone();
    unknown.environment = "no-such-environment".to_owned();
    let mut wrong_release = context.clone();
    wrong_release.release = "9.0".to_owned();
    let mut missing_overlay = context.clone();
    missing_overlay.overlay_generation = OVERLAY;
    for (what, refused) in [
        ("unknown", unknown),
        ("release", wrong_release),
        ("overlay", missing_overlay.clone()),
    ] {
        assert!(vm.pin_context(&refused).is_err(), "{what}");
        assert!(std::ptr::eq(vm.dialect_profile(), profile), "{what}");
        assert_eq!(vm.runtime_context(), &context, "{what}");
    }
    assert!(matches!(
        vm.pin_context(&missing_overlay),
        Err(PinError::OverlayMiss(miss)) if miss.overlay == OVERLAY
    ));

    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    vm.pin_context(&missing_overlay)
        .expect("installed, so it pins");
    assert_eq!(vm.runtime_context().overlay_generation, OVERLAY);
    assert!(std::ptr::eq(vm.dialect_profile(), profile));
}

/// `tk` is a runnable compile target (#2314): a module compiled for it
/// states its own environment at its 8.6 point rather than `tcl/9.0`, a VM
/// pinned to `tk` on its 8.6 core admits it, and code runs with 8.6
/// semantics.
#[test]
fn a_module_compiled_for_tk_states_tk_and_runs_under_a_tk_pin() {
    let profile = DialectProfile::tk();
    let module = BytecodeCompileService::for_profile(profile)
        .compile("set x 1")
        .expect("compiles");
    let manifest = module
        .manifest
        .as_ref()
        .expect("a compiler states a manifest");
    assert_eq!(manifest.environment, "tk");
    assert_eq!(manifest.release, "8.6");

    let host = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(host, Box::new(std::io::sink()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    vm.set_dialect_profile(profile);
    assert_eq!(vm.held_identity(), &**manifest);
    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    let completion = vm
        .try_eval_source("list [expr {010}] [catch {string insert abc 1 X}]")
        .expect("the VM's own compile entry");
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    assert_eq!(
        &*completion.result.to_str(),
        "8 1",
        "8.6: octal 010, no string insert"
    );
}
