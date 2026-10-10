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

//! The native tier's ABI additions, beside the ABI v2
//! groundwork in [`crate::codegen_abi`].
//!
//! Every export here contributes **transport**, never semantics: the
//! runtime's own `incr`/`append`/`lappend`, its expression evaluator, its
//! `::tcl::mathop` operators, and its `::tcl::mathfunc` dispatch run over
//! prebuilt operands, so bignum promotion, copy-on-write growth, traces,
//! `const`, and every error message are exactly interpreted Tcl's. The
//! descriptors live in `tcl-runtime-api`'s `codegen_abi.rs`.
//!
//! Two reads are deliberately *non-erroring* (`tcl_codegen_value_try_*`):
//! they answer whether a boxed value already has, or parses to, a native
//! representation, and set no interpreter error when it does not. Generated
//! code uses them to choose a native fast path; the slow edge it takes
//! otherwise is one of the runtime operators below, which raises Tcl's own
//! error if the value is not numeric at all.

use core::ptr;

use crate::codegen_abi::{current_interp, TclCompletionAbi, TCL_INVOKE_ABI_HOST_REFUSED};
use crate::interp::native_operation_currency::NativeOperationCurrency;
use crate::interp::{Code, Interp};
use crate::obj::{self, TclObj};

/// `tcl_codegen_value_try_*`: the value has the native representation.
pub const TCL_VALUE_TRY_NATIVE: i32 = 1;
/// `tcl_codegen_value_try_*`: the value has no such native representation;
/// no interpreter error was set.
pub const TCL_VALUE_TRY_NOT_NATIVE: i32 = 0;

/// The completion intrinsics ran and wrote the completion triple.
pub const TCL_NATIVE_ABI_OK: i32 = 0;
/// A null pointer or missing interpreter kept the intrinsic from running.
pub const TCL_NATIVE_ABI_INVALID: i32 = 1;

unsafe fn input_bytes<'a>(ptr: *const u8, len: i32) -> &'a [u8] {
    if ptr.is_null() || len <= 0 {
        return b"";
    }
    // SAFETY: generated code passes a data-segment address and its exact
    // byte length.
    unsafe { core::slice::from_raw_parts(ptr, len as usize) }
}

/// Capture the actual completion and publish it only while the original
/// operation remains current. A metadata getter's first Host cause leaves
/// caller storage untouched; any retained transport handles are released.
///
/// # Safety
/// `out` must be writable, aligned completion storage.
unsafe fn write_completion(
    interp: &mut Interp,
    currency: &NativeOperationCurrency,
    code: Code,
    out: *mut TclCompletionAbi,
) -> i32 {
    if let Err(error) = currency.ensure_current() {
        interp.refuse_native_execution(error);
        return TCL_INVOKE_ABI_HOST_REFUSED;
    }
    let completion = crate::state_traits::capture_completion(interp, code);
    if let Err(error) = currency.ensure_current() {
        interp.refuse_native_execution(error);
        // SAFETY: capture_completion owns one reference to each non-null
        // handle. Host refusal may instead have returned null transport slots.
        unsafe {
            obj::decr_ref_count(completion.result);
            obj::decr_ref_count(completion.options);
        }
        return TCL_INVOKE_ABI_HOST_REFUSED;
    }
    let code = match completion.code {
        tcl_runtime_api::Code::Ok => 0,
        tcl_runtime_api::Code::Error => 1,
        tcl_runtime_api::Code::Return => 2,
        tcl_runtime_api::Code::Break => 3,
        tcl_runtime_api::Code::Continue => 4,
        tcl_runtime_api::Code::Other(code) => code,
    };
    // SAFETY: `out` is writable aligned storage per the caller's contract.
    unsafe {
        out.write(TclCompletionAbi {
            code,
            result: completion.result,
            options: completion.options,
        });
    }
    TCL_NATIVE_ABI_OK
}

/// Enter before any getter or callback; retain the first typed Host refusal.
fn completion_entry(interp: &mut Interp) -> Result<NativeOperationCurrency, i32> {
    NativeOperationCurrency::issue(interp).map_err(|error| {
        interp.refuse_native_execution(error);
        TCL_INVOKE_ABI_HOST_REFUSED
    })
}

/// One of the runtime's own commands over a prebuilt argv.
type CellCommand = fn(&mut Interp, &[*mut TclObj]) -> Code;

/// Retain every argv word for one dispatch and release on drop, exactly as
/// the generic ABI does, so the caller's counts are restored on every path.
struct BorrowedArgv<'a> {
    words: &'a [*mut TclObj],
}

impl<'a> BorrowedArgv<'a> {
    /// # Safety
    /// Every word must be a live object.
    unsafe fn retain(words: &'a [*mut TclObj]) -> Self {
        for &word in words {
            // SAFETY: upheld by this method's contract.
            unsafe { obj::incr_ref_count(word) };
        }
        Self { words }
    }
}

impl Drop for BorrowedArgv<'_> {
    fn drop(&mut self) {
        for &word in self.words {
            // SAFETY: `retain` took exactly one reference on every word.
            unsafe { obj::decr_ref_count(word) };
        }
    }
}

/// Run one of the runtime's own commands over `head name values…`.
///
/// # Safety
/// `interp` must be live; every value must be a live object.
unsafe fn run_named_cell_command(
    interp: &mut Interp,
    head: &[u8],
    name: &[u8],
    values: &[*mut TclObj],
    command: CellCommand,
) -> Code {
    let head_obj = obj::new_string_bytes(head);
    let name_obj = obj::new_string_bytes(name);
    let mut argv = vec![head_obj, name_obj];
    argv.extend_from_slice(values);
    // SAFETY: every word is live for the whole call. The borrow's `+1` is the
    // only reference the two fresh words have, so releasing it also frees
    // them; a second release here would corrupt the allocator.
    let borrowed = unsafe { BorrowedArgv::retain(&argv) };
    let code = command(interp, &argv);
    drop(borrowed);
    code
}

/// `tcl_codegen_var_set_element(name, key, value) -> code` — store `value`
/// into the array element `name(key)`; the element half of
/// [`crate::codegen_abi::tcl_codegen_var_set`]. **Adopts** the value's
/// generated reference. Returns the Tcl completion code.
///
/// # Safety
/// Both byte ranges must be readable; `value` must be a live owned reference.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_set_element(
    name_ptr: *const u8,
    name_len: i32,
    key_ptr: *const u8,
    key_len: i32,
    value: *mut TclObj,
) -> i32 {
    let interp = current_interp();
    if interp.is_null() || value.is_null() {
        return 1;
    }
    // SAFETY: the ranges are readable data-segment bytes per the contract.
    let (name, key) = unsafe {
        (
            input_bytes(name_ptr, name_len),
            input_bytes(key_ptr, key_len),
        )
    };
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    let code = match interp.var_set_elem(name, key, value) {
        Ok(()) => 0,
        Err(error) => {
            let mut spelled = name.to_vec();
            spelled.push(b'(');
            spelled.extend_from_slice(key);
            spelled.push(b')');
            i32::try_from(crate::builtins::var_error(interp, &spelled, error).as_int()).unwrap_or(1)
        }
    };
    // SAFETY: the generated assignment transfers its reference here.
    unsafe { obj::decr_ref_count(value) };
    code
}

