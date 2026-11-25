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

use super::*;
use crate::hooks::LoweringHookId;

mod compiled_bodies;
mod grammar_selection;
mod hook_registration;
mod introspection_grammars;
mod name_effects;
mod string_grammars;
mod successful_handler;
mod variable_and_list_grammars;

fn context(frame: NativeCompilationFrame) -> NativeCompilationContext {
    NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame,
        loop_depth: 0,
        catch_depth: Some(0),
    }
}

#[test]
fn every_sampled_stock_registration_has_a_matching_availability_descriptor() {
    let registry = crate::CommandRegistry::build_default();
    let mut observed = 0;
    for row in include_str!("../native_compilation_hooks.tsv")
        .lines()
        .filter(|row| !row.starts_with('#'))
    {
        let cells: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        assert_eq!(cells.len(), 6);
        for (version, registration) in TclVersion::ALL.into_iter().zip(&cells[1..]) {
            if *registration == "ABSENT" {
                continue;
            }
            let dialect = InvocationDialect::for_version(version);
            let spec = registry
                .get_for_surface(cells[0], dialect.authoring_query())
                .unwrap_or_else(|| panic!("missing stock command {}", cells[0]));
            let native = spec
                .native_compilation
                .unwrap_or_else(|| panic!("missing native contract {}", cells[0]));
            let hook = native
                .compiler_hook_presence(InvocationDialect::of_profile(
                    tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
                ))
                .expect("pinned native C registration");
            assert_eq!(
                hook,
                *registration != "NULL",
                "{} {version:?}: {registration}",
                cells[0]
            );
            observed += 1;
        }
    }
    assert_eq!(observed, 421);
}

#[test]
fn expression_compilation_does_not_invent_a_legacy_math_function_table() {
    let operand = NativeCompiledExpressionOperand {
        argument: 0,
        error_context: NativeCompiledExpressionErrorContext::None,
    };
    for version in TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
        assert_eq!(
            operand.validate("1 + 2", &context),
            NativeExpressionCompilation::Accepted
        );
        for expression in ["future_function(1)", "abs()", "int(1)"] {
            assert_eq!(
                operand.validate(expression, &context),
                if version == TclVersion::V8_4 {
                    NativeExpressionCompilation::Unknown
                } else {
                    NativeExpressionCompilation::Accepted
                }
            );
        }
        assert!(
            matches!(operand.validate("+", &context),
                NativeExpressionCompilation::Rejected(_) if version == TclVersion::V8_4)
                || operand.validate("+", &context) == NativeExpressionCompilation::Accepted
        );
    }
}

#[test]
fn fixed_math_compiler_visits_required_children_before_argument_count_errors() {
    let profile = crate::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
    let operand = NativeCompiledExpressionOperand {
        argument: 0,
        error_context: NativeCompiledExpressionErrorContext::None,
    };
    for (text, script_count, message) in [
        (
            "future_function([set x extra bad])",
            0,
            "unknown math function \"future_function\"",
        ),
        (
            "abs([set x extra bad],2)",
            1,
            "too many arguments for math function",
        ),
        (
            "pow([set x extra bad])",
            1,
            "too few arguments for math function",
        ),
        (
            "abs(1,[set x extra bad])",
            0,
            "too many arguments for math function",
        ),
        ("abs(unknown(1),2)", 0, "unknown math function \"unknown\""),
    ] {
        let steps = operand.compiler_steps(text, &context, |name| match name {
            "abs" => NativeMathFunctionResolution::Known { arity: 1 },
            "pow" => NativeMathFunctionResolution::Known { arity: 2 },
            _ => NativeMathFunctionResolution::Absent,
        });
        assert_eq!(
            steps
                .iter()
                .filter(|step| matches!(step, NativeExpressionCompilerStep::Script(_)))
                .count(),
            script_count,
            "{text}"
        );
        assert!(
            matches!(steps.last(), Some(NativeExpressionCompilerStep::Failure(failure)) if failure.message.as_deref()==Some(message)),
            "{text}: {steps:?}"
        );
    }
    assert_eq!(
        operand.compiler_steps("abs(1)", &context, |_| {
            NativeMathFunctionResolution::Unknown
        }),
        vec![NativeExpressionCompilerStep::Unknown]
    );
}
