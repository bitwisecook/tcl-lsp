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

//! Expression and arithmetic semantics.
//!
//! The numeric/comparison/unary logic is single-sourced here and used by *both*
//! the inline arithmetic opcodes (`ADD`/`LT`/`UMINUS`/…) and `EXPR_STK` (via the
//! [`ExprEval`] adapter over [`tcl_syntax::expr::ExprOps`]), so the VM shares
//! the expr tower with the const-folder and the runtime.
// Integer→double coercion in the shared arithmetic is intentional (Tcl `expr`
// promotes to double), so the precision loss is the defined behaviour.
#![allow(clippy::cast_precision_loss)]

mod native_boolean_truth;
mod original_list_membership;
mod original_numeric_context;
pub(crate) use native_boolean_truth::{
    boolean_for_vm, expression_boolean_for_vm, logical_right_for_vm, logical_value_for_vm,
    normalize_numeric_instruction_for_vm, normalize_result_for_vm, unary_for_vm,
};
pub(crate) use original_numeric_context::{
    OriginalArithmeticStages, OriginalNumericCommandContext, numeric_command_guest_error,
    original_numeric_command_ops, with_original_numeric_command, with_original_numeric_context,
};

use std::cmp::Ordering;

use tcl_runtime_api::Code;
use tcl_syntax::expr::{BinOp, ExprOps, NumericCompare, UnaryOp};
use tcl_syntax::number::{self, Number};

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

mod arithmetic;
mod comparison;
mod context_operators;

#[cfg(test)]
use arithmetic::boolean_context_err;
use arithmetic::{Num, to_num, unary_operand_err};
pub(crate) use arithmetic::{big_to_f64, big_value, int_value, value_as_bigint_with_stages};
pub use comparison::compare;
#[cfg(test)]
use comparison::compare_string_units;
use comparison::{compare_string_units_using, compare_with_numeric_using};
#[cfg(test)]
pub(crate) use context_operators::boolean_operand_error;
pub(crate) use context_operators::{
    arith_in, authored_boolean, compare_in, native_boolean_result, numeric_value_in, unary_in,
};
use context_operators::{compare_values_numeric_checked_in, native_literal};

/// Numeric authority for one reached VM activation. The simulation provider
/// is explicit and carries no authentic native getter or cache attestation.
#[derive(Clone, Copy)]
pub(crate) struct NumericContext<'a> {
    pub(crate) environment: Option<&'a dyn tcl_platform::NumericEnvironment>,
    pub(crate) original: Option<&'a dyn OriginalArithmeticStages>,
    pub(crate) dialect: tcl_native_model::InvocationDialect,
    pub(crate) simulation:
        Option<tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation>,
}

impl From<tcl_native_model::InvocationDialect> for NumericContext<'_> {
    fn from(dialect: tcl_native_model::InvocationDialect) -> Self {
        Self {
            dialect,
            simulation: None,
            environment: None,
            original: None,
        }
    }
}

impl std::ops::Deref for NumericContext<'_> {
    type Target = tcl_native_model::InvocationDialect;
    fn deref(&self) -> &Self::Target {
        &self.dialect
    }
}