/// `tcl_codegen_var_incr(name, delta) -> obj` — Tcl `incr` on the named
/// variable by the boxed `delta`, returning the cell's new value with one
/// owned reference, or null with the interpreter carrying the Tcl error.
///
/// Full `incr` semantics: an unset variable is created (Tcl 8.5+), the sum
/// promotes to a bignum, a non-integer cell or delta raises C's message, and
/// `const` and traces apply. `delta` is borrowed.
///
/// # Safety
/// The name range must be readable; `delta` must be a live object.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_incr(
    name_ptr: *const u8,
    name_len: i32,
    delta: *mut TclObj,
) -> *mut TclObj {
    let interp = current_interp();
    if interp.is_null() || delta.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: readable per the contract; the interpreter is live.
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let interp = unsafe { &mut *interp };
    // SAFETY: `delta` is live per the contract.
    // The trace-safe `incr`, not `builtins::incr`: see `cmd_var::installed_incr`.
    let code = unsafe {
        run_named_cell_command(
            interp,
            b"incr",
            name,
            &[delta],
            crate::cmd_var::installed_incr(),
        )
    };
    if code != Code::Ok {
        return ptr::null_mut();
    }
    let result = interp.result_obj();
    // SAFETY: the result is interp-owned; the caller claims one reference.
    unsafe { obj::incr_ref_count(result) };
    result
}

/// `tcl_codegen_var_update(name, argv, argc, list) -> code` — Tcl `append`
/// (`list` = 0) or `lappend` (`list` = 1) of the `argc` borrowed values onto
/// the named variable. Returns the Tcl completion code.
///
/// # Safety
/// The name range must be readable; `argv` must point to `argc` live objects.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_update(
    name_ptr: *const u8,
    name_len: i32,
    argv: *const *mut TclObj,
    argc: i32,
    list: i32,
) -> i32 {
    let interp = current_interp();
    if interp.is_null() {
        return 1;
    }
    let Ok(argc) = usize::try_from(argc) else {
        return 1;
    };
    if argc > 0 && argv.is_null() {
        return 1;
    }
    // SAFETY: readable per the contract; the interpreter is live.
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let interp = unsafe { &mut *interp };
    // SAFETY: `argv` references `argc` readable pointers per the contract.
    let values = if argc == 0 {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(argv, argc) }
    };
    if values.iter().any(|value| value.is_null()) {
        return 1;
    }
    let (head, command): (&[u8], CellCommand) = if list == 0 {
        (b"append", crate::cmd_string::append)
    } else {
        (b"lappend", crate::cmd_list::lappend)
    };
    // SAFETY: every value is live per the contract.
    let code = unsafe { run_named_cell_command(interp, head, name, values, command) };
    i32::try_from(code.as_int()).unwrap_or(1)
}

/// `tcl_codegen_value_try_wide_int(value, out) -> native?` — read a boxed
/// value as an `i64` when it is one (a cached integer rep, or a spelling
/// that parses as a wide), writing it through `out` and caching the parsed
/// rep. Returns [`TCL_VALUE_TRY_NOT_NATIVE`] — and sets **no** error — when
/// the value is a double, a bignum, or not numeric at all.
///
/// # Safety
/// `value` must be a live object; `out` must be writable aligned `i64`
/// storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_value_try_wide_int(value: *mut TclObj, out: *mut i64) -> i32 {
    if value.is_null() || out.is_null() {
        return TCL_VALUE_TRY_NOT_NATIVE;
    }
    let interp = current_interp();
    if interp.is_null() || unsafe { (*interp).host_refusal_pending() } {
        return TCL_VALUE_TRY_NOT_NATIVE;
    }
    match crate::typed_value::native_scalar_probe(
        value,
        unsafe { (*interp).native_invocation_dialect() },
        tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
    ) {
        Ok(Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(parsed))) => {
            // A wrapped primitive Wide view is not an integer arithmetic cache.
            // Big magnitude and Jim's coerced view remain available to the boxed
            // arithmetic path; the reached probe effects still remain applied.
            if !matches!(
                crate::obj::native_scalar_cache(value),
                Ok(Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(_)
                )))
            ) {
                return TCL_VALUE_TRY_NOT_NATIVE;
            }
            // SAFETY: `out` is writable aligned storage per the contract.
            unsafe { out.write(parsed) };
            TCL_VALUE_TRY_NATIVE
        }
        Err(error) => {
            if let Some(refusal) = error.native_access_refusal() {
                // SAFETY: the current interpreter was checked above.
                unsafe { (*interp).refuse_native_access(refusal) };
            }
            TCL_VALUE_TRY_NOT_NATIVE
        }
        Ok(_) => TCL_VALUE_TRY_NOT_NATIVE,
    }
}

/// `tcl_codegen_value_try_double(value, out) -> native?` —
/// [`tcl_codegen_value_try_wide_int`] over an `f64`, using the same selected
/// primitive conversion and its original-object cache effects.
///
/// # Safety
/// `value` must be a live object; `out` must be writable aligned `f64`
/// storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_value_try_double(value: *mut TclObj, out: *mut f64) -> i32 {
    if value.is_null() || out.is_null() {
        return TCL_VALUE_TRY_NOT_NATIVE;
    }
    let interp = current_interp();
    if interp.is_null() || unsafe { (*interp).host_refusal_pending() } {
        return TCL_VALUE_TRY_NOT_NATIVE;
    }
    match crate::typed_value::native_scalar_probe(
        value,
        unsafe { (*interp).native_invocation_dialect() },
        tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
    ) {
        Ok(Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Double(parsed))) => {
            // SAFETY: `out` is writable aligned storage per the contract.
            unsafe { out.write(parsed) };
            TCL_VALUE_TRY_NATIVE
        }
        Err(error) => {
            if let Some(refusal) = error.native_access_refusal() {
                // SAFETY: the current interpreter was checked above.
                unsafe { (*interp).refuse_native_access(refusal) };
            }
            TCL_VALUE_TRY_NOT_NATIVE
        }
        Ok(_) => TCL_VALUE_TRY_NOT_NATIVE,
    }
}

