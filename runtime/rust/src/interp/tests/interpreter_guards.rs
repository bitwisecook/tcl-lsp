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

//! Interpreter-wide guards: the mutations of traces, visibility, safety, topology, execution policy and version that make them stale.

use super::*;
use crate::interp::commands::CommandVisibilityOutcome;

#[test]
fn trace_registration_invalidates_matching_guard_domains() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let command_token = i
            .prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::CommandTrace),
            )
            .unwrap();
        assert_eq!(
            i.eval_str(b"trace add command guarded rename callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard(command_token, b"guarded"));

        let variable_token = i
            .prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::VariableTrace),
            )
            .unwrap();
        assert_eq!(
            i.eval_str(b"trace add variable watched write callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard(variable_token, b"guarded"));
    });
}

#[test]
fn object_dispatch_remains_poisoned_but_interpreter_is_guardable() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        assert_eq!(
            i.prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::ObjectDispatch)
            ),
            Err(GuardError::Poisoned)
        );
        let token = prepare_interpreter_guard(i);
        assert!(i.check_command_guard(token, b"guarded"));
    });
}

#[test]
fn command_identity_survives_unrelated_mutation_and_follows_its_actual_generation() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        let token = prepare_interpreter_guard(i);
        i.register_builtin(b"unrelated", resultless_builtin);
        assert!(i.check_command_guard(token, b"guarded"));
        let fresh = i
            .prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        assert_eq!(
            i.rename_command(b"guarded", b"moved"),
            RenameOutcome::Renamed
        );
        // A rename preserves the actual command token and both guards.
        assert!(i.check_command_guard(fresh, b"moved"));
        assert!(i.check_command_guard(token, b"moved"));
        let moved = i
            .prepare_command_guard(
                b"moved",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::Interpreter),
            )
            .unwrap();
        i.register_guarded_builtin(b"moved", guarded_builtin, GUARDED_IDENTITY);
        assert!(!i.check_command_guard(token, b"moved"));
        assert!(!i.check_command_guard(moved, b"moved"));
        let replacement = i
            .prepare_command_guard(
                b"moved",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::Interpreter),
            )
            .unwrap();
        assert!(i.check_command_guard(replacement, b"moved"));
        assert!(i.release_command_guard(token));
        assert!(i.release_command_guard(fresh));
        assert!(i.release_command_guard(moved));
        assert!(i.release_command_guard(replacement));
    });
}

#[test]
fn string_length_guard_accepts_irreducible_base_domains() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter);
        let token = i
            .prepare_command_guard(b"string", identity, domains)
            .expect("the registry BASE domains should be live");
        assert!(i.check_command_guard_identity(token, b"string", identity));
        assert!(i.release_command_guard(token));
    });
}

#[test]
fn visibility_safety_and_topology_mutations_stale_interpreter_guards() {
    leak_free(|i| {
        let token = prepare_interpreter_guard(i);
        assert_eq!(
            i.hide_command(b"set", b"set"),
            CommandVisibilityOutcome::Moved
        );
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        assert_eq!(
            i.expose_command(b"set", b"set"),
            CommandVisibilityOutcome::Moved
        );
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        assert_eq!(i.create_child(Some(b"child".to_vec())), b"child");
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        assert!(i.delete_child(b"child"));
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.make_safe();
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.mark_trusted();
        assert_interpreter_guard_stale(i, token);
    });
}

/// A child is another interpreter of the same build, so it states the
/// world its parent is pinned to, the overlay included.
#[test]
fn a_child_states_the_context_its_parent_is_pinned_to() {
    const OVERLAY: u64 = 0x0C0_1705;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    leak_free(|parent| {
        let mut context = tcl_registry::model::runtime_context_for_profile(profile);
        context.overlay_generation = OVERLAY;
        context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
        parent.pin_context(&context).expect("installed, so it pins");
        parent.create_child(Some(b"child".to_vec()));
        let child = parent
            .children
            .borrow()
            .get(b"child".as_slice())
            .expect("child")
            .clone();
        assert_eq!(child.runtime_context(), context);
        assert_eq!(child.held_identity(), parent.held_identity());
    });
}

#[test]
fn child_parent_association_stales_interpreter_guard() {
    leak_free(|parent| {
        parent.create_child(Some(b"child".to_vec()));
        let mut child = parent
            .children
            .borrow()
            .get(b"child".as_slice())
            .expect("child")
            .clone();
        let token = prepare_interpreter_guard(&mut child);
        assert!(child.check_command_guard(token, b"guarded"));

        parent.with_child(b"child", |_| ());
        assert_interpreter_guard_stale(&mut child, token);
    });
}

#[test]
fn execution_policy_and_runtime_version_mutations_stale_interpreter_guards() {
    leak_free(|i| {
        let token = prepare_interpreter_guard(i);
        i.set_host(i.host());
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.set_bgerror_handler(b"handler");
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        let frame = new_string(b"-frame");
        let enabled = new_string(b"1");
        let result = i.debug_apply(&[frame, enabled]).expect("debug policy");
        drop_fresh(frame);
        drop_fresh(enabled);
        drop_fresh(result);
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        let limit = new_string(b"250");
        assert_eq!(i.recursion_limit_apply(Some(limit)), Ok(250));
        drop_fresh(limit);
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        let option = new_string(b"-value");
        let value = new_string(b"100");
        let result = i
            .limit_apply(b"commands", &[option, value])
            .expect("command limit policy");
        drop_fresh(option);
        drop_fresh(value);
        drop_fresh(result);
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_interpreter_guard_stale(i, token);
    });
}

#[test]
fn expression_policy_keeps_native_dispatch_separate_from_authored_parser() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    use tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin;
    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        let native = interp.expression_evaluation_policy().unwrap();
        assert!(matches!(
            native.origin,
            ExpressionEvaluationOrigin::Native(_)
        ));
        assert_eq!(
            tcl_registry::native_expression_program::expression_function_dispatch(
                Some(&native),
                interp.native_invocation_dialect()
            ),
            Some(tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable)
        );
        interp.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(interp.expression_evaluation_policy().is_none());
        let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        assert!(interp.set_logical_expression_parse_provider(
            LogicalExpressionParseProvider::Tcl84CoreSimulation,
            host
        ));
        let authored = interp.expression_evaluation_policy().unwrap();
        assert_eq!(
            authored.origin,
            ExpressionEvaluationOrigin::AuthoredTcl84Parser
        );
        assert!(authored.numeric_simulation.is_none());
        assert_eq!(
            tcl_registry::native_expression_program::expression_function_dispatch(
                Some(&authored),
                tcl_registry::InvocationDialect::of_profile(host)
            ),
            None
        );
    });
}
