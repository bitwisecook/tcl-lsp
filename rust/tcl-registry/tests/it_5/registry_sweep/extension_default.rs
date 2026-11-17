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

//! The extension default sits at the top of every axis: its spec, its resolution and its narrowing.

use super::*;

/// The conservative fact for a command a native extension registers is the top
/// of every axis, stated once (`docs/design/compiler/registry-consumer-contracts.md`
/// § *C Tcl extensions*): unknown arity; every argument may be a script or a
/// variable name at any level; unknown reads and writes; may create, rename
/// and delete commands and establish traces; may complete with any code, a
/// normal completion among them; a taint sink and source; unsafe and hidden in
/// a safe interpreter; never pure; and host-native, so it names no stamp and no
/// window and is dispatched plain at every release.
///
/// Two axes are the registry's own wildcard and are left unstated: a command
/// with no state-transition descriptor resolves to `UnknownInvocation`, which a
/// closed "unknown rebinding" statement could only narrow. The negative is the
/// narrowing itself: a stated purity replaces the effect axes and no other, so
/// a pure extension command is still a taint source and still hidden in a safe
/// interpreter.
/// The traits the design page names for the extension default, each by the name
/// `traits.rs` gives it, and nothing else: a trait added to the default is a
/// decision, not drift.
fn extension_default_traits() -> Traits {
    [
        "EVALUATES_CODE",
        "CREATES_BARRIER",
        "CREATES_DYNAMIC_BARRIER",
        "UNSAFE",
        "SAFE_INTERP_HIDDEN",
        "TAINT_SINK",
        "TAINT_SOURCE",
        "ESTABLISHES_VARIABLE_TRACE",
    ]
    .iter()
    .map(|name| tcl_registry::traits::Trait::from_name(name).expect(name))
    .collect()
}

/// What the default's own spec states, and what it leaves to the registry's
/// wildcard.
fn check_extension_default_spec(spec: &tcl_registry::CommandSpec) {
    use tcl_registry::RuntimeBacking;
    use tcl_registry::completion::{CompletionCodeDomain, CompletionDescriptor};
    use tcl_registry::side_effects::SideEffect;

    let expected = extension_default_traits();
    assert_eq!(spec.traits, expected, "got {}", spec.traits);
    assert!(!spec.traits.contains(Traits::PURE), "never pure");

    assert_eq!(spec.arity, Arity::any());
    assert_eq!(
        spec.side_effects.lower_bound(),
        [SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        "unknown reads and writes"
    );
    assert_eq!(
        spec.completion,
        tcl_registry::knowledge::Knowledge::Partial(CompletionDescriptor::CONSERVATIVE)
    );
    assert_eq!(
        CompletionDescriptor::CONSERVATIVE.codes,
        CompletionCodeDomain::Any
    );
    assert_eq!(spec.runtime_backing, RuntimeBacking::HostNative);

    // No stamp, no window and no hook: a host-native command is never
    // specialised, whatever the release.
    assert!(
        spec.codegen_hook.is_none()
            && spec.codegen_hook_windows.is_empty()
            && spec.inline_codegen_hook.is_none()
            && spec.inline_codegen_hook_windows.is_empty()
            && spec.semantic_operation.is_none()
            && spec.semantic_operation_windows.is_empty()
            && spec.native_lowering.is_none()
            && spec.native_lowering_windows.is_empty()
            && spec.lowering_hook.is_none()
            && spec.analyser_hook.is_none()
    );
    // The wildcard axes are unstated, not stated narrower.
    assert!(
        spec.command_table_effect.is_none()
            && spec.state_transitions.is_none()
            && spec.world_effects.is_unknown()
    );
}

/// What a registry makes of the default at every arity and at every release.
fn check_extension_default_resolution(spec: tcl_registry::CommandSpec) {
    use tcl_registry::completion::CompletionDescriptor;
    use tcl_registry::{SemanticOperationId, StateTransitionKnowledge};

    let mut registry = CommandRegistry::build_default();
    registry.insert(spec);
    for words in [
        &[][..],
        &["a"][..],
        &["a", "b", "c", "d", "e", "f", "g"][..],
    ] {
        let resolved = registry
            .resolve_invocation("ext_cmd", words, None)
            .expect("the extension command resolves");
        let facts = resolved.facts();
        assert!(
            facts
                .arity
                .accepts(u16::try_from(words.len()).expect("small"))
        );
        assert_eq!(facts.completion, CompletionDescriptor::CONSERVATIVE);
        assert_eq!(facts.operation, SemanticOperationId::Invoke);
        assert!(
            matches!(
                facts.state_transitions,
                StateTransitionKnowledge::UnknownInvocation
            ),
            "it may rebind any command and establish any trace"
        );
        assert!(resolved.effects().requires_world_barrier());
    }
    for point in [
        None,
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        Some(SurfaceQuery::any_release(Family::Tcl)),
    ] {
        let call = registry
            .resolve_call("ext_cmd", &["a"], point)
            .expect("the extension command resolves");
        assert!(
            call.codegen_hook.is_none()
                && call.inline_codegen_hook.is_none()
                && call.lowering_hook.is_none()
                && call.analyser_hook.is_none(),
            "dispatched plain at {point:?}"
        );
    }
}

/// The declared form of the default is the spec's, and a stated purity narrows
/// the effect axes and no other.
fn check_extension_default_narrowing() {
    use tcl_dialect::model::Provenance;
    use tcl_registry::extension_default;
    use tcl_registry::model::DeclaredCommand;

    let declared =
        DeclaredCommand::extension("ext_cmd".to_owned(), Vec::new(), Provenance::Document);
    assert_eq!(declared.traits, extension_default_traits());
    let pure = declared.narrowed_by(Traits::PURE, Vec::new());
    assert!(pure.traits.contains(Traits::PURE));
    assert!(
        !pure.traits.intersects(extension_default::EFFECT_AXES) && pure.side_effects.is_empty(),
        "purity replaces the effect axes: {}",
        pure.traits
    );
    assert!(
        pure.traits.contains(Traits::TAINT_SOURCE)
            && pure.traits.contains(Traits::TAINT_SINK)
            && pure.traits.contains(Traits::UNSAFE)
            && pure.traits.contains(Traits::SAFE_INTERP_HIDDEN),
        "purity says nothing of taint or safety: {}",
        pure.traits
    );
}

/// The conservative fact for a command a native extension registers is the top
/// of every axis, stated once (`docs/design/compiler/registry-consumer-contracts.md`
/// § *C Tcl extensions*): unknown arity; every argument may be a script or a
/// variable name at any level; unknown reads and writes; may create, rename
/// and delete commands and establish traces; may complete with any code, a
/// normal completion among them; a taint sink and source; unsafe and hidden in
/// a safe interpreter; never pure; and host-native, so it names no stamp and no
/// window and is dispatched plain at every release.
///
/// Two axes are the registry's own wildcard and are left unstated: a command
/// with no state-transition descriptor resolves to `UnknownInvocation`, which a
/// closed "unknown rebinding" statement could only narrow. The negative is the
/// narrowing itself: a stated purity replaces the effect axes and no other, so
/// a pure extension command is still a taint source and still hidden in a safe
/// interpreter.
#[test]
fn the_extension_default_is_at_the_top_of_every_axis() {
    let spec = tcl_registry::CommandSpec::extension_default("ext_cmd");
    check_extension_default_spec(&spec);
    check_extension_default_resolution(spec);
    check_extension_default_narrowing();
}
