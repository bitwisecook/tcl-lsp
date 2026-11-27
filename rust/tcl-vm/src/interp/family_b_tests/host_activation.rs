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

//! Explicit host and authored activations: numeric and expression policies, source recipes and their caches.

use super::*;

#[test]
fn default_native_entry_retains_actual_execution_point_and_assistance_profile() {
    use tcl_dialect::model::DialectPoint;
    let vm = Vm::new();
    assert!(vm.dialect_profile().is_fallback());
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(entry.profile, vm.dialect_profile().cache_key());
    assert_eq!(
        entry.execution_point,
        Some(DialectPoint::for_tcl_version(vm.runtime_version()))
    );
    for version in tcl_dialect::TclVersion::ALL {
        assert_eq!(
            release_vm(version)
                .native_compilation_entry_for_namespace("", false)
                .unwrap()
                .execution_point,
            Some(DialectPoint::for_tcl_version(version))
        );
    }
}

#[test]
fn explicit_host_engine_keeps_source_logical_and_availability_axes_separate() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    vm.set_dialect_profile(source);
    let logical = vm.native_invocation_dialect();
    let lexical = vm.lexer_config();
    assert!(vm.set_command_surface_profile(host));
    assert!(!vm.set_native_engine_profile(source));
    let old_generation = vm.profile_generation();
    assert!(vm.set_native_engine_profile(host));
    assert_ne!(vm.profile_generation(), old_generation);
    assert_eq!(vm.dialect_profile().cache_key(), source.cache_key());
    assert_eq!(vm.native_invocation_dialect().numbers, logical.numbers);
    assert_eq!(
        vm.native_invocation_dialect().characters,
        logical.characters
    );
    assert_eq!(
        vm.native_invocation_dialect().arithmetic(),
        logical.arithmetic()
    );
    assert_eq!(
        vm.native_invocation_dialect().completion_options_policy(),
        logical.completion_options_policy()
    );
    assert_eq!(
        vm.lexer_config().grammar_over(source.grammar),
        lexical.grammar_over(source.grammar)
    );
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(entry.profile, source.cache_key());
    assert_eq!(
        entry.execution_point,
        tcl_registry::InvocationDialect::of_profile(host).execution_point()
    );
    assert_eq!(
        vm.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
    let child = vm.fork_child_state();
    assert_eq!(child.source_profile().cache_key(), source.cache_key());
    assert_eq!(child.native_invocation_dialect().numbers, logical.numbers);
    assert_eq!(
        child.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
    // Explicit capability activation selects C9 policies only for that
    // host call, then restores the original F5 logical invocation.
    vm.active_native_profile = Some(host);
    assert_eq!(
        vm.native_invocation_dialect().numbers,
        tcl_registry::InvocationDialect::of_profile(host).numbers
    );
    vm.active_native_profile = None;
    assert_eq!(vm.native_invocation_dialect().numbers, logical.numbers);
}

#[test]
fn authored_numeric_provider_is_explicit_and_scoped_to_logical_activations() {
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
    use tcl_syntax::value::ValueOps;
    let provider = AuthoredLogicalNumericSimulation::Tcl84Core;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(
        vm.native_invocation_dialect()
            .scalar_numeric_input_policy()
            .is_none()
    );
    assert_eq!(
        ValueOps::as_int(&mut vm, &Value::int(1)),
        Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
    );
    let old_policy = vm.native_compiler_policy();
    assert!(vm.set_logical_numeric_provider(provider));
    assert_ne!(vm.native_compiler_policy(), old_policy);
    assert!(
        vm.native_invocation_dialect()
            .scalar_numeric_input_policy()
            .is_none()
    );
    let original = Value::string("077");
    assert_eq!(ValueOps::as_int(&mut vm, &original), Ok(63));
    assert!(original.integer_representation().is_none());
    assert_eq!(ValueOps::as_bool(&mut vm, &Value::string("2")), Ok(true));
    assert!(ValueOps::as_bool(&mut vm, &Value::string("NaN")).is_err());
    assert_eq!(
        ValueOps::as_bool(&mut vm, &Value::double(f64::NAN)),
        Ok(true)
    );
    assert!(
        crate::expr::authored_boolean(vm.numeric_context(), &Value::string("NaN"))
            .expect("authored logical numeric truth")
    );
    assert!(ValueOps::as_int(&mut vm, &Value::from_string_bytes(b"1\0tail".to_vec())).is_err());
    assert!(ValueOps::as_int(&mut vm, &Value::string("18446744073709551616")).is_err());
    let child = vm.fork_child_state();
    let parent_policy = vm.native_compiler_policy();
    let child_id = vm.new_interp_slot(child, vm.cur);
    vm.in_interp(child_id, |child| {
        assert_eq!(child.logical_providers.numeric, Some(provider));
        assert_eq!(child.numeric_context().simulation, Some(provider));
        assert_eq!(child.native_compiler_policy(), parent_policy);
    });
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_numeric_provider(provider));
    assert!(vm.numeric_context().simulation.is_none());
}