/// `tcl_codegen_expr_eval(expr, out) -> status` — evaluate the borrowed
/// expression object with the runtime's expression evaluator and write the
/// completion triple: the value on success, or the Tcl error (or a
/// `return`/`break`/`continue` a `[cmd]` operand raised) exactly as `expr`
/// itself would report it.
///
/// First Host refusal returns `TCL_INVOKE_ABI_HOST_REFUSED` and leaves
/// completion storage untouched.
///
/// # Safety
/// `expr` must be a live caller-owned object; `out` must be writable aligned
/// completion storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_expr_eval(
    expr: *mut TclObj,
    out: *mut TclCompletionAbi,
) -> i32 {
    let interp = current_interp();
    if interp.is_null() || expr.is_null() || out.is_null() {
        return TCL_NATIVE_ABI_INVALID;
    }
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    let currency = match completion_entry(interp) {
        Ok(currency) => currency,
        Err(status) => return status,
    };
    let code = expr_eval_impl(interp, expr, &currency);
    // SAFETY: `out` is writable per the contract.
    unsafe { write_completion(interp, &currency, code, out) }
}

#[cfg(have_tommath)]
fn expr_eval_impl(
    interp: &mut Interp,
    expr: *mut TclObj,
    currency: &NativeOperationCurrency,
) -> Code {
    let source = match tcl_syntax::value::ValueOps::native_string_bytes(interp, &expr) {
        Ok(source) => source,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    if let Err(error) = currency.ensure_current() {
        interp.refuse_native_execution(error);
        return Code::Error;
    }
    let result = crate::builtins::eval_expr_obj_original(interp, expr, &source);
    if let Err(error) = currency.ensure_current() {
        interp.refuse_native_execution(error);
        if let Ok(result) = result {
            // SAFETY: the evaluator returned exactly one owned reference.
            unsafe { obj::decr_ref_count(result) };
        }
        return Code::Error;
    }
    match result {
        Ok(result) => {
            interp.set_result(result);
            // SAFETY: `eval_expr_obj` handed over one owned reference, which
            // the interpreter result now holds.
            unsafe { obj::decr_ref_count(result) };
            Code::Ok
        }
        Err(code) => code,
    }
}

#[cfg(not(have_tommath))]
fn expr_eval_impl(
    interp: &mut Interp,
    _expr: *mut TclObj,
    _currency: &NativeOperationCurrency,
) -> Code {
    interp.set_error(b"arithmetic support is not available")
}

/// Evaluate a borrowed expression and perform its actual public result
/// producer and Boolean conversion once. Guest and propagated completions use
/// the existing owned completion transport; only success writes `truth_out`.
/// First Host refusal returns `TCL_INVOKE_ABI_HOST_REFUSED` and leaves both
/// outputs untouched.
///
/// # Safety
/// `expr` must remain a live caller-owned object. Both output pointers must be
/// writable, aligned, non-overlapping storage; the completion owns its result
/// and options references. The caller must release that completion.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_expr_bool(
    expr: *mut TclObj,
    completion_out: *mut TclCompletionAbi,
    truth_out: *mut i32,
) -> i32 {
    let interp = current_interp();
    if interp.is_null() || expr.is_null() || completion_out.is_null() || truth_out.is_null() {
        return TCL_NATIVE_ABI_INVALID;
    }
    // SAFETY: current_interp and all outputs satisfy this entry's contract.
    let interp = unsafe { &mut *interp };
    let currency = match completion_entry(interp) {
        Ok(currency) => currency,
        Err(status) => return status,
    };
    let result = expr_bool_eval_impl(interp, expr);
    let code = result.as_ref().map_or_else(|code| *code, |_| Code::Ok);
    // Actual completion getters run before either output is published.
    let status = unsafe { write_completion(interp, &currency, code, completion_out) };
    if status == TCL_NATIVE_ABI_OK {
        if let Ok(truth) = result {
            // SAFETY: truth_out is writable aligned caller-owned storage.
            unsafe { truth_out.write(i32::from(truth)) };
        }
    }
    status
}

#[cfg(have_tommath)]
fn expr_bool_eval_impl(interp: &mut Interp, expr: *mut TclObj) -> Result<bool, Code> {
    crate::builtins::eval_bool_expr(interp, expr)
}

#[cfg(not(have_tommath))]
fn expr_bool_eval_impl(interp: &mut Interp, _expr: *mut TclObj) -> Result<bool, Code> {
    Err(interp.set_error(b"arithmetic support is not available"))
}

/// A no-op expression context: operator operands are already evaluated, so
/// variable, command, and function resolution is never reached.
#[cfg(have_tommath)]
struct NoCtx<'a>(&'a mut Interp);

#[cfg(have_tommath)]
impl crate::expr::ExprCtx for NoCtx<'_> {
    fn boolean_interpreter(&mut self) -> Option<&mut Interp> {
        Some(self.0)
    }
    fn numeric_host(&self) -> Option<std::rc::Rc<dyn tcl_platform::Host>> {
        Some(self.0.host())
    }
    fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        self.0.native_invocation_dialect()
    }
    fn read_var(&mut self, _: &str) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        unreachable!("operator operands are pre-evaluated")
    }
    fn eval_command(&mut self, _: &str) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        unreachable!("operator operands are pre-evaluated")
    }
    fn call_function(
        &mut self,
        _: &str,
        _: &[crate::obj::Owned],
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        unreachable!("operator operands are pre-evaluated")
    }
}

/// `tcl_codegen_mathop(op, argv, argc, out) -> status` — apply the `expr`
/// operator spelled by the `op` bytes (`+`, `**`, `eq`, `in`, …) to `argc`
/// borrowed, already-evaluated operands through the runtime's own
/// `::tcl::mathop` implementation, writing the completion triple. This is
/// the slow edge of every native arithmetic fast path and the only path for
/// operators with no native shape.
///
/// # Safety
/// The `op` range must be readable; `argv` must point to `argc` live objects;
/// `out` must be writable aligned completion storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_mathop(
    op_ptr: *const u8,
    op_len: i32,
    argv: *const *mut TclObj,
    argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    let interp = current_interp();
    if interp.is_null() || out.is_null() {
        return TCL_NATIVE_ABI_INVALID;
    }
    let Ok(argc) = usize::try_from(argc) else {
        return TCL_NATIVE_ABI_INVALID;
    };
    if argc > 0 && argv.is_null() {
        return TCL_NATIVE_ABI_INVALID;
    }
    // SAFETY: readable per the contract; the interpreter is live.
    let op = unsafe { input_bytes(op_ptr, op_len) };
    let interp = unsafe { &mut *interp };
    let words = if argc == 0 {
        &[][..]
    } else {
        // SAFETY: `argv` references `argc` readable pointers per the contract.
        unsafe { core::slice::from_raw_parts(argv, argc) }
    };
    if words.iter().any(|word| word.is_null()) {
        return TCL_NATIVE_ABI_INVALID;
    }
    let currency = match completion_entry(interp) {
        Ok(currency) => currency,
        Err(status) => return status,
    };
    let code = mathop_eval_impl(interp, op, words);
    // SAFETY: `out` is writable per the contract.
    unsafe { write_completion(interp, &currency, code, out) }
}

