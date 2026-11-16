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

//! Expressions: preparing and evaluating expression source, the fixed math-function table, `rand`, and expression syntax errors.

use super::namespace_context::NamespaceExecutionContext;
use super::{NativeFixedMathCall, Vm, err, native_numeric_ops, native_operation_currency};
use crate::error::TclError;
use crate::expr::ExprEval;
use crate::value::Value;
use std::collections::HashMap;
use std::rc::Rc;
use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::expr::eval;

impl Vm {
    /// Reseed the `rand()` generator (`srand(n)`): mask to 31 bits, and nudge a
    /// 0 / `2^31-1` seed off the generator's two fixed points (Tcl's
    /// `srand`).
    pub(crate) fn rand_seed_set(&mut self, value: i64) {
        self.rand_seed = tcl_syntax::expr::rand::seed_from_wide(value);
    }

    /// Advance the Park–Miller minimal-standard generator one step and return a
    /// double in `(0, 1)` (`expr rand()`).
    ///
    /// The step and the scaling are the shared owner's
    /// (`tcl_syntax::expr::rand`), not a second transcription: dividing by
    /// `IM` where C multiplies by `1.0/IM` would diverge, because
    /// `1.0/2147483647` is not exactly representable, so the two would
    /// disagree by one ulp for a dense family of seeds — visible in the
    /// result string (`srand(251)` printing `0.0019644186841158285` instead
    /// of C's `0.001964418684115828`, and the 145th draw of `srand(1)`'s
    /// stream already differing).
    pub(crate) fn rand_next(&mut self) -> f64 {
        tcl_syntax::expr::rand::next_draw(&mut self.rand_seed)
    }

    /// Invoke the pre-TIP-232 fixed `expr` math-function entry selected by the
    /// registry. This intentionally bypasses normal command lookup: Tcl 8.4's
    /// `expr` has a C function table, not an overridable Tcl command namespace.
    pub(crate) fn invoke_fixed_math_builtin(
        &mut self,
        spec: &'static tcl_registry::CommandSpec,
        args: &[Value],
    ) -> Completion<Value> {
        let key = tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name);
        let Some(handler) = self.fixed_math.handlers.get(key).copied() else {
            return err(format!("unknown math function \"{}\"", spec.name));
        };
        self.set_invoked_name(key);
        self.native_invocation.builtin_identity = Some(spec.name.to_owned());
        handler(self, args)
    }

    /// Consume the actual fixed-function registration retained by an original
    /// expression compiler. A command-table registration cannot satisfy this
    /// independent function-table contract.
    pub(crate) fn invoke_native_fixed_math_call(
        &mut self,
        selected: &NativeFixedMathCall,
        args: &[Value],
    ) -> Completion<Value> {
        if selected.interpreter != self.native_interpreter_identity()
            || selected.profile_generation != self.profile_generation
            || selected.binding.arity != Some(args.len())
        {
            return self
                .refuse_host_command("original fixed math-function handler is unavailable".into());
        }
        self.set_invoked_name(
            tcl_syntax::naming::unroot_rooted_key(selected.spec.name).unwrap_or(selected.spec.name),
        );
        self.native_invocation.builtin_identity = Some(selected.spec.name.to_owned());
        (selected.handler)(self, args)
    }

    pub(super) fn retain_native_fixed_math_calls(
        &self,
        asm: &FunctionAsm,
    ) -> Option<HashMap<usize, NativeFixedMathCall>> {
        if !asm
            .instructions
            .iter()
            .any(|instruction| instruction.op == tcl_bytecode::Op::CALL_FUNC1)
        {
            return None;
        }
        if asm.validate_native_compilation_entry().is_err()
            || !self.native_math_table_prerequisite_matches(asm)
        {
            return None;
        }
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.actual_native_execution_profile(),
        );
        let mut calls = HashMap::new();
        for (index, instruction) in asm.instructions.iter().enumerate() {
            if instruction.op != tcl_bytecode::Op::CALL_FUNC1 {
                continue;
            }
            let binding = instruction.native_fixed_math_call.as_ref()?;
            let name = binding.name.try_utf8().ok()?;
            let spec = surface.builtin_math_function(name)?;
            if binding
                .registry_identity
                .as_deref()
                .and_then(tcl_registry::mathfunc::global_command_bare_name)
                != Some(name)
            {
                return None;
            }
            let handler = *self
                .fixed_math
                .handlers
                .get(tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name))?;
            calls.insert(
                index,
                NativeFixedMathCall {
                    binding: binding.clone(),
                    interpreter: self.native_interpreter_identity(),
                    profile_generation: self.profile_generation,
                    spec,
                    handler,
                },
            );
        }
        Some(calls)
    }

    /// The names `info functions` exposes on the selected Tcl release.
    ///
    /// Tcl 8.4 introspects its closed fixed table, while Tcl 8.5 and later
    /// enumerate the open command table (which also admits user functions).
    /// Both branches are selected through the registry surface.
    pub(crate) fn math_function_names(&self) -> Vec<String> {
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.native_execution_profile(),
        );
        if !surface.has_math_function_command_table() {
            return surface
                .builtin_math_function_names()
                .into_iter()
                .map(str::to_owned)
                .collect();
        }
        let mut names: Vec<String> = self
            .commands
            .iter()
            .filter(|(name, command)| self.builtin_command_visible_for_surface(name, command))
            .filter_map(|(name, _)| {
                let display = self.command_display_key_bytes(name);
                tcl_registry::mathfunc::global_command_bare_name(display.try_utf8().ok()?)
                    .map(str::to_owned)
            })
            .collect();
        names.sort_unstable();
        names
    }
}