#[test]
fn authored_expression_policy_has_no_fixed_table_and_refuses_before_function_operands() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    use tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin;
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_native_engine_profile(host));
    let absent = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert!(absent.expression_policy.is_none());
    assert!(vm.set_logical_expression_parse_provider(
        LogicalExpressionParseProvider::Tcl84CoreSimulation
    ));
    let parser = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(
        parser.expression_policy.unwrap().origin,
        ExpressionEvaluationOrigin::AuthoredTcl84Parser
    );
    assert!(
        parser
            .expression_policy
            .unwrap()
            .numeric_simulation
            .is_none()
    );
    assert!(!absent.same_compilation_world(&parser));
    assert!(vm.set_logical_numeric_provider(AuthoredLogicalNumericSimulation::Tcl84Core));
    let scalar = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert!(!parser.same_compilation_world(&scalar));
    assert!(scalar.math_functions.is_none());
    assert_eq!(
        scalar.execution_point,
        tcl_registry::InvocationDialect::of_profile(host).execution_point()
    );
    assert_eq!(
        vm.try_eval_expr("077+1")
            .unwrap()
            .result
            .string_bytes()
            .as_ref(),
        b"64"
    );
    assert!(
        vm.try_eval_expr("abs([set function_operand_entered 1])")
            .is_err()
    );
    assert!(vm.get_var("function_operand_entered").is_none());
}

#[test]
fn host_activation_preserves_separate_authored_numeric_and_expression_policies() {
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
    use tcl_syntax::value::ValueOps;
    let provider = AuthoredLogicalNumericSimulation::Tcl84Core;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        Rc::new(crate::host_native::NativeHost::new()),
        host,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_numeric_provider(provider));
    assert!(vm.set_logical_expression_parse_provider(
        tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation
    ));
    assert_eq!(
        vm.expression_source_profile().cache_key(),
        source.cache_key()
    );
    let logical_sum = crate::expr::arith_in(
        vm.numeric_context(),
        tcl_syntax::expr::BinOp::Add,
        &Value::string("077"),
        &Value::int(1),
    )
    .expect("authored octal arithmetic");
    assert_eq!(logical_sum.string_bytes().as_ref(), b"64");
    let logical_sum_expression = vm
        .try_eval_expr("077+1")
        .expect("explicit authored scalar evaluator");
    assert_eq!(logical_sum_expression.result.string_bytes().as_ref(), b"64");
    let huge = Value::string("18446744073709551616");
    assert!(
        crate::expr::arith_in(
            vm.numeric_context(),
            tcl_syntax::expr::BinOp::Add,
            &huge,
            &Value::int(1)
        )
        .is_err()
    );
    assert_eq!(ValueOps::try_char_len(&mut vm, &Value::string("😀")), Ok(2));
    let user_policy = vm.native_compiler_policy();
    vm.active_native_profile = Some(host);
    assert!(matches!(
        vm.expression_evaluation_policy().unwrap().origin,
        tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::Native(_)
    ));
    let native_abs = vm
        .try_eval_expr("abs(077)")
        .expect("actual C9 command-table math");
    assert_eq!(native_abs.result.string_bytes().as_ref(), b"77");
    assert!(vm.numeric_context().simulation.is_none());
    assert_eq!(vm.expression_source_profile().cache_key(), host.cache_key());
    assert_eq!(ValueOps::as_int(&mut vm, &Value::string("077")), Ok(77));
    assert!(ValueOps::as_int(&mut vm, &Value::string("18446744073709551616")).is_err());
    let native_sum = crate::expr::arith_in(
        vm.numeric_context(),
        tcl_syntax::expr::BinOp::Add,
        &huge,
        &Value::int(1),
    )
    .expect("actual C9 bignum arithmetic");
    assert_eq!(native_sum.string_bytes().as_ref(), b"18446744073709551617");
    assert_eq!(ValueOps::try_char_len(&mut vm, &Value::string("😀")), Ok(1));
    let membership = vm
        .try_eval_expr("\"a\" in {a b c}")
        .expect("host expression grammar");
    assert_eq!(membership.result.string_bytes().as_ref(), b"1");
    assert_ne!(vm.native_compiler_policy(), user_policy);
    vm.active_native_profile = None;
    assert_eq!(
        vm.expression_evaluation_policy().unwrap().origin,
        tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::AuthoredTcl84Parser
    );
    assert_eq!(
        vm.expression_source_profile().cache_key(),
        source.cache_key()
    );
    assert_eq!(vm.native_compiler_policy(), user_policy);
    let child = vm.fork_child_state();
    let child_id = vm.new_interp_slot(child, vm.cur);
    vm.in_interp(child_id, |child| {
        assert_eq!(child.logical_providers.numeric, Some(provider));
        assert_eq!(child.numeric_context().simulation, Some(provider));
        assert_eq!(child.native_compiler_policy(), user_policy);
    });
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_numeric_provider(provider));
    assert!(vm.numeric_context().simulation.is_none());
}