/// Apply the operator through the runtime's own `::tcl::mathop`.
///
/// Split out so the whole intrinsic keeps its ABI without the bignum backend,
/// exactly as [`expr_eval_impl`] does: `crate::expr` is `have_tommath`-gated,
/// so a build without libtommath has no operator evaluator to call.
#[cfg(have_tommath)]
fn mathop_eval_impl(interp: &mut Interp, op: &[u8], words: &[*mut TclObj]) -> Code {
    use tcl_cmd_core::mathop::MathopError;
    let op_str = core::str::from_utf8(op).unwrap_or("");
    let args: Vec<crate::obj::Owned> = words
        .iter()
        .map(|&word| crate::obj::Owned::retain(word))
        .collect();
    match crate::expr::eval_mathop(op_str, args, &mut NoCtx(interp)) {
        Ok(result) => {
            interp.set_result(result.as_ptr());
            Code::Ok
        }
        Err(MathopError::WrongArgs(usage)) => {
            let mut message = b"wrong # args: should be \"::tcl::mathop::".to_vec();
            message.extend_from_slice(op);
            message.push(b' ');
            message.extend_from_slice(usage.as_bytes());
            message.push(b'"');
            interp.wrong_arguments_message(&message)
        }
        Err(MathopError::Op(error)) => interp.report_expr_error(error),
    }
}

#[cfg(not(have_tommath))]
fn mathop_eval_impl(interp: &mut Interp, _op: &[u8], _words: &[*mut TclObj]) -> Code {
    interp.set_error(b"arithmetic support is not available")
}