/// Apply a unary operator.
pub fn unary(op: UnaryOp, v: &Value) -> Result<Value, TclError> {
    use UnaryOp::{BitNot, Neg, Not, Pos, WordNot};
    match op {
        // `to_num` promotes an out-of-wide literal to `Big` (then `Huge`), so
        // `-2^63` — and any magnitude beyond — negates exactly: `int_value` /
        // `big_value` narrow back to a wide when the result re-fits. A
        // non-numeric operand is the C operand-type error (`as operand of "-"`).
        Neg => match to_num(v) {
            // `-i64::MIN` does not fit a wide — promote through the i128
            // tier exactly (a *computed* MIN reaches this arm as `Int`).
            Ok(Num::Int(n)) => Ok(int_value(-i128::from(n))),
            Ok(Num::Big(b)) => Ok(int_value(b.wrapping_neg())),
            // An integer past `i128` negates exactly in arbitrary precision,
            // never as a lossy float.
            Ok(Num::Huge(b)) => Ok(big_value(&-b)),
            Ok(Num::Dbl(f)) => Ok(Value::double(-f)),
            Err(_) => Err(unary_operand_err(v, "-")),
        },
        Pos => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(n)),
            Ok(Num::Big(b)) => Ok(int_value(b)),
            Ok(Num::Huge(b)) => Ok(big_value(&b)),
            Ok(Num::Dbl(f)) => Ok(Value::double(f)),
            Err(_) => Err(unary_operand_err(v, "+")),
        },
        // `~` needs an integer; a double is a "floating-point value" operand
        // error, a non-number a "non-numeric string" one.
        BitNot => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(!n)),
            Ok(Num::Big(b)) => Ok(int_value(!b)),
            // An integer past `i128` bit-complements exactly (`~x == -x-1`).
            Ok(Num::Huge(b)) => Ok(big_value(&!b)),
            Ok(Num::Dbl(_)) | Err(_) => Err(unary_operand_err(v, "~")),
        },
        // `!` accepts any boolean (incl. numbers and the boolean words); a NaN or
        // non-numeric non-boolean is the operand error (not "expected boolean").
        // The iRules dialect's `not` is the word spelling of the same operator
        // (`Op::IRULE_WORD_NOT`), so it shares the coercion and only differs in
        // the spelling it reports for a bad operand.
        Not | WordNot => {
            let spelling = if matches!(op, WordNot) { "not" } else { "!" };
            if matches!(
                v.try_to_str()
                    .ok()
                    .and_then(|text| number::parse_whole(text.trim())),
                Some(Number::Nan { .. })
            ) {
                return Err(unary_operand_err(v, spelling));
            }
            match v.as_bool() {
                Ok(b) => Ok(Value::bool(!b)),
                Err(_) => Err(unary_operand_err(v, spelling)),
            }
        }
    }
}

/// Apply an iRules-dialect binary operator to two already-evaluated operands
/// (`left` is the subject, `right` the needle/pattern/right-hand side — the
/// order `Op::from_binop` codegen pushes them in).
///
/// One home for the dialect operator semantics, shared by the `IRULE_*` opcodes
/// and available to any tree-walking consumer that gains dialect support (the
/// shared walker routes them to `ExprOps::binary_other`). The string tests reuse
/// the same helpers the equivalent core commands do — `string match`'s glob
/// matcher, the `regexp` ARE engine, and this module's string comparison — so
/// the dialect operators cannot drift from `[string match]` / `[regexp]` /
/// `[string equal]`.
pub(crate) fn irule_binary(
    provider: Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider>,
    op: BinOp,
    left: &Value,
    right: &Value,
) -> Result<Value, TclError> {
    use BinOp::{
        Contains, EndsWith, Matches, MatchesGlob, MatchesRegex, StartsWith, StrEquals, WordAnd,
        WordOr,
    };
    // `and`/`or` are boolean tests, not string ones — they never render an
    // operand.
    match op {
        WordAnd => return word_and(left, right).map(Value::bool),
        WordOr => return word_or(left, right).map(Value::bool),
        _ => {}
    }
    let predicate = if matches!(op, Contains | StartsWith | EndsWith | Matches | MatchesGlob) {
        Some(provider.and_then(|provider| provider.predicate(op)).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "authored F5 string predicate policy",
            ),
        )?)
    } else {
        None
    };
    let (subject, operand) = (left.try_to_str()?, right.try_to_str()?);
    let truth = match op {
        StrEquals => compare(BinOp::StrEq, left, right)?,
        Contains | StartsWith | EndsWith | Matches | MatchesGlob => predicate
            .expect("selected F5 string predicate")
            .evaluate(&subject, &operand),
        // iRules embeds Tcl 8.4, whose ARE has no `\z`.
        MatchesRegex => crate::cmd_regexp::regexp_matches(
            &operand,
            &subject,
            false,
            tcl_dialect::TclVersion::V8_4,
        )
        .map_err(TclError::new)?,
        _ => return Err(TclError::new("unsupported operator")),
    };
    Ok(Value::bool(truth))
}