#[test]
fn source_word_recipe_requires_its_own_provider_and_cache_identity() {
    use tcl_registry::invocation_words::LogicalSourceWordProvider;
    let provider = LogicalSourceWordProvider::Tcl84CoreSimulation;
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_native_engine_profile(host));
    let before = vm.native_compiler_policy();
    assert!(vm.set_logical_source_word_provider(provider));
    assert_ne!(vm.native_compiler_policy(), before);
    assert_eq!(
        vm.source_string_protocol(),
        Some(tcl_syntax::native_string::NativeStringProtocol::C(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.native_invocation_dialect().execution_point().is_none());
    let child = vm.fork_child_state();
    assert_eq!(child.logical_providers.source_words, Some(provider));
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_source_word_provider(provider));
    assert_eq!(
        vm.source_string_protocol(),
        Some(tcl_syntax::native_string::NativeStringProtocol::C(
            tcl_dialect::TclVersion::V9_0
        ))
    );
}

#[test]
fn expression_source_cache_follows_selected_success_and_failure_preparation() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::new();
        vm.set_dialect_profile(profile);
        let valid = Value::list(vec![
            Value::string("1"),
            Value::string("+"),
            Value::string("2"),
        ]);
        assert!(vm.prepare_expression_value(&valid).is_ok(), "{engine}");
        assert!(!valid.has_list_representation(), "{engine}");
        assert_eq!(valid.string_bytes().as_ref(), b"1 + 2");
        let invalid = Value::list(vec![Value::string("1"), Value::string("+")]);
        assert!(vm.prepare_expression_value(&invalid).is_err(), "{engine}");
        assert_eq!(
            invalid.has_list_representation(),
            engine == "tcl8.4",
            "{engine}"
        );
        assert_eq!(invalid.string_bytes().as_ref(), b"1 +");
    }
}

#[test]
fn logical_parser_capability_has_explicit_host_cache_and_child_ownership() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    let provider = LogicalExpressionParseProvider::Tcl84CoreSimulation;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.expression_template_policy().is_none());
    assert!(vm.set_native_engine_profile(host));
    let policy = vm.native_compiler_policy();
    assert!(vm.set_logical_expression_parse_provider(provider));
    assert_ne!(vm.native_compiler_policy(), policy);
    assert!(vm.native_invocation_dialect().execution_point().is_none());
    assert!(vm.expression_template_policy().is_some());
    let object = Value::from_native_string_bytes(&b"{\xff\0tail}"[..]);
    vm.prepare_expression_value(&object)
        .expect("explicit logical parser");
    assert_eq!(object.string_bytes().as_ref(), b"{\xff\0tail}");
    let child = vm.fork_child_state();
    assert_eq!(child.logical_providers.expression_parse, Some(provider));
    let policy = vm.native_compiler_policy();
    let child_id = vm.new_interp_slot(child, vm.cur);
    vm.in_interp(child_id, |child| {
        assert_eq!(child.native_compiler_policy(), policy);
    });
    vm.active_native_profile = Some(host);
    assert_eq!(
        vm.expression_parse_context().native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V9_0)
    );
    vm.active_native_profile = None;
    assert_eq!(
        vm.expression_parse_context().native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4)
    );
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_expression_parse_provider(provider));
}