impl Vm {
    /// Parse and evaluate a Tcl expression string against this VM.
    ///
    /// Proved native syntax failures use the shared exact presenter. Unsupported
    /// expression domains refuse execution through the neutral host channel.
    ///
    /// Parsing uses the emulated release's grammar; registry surface validation
    /// then checks the parsed operators and math functions. The resumable VM
    /// expression frame uses the same preparation and shared AST evaluator.
    pub fn eval_expr(&mut self, src: &str) -> Result<Value, TclError> {
        self.host_execution_depth += 1;
        let result = self
            .eval_expr_internal(src)
            .map(|completion| completion.result);
        self.host_execution_depth -= 1;
        if let Some(refusal) = self.take_host_boundary_refusal() {
            return Err(TclError::from_execution_failure(refusal));
        }
        result
    }

    /// Evaluate an expression while preserving neutral host refusal separately
    /// from genuine catch-visible Tcl completion. Earlier effects stay applied.
    ///
    /// # Errors
    /// Returns a host refusal when parsing or native presentation is unsupported.
    pub fn try_eval_expr(
        &mut self,
        src: &str,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = match self.eval_expr_internal(src) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    fn eval_expr_internal(&mut self, src: &str) -> Result<Completion<Value>, TclError> {
        if let Some(refusal) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        let node = self.prepare_expression(src)?;
        let mut ops = ExprEval::new(self);
        let value = eval(&node, &mut ops)?;
        ops.finish_expression_result(&value)
    }

    fn check_jim_expression_context(
        ops: &native_numeric_ops::OriginalNumericOps<'_>,
        original: &Value,
        context: &Rc<crate::value::NativeJimObjectContext>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        ops.with_vm_for_value(original, |vm| {
            let actual = vm.jim_object_context.borrow().as_ref().cloned();
            if !context.is_live()
                || actual
                    .as_ref()
                    .is_none_or(|actual| !Rc::ptr_eq(actual, context))
            {
                return Err(tcl_cmd_core::CmdError::from_execution_refusal(
                vm.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original Jim expression interpreter")))));
            }
            Ok(())
        })
    }

    fn jim_integer_expression_error(message: Vec<u8>) -> tcl_cmd_core::CmdError {
        tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
            string_result: None,
            message,
            error_code: tcl_cmd_core::CmdErrorCodeUpdate::Unchanged,
            error_info: None,
            error_line: None,
            primitive_getter: None,
        })
    }

    pub(crate) fn native_jim_wide_expression(
        &mut self,
        original: &Value,
    ) -> Result<i64, tcl_cmd_core::CmdError> {
        use native_numeric_ops::OriginalNumericOps;
        use tcl_syntax::scalar_getter::{
            NativeScalarCache, NativeScalarGetterKind, NativeScalarGetterValue,
        };
        let scope = native_operation_currency::NativeOperationScope::enter_native(self)
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        let ops = OriginalNumericOps::in_original_scope(self, &scope)?;
        let dialect = ops.dialect();
        if dialect.native_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into());
        }
        let context = ops.with_vm_for_value(original, |vm| Ok(vm.native_jim_object_context()?))?;
        let bound = original.bind_native_jim_context(&context);
        ops.current_for_value(original)?;
        bound?;
        // Context ownership is independent of the original header and scope.
        let current_context = || Self::check_jim_expression_context(&ops, original, &context);
        current_context()?;
        if original.native_jim_optimistic_integer() {
            let _ = ops.probe(original, NativeScalarGetterKind::Wide)?;
        }
        current_context()?;
        if let Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(value))) =
            original.native_scalar_cache()
        {
            return Ok(value);
        }
        let prepared = ops.with_vm_for_value(original, |vm| {
            Ok(vm.prepare_expression_value_in_original_scope(original, &scope))
        })?;
        current_context()?;
        let evaluated = match prepared {
            Ok(node) => {
                let restore = original.retain_expression_primary();
                let objects = original.native_jim_expression_objects().ok_or(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original Jim expression term backing",
                    ),
                )?;
                // The same captured primary is restored even when evaluation refuses.
                ops.with_vm_for_value(original, |vm| {
                    let reached = tcl_syntax::expr::eval(
                        &node,
                        &mut crate::expr::ExprEval::native(vm, objects, true),
                    );
                    if let Some(restore) = restore {
                        restore();
                    }
                    Ok(reached)
                })?
            }
            Err(error) => Err(error),
        };
        current_context()?;
        match evaluated {
            Ok(result) => {
                ops.current_for_value(&result)?;
                let bound = result.bind_native_jim_context(&context);
                ops.current_for_value(&result)?;
                bound?;
                current_context()?;
                context.publish_result(&result);
                current_context()?;
                ops.current_for_value(&result)?;
                let reached = ops.probe(&result, NativeScalarGetterKind::Wide);
                drop(result);
                current_context()?;
                if let Ok(NativeScalarGetterValue::Wide(value)) = reached? {
                    return Ok(value);
                }
            }
            Err(crate::error::TclError::Host(first)) => {
                return ops.with_vm_for_value(original, |vm| {
                    Err(tcl_cmd_core::CmdError::from_execution_refusal(
                        vm.retain_tcl_host_failure(first),
                    ))
                });
            }
            Err(error @ crate::error::TclError::Guest(_)) => {
                drop(error);
                current_context()?;
            }
        }
        let bytes = ops.string(
            original,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )?;
        current_context()?;
        let message = tcl_syntax::expr::native_objects::jim_integer_expression_message(&bytes);
        let error_result = Value::new_native_string_bytes(message.as_slice());
        context.publish_result(&error_result);
        drop(error_result);
        current_context()?;
        Err(Self::jim_integer_expression_error(message))
    }

    /// Explicit independent expression-source route for authored and oracle inputs.
    pub(crate) fn prepare_expression_value(
        &mut self,
        source: &Value,
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        self.prepare_expression_value_using(source, None)
    }

    /// Enter the actual source operation before any selected Host String worker.
    pub(crate) fn prepare_original_expression_value(
        &mut self,
        source: &Value,
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        if self.authored_numeric_context().is_some() {
            return self.prepare_expression_value(source);
        }
        let scope = native_operation_currency::NativeOperationScope::enter_native(self)
            .map_err(TclError::from_execution_failure)?;
        self.prepare_expression_value_in_original_scope(source, &scope)
    }

    fn original_expression_source_current(
        &mut self,
        source: &Value,
        scope: Option<&native_operation_currency::NativeOperationScope>,
    ) -> Result<(), TclError> {
        if let Some(scope) = scope {
            let ops = native_numeric_ops::OriginalNumericOps::in_original_scope_native(self, scope)
                .map_err(TclError::from_execution_failure)?;
            crate::expr::OriginalArithmeticStages::current_for_value(&ops, source)?;
        }
        Ok(())
    }

    pub(super) fn prepare_expression_value_in_original_scope(
        &mut self,
        source: &Value,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        self.prepare_expression_value_using(source, Some(scope))
    }

    fn materialize_expression_source(
        &mut self,
        source: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        scope: Option<&native_operation_currency::NativeOperationScope>,
    ) -> Result<Result<Rc<[u8]>, TclError>, TclError> {
        Ok(if let Some(scope) = scope {
            let ops = native_numeric_ops::OriginalNumericOps::in_original_scope_native(self, scope)
                .map_err(TclError::from_execution_failure)?;
            if ops.dialect().native_string_protocol() != Some(protocol) {
                return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into());
            }
            crate::expr::OriginalArithmeticStages::string(&ops, source)
        } else {
            source.native_string_bytes(protocol).map_err(|error| {
                tcl_syntax::value::ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
                .into()
            })
        })
    }

    fn check_expression_source_entry(
        &mut self,
        source: &Value,
        scope: Option<&native_operation_currency::NativeOperationScope>,
    ) -> Result<(), TclError> {
        self.original_expression_source_current(source, scope)?;
        if let Some(refusal) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        Ok(())
    }

    fn prepare_expression_value_using(
        &mut self,
        source: &Value,
        scope: Option<&native_operation_currency::NativeOperationScope>,
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        self.check_expression_source_entry(source, scope)?;
        self.claim_number_grammar();
        let dialect = self.native_invocation_dialect();
        let profile = self.expression_source_profile().cache_key();
        let protocol = self.actual_native_invocation_dialect().native_string_materialization(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation))
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable("native expression source materialization"))?.protocol();
        let source_info = if protocol.is_jim084() {
            let reached = (|| {
                let context = self.native_jim_object_context()?;
                source.bind_native_jim_context(&context)?;
                source.pin_native_jim_source_info(&context)
            })();
            self.original_expression_source_current(source, scope)?;
            Some(reached?)
        } else {
            None
        };
        if protocol.is_jim084()
            && source.expression_rejected(dialect, &profile, &self.native_compiler_policy())
        {
            let context = self.native_jim_object_context()?;
            let options = self.current_jim_options_for_exit_code(Code::Error);
            self.original_expression_source_current(source, scope)?;
            return Err(TclError::from_completion(Completion::new(
                Code::Error,
                context.result_object(),
                options.map_err(|error| crate::command::completion_option_failure(self, error))?,
            )));
        }
        let materialized = self.materialize_expression_source(source, protocol, scope)?;
        // Even a failed String worker must be checked before parser or cache work.
        self.original_expression_source_current(source, scope)?;
        let materialized = materialized?;
        let preparation = self
            .expression_parse_context()
            .native_syntax
            .source_cache_preparation()
            .ok_or_else(|| {
                self.refuse_expression_bytes(
                    &materialized,
                    tcl_runtime_api::NativeExpressionRefusal::UnsupportedGrammar,
                )
            })?;
        let policy = self.native_compiler_policy();
        let bytes = if scope.is_some() {
            source.expression_source_checked(
                &crate::value::ExpressionSourceInput {
                    dialect,
                    profile: &profile,
                    protocol,
                    policy: &policy,
                    preparation,
                },
                &materialized,
                &mut || self.original_expression_source_current(source, scope),
            )
        } else {
            source
                .expression_source(dialect, &profile, protocol, &policy, preparation)
                .map_err(TclError::from)
        };
        self.original_expression_source_current(source, scope)?;
        let bytes = bytes?;
        // Fixed-function preparation retains its installed table and Jim tree guards.
        if !tcl_registry::runtime_expr_validation::requires_fixed_function_preparation(
            &self.expression_parse_context(),
        ) && let Some(node) =
            source.cached_expression(dialect, &profile, &self.native_compiler_policy())
        {
            return Ok(node);
        }
        let node = if preparation
            == tcl_syntax::expr::parser::ExpressionSourceCachePreparation::JimOutcome
        {
            self.prepare_fixed_function_expression_bytes_on(
                &bytes,
                &self.expression_parse_context(),
                tcl_registry::native_expression_error::ExpressionErrorPublication::Eval,
                Some(source),
                source_info.as_ref(),
            )
        } else {
            self.prepare_expression_bytes_in(
                &bytes,
                tcl_registry::native_expression_error::ExpressionErrorPublication::Eval,
            )
        };
        self.original_expression_source_current(source, scope)?;
        let node = node?;
        let policy = self.native_compiler_policy();
        if scope.is_some() {
            source.cache_expression_checked(dialect, &profile, &policy, &node, &mut || {
                self.original_expression_source_current(source, scope)
            })?;
        } else {
            source.cache_expression(dialect, &profile, &policy, &node);
        }
        self.original_expression_source_current(source, scope)?;
        Ok(node)
    }

    /// Parse the exact bytes supplied by a native expression object.
    pub(crate) fn prepare_expression_bytes(
        &mut self,
        source: &[u8],
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        self.prepare_expression_bytes_in(
            source,
            tcl_registry::native_expression_error::ExpressionErrorPublication::Direct,
        )
    }

    fn prepare_expression_bytes_in(
        &mut self,
        source: &[u8],
        publication: tcl_registry::native_expression_error::ExpressionErrorPublication,
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        use tcl_syntax::expr::parser::CheckedExprParse;
        self.claim_number_grammar();
        if let Some(prepared) = self.prepare_authored_expression_bytes(source) {
            return prepared;
        }
        let context = self.expression_parse_context();
        if tcl_registry::runtime_expr_validation::requires_fixed_function_preparation(&context) {
            return self.prepare_fixed_function_expression_bytes(source, &context, publication);
        }
        let node =
            match tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(source, &context)
            {
                CheckedExprParse::Parsed(node) => node,
                CheckedExprParse::Unsupported(_) => {
                    return Err(self.refuse_expression_bytes(
                        source,
                        tcl_runtime_api::NativeExpressionRefusal::UnsupportedGrammar,
                    ));
                }
                CheckedExprParse::ProvedSyntaxFailure(failure) => {
                    let diagnostic = failure
                        .native_diagnostic_bytes_with_context(source, &context)
                        .or_else(|| {
                            std::str::from_utf8(source)
                                .ok()
                                .and_then(|source| {
                                    self.fixed_bareword_diagnostic(failure, source, &context)
                                })
                                .map(|diagnostic| {
                                    tcl_syntax::expr::parser::NativeExprSyntaxDiagnostic {
                                        message: diagnostic.message.into_bytes(),
                                        error_code: diagnostic.error_code.map(String::into_bytes),
                                    }
                                })
                        });
                    let Some(diagnostic) = diagnostic else {
                        return Err(self.refuse_expression_bytes(
                            source,
                            tcl_runtime_api::NativeExpressionRefusal::UnpresentedSyntaxFailure,
                        ));
                    };
                    self.seed_parsing_expression_frame_bytes(&diagnostic.message, source);
                    return Err(self.expression_syntax_error(
                        diagnostic.message,
                        diagnostic.error_code.as_deref(),
                        context.native_syntax,
                        publication,
                    ));
                }
            };
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.native_execution_profile(),
        );
        if surface.validate(&node).is_err() {
            return Err(self.refuse_expression_bytes(
                source,
                tcl_runtime_api::NativeExpressionRefusal::UnsupportedGrammar,
            ));
        }
        self.validate_expression_function_policy(source, &node)?;
        Ok(node)
    }

    fn validate_expression_function_policy<Text>(
        &mut self,
        source: &[u8],
        node: &tcl_syntax::expr::ExprNode<Text>,
    ) -> Result<(), TclError> {
        if !tcl_registry::native_expression_program::expression_tree_is_call_free(node)
            && tcl_registry::native_expression_program::expression_function_dispatch(
                self.expression_evaluation_policy().as_ref(),
                self.actual_native_invocation_dialect(),
            )
            .is_none()
        {
            return Err(self.refuse_expression_bytes(
                source,
                tcl_runtime_api::NativeExpressionRefusal::FunctionDispatchPolicyUnavailable,
            ));
        }
        Ok(())
    }

    fn prepare_fixed_function_expression_bytes(
        &mut self,
        source: &[u8],
        context: &tcl_syntax::expr::parser::ExprParseContext,
        publication: tcl_registry::native_expression_error::ExpressionErrorPublication,
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        self.prepare_fixed_function_expression_bytes_on(source, context, publication, None, None)
    }

    fn prepare_fixed_function_expression_bytes_on(
        &mut self,
        source: &[u8],
        context: &tcl_syntax::expr::parser::ExprParseContext,
        publication: tcl_registry::native_expression_error::ExpressionErrorPublication,
        original: Option<&Value>,
        source_info: Option<&crate::value::NativeJimSourceInfo>,
    ) -> Result<tcl_syntax::expr::NativeExprNode, TclError> {
        use tcl_registry::native_compilation::NativeMathFunctionResolution as Prepared;
        use tcl_registry::runtime_expr_validation::{
            RuntimeExpressionPreparation, prepare_fixed_function_expression_bytes_with_preparation,
        };
        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution as Installed;
        if let Some(diagnostic) = tcl_syntax::expr::parser::tcl84_parser_diagnostic(source, context)
        {
            self.seed_parsing_expression_frame_bytes(&diagnostic.message, source);
            return Err(self.expression_syntax_error(
                diagnostic.message,
                diagnostic.error_code.as_deref(),
                context.native_syntax,
                publication,
            ));
        }
        let table = self.native_math_function_table();
        let prepared =
            prepare_fixed_function_expression_bytes_with_preparation(source, context, |name| {
                match table.as_ref().map(|table| table.lookup(name)) {
                    Some(Installed::Present(binding)) => binding
                        .arity
                        .map_or(Prepared::Unknown, |arity| Prepared::Known { arity }),
                    Some(Installed::Absent) => Prepared::Absent,
                    Some(Installed::Unknown) | None => Prepared::Unknown,
                }
            });
        if let Some(original) = original
            && prepared.jim.as_ref().is_some_and(|receipt| {
                receipt.action
                    == tcl_syntax::expr::native_objects::JimExpressionCacheAction::Rejected
            })
        {
            original.reject_expression(
                self.native_invocation_dialect(),
                &self.expression_source_profile().cache_key(),
                &self.native_compiler_policy(),
            );
        }
        match prepared.tree {
            RuntimeExpressionPreparation::Parsed(node) => {
                if let (Some(original), Some(info), Some(receipt)) =
                    (original, source_info, prepared.jim.as_ref())
                    && original
                        .cached_expression(
                            self.native_invocation_dialect(),
                            &self.expression_source_profile().cache_key(),
                            &self.native_compiler_policy(),
                        )
                        .is_none()
                {
                    original.install_jim_expression(&crate::value::JimExpressionInstall {
                        dialect: self.native_invocation_dialect(),
                        profile: self.expression_source_profile().cache_key(),
                        policy: self.native_compiler_policy(),
                        node: &node,
                        source,
                        preparation: receipt,
                        info,
                    })?;
                }
                Ok(node)
            }
            RuntimeExpressionPreparation::Rejected(diagnostic) => {
                self.seed_parsing_expression_frame_bytes(&diagnostic.message, source);
                Err(self.expression_syntax_error(
                    diagnostic.message,
                    diagnostic.error_code.as_deref(),
                    context.native_syntax,
                    publication,
                ))
            }
            RuntimeExpressionPreparation::Unknown => Err(self.refuse_expression_bytes(
                source,
                tcl_runtime_api::NativeExpressionRefusal::UnsupportedGrammar,
            )),
        }
    }

    fn expression_syntax_error(
        &mut self,
        message: impl AsRef<[u8]>,
        code: Option<&[u8]>,
        syntax: tcl_syntax::expr::parser::NativeExprSyntax,
        publication: tcl_registry::native_expression_error::ExpressionErrorPublication,
    ) -> TclError {
        let Some(stage) =
            tcl_registry::native_expression_error::NativeExpressionErrorStage::syntax(syntax, code)
        else {
            return tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native expression syntax error state",
            )
            .into();
        };
        let update = match stage.code_update(publication).resolve(|| {
            Err(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression syntax error state",
                ),
            )
        }) {
            Ok(update) => update,
            Err(error) => return tcl_syntax::value::ValueError::from(error).into(),
        };
        let options = if self.uses_jim_error_stack() {
            match self.current_jim_options_for_exit_code(Code::Error) {
                Ok(options) => options,
                Err(error) => return crate::command::completion_option_failure(self, error),
            }
        } else {
            let set = matches!(update, tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(_));
            let code = self.apply_primitive_error_code(update);
            // C8.4's parser sets a range error's code with `Tcl_SetErrorCode`,
            // so the command log keeps it rather than defaulting to NONE.
            if set && self.uses_c84_global_error_info() && code.to_str().as_ref() != "NONE" {
                self.restore_guest_error_code(code.clone());
            }
            crate::command::options_dict(Code::Error, 0, &[("-errorcode", code)])
        };
        TclError::from_completion(Completion::new_error_metadata(
            Code::Error,
            Value::from_string_bytes(message.as_ref()),
            options,
        ))
    }

    /// Evaluate an original byte expression with a full guest completion.
    ///
    /// # Errors
    /// Returns a typed host refusal when the selected provider is unavailable.
    pub fn try_eval_expr_bytes(
        &mut self,
        source: &[u8],
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = self.expression_bytes_completion(source);
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    /// Evaluate an expression and its Boolean result in one original operation.
    ///
    /// Truth is present only for a successful guest completion. Abrupt guest
    /// completions retain their result and return options.
    ///
    /// # Errors
    /// Returns the first typed host refusal independently of guest completion.
    pub fn try_eval_expr_bool_bytes(
        &mut self,
        source: &[u8],
    ) -> Result<(Completion<Value>, Option<bool>), tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let result = (|| {
            let scope = native_operation_currency::NativeOperationScope::enter_native(self)?;
            let completion = self.expression_bytes_completion(source);
            let (completion, truth) = if completion.code == Code::Ok {
                match crate::expr::expression_boolean_for_vm(
                    self,
                    &completion.result,
                    tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi,
                ) {
                    Ok(truth) => (completion, Some(truth)),
                    Err(error) => (crate::command::completion_from_tcl_error(self, error), None),
                }
            } else {
                (completion, None)
            };
            scope.check_native(self)?;
            Ok((completion, truth))
        })();
        self.host_execution_depth -= 1;
        match result {
            Ok((completion, truth)) => self
                .finish_host_execution(completion)
                .map(|completion| (completion, truth)),
            Err(error) => Err(self.take_host_boundary_refusal().unwrap_or(error)),
        }
    }

    fn expression_bytes_completion(&mut self, source: &[u8]) -> Completion<Value> {
        match self.prepare_expression_bytes(source).and_then(|node| {
            let mut ops = ExprEval::new(self);
            let value = eval(&node, &mut ops)?;
            ops.finish_expression_result(&value)
        }) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        }
    }

    /// Evaluate native expression bytes without projecting their source to Unicode.
    ///
    /// # Errors
    /// Preserves host refusals and all abrupt guest completions.
    pub fn eval_expr_bytes(&mut self, source: &[u8]) -> Result<Value, TclError> {
        let completion = self
            .try_eval_expr_bytes(source)
            .map_err(TclError::from_execution_failure)?;
        if completion.code == Code::Ok {
            Ok(completion.result)
        } else {
            Err(TclError::from_completion(completion))
        }
    }

    /// Parse and validate once for both blocking and resumable expression engines.
    pub(crate) fn prepare_expression(
        &mut self,
        src: &str,
    ) -> Result<tcl_syntax::expr::ExprNode, TclError> {
        use tcl_syntax::expr::parser::CheckedExprParse;
        if let Some(refusal) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        self.claim_number_grammar();
        if let Some(prepared) = self.prepare_authored_expression_bytes(src.as_bytes()) {
            return prepared.map(|tree| {
                tree.map_text(|bytes| String::from_utf8(bytes).expect("authored UTF-8 source"))
            });
        }
        let context = self.expression_parse_context();
        if tcl_registry::runtime_expr_validation::requires_fixed_function_preparation(&context) {
            return self.prepare_fixed_function_expression(src, &context);
        }
        let node = match tcl_syntax::expr::parser::parse_expr_checked_with_context(src, &context) {
            CheckedExprParse::Parsed(node) => node,
            CheckedExprParse::Unsupported(_) => {
                return Err(self.refuse_expression(
                    src,
                    tcl_runtime_api::NativeExpressionRefusal::UnsupportedGrammar,
                ));
            }
            CheckedExprParse::ProvedSyntaxFailure(failure) => {
                let diagnostic = failure
                    .native_diagnostic_with_context(src, &context)
                    .or_else(|| self.fixed_bareword_diagnostic(failure, src, &context));
                let Some(diagnostic) = diagnostic else {
                    return Err(self.refuse_expression(
                        src,
                        tcl_runtime_api::NativeExpressionRefusal::UnpresentedSyntaxFailure,
                    ));
                };
                self.seed_parsing_expression_frame(src, &diagnostic.message);
                return Err(self.expression_syntax_error(
                    diagnostic.message,
                    diagnostic.error_code.as_deref().map(str::as_bytes),
                    context.native_syntax,
                    tcl_registry::native_expression_error::ExpressionErrorPublication::Direct,
                ));
            }
        };
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.native_execution_profile(),
        );
        if let Err(error) = surface.validate(&node) {
            let message = error.message(src);
            self.seed_parsing_expression_frame(src, &message);
            return Err(if self.runtime_version.tags_expression_parse_errors() {
                TclError::with_error_code(message, error.error_code())
            } else {
                TclError::new(message)
            });
        }
        self.validate_expression_function_policy(src.as_bytes(), &node)?;
        Ok(node)
    }

    pub(super) fn expression_parse_context(&self) -> tcl_syntax::expr::parser::ExprParseContext {
        use tcl_syntax::expr::parser::ExprParseContext;
        if self.active_native_profile.is_none()
            && let Some(context) = self
                .logical_providers
                .expression_parse
                .and_then(|provider| {
                    tcl_registry::InvocationDialect::of_profile(self.dialect_profile())
                        .logical_expression_parse_context(
                            provider,
                            self.expression_source_profile(),
                        )
                })
        {
            return context;
        }
        let mut context = ExprParseContext::for_profile(self.expression_source_profile());
        context.native_syntax =
            ExprParseContext::for_profile(self.native_execution_profile()).native_syntax;
        context
    }

    fn prepare_fixed_function_expression(
        &mut self,
        source: &str,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> Result<tcl_syntax::expr::ExprNode, TclError> {
        use tcl_registry::native_compilation::NativeMathFunctionResolution as Prepared;
        use tcl_registry::runtime_expr_validation::{
            RuntimeExpressionPreparation, prepare_fixed_function_expression,
        };
        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution as Installed;
        if let Some(diagnostic) =
            tcl_syntax::expr::parser::tcl84_parser_diagnostic(source.as_bytes(), context)
        {
            self.seed_parsing_expression_frame_bytes(&diagnostic.message, source.as_bytes());
            return Err(self.expression_syntax_error(
                diagnostic.message,
                diagnostic.error_code.as_deref(),
                context.native_syntax,
                tcl_registry::native_expression_error::ExpressionErrorPublication::Direct,
            ));
        }
        let table = self.native_math_function_table();
        let prepared = prepare_fixed_function_expression(source, context, |name| {
            match table.as_ref().map(|table| table.lookup(name)) {
                Some(Installed::Present(binding)) => binding
                    .arity
                    .map_or(Prepared::Unknown, |arity| Prepared::Known { arity }),
                Some(Installed::Absent) => Prepared::Absent,
                Some(Installed::Unknown) | None => Prepared::Unknown,
            }
        });
        match prepared {
            RuntimeExpressionPreparation::Parsed(node) => Ok(node),
            RuntimeExpressionPreparation::Rejected(diagnostic) => {
                self.seed_parsing_expression_frame(source, &diagnostic.message);
                Err(self.expression_syntax_error(
                    diagnostic.message,
                    diagnostic.error_code.as_deref().map(str::as_bytes),
                    context.native_syntax,
                    tcl_registry::native_expression_error::ExpressionErrorPublication::Direct,
                ))
            }
            RuntimeExpressionPreparation::Unknown => Err(self.refuse_expression(
                source,
                tcl_runtime_api::NativeExpressionRefusal::UnsupportedGrammar,
            )),
        }
    }

    /// The fixed VM math table is independent of command renames. Query the
    /// same selector used to execute
    /// functions; neither written spelling nor catalogue presence alone proves
    /// that a selected fixed handler is installed.
    fn fixed_bareword_diagnostic(
        &self,
        failure: tcl_syntax::expr::parser::ExprSyntaxFailure,
        source: &str,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> Option<tcl_syntax::expr::parser::NativeExprSyntaxDiagnostic> {
        use tcl_registry::expr_surface::{MathFunctionCallTarget, RuntimeExprSurface};
        use tcl_syntax::expr::parser::NativeFunctionNameResolution;
        let name = failure.bareword_name(source, context)?;
        let surface = RuntimeExprSurface::for_profile(self.native_execution_profile());
        let resolution = match surface.math_function_call_target(&name) {
            MathFunctionCallTarget::FixedBuiltin(spec)
                if self.fixed_math.handlers.contains_key(
                    tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name),
                ) =>
            {
                NativeFunctionNameResolution::Function(&name)
            }
            MathFunctionCallTarget::FixedTableMiss
                if !self.fixed_math.handlers.contains_key(
                    tcl_syntax::naming::unroot_rooted_key(
                        &tcl_registry::mathfunc::qualified_name(&name),
                    )?,
                ) =>
            {
                NativeFunctionNameResolution::NonFunction(&name)
            }
            MathFunctionCallTarget::FixedBuiltin(_)
            | MathFunctionCallTarget::FixedTableMiss
            | MathFunctionCallTarget::CommandTable => NativeFunctionNameResolution::Unknown,
        };
        failure.native_diagnostic_with_bareword_resolution(source, context, resolution)
    }

    pub(super) fn expression_source_profile(&self) -> &'static tcl_dialect::DialectProfile {
        if self.source_profile().is_fallback() {
            self.native_execution_profile()
        } else {
            self.source_profile()
        }
    }

    pub(super) fn native_refusal_namespace(&self) -> (std::sync::Arc<str>, Option<u64>) {
        let namespace = self
            .resolution_stacks
            .ns_stack
            .last()
            .and_then(Option::as_ref)
            .map_or_else(
                || "<unavailable namespace>".to_owned(),
                |name| tcl_syntax::native_string::resident_name_label(name.as_bytes()),
            );
        let token = self
            .resolution_stacks
            .ns_id_stack
            .last()
            .and_then(NamespaceExecutionContext::resolved_token)
            .map(|token| u64::from(token.0));
        (namespace.into(), token)
    }

    /// Add C's `(parsing expression "…")` `errorInfo` frame for a rejected
    /// expression (`tclCompExpr.c:1461-1466`).
    fn seed_parsing_expression_frame(&mut self, src: &str, message: &str) {
        self.seed_parsing_expression_frame_bytes(message.as_bytes(), src.as_bytes());
    }

    /// C8.4's expression parser writes no frame: the failing command's own
    /// log starts the trace (`while executing`).
    fn seed_parsing_expression_frame_bytes(&mut self, message: &[u8], source: &[u8]) {
        if self.runtime_version() != tcl_dialect::TclVersion::V8_4 {
            let frame = tcl_syntax::expr::ExprSyntaxError::error_info_frame_bytes(source);
            self.seed_error_info_frame(message, frame);
        }
    }
}