/// The iRules `and` word operator over two already-evaluated operands.
///
/// The shared tree-walk coerces the right operand only when the left does not
/// already decide the result; these reproduce that with both operands in hand
/// (as the opcode form has them), so a non-boolean right operand is an error in
/// exactly the cases the walker treats as one.
fn word_and(left: &Value, right: &Value) -> Result<bool, TclError> {
    if left.as_bool()? {
        right.as_bool()
    } else {
        Ok(false)
    }
}

/// The iRules `or` word operator — see [`word_and`].
fn word_or(left: &Value, right: &Value) -> Result<bool, TclError> {
    if left.as_bool()? {
        Ok(true)
    } else {
        right.as_bool()
    }
}

/// A math-function command's error completion as an expr error, **keeping the
/// `-errorcode` it published**. Rebuilding the error from the message alone
/// on the dynamic `expr $e` path would report `ARITH DOMAIN` when
/// compiled as `expr {…}` but `NONE` when evaluated from a variable.
/// An [`ExprOps`] adapter that evaluates an expression AST against the VM
/// (resolving `$var` / `[cmd]` / math functions through it), reusing the shared
/// `tcl-syntax` expr walker.
pub struct ExprEval<'a> {
    /// The VM the expression resolves variables/commands against.
    pub vm: &'a mut Vm,
    /// Return options retained by this in-progress expression activation.
    /// A finished result is paired with these options by `finish`.
    options: Value,
    jim: Option<std::rc::Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>>,
    safe: bool,
}

impl<'a> ExprEval<'a> {
    pub(crate) fn new(vm: &'a mut Vm) -> Self {
        Self {
            vm,
            options: Value::empty(),
            jim: None,
            safe: false,
        }
    }

    pub(crate) fn native(
        vm: &'a mut Vm,
        objects: std::rc::Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>,
        safe: bool,
    ) -> Self {
        Self {
            vm,
            options: Value::empty(),
            jim: Some(objects),
            safe,
        }
    }

    pub(crate) fn with_jim_objects(
        vm: &'a mut Vm,
        objects: Option<std::rc::Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>>,
    ) -> Self {
        Self {
            vm,
            options: Value::empty(),
            jim: objects,
            safe: false,
        }
    }

    fn original_term(
        &self,
        start: u32,
        end: Option<u32>,
    ) -> Result<Option<(tcl_lexer::ExprTermKind, Value)>, TclError> {
        let Some(objects) = &self.jim else {
            return Ok(None);
        };
        let (term, value) = objects.at(start, end).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim expression leaf extent",
            ),
        )?;
        Ok(Some((term.kind, value.clone())))
    }

    fn accept_completion(
        &mut self,
        completion: tcl_runtime_api::Completion<Value>,
    ) -> Result<Value, TclError> {
        if let Some(refusal) = self.vm.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        if completion.code != Code::Ok {
            return Err(TclError::from_completion(completion));
        }
        self.options = completion.options;
        // The operand keeps its own reference across later substitutions,
        // which may reset the interpreter result.
        Ok(completion.result.into_native_reference())
    }

    /// An error leaving an expression's command substitution
    /// ([`Vm::settle_expression_substitution_error`]).
    fn settle_substitution_error_logging(
        &mut self,
        completion: &tcl_runtime_api::Completion<Value>,
    ) {
        if completion.code == Code::Error {
            self.vm.settle_expression_substitution_error();
        }
    }

    pub(crate) fn finish_expression_result(
        self,
        result: &Value,
    ) -> Result<tcl_runtime_api::Completion<Value>, TclError> {
        let result = normalize_result_for_vm(self.vm, result,
            tcl_native_model::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi)?;
        Ok(self.finish(result))
    }

    pub(crate) fn finish(self, result: Value) -> tcl_runtime_api::Completion<Value> {
        tcl_runtime_api::Completion::new(Code::Ok, result, self.options)
    }
}

