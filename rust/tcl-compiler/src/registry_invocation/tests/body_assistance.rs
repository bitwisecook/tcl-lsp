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

//! Body assistance: navigation, prepared views, packaged availability and unprofiled entries.

use super::*;

#[test]
fn body_assistance_keeps_navigation_separate_from_unanimous_context() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    let tokens = last_source_tokens("interp alias {} declare {} proc p {x}; declare {}");
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        view.possible_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert!(
        view.definite_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert!(
        !view
            .possible_roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::ParamList)
    );

    let tokens = last_source_tokens(
        "if {$flag} {rename if original; proc if args {return ordinary}}; if {$other} {set x 1}",
    );
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        view.possible_roles
            .contains(&(1, tcl_registry::ArgRole::Body))
    );
    assert!(view.unknown_residual);
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
    assert!(view.definite_scope.is_none());

    let tokens = last_source_tokens(
        "interp alias {} declare {} proc p; rename declare {}; proc declare args {return ordinary}; declare {} {}",
    );
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert_eq!(
        view.possible_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );

    let tokens = last_source_tokens(
        "rename if original; proc if args {return ordinary}; if {$other} {set x 1}",
    );
    assert!(catalogue_invocation_assistance(registry, test_context(), &tokens).is_none());
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert_eq!(
        view.possible_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_invocation.is_none());

    let tokens = last_source_tokens(
        "if {$flag} {rename proc original; original proc args {return ordinary}}; proc p {x} {}",
    );
    assert!(catalogue_invocation_assistance(registry, test_context(), &tokens).is_none());
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        view.possible_roles
            .contains(&(2, tcl_registry::ArgRole::Body))
    );
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_invocation.is_none());
}

#[test]
fn body_assistance_does_not_promote_authoring_facts_without_an_actual_axis() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    let original = last_source_tokens("proc p {x} {}");
    let actual = invocation_body_assistance(registry, test_context(), &surface, &original);
    assert!(actual.definite_invocation.is_some());
    let mut binding = original.source_binding.as_ref().unwrap().clone();
    std::sync::Arc::make_mut(&mut binding.variable_context).invocation_dialect = None;
    let tokens = original.with_source_binding(binding);
    let advisory = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        advisory
            .possible_roles
            .contains(&(2, tcl_registry::ArgRole::Body))
    );
    assert_eq!(
        advisory.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(advisory.definite_traits.is_empty());
    assert!(advisory.definite_invocation.is_none());
    assert!(advisory.conditional_invocation.is_none());
}

#[test]
fn prepared_body_views_preserve_residual_and_selected_projection_boundaries() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    for source in [
        "proc p {x} {}",
        "interp alias {} declare {} proc p; declare {x} {}",
        "interp alias {} declare {} proc p {x}; declare {}",
        "proc p {} {}; interp alias {} inspect {} info body; inspect p",
        "if {$flag} {rename proc original; original proc args {return ordinary}}; proc p {x} {}",
        "rename proc original; original proc args {return ordinary}; proc p {x} {}",
        "proc p {*}$unknown",
        "external p {x} {}",
    ] {
        let tokens = last_source_tokens(source);
        let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert_eq!(
            view.possible_roles,
            invocation_argument_role_assistance(registry, test_context(), &surface, &tokens),
            "possible original roles: {source}"
        );
        assert_eq!(
            view.definite_roles,
            invocation_argument_role_consensus(registry, test_context(), &tokens),
            "unanimous original roles: {source}"
        );
        let selected = resolved_tokens_invocation(registry, Some(test_context()), &tokens)
            .filter(|_| !view.unknown_residual && !view.may_be_absent)
            .filter(|invocation| {
                invocation.facts.arg_roles_complete
                    && invocation.facts.arity_accepts_frozen_arguments() == Some(true)
            });
        assert_eq!(
            view.definite_invocation, selected,
            "selected implementation: {source}"
        );
        if view.unknown_residual || view.may_be_absent {
            assert!(view.definite_roles.is_empty(), "{source}");
            assert!(view.definite_traits.is_empty(), "{source}");
            assert!(view.definite_scope.is_none(), "{source}");
        }
    }
}

#[test]
fn packaged_body_assistance_keeps_availability_separate_from_native_entry() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    let source = "bind .widget click {return event}";
    let config = tcl_lexer::LexerConfig::for_dialect("tk");
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let command = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command);
    bindings.stamp_original_tokens(&mut tokens);
    let context = body_assistance_context(registry, &tokens).unwrap();
    assert_eq!(
        context,
        SemanticContext::for_profile(registry.profile().unwrap())
    );
    assert!(!context.context().authoring_query().packages.is_empty());
    let view =
        segmented_body_assistance(&surface, &bindings, source, config, &command, 0, None).unwrap();
    assert!(
        view.possible_roles
            .contains(&(2, tcl_registry::ArgRole::Body))
    );
    let opaque =
        tokens.with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    let view = invocation_body_assistance(registry, context, &surface, &opaque);
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
    assert!(view.definite_scope.is_none());
    assert!(view.definite_invocation.is_none());
}

#[test]
fn unprofiled_body_assistance_uses_retained_entry_without_definite_offset_free_facts() {
    let registry = CommandRegistry::build_default();
    assert!(registry.profile().is_none());
    let surface = tcl_registry::model::DocumentCommandSurface::new(&registry, None);
    let source = "eval {$script}";
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let bindings =
        crate::command_binding::SourceCommandBindings::analyse(source, config, &registry);
    let command = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let view =
        segmented_body_assistance(&surface, &bindings, source, config, &command, 0, None).unwrap();
    assert!(
        view.possible_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert!(
        view.possible_traits
            .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
    );
    let view =
        unpositioned_body_assistance(&surface, &bindings, source, config, &command, None).unwrap();
    assert!(
        view.possible_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
    assert!(view.definite_invocation.is_none());
    let unknown = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        crate::command_binding::SourceAnalysisOptions::default(),
    );
    assert!(
        segmented_body_assistance(&surface, &unknown, source, config, &command, 0, None).is_none()
    );
    assert!(
        unpositioned_body_assistance(&surface, &unknown, source, config, &command, None).is_none()
    );
}