#[test]
fn explicit_host_source_keeps_parser_compiler_and_cache_profile_together() {
    let mut vm = Vm::new();
    let user = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    vm.set_dialect_profile(user);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_source_word_provider(
        tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation,
    ));
    let original = without_literal_observation(
        vm.native_compilation_entry_for_namespace("", false)
            .unwrap(),
    );
    assert_eq!(original.profile, user.cache_key());
    assert!(!vm.lexer_config().expand_syntax);
    assert_eq!(
        vm.source_string_protocol().unwrap().escape_syntax(),
        user.grammar.escapes,
    );

    vm.active_native_profile = Some(host);
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(entry.profile, host.cache_key());
    assert_eq!(vm.source_profile().cache_key(), host.cache_key());
    assert_eq!(
        vm.lexer_config(),
        tcl_lexer::LexerConfig::from_grammar(host.grammar)
    );
    assert_eq!(vm.escape_syntax(), host.grammar.escapes);
    assert_eq!(vm.braced_var_style(), host.grammar.braced_var);
    assert_eq!(entry.lexer_grammar, Some(host.grammar));
    assert_eq!(
        vm.source_string_protocol().unwrap().escape_syntax(),
        host.grammar.escapes,
    );
    assert_eq!(vm.dialect_profile().cache_key(), user.cache_key());

    vm.active_native_profile = None;
    assert_eq!(
        without_literal_observation(
            vm.native_compilation_entry_for_namespace("", false)
                .unwrap()
        ),
        original
    );
    assert_eq!(vm.source_profile().cache_key(), user.cache_key());
    assert!(!vm.lexer_config().expand_syntax);
}

#[test]
fn counted_syntax_failure_state_distinguishes_direct_from_script_preparation() {
    for &(engine, case, source, expected_message, direct_code, eval_code) in
        tcl_test_support::expressions::NATIVE_EXPRESSION_SYNTAX_STATES
    {
        let mut vm = Vm::new();
        let profile = tcl_registry::model::ingress::resolve_environment(engine).analyser_profile();
        vm.set_dialect_profile(profile);
        let seed = vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
            b"PROBE BEFORE".to_vec(),
        ));
        if engine == "jim" {
            vm.set_var("::errorCode", seed.clone()).unwrap();
        }
        let completion = vm.try_eval_expr_bytes(source).unwrap();
        assert_eq!(completion.code, Code::Error, "{engine}/{case}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            expected_message,
            "{engine}/{case}"
        );
        let original_code = crate::command::opt_get(&completion.options, "-errorcode").unwrap();
        assert_eq!(
            original_code.string_bytes().as_ref(),
            direct_code,
            "{engine}/{case} Direct"
        );
        if direct_code == b"PROBE BEFORE" {
            assert!(
                original_code.is_same_object(&seed),
                "{engine}/{case} original code object"
            );
        }
        let error = vm
            .prepare_expression_value(&Value::from_string_bytes(source))
            .unwrap_err();
        let completion = crate::command::completion_from_tcl_error(&mut vm, error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            expected_message,
            "{engine}/{case}"
        );
        let code = crate::command::opt_get(&completion.options, "-errorcode").unwrap();
        assert_eq!(
            code.string_bytes().as_ref(),
            eval_code,
            "{engine}/{case} Eval"
        );
    }
}

#[test]
fn primitive_unchanged_error_code_does_not_read_guest_global_or_restore_a_store() {
    let mut vm = Vm::new();
    let original = vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
        b"PRIOR \xff\0code".to_vec(),
    ));
    vm.set_var("::errorCode", Value::string("GUEST MUTATION"))
        .expect("guest state");
    let unchanged =
        vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged);
    assert!(unchanged.is_same_object(&original));
    assert!(
        vm.native_errors
            .primitive_error_code
            .as_ref()
            .expect("original state")
            .is_same_object(&original)
    );
    assert_eq!(
        vm.get_var("::errorCode")
            .expect("guest global")
            .string_bytes()
            .as_ref(),
        b"GUEST MUTATION"
    );
    let changed = vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
        b"NEXT \xfe".to_vec(),
    ));
    assert_eq!(changed.string_bytes().as_ref(), b"NEXT \xfe");
    assert!(!changed.is_same_object(&original));
}