impl ExprOps for ExprEval<'_> {
    type Value = Value;
    type Error = TclError;

    fn literal_bytes_at(&mut self, text: &[u8], start: u32, end: u32) -> Result<Value, TclError> {
        if let Some((_, value)) = self.original_term(start, Some(end))? {
            return Ok(value);
        }
        self.literal_bytes(text)
    }

    fn string_bytes_at(
        &mut self,
        text: &[u8],
        substitutes: bool,
        start: u32,
        end: u32,
    ) -> Result<Value, TclError> {
        if let Some((kind, value)) = self.original_term(start, Some(end))? {
            if kind == tcl_lexer::ExprTermKind::String {
                return Ok(value);
            }
            if self.safe {
                return Err(TclError::new(b""));
            }
        }
        self.string_bytes(text, substitutes)
    }

    fn command_bytes_at(&mut self, text: &[u8], start: u32, end: u32) -> Result<Value, TclError> {
        if let Some((_, original)) = self.original_term(start, Some(end))? {
            if self.safe {
                return Err(TclError::new(b""));
            }
            let completion = self
                .vm
                .eval_value_at_level(self.vm.current_level(), &original);
            return self.accept_completion(completion);
        }
        self.command_bytes(text)
    }

    fn literal_bytes(&mut self, text: &[u8]) -> Result<Value, TclError> {
        let text = std::str::from_utf8(text).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native numeric token grammar",
            )
        })?;
        self.literal(text)
    }

    fn string_bytes(&mut self, inner: &[u8], substitutes: bool) -> Result<Value, TclError> {
        if !substitutes {
            return Ok(Value::from_string_bytes(
                tcl_syntax::backslash::collapse_brace_continuations_for(
                    inner,
                    self.vm.native_invocation_dialect().word_values.brace,
                )
                .as_ref(),
            ));
        }
        let policy = self.vm.expression_quote_control().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native expression quote settlement",
            ),
        )?;
        let completion = crate::subst::subst_expression_string(
            self.vm,
            inner,
            crate::subst::SubstitutionControl::Expression(policy),
        )?;
        self.accept_completion(completion)
    }

    fn variable_reference_bytes_at(
        &mut self,
        reference: &[u8],
        start: u32,
    ) -> Result<Value, TclError> {
        if self.safe {
            if let Some((_, original)) = self.original_term(start, None)? {
                return Ok(original);
            }
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim safe variable term",
            )
            .into());
        }
        crate::subst::subst_variable_reference_bytes(reference, self.vm)
    }

    fn command_bytes(&mut self, script: &[u8]) -> Result<Value, TclError> {
        let completion = self
            .vm
            .eval_source_image_at_internal(&tcl_lexer::SourceImage::native(script), None)?;
        self.settle_substitution_error_logging(&completion);
        self.accept_completion(completion)
    }

    fn call_bytes_at(
        &mut self,
        function: &[u8],
        args: Vec<Value>,
        start: u32,
    ) -> Result<Value, TclError> {
        let function = std::str::from_utf8(function).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native math function name grammar",
            )
        })?;
        self.call_at(function, args, start)
    }

    fn literal(&mut self, text: &str) -> Result<Value, TclError> {
        let dialect = self.vm.native_invocation_dialect();
        with_original_numeric_context(self.vm, |context| native_literal(context, text)).map(
            |value| {
                self.vm
                    .format_authored_math_result(value.with_native_double_format(dialect))
            },
        )
    }

    fn string(&mut self, inner: &str, substitutes: bool) -> Result<Value, TclError> {
        // A `"…"` expr operand is a double-quoted word: substitute `$var` /
        // `[cmd]` / backslashes (the runtime-`expr` analogue of the compiler's
        // `emit_expr_string`), so `expr {"item $i"}` is `item 0`, not `item $i`.
        //
        // A `{…}` operand is literal but for its backslash-newlines, which
        // fold even inside braces. Substituting it fully ran the `[id 9]` in
        // `set e {{[id 9]}}; expr $e`, which tclsh 8.4.20 through 9.1b0 all
        // return as the text `[id 9]` (#2227).
        if !substitutes {
            return Ok(Value::string(
                self.vm
                    .native_invocation_dialect()
                    .word_values
                    .collapse_braced_word(inner)
                    .as_ref(),
            ));
        }
        let Some(policy) = self.vm.expression_quote_control() else {
            let _ = self
                .vm
                .refuse_host_command("native expression quote settlement is unavailable".into());
            return Err(TclError::from_execution_failure(
                self.vm
                    .execution_refusal
                    .clone()
                    .expect("quote refusal retained"),
            ));
        };
        let completion = crate::subst::subst_expression_string(
            self.vm,
            inner,
            crate::subst::SubstitutionControl::Expression(policy),
        )?;
        self.accept_completion(completion)
    }

    fn variable_reference(&mut self, reference: &str) -> Result<Value, TclError> {
        crate::subst::subst_word(reference, self.vm)
    }

    fn var(&mut self, name: &str) -> Result<Value, TclError> {
        match self.vm.read_var_traced(name) {
            Err(c) => Err(TclError::from_completion(c)),
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(TclError::new(format!(
                "can't read \"{name}\": no such variable"
            ))),
        }
    }

    fn command(&mut self, script: &str) -> Result<Value, TclError> {
        // A command substitution inside an expression yields the command's result
        // *only* when it completes normally; otherwise the completion must
        // propagate, not be taken as the value — otherwise `expr {[error msg]}`
        // would silently evaluate to the string "msg" (if-5.2). An error becomes a
        // plain `TclError`; a `break`/`continue`/`return` escaping the substitution
        // carries its code so the enclosing construct sees it.
        let c = self.vm.eval_source(script)?;
        self.settle_substitution_error_logging(&c);
        self.accept_completion(c)
    }

    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, TclError> {
        if self.vm.authored_math_provider().is_some() {
            let completion = crate::cmd_math::invoke_authored_function(self.vm, function, &args);
            return self.accept_completion(completion);
        }
        if tcl_registry::native_expression_program::expression_function_dispatch(
            self.vm.expression_evaluation_policy().as_ref(),
            self.vm.actual_native_invocation_dialect(),
        )
        .is_none()
        {
            let completion = self
                .vm
                .refuse_host_command("expression function dispatch policy is unavailable".into());
            return Err(TclError::from_completion(completion));
        }
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.vm.native_execution_profile(),
        );
        match surface.math_function_call_target(function) {
            tcl_registry::expr_surface::MathFunctionCallTarget::FixedBuiltin(spec) => {
                // Tcl 8.4 predates TIP 232: builtin expressions use the fixed
                // function table selected by the registry, never a script
                // command that happens to share the `tcl::mathfunc` spelling.
                let c = self.vm.invoke_fixed_math_builtin(spec, &args);
                return self.accept_completion(c);
            }
            tcl_registry::expr_surface::MathFunctionCallTarget::FixedTableMiss => {
                return Err(TclError::new(format!(
                    "unknown math function \"{function}\""
                )));
            }
            tcl_registry::expr_surface::MathFunctionCallTarget::CommandTable => {}
        }

        // TIP 232: a math function is an ordinary command, so a user
        // `proc tcl::mathfunc::f {…} {…}` (the canonical custom-function
        // mechanism) must dispatch exactly like a builtin one — full command
        // dispatch, not builtin-only. The name stays *relative*, so
        // resolution is current-namespace-first (a namespace-local
        // `tcl::mathfunc::f` shadows the global; tclsh-pinned by the
        // mathfunc conformance vectors).
        let qualified = tcl_registry::mathfunc::qualified_name(function);
        let name = tcl_syntax::naming::unroot_rooted_key(&qualified).unwrap_or(&qualified);
        if self.vm.lookup_command(name).is_none() {
            // C reports the command miss, not a special math-function error
            // (tclsh 8.6.16 / 9.0.4: `expr {frobnicate(1)}` →
            // `invalid command name "tcl::mathfunc::frobnicate"`).
            let message = format!("invalid command name \"{name}\"");
            return Err(
                if self.vm.runtime_version().has_lookup_error_code("COMMAND") {
                    TclError::with_error_code(message, format!("TCL LOOKUP COMMAND {name}"))
                } else {
                    TclError::new(message)
                },
            );
        }
        let c = self.vm.invoke_command(name, &args);
        self.accept_completion(c)
    }

    fn arith(&mut self, op: BinOp, l: Value, r: Value) -> Result<Value, TclError> {
        let dialect = self.vm.native_invocation_dialect();
        with_original_numeric_context(self.vm, |context| arith_in(context, op, &l, &r)).map(
            |value| {
                self.vm
                    .format_authored_math_result(value.with_native_double_format(dialect))
            },
        )
    }

    fn unary(&mut self, op: UnaryOp, v: Value) -> Result<Value, TclError> {
        let dialect = self.vm.native_invocation_dialect();
        unary_for_vm(self.vm, op, &v).map(|value| {
            self.vm
                .format_authored_math_result(value.with_native_double_format(dialect))
        })
    }

    fn compare_numeric(&mut self, _left: &Value, _right: &Value) -> Option<NumericCompare> {
        // The executable adapter requires the fallible owner below.
        None
    }

    fn compare_numeric_checked(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<Option<NumericCompare>, TclError> {
        if let Some(first) = self.vm.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(first));
        }
        with_original_numeric_context(self.vm, |context| {
            compare_values_numeric_checked_in(context, left, right)
        })
    }

    fn compare_string(&mut self, l: &Value, r: &Value) -> Result<Ordering, TclError> {
        with_original_numeric_context(self.vm, |context| {
            compare_string_units_using(context.characters, l, r, context.original.map(|_| context))
        })
    }

    fn equal_string(&mut self, l: &Value, r: &Value) -> Result<bool, TclError> {
        with_original_numeric_context(self.vm, |context| {
            compare_with_numeric_using(
                BinOp::StrEq,
                l,
                r,
                None,
                context.characters,
                context.original.map(|_| context),
            )
        })
    }

    fn in_list(&mut self, needle: &Value, list: &Value) -> Result<bool, TclError> {
        original_list_membership::in_list(self.vm, needle, list)
    }

    fn to_bool(&mut self, v: &Value) -> Result<bool, TclError> {
        boolean_for_vm(
            self.vm,
            v,
            tcl_native_model::native_boolean_truth::NativeBooleanTruthPurpose::ConditionalJump,
        )
    }

    fn to_bool_for_purpose(
        &mut self,
        value: &Value,
        purpose: tcl_syntax::native_boolean_truth::NativeBooleanTruthPurpose,
    ) -> Result<bool, TclError> {
        boolean_for_vm(self.vm, value, purpose)
    }

    fn logical_right_truth(
        &mut self,
        left: &Value,
        right: &Value,
        conjunction: bool,
    ) -> Result<bool, TclError> {
        logical_right_for_vm(self.vm, left, right, conjunction)
    }

    fn bool_value(&mut self, b: bool) -> Value {
        native_boolean_result(self.vm.numeric_policy_context(), b)
    }

    fn binary_other(&mut self, op: BinOp, left: Value, right: Value) -> Result<Value, TclError> {
        let provider =
            tcl_registry::native_expression_program::authored_f5_string_predicate_provider(
                self.vm.expression_evaluation_policy().as_ref(),
            )
            .ok_or_else(|| self.unsupported("operator"))?;
        let predicate = provider
            .predicate(op)
            .ok_or_else(|| self.unsupported("operator"))?;
        let left = left.try_to_str()?;
        let right = right.try_to_str()?;
        Ok(Value::bool(predicate.evaluate(&left, &right)))
    }

    fn unsupported(&mut self, what: &str) -> TclError {
        TclError::new(what)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "expr_float_tests.rs"]
mod float_error_tests;