/// `tcl_codegen_mathfunc(name, argv, argc, out) -> status` — call the math
/// function `::tcl::mathfunc::<name>` through ordinary command dispatch (so
/// a user-defined or renamed function is honoured exactly as `expr` honours
/// it) over `argc` borrowed, already-evaluated arguments, writing the
/// completion triple.
///
/// # Safety
/// The name range must be readable; `argv` must point to `argc` live
/// objects; `out` must be writable aligned completion storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_mathfunc(
    name_ptr: *const u8,
    name_len: i32,
    argv: *const *mut TclObj,
    argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    let interp = current_interp();
    if interp.is_null() || out.is_null() {
        return TCL_NATIVE_ABI_INVALID;
    }
    let Ok(argc) = usize::try_from(argc) else {
        return TCL_NATIVE_ABI_INVALID;
    };
    if argc > 0 && argv.is_null() {
        return TCL_NATIVE_ABI_INVALID;
    }
    // SAFETY: readable per the contract; the interpreter is live.
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let interp = unsafe { &mut *interp };
    let words = if argc == 0 {
        &[][..]
    } else {
        // SAFETY: `argv` references `argc` readable pointers per the contract.
        unsafe { core::slice::from_raw_parts(argv, argc) }
    };
    if words.iter().any(|word| word.is_null()) {
        return TCL_NATIVE_ABI_INVALID;
    }
    let currency = match completion_entry(interp) {
        Ok(currency) => currency,
        Err(status) => return status,
    };
    let mut head = b"::tcl::mathfunc::".to_vec();
    head.extend_from_slice(name);
    let head_obj = obj::new_string_bytes(&head);
    let mut full = vec![head_obj];
    full.extend_from_slice(words);
    // SAFETY: every word is live for the whole call; releasing the borrow's
    // `+1` frees the fresh head word.
    let borrowed = unsafe { BorrowedArgv::retain(&full) };
    let code = interp.dispatch(&full);
    drop(borrowed);
    // SAFETY: `out` is writable per the contract.
    unsafe { write_completion(interp, &currency, code, out) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capi::{tcl_runtime_create_interp, tcl_runtime_delete_interp};
    use crate::codegen_abi::tcl_runtime_set_current_interp;
    use crate::counters;
    use crate::interp::obj_bytes;

    /// A compiled `incr` on a cell whose write trace rewrites it must return
    /// the cell's post-trace value, exactly as the interpreted `incr` does.
    /// Dispatching `builtins::incr` here instead of the registered
    /// trace-safe body would return the pre-trace sum — and drop the
    /// only reference to it while doing so.
    #[cfg(have_tommath)]
    #[test]
    fn a_compiled_incr_returns_the_value_a_write_trace_left() {
        leak_free(|interp| {
            let setup = b"set x 1\n                 proc R {n1 n2 op} { set ::x 99 }\n                 trace add variable x write R\n";
            assert_eq!(interp.eval_str(setup), Code::Ok);
            let delta = obj::new_wide_int_obj(1);
            // `tcl_codegen_var_incr` *borrows* its delta: the argv pin it
            // takes is released after the call, which frees a bare `rc 0`
            // word. Hold a reference of our own across the call and release
            // exactly that one, or the release below is a double free.
            // SAFETY: `delta` is a fresh live object.
            unsafe { obj::incr_ref_count(delta) };
            // SAFETY: a live name range and a live delta on the current interp.
            let result = unsafe { tcl_codegen_var_incr(b"x".as_ptr(), 1, delta) };
            // SAFETY: releases exactly the reference taken above.
            unsafe { obj::decr_ref_count(delta) };
            assert!(!result.is_null(), "the incr completed");
            assert_eq!(
                String::from_utf8_lossy(&obj_bytes(result)),
                "99",
                "the trace's value, not the pre-trace sum"
            );
            // SAFETY: the call handed us one reference.
            unsafe { obj::decr_ref_count(result) };
            assert_eq!(interp.eval_str(b"set x"), Code::Ok);
            assert_eq!(String::from_utf8_lossy(&interp.result_bytes()), "99");
        });
    }

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            // SAFETY: freshly created and current.
            body(unsafe { &mut *interp });
            tcl_runtime_set_current_interp(ptr::null_mut());
            // SAFETY: created above and no longer current.
            unsafe { tcl_runtime_delete_interp(interp) };
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0, "double frees detected");
    }

    fn owned(bytes: &[u8]) -> *mut TclObj {
        let value = obj::new_string_bytes(bytes);
        // SAFETY: fresh object; the test owns this reference.
        unsafe { obj::incr_ref_count(value) };
        value
    }

    fn release(value: *mut TclObj) {
        // SAFETY: balances `owned`.
        unsafe { obj::decr_ref_count(value) };
    }

    fn text(value: *mut TclObj) -> String {
        String::from_utf8_lossy(&obj_bytes(value)).into_owned()
    }

    fn empty() -> TclCompletionAbi {
        TclCompletionAbi {
            code: 0,
            result: ptr::null_mut(),
            options: ptr::null_mut(),
        }
    }

    #[cfg(have_tommath)]
    fn release_completion(completion: &mut TclCompletionAbi) {
        // SAFETY: the intrinsic gave the test one owned reference to each.
        unsafe {
            obj::decr_ref_count(completion.result);
            obj::decr_ref_count(completion.options);
        }
    }

    #[test]
    fn var_incr_creates_and_increments_with_full_incr_semantics() {
        leak_free(|interp| {
            let delta = owned(b"5");
            // SAFETY: readable name bytes, live delta.
            let first = unsafe { tcl_codegen_var_incr(b"n".as_ptr(), 1, delta) };
            assert!(!first.is_null());
            assert_eq!(text(first), "5", "an unset variable starts at zero");
            release(first);
            let second = unsafe { tcl_codegen_var_incr(b"n".as_ptr(), 1, delta) };
            assert_eq!(text(second), "10");
            release(second);
            release(delta);
            let bad = owned(b"x");
            let failed = unsafe { tcl_codegen_var_incr(b"n".as_ptr(), 1, bad) };
            assert!(failed.is_null());
            assert_eq!(text(interp.result_obj()), "expected integer but got \"x\"");
            release(bad);
        });
    }

    #[test]
    fn incr_result_survives_a_generic_puts_invocation_in_a_loop() {
        use crate::codegen_abi::{tcl_codegen_var_set, tcl_invoke_argv};
        leak_free(|_| {
            let zero = owned(b"0");
            // SAFETY: readable name, owned value (adopted).
            assert_eq!(unsafe { tcl_codegen_var_set(b"i".as_ptr(), 1, zero) }, 0);
            let head = owned(b"puts");
            let mut current: *mut TclObj = ptr::null_mut();
            let mut result = ptr::null_mut();
            let mut options = ptr::null_mut();
            for _ in 0..4 {
                let delta = owned(b"1");
                // SAFETY: readable name, live delta.
                let incremented = unsafe { tcl_codegen_var_incr(b"i".as_ptr(), 1, delta) };
                assert!(!incremented.is_null());
                release(delta);
                release(current);
                current = incremented;
                let argv = [head, current];
                let mut completion = empty();
                // SAFETY: live words and local completion storage.
                unsafe {
                    tcl_invoke_argv(argv.as_ptr(), 2, &mut completion);
                }
                assert_eq!(completion.code, 0, "{}", text(completion.result));
                release(result);
                release(options);
                result = completion.result;
                options = completion.options;
                let mut wide = 0i64;
                // SAFETY: readable name.
                let read = unsafe { crate::codegen_abi::tcl_codegen_var_get(b"i".as_ptr(), 1) };
                // SAFETY: live object and local storage.
                assert_eq!(
                    unsafe { tcl_codegen_value_try_wide_int(read, &mut wide) },
                    TCL_VALUE_TRY_NATIVE
                );
                release(read);
            }
            release(current);
            release(result);
            release(options);
            release(head);
        });
    }

    #[test]
    fn try_reads_answer_without_setting_an_error() {
        leak_free(|_| {
            let mut wide = 0i64;
            let mut double = 0f64;
            let int = owned(b"12");
            let float = owned(b"2.5");
            let word = owned(b"abc");
            // SAFETY: live objects and writable local storage.
            unsafe {
                assert_eq!(
                    tcl_codegen_value_try_wide_int(int, &mut wide),
                    TCL_VALUE_TRY_NATIVE
                );
                assert_eq!(wide, 12);
                assert_eq!(
                    tcl_codegen_value_try_wide_int(float, &mut wide),
                    TCL_VALUE_TRY_NOT_NATIVE
                );
                assert_eq!(
                    tcl_codegen_value_try_double(float, &mut double),
                    TCL_VALUE_TRY_NATIVE
                );
                assert!((double - 2.5).abs() < f64::EPSILON);
                assert_eq!(
                    tcl_codegen_value_try_double(word, &mut double),
                    TCL_VALUE_TRY_NOT_NATIVE
                );
            }
            release(int);
            release(float);
            release(word);
        });
    }

    #[test]
    fn speculative_getter_retains_failure_cache_without_changing_guest_state() {
        leak_free(|interp| {
            interp.set_error_state(b"KEEP CODE");
            interp.set_result_bytes(b"keep result");
            let value = owned(b"NaN");
            let mut answer = 17.0;
            // SAFETY: the owned value and local output are live.
            assert_eq!(
                unsafe { tcl_codegen_value_try_double(value, &mut answer) },
                TCL_VALUE_TRY_NOT_NATIVE
            );
            assert_eq!(answer, 17.0);
            assert!(core::ptr::eq(
                obj::obj_type_ptr(value),
                &obj::TCL_DOUBLE_TYPE
            ));
            assert!(obj::double_of(value).is_nan());
            assert_eq!(interp.result_bytes(), b"keep result");
            assert_eq!(interp.error_code(), b"KEEP CODE");
            assert!(!interp.host_refusal_pending());
            release(value);
        });
    }

    #[test]
    #[cfg(have_tommath)]
    fn wide_arithmetic_admission_preserves_big_primitive_cache() {
        leak_free(|interp| {
            interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            interp.set_error_state(b"KEEP CODE");
            interp.set_result_bytes(b"keep result");
            let value = owned(b"18446744073709551615");
            let mut answer = 17;
            assert_eq!(
                unsafe { tcl_codegen_value_try_wide_int(value, &mut answer) },
                TCL_VALUE_TRY_NOT_NATIVE
            );
            assert_eq!(answer, 17);
            assert!(matches!(
                obj::native_scalar_cache(value),
                Ok(Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Big { .. }
                )))
            ));
            assert_eq!(interp.result_bytes(), b"keep result");
            assert_eq!(interp.error_code(), b"KEEP CODE");
            release(value);
        });
    }

    #[test]
    fn speculative_getter_keeps_missing_engine_out_of_guest_state() {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("f5-irules"));
            interp.set_error_state(b"KEEP CODE");
            interp.set_result_bytes(b"keep result");
            let value = owned(b"3");
            let mut answer = 17;
            // SAFETY: the owned value and local output are live.
            assert_eq!(
                unsafe { tcl_codegen_value_try_wide_int(value, &mut answer) },
                TCL_VALUE_TRY_NOT_NATIVE
            );
            assert_eq!(answer, 17);
            assert!(obj::obj_type_ptr(value).is_null());
            assert_eq!(interp.result_bytes(), b"keep result");
            assert_eq!(interp.error_code(), b"KEEP CODE");
            assert!(interp.host_refusal_pending());
            release(value);
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn mathop_and_mathfunc_write_exact_completions() {
        leak_free(|_| {
            let a = owned(b"7");
            let b = owned(b"2");
            let argv = [a, b];
            let mut completion = empty();
            // SAFETY: live words and local completion storage.
            unsafe {
                assert_eq!(
                    tcl_codegen_mathop(b"/".as_ptr(), 1, argv.as_ptr(), 2, &mut completion),
                    TCL_NATIVE_ABI_OK
                );
            }
            assert_eq!(completion.code, 0);
            assert_eq!(text(completion.result), "3");
            release_completion(&mut completion);
            let zero = owned(b"0");
            let argv = [a, zero];
            unsafe {
                tcl_codegen_mathop(b"%".as_ptr(), 1, argv.as_ptr(), 2, &mut completion);
            }
            assert_eq!(completion.code, 1);
            assert_eq!(text(completion.result), "divide by zero");
            release_completion(&mut completion);
            let argv = [a, b];
            unsafe {
                tcl_codegen_mathop(b"**".as_ptr(), 2, argv.as_ptr(), 2, &mut completion);
            }
            assert_eq!(text(completion.result), "49");
            release_completion(&mut completion);
            let neg = owned(b"-3");
            let argv = [neg];
            unsafe {
                assert_eq!(
                    tcl_codegen_mathfunc(b"abs".as_ptr(), 3, argv.as_ptr(), 1, &mut completion),
                    TCL_NATIVE_ABI_OK
                );
            }
            assert_eq!(completion.code, 0);
            assert_eq!(text(completion.result), "3");
            release_completion(&mut completion);
            release(a);
            release(b);
            release(zero);
            release(neg);
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn expr_eval_writes_the_value_or_the_exact_error() {
        leak_free(|_| {
            let expr = owned(b"1 + 2 * 3");
            let mut completion = empty();
            // SAFETY: live object and local completion storage.
            unsafe {
                assert_eq!(
                    tcl_codegen_expr_eval(expr, &mut completion),
                    TCL_NATIVE_ABI_OK
                );
            }
            assert_eq!(completion.code, 0);
            assert_eq!(text(completion.result), "7");
            release_completion(&mut completion);
            release(expr);
            let bad = owned(b"$missing + 1");
            unsafe {
                tcl_codegen_expr_eval(bad, &mut completion);
            }
            assert_eq!(completion.code, 1);
            assert_eq!(
                text(completion.result),
                "can't read \"missing\": no such variable"
            );
            release_completion(&mut completion);
            release(bad);
        });
    }

    #[test]
    fn element_writes_and_appends_run_the_runtime_commands() {
        leak_free(|interp| {
            let value = owned(b"v");
            // SAFETY: readable ranges; `value` is an owned reference that the
            // call adopts.
            unsafe {
                assert_eq!(
                    tcl_codegen_var_set_element(b"a".as_ptr(), 1, b"k".as_ptr(), 1, value),
                    0
                );
            }
            assert_eq!(text(interp.var_get_elem(b"a", b"k").expect("element")), "v");
            let x = owned(b"x");
            let y = owned(b"y");
            let argv = [x, y];
            // SAFETY: readable name; live borrowed values.
            unsafe {
                assert_eq!(
                    tcl_codegen_var_update(b"s".as_ptr(), 1, argv.as_ptr(), 2, 0),
                    0
                );
                assert_eq!(
                    tcl_codegen_var_update(b"l".as_ptr(), 1, argv.as_ptr(), 2, 1),
                    0
                );
            }
            assert_eq!(text(interp.var_get(b"s").expect("s")), "xy");
            assert_eq!(text(interp.var_get(b"l").expect("l")), "x y");
            release(x);
            release(y);
        });
    }
}

#[cfg(all(test, have_tommath, not(target_arch = "wasm32")))]
mod original_expression_completion_tests {
    use super::*;
    use crate::interp::ObjCommand;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use tcl_platform::{Host, NumericEnvironment};

    // Observe and delegate the real selected Host conversion. No conversion
    // result, errno value, object header or native result recipe is fabricated.
    struct CountingHost {
        actual: Rc<dyn Host>,
        doubles: RefCell<Vec<(Vec<u8>, bool)>>,
    }
    impl Host for CountingHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            self.actual.capabilities()
        }
        fn clock(&self) -> &dyn tcl_platform::Clock {
            self.actual.clock()
        }
        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            self.actual.stdio()
        }
        fn env(&self) -> &dyn tcl_platform::Env {
            self.actual.env()
        }
        fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
            Some(self)
        }
        fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
            self.actual.native_integer_formatter()
        }
    }
    impl CountingHost {
        fn actual(&self) -> &dyn NumericEnvironment {
            self.actual
                .numeric_environment()
                .expect("actual native C numeric provider")
        }
    }
    impl NumericEnvironment for CountingHost {
        fn c_integer_abi(
            &self,
        ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable>
        {
            self.actual().c_integer_abi()
        }
        fn state(
            &self,
        ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable>
        {
            self.actual().state()
        }
        fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
            self.actual().reset()
        }
        fn unsigned_c84(
            &self,
            input: &[u8],
            offset: usize,
            long: bool,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned_c84(input, offset, long)
        }
        fn unsigned(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned(input, offset, base)
        }
        fn signed_long(
            &self,
            input: &[u8],
            base: u32,
        ) -> Result<
            tcl_platform::SignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().signed_long(input, base)
        }
        fn double(
            &self,
            input: &[u8],
            reset: bool,
        ) -> Result<
            tcl_platform::DoubleNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.doubles.borrow_mut().push((input.to_vec(), reset));
            self.actual().double(input, reset)
        }
    }

    #[derive(Clone, Copy)]
    enum CallbackMode {
        True,
        False,
        Guest(i32),
        ChangeWorld(i32),
    }
    struct CallbackState {
        calls: Cell<usize>,
        mode: CallbackMode,
    }
    unsafe extern "C" fn original_callback(
        client: *mut core::ffi::c_void,
        interp: *mut Interp,
        _argc: core::ffi::c_int,
        _argv: *const *mut TclObj,
    ) -> core::ffi::c_int {
        // SAFETY: installed client data and interpreter remain live for dispatch.
        let state = unsafe { &*client.cast::<CallbackState>() };
        let interp = unsafe { &mut *interp };
        state.calls.set(state.calls.get() + 1);
        match state.mode {
            CallbackMode::True => {
                interp.set_result_bytes(b"true");
                0
            }
            CallbackMode::False => {
                interp.set_result_bytes(b"false");
                0
            }
            CallbackMode::Guest(code) => {
                if code == 2 {
                    interp.set_return_state(1, Code::Ok);
                }
                interp.set_result_bytes(b"GUEST\0\xff");
                interp.set_c_error_code(b"ORIGINAL CALLBACK CODE");
                code
            }
            CallbackMode::ChangeWorld(code) => {
                interp.set_result_bytes(b"true");
                let original = interp.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("boolean-eval-currency-control".into(), "1.0".into())];
                interp
                    .pin_context(&changed)
                    .expect("genuine context change");
                interp
                    .pin_context(&original)
                    .expect("restore original context");
                code
            }
        }
    }

    struct CurrentEntry;
    impl CurrentEntry {
        fn enter(interp: &mut Interp) -> Self {
            crate::codegen_abi::tcl_runtime_set_current_interp(interp);
            Self
        }
    }
    impl Drop for CurrentEntry {
        fn drop(&mut self) {
            crate::codegen_abi::tcl_runtime_set_current_interp(ptr::null_mut());
        }
    }
    fn native() -> (Interp, Rc<CountingHost>) {
        let host = Rc::new(CountingHost {
            actual: crate::interp::default_host(),
            doubles: RefCell::new(Vec::new()),
        });
        let profile = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
            .expect("known independently selected original C8.4 core")
            .unit_profile();
        let interp = Interp::with_native_core(
            host.clone(),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("genuine native bootstrap");
        host.doubles.borrow_mut().clear();
        (interp, host)
    }
    fn install(interp: &mut Interp, state: &CallbackState) {
        let client = ptr::from_ref(state).cast_mut().cast();
        assert!(interp
            .create_obj_command(
                b"::boolean_probe",
                ObjCommand::new(original_callback, client, None)
            )
            .is_some());
    }
    fn empty() -> TclCompletionAbi {
        TclCompletionAbi {
            code: -99,
            result: ptr::null_mut(),
            options: ptr::null_mut(),
        }
    }
    fn release(completion: TclCompletionAbi) {
        // SAFETY: each successful settlement transfers one reference per handle.
        unsafe {
            obj::decr_ref_count(completion.result);
            obj::decr_ref_count(completion.options);
        }
    }
    fn same_completion(actual: &TclCompletionAbi, before: &TclCompletionAbi) {
        assert_eq!(actual.code, before.code);
        assert_eq!(actual.result, before.result);
        assert_eq!(actual.options, before.options);
    }

    #[test]
    fn original_combined_expression_truth_runs_public_producer_once() {
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        // This software integration counts real Host.double calls reached by
        // an actual original object command; external provider rows are separate.
        for route in 0..3 {
            let (mut interp, host) = native();
            let state = CallbackState {
                calls: Cell::new(0),
                mode: CallbackMode::True,
            };
            install(&mut interp, &state);
            let source = Owned::fresh(obj::new_string_bytes(b"[::boolean_probe]"));
            let refs = unsafe { (*source.as_ptr()).ref_count };
            let _entry = CurrentEntry::enter(&mut interp);
            let mut completion = empty();
            let mut truth = 777;
            let status = unsafe {
                if route == 2 {
                    tcl_codegen_expr_bool(source.as_ptr(), &mut completion, &mut truth)
                } else {
                    tcl_codegen_expr_eval(source.as_ptr(), &mut completion)
                }
            };
            assert_eq!(status, TCL_NATIVE_ABI_OK);
            assert_eq!(completion.code, 0);
            assert_eq!(obj::bytes_of(completion.result), b"true");
            assert!(!completion.options.is_null());
            if route == 1 {
                // Deliberately compose the old two public operations as a
                // discriminator. Each independently runs its real producer.
                assert_eq!(
                    unsafe {
                        crate::codegen_abi::tcl_value_get_expression_bool(
                    completion.result,
                    tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi as i32,
                    &mut truth,
                )
                    },
                    0
                );
            }
            assert_eq!(truth, if route == 0 { 777 } else { 1 });
            assert_eq!(state.calls.get(), 1);
            let expected = vec![(b"true".to_vec(), true); if route == 1 { 2 } else { 1 }];
            assert_eq!(*host.doubles.borrow(), expected, "route {route}");
            assert_eq!(
                unsafe { (*source.as_ptr()).ref_count },
                refs,
                "source is borrowed"
            );
            assert!(!interp.host_refusal_pending());
            release(completion);
        }
    }

    #[test]
    fn original_expression_completion_preserves_guest_codes_and_rejects_changed_worlds() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for combined in [false, true] {
            for code in [1, 3, 7] {
                let (mut interp, host) = native();
                let state = CallbackState {
                    calls: Cell::new(0),
                    mode: CallbackMode::Guest(code),
                };
                install(&mut interp, &state);
                let source = Owned::fresh(obj::new_string_bytes(b"[::boolean_probe]"));
                let _entry = CurrentEntry::enter(&mut interp);
                let mut completion = empty();
                let mut truth = 777;
                let status = unsafe {
                    if combined {
                        tcl_codegen_expr_bool(source.as_ptr(), &mut completion, &mut truth)
                    } else {
                        tcl_codegen_expr_eval(source.as_ptr(), &mut completion)
                    }
                };
                assert_eq!(status, TCL_NATIVE_ABI_OK);
                assert_eq!(completion.code, code);
                assert_eq!(obj::bytes_of(completion.result), b"GUEST\0\xff");
                let option_code = crate::dict::dict_get(completion.options, b"-code")
                    .expect("actual owned completion dictionary")
                    .expect("exact Guest code option");
                assert_eq!(obj::bytes_of(option_code), code.to_string().as_bytes());
                if code == 1 {
                    let error_code = crate::dict::dict_get(completion.options, b"-errorcode")
                        .expect("actual owned completion dictionary")
                        .expect("original explicit error-code option");
                    assert_eq!(obj::bytes_of(error_code), b"ORIGINAL CALLBACK CODE");
                }
                assert_eq!(truth, 777);
                assert_eq!(state.calls.get(), 1);
                assert!(host.doubles.borrow().is_empty());
                assert!(!interp.host_refusal_pending());
                release(completion);
            }
            for code in [0, 7] {
                let (mut interp, host) = native();
                let state = CallbackState {
                    calls: Cell::new(0),
                    mode: CallbackMode::ChangeWorld(code),
                };
                install(&mut interp, &state);
                let world = interp.runtime_context();
                let source = Owned::fresh(obj::new_string_bytes(b"[::boolean_probe]"));
                let sentinel = Owned::fresh(obj::new_string_bytes(b"UNTOUCHED"));
                let before = TclCompletionAbi {
                    code: -99,
                    result: sentinel.as_ptr(),
                    options: sentinel.as_ptr(),
                };
                let mut completion = before;
                let mut truth = 777;
                let _entry = CurrentEntry::enter(&mut interp);
                let status = unsafe {
                    if combined {
                        tcl_codegen_expr_bool(source.as_ptr(), &mut completion, &mut truth)
                    } else {
                        tcl_codegen_expr_eval(source.as_ptr(), &mut completion)
                    }
                };
                assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
                same_completion(&completion, &before);
                assert_eq!(truth, 777);
                assert_eq!(state.calls.get(), 1, "earlier callback effect is retained");
                assert_eq!(
                    interp.runtime_context(),
                    world,
                    "restoration cannot revive the entry"
                );
                assert_eq!(interp.result_bytes(), b"true");
                assert!(
                    host.doubles.borrow().is_empty(),
                    "no fresh result producer after re-entry"
                );
                assert_eq!(interp.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation"))));
            }
        }
    }

    #[test]
    fn original_combined_condition_keeps_return_continue_and_false_completion() {
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        for mode in [
            CallbackMode::False,
            CallbackMode::Guest(2),
            CallbackMode::Guest(4),
        ] {
            let (mut interp, host) = native();
            let state = CallbackState {
                calls: Cell::new(0),
                mode,
            };
            install(&mut interp, &state);
            let source = Owned::fresh(obj::new_string_bytes(b"[::boolean_probe]"));
            let _entry = CurrentEntry::enter(&mut interp);
            let mut completion = empty();
            let mut truth = 777;
            assert_eq!(
                unsafe { tcl_codegen_expr_bool(source.as_ptr(), &mut completion, &mut truth) },
                TCL_NATIVE_ABI_OK
            );
            let (code, option_code, level) = match mode {
                CallbackMode::False => {
                    assert_eq!(truth, 0);
                    assert_eq!(
                        host.doubles.borrow().len(),
                        1,
                        "one actual public result producer"
                    );
                    (0, b"0".as_slice(), b"0".as_slice())
                }
                CallbackMode::Guest(code) => {
                    assert_eq!(truth, 777);
                    assert_eq!(obj::bytes_of(completion.result), b"GUEST\0\xff");
                    assert!(
                        host.doubles.borrow().is_empty(),
                        "Guest completion has no truth stage"
                    );
                    if code == 2 {
                        (2, b"0".as_slice(), b"1".as_slice())
                    } else {
                        (4, b"4".as_slice(), b"0".as_slice())
                    }
                }
                _ => unreachable!(),
            };
            assert_eq!(completion.code, code);
            for (key, expected) in [
                (b"-code".as_slice(), option_code),
                (b"-level".as_slice(), level),
            ] {
                let value = crate::dict::dict_get(completion.options, key)
                    .expect("actual completion dictionary")
                    .expect("original option");
                assert_eq!(obj::bytes_of(value), expected);
            }
            assert_eq!(state.calls.get(), 1);
            assert!(!interp.host_refusal_pending());
            release(completion);
        }
    }

    #[test]
    fn original_codegen_completion_exports_preserve_first_host_and_output_storage() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for route in 0..4 {
            let (mut interp, host) = native();
            let state = CallbackState {
                calls: Cell::new(0),
                mode: CallbackMode::True,
            };
            install(&mut interp, &state);
            let source = Owned::fresh(obj::new_string_bytes(b"[::boolean_probe]"));
            let operand = Owned::fresh(obj::new_string_bytes(b"true"));
            let argv = [operand.as_ptr()];
            interp.set_result_bytes(b"PRIOR\0\xff");
            let prior = interp.result_obj();
            interp.refuse_host_command("original completion first cause");
            let first = interp.native_execution_refusal().expect("typed first Host");
            let before = TclCompletionAbi {
                code: -99,
                result: source.as_ptr(),
                options: operand.as_ptr(),
            };
            let mut completion = before;
            let mut truth = 777;
            let _entry = CurrentEntry::enter(&mut interp);
            let status = unsafe {
                match route {
                    0 => tcl_codegen_expr_eval(source.as_ptr(), &mut completion),
                    1 => tcl_codegen_expr_bool(source.as_ptr(), &mut completion, &mut truth),
                    2 => tcl_codegen_mathop(b"!".as_ptr(), 1, argv.as_ptr(), 1, &mut completion),
                    _ => {
                        tcl_codegen_mathfunc(b"abs".as_ptr(), 3, argv.as_ptr(), 1, &mut completion)
                    }
                }
            };
            assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
            same_completion(&completion, &before);
            assert_eq!(truth, 777);
            assert_eq!(state.calls.get(), 0);
            assert!(host.doubles.borrow().is_empty());
            assert_eq!(interp.result_obj(), prior);
            assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
            assert_eq!(interp.native_execution_refusal(), Some(first));
        }
    }

    thread_local! { static METADATA_GETTERS: Cell<usize> = const { Cell::new(0) }; }
    extern "C" fn refusing_metadata(original: *mut TclObj) {
        METADATA_GETTERS.with(|calls| calls.set(calls.get() + 1));
        // The genuine extension updater reaches a first Host cause and finishes
        // its own bytes. Those bytes cannot authorize completion publication.
        let interp = current_interp();
        assert!(!interp.is_null());
        unsafe {
            (*interp).refuse_host_command("first cause in completion metadata getter");
            obj::set_string_rep(original, b"REACHED ORIGINAL BYTES");
        }
    }
    static REFUSING_METADATA: obj::TclObjType = obj::TclObjType {
        name: c"originalCompletionMetadataRefusal".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(refusing_metadata),
        set_from_any_proc: None,
    };

    #[test]
    fn original_codegen_completion_capture_refusal_releases_handles_without_publication() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut interp, _) = native();
        let original = Owned::fresh(obj::alloc_typed(&REFUSING_METADATA, 0));
        interp.set_error_state(b"ORIGINAL COMPLETION CODE");
        interp.set_result(original.as_ptr());
        let refs = unsafe { (*original.as_ptr()).ref_count };
        let before = TclCompletionAbi {
            code: -99,
            result: original.as_ptr(),
            options: original.as_ptr(),
        };
        let mut completion = before;
        let _entry = CurrentEntry::enter(&mut interp);
        let currency = NativeOperationCurrency::issue(&interp).unwrap();
        METADATA_GETTERS.with(|calls| calls.set(0));
        assert_eq!(
            unsafe { write_completion(&mut interp, &currency, Code::Error, &mut completion) },
            TCL_INVOKE_ABI_HOST_REFUSED
        );
        same_completion(&completion, &before);
        METADATA_GETTERS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(
            unsafe { (*original.as_ptr()).ref_count },
            refs,
            "capture retained handles are released"
        );
        assert_eq!(interp.result_obj(), original.as_ptr());
        assert_eq!(obj::bytes_of(original.as_ptr()), b"REACHED ORIGINAL BYTES");
        assert!(interp.host_refusal_pending());
    }
}
