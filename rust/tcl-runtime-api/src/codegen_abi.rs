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

//! Target-neutral vocabulary for the compiler/runtime WASM code-generation ABI.
//!
//! Concrete runtimes export these imports over their shared linear memory, and
//! target emitters lower the descriptors to their native IR. This crate owns
//! the spelling, wasm32 layout, and signatures so neither side grows a parallel
//! copy of the transport contract.

/// A scalar ABI value type used by the current WASM import subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenAbiValueType {
    /// A wasm32 integer, pointer, length, status, or Tcl completion code.
    I32,
    /// A WASM i64 scalar, used for guard tokens and stable identity values.
    I64,
    /// A WASM f64 scalar — the native representation of a Tcl double at a
    /// typed-value boundary.
    F64,
}

/// One compiler/runtime import descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodegenAbiImport {
    /// The runtime import module.
    pub module: &'static str,
    /// The stable import field name.
    pub name: &'static str,
    /// Parameter types in call order.
    pub parameters: &'static [CodegenAbiValueType],
    /// Result types in return order.
    pub results: &'static [CodegenAbiValueType],
}

const I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32];
const I64: &[CodegenAbiValueType] = &[CodegenAbiValueType::I64];
const I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 2];
const I32_I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 3];
const I32_I32_I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 4];
const I32_I32_I32_I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 5];
const I32_I32_I32_I32_I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 6];
const I32_I32_I32_I32_I32_I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 7];
const I32_I32_I32_I32_I64_I32: &[CodegenAbiValueType] = &[
    CodegenAbiValueType::I32,
    CodegenAbiValueType::I32,
    CodegenAbiValueType::I32,
    CodegenAbiValueType::I32,
    CodegenAbiValueType::I64,
    CodegenAbiValueType::I32,
];
const I32_I64_I32: &[CodegenAbiValueType] = &[
    CodegenAbiValueType::I32,
    CodegenAbiValueType::I64,
    CodegenAbiValueType::I32,
];
const I64_I32_I32_I32: &[CodegenAbiValueType] = &[
    CodegenAbiValueType::I64,
    CodegenAbiValueType::I32,
    CodegenAbiValueType::I32,
    CodegenAbiValueType::I32,
];
const F64: &[CodegenAbiValueType] = &[CodegenAbiValueType::F64];
const NONE: &[CodegenAbiValueType] = &[];

const fn tcl_import(
    name: &'static str,
    parameters: &'static [CodegenAbiValueType],
    results: &'static [CodegenAbiValueType],
) -> CodegenAbiImport {
    CodegenAbiImport {
        module: "tcl",
        name,
        parameters,
        results,
    }
}

/// Compiler/runtime code-generation imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenAbiImportId {
    /// Allocate a re-entrant transient shared-memory call frame.
    CallFrameAlloc,
    /// Free a transient shared-memory call frame using its runtime-recorded layout.
    CallFrameFree,
    /// Copy bytes to a Tcl object with one caller-owned reference.
    NewOwnedString,
    /// Dispatch one fully evaluated argv vector.
    InvokeArgv,
    /// Release the owned result/options pair stored in a completion triple.
    CompletionRelease,
    /// Duplicate a Tcl object owned reference for completion forwarding.
    ObjectRetain,
    /// Release one Tcl object owned reference.
    ObjectRelease,
    /// Construct a Tcl object from bytes in linear memory.
    ObjectNewString,
    /// Evaluate a boxed Tcl script object.
    EvalCode,
    /// Evaluate Tcl expression text as a boolean.
    ExprBool,
    /// Construct an analysed value from bytes in linear memory.
    ValueNewString,
    /// Materialise a native signed 64-bit value as an owned Tcl wide integer.
    ValueNewWideInt,
    /// Enter an analysed-code lexical frame.
    FramePush,
    /// Leave an analysed-code lexical frame.
    FramePop,
    /// Bind a local slot in an analysed-code lexical frame.
    LocalBind,
    /// Assign an analysed-code local slot.
    LocalSet,
    /// Read an analysed-code local slot.
    LocalGet,
    /// Assign a Tcl variable through analysed-code lowering.
    VarSet,
    /// Read a Tcl variable through analysed-code lowering.
    VarGet,
    /// Add two analysed numeric values.
    ExprAdd,
    /// Write one analysed value to stdout.
    Puts,
    /// Register an analysed Tcl procedure.
    ProcRegister,
    /// Create an interpreter for a standalone module.
    RuntimeCreateInterp,
    /// Select the current interpreter for a standalone module.
    RuntimeSetCurrentInterp,
    /// Initialise the Tcl library for a standalone module.
    RuntimeInitLibrary,
    /// Write the runtime's own identity manifest — what it states of itself, as
    /// `ArtefactIdentityManifest::to_bytes` encodes it — to a caller buffer, and
    /// answer its byte length (written only when the buffer holds it). `0` when
    /// no interpreter is current.
    RuntimeIdentity,
    /// Prepare a runtime-issued guard for one registry intrinsic implementation.
    ///
    /// Parameters are the intrinsic stable ID, fully evaluated argv pointer
    /// and argc, expected identity namespace and value, and canonical domain
    /// mask. The runtime resolves that argv subject after substitutions. The
    /// returned i64 is an opaque guard token, not an epoch.
    GuardPrepare,
    /// Re-check a guard token against the current implementation of one intrinsic.
    ///
    /// Parameters are the opaque i64 token, intrinsic stable ID, and the same
    /// fully evaluated argv pointer and argc. The runtime re-resolves that
    /// subject before returning its i32 boolean admission decision.
    GuardCheck,
    /// Release one runtime-issued opaque guard token.
    GuardRelease,
    /// Invoke one registry intrinsic over a fully evaluated, boxed argv.
    ///
    /// Parameters are the intrinsic stable ID, argv handles pointer, argc, and
    /// caller-owned standard completion-triple output storage.
    /// The i32 result is ABI status; the output holds the owned completion
    /// triple on every non-null output path, matching [`Self::InvokeArgv`].
    InvokeIntrinsicArgv,
    /// Read one Tcl array element as an owned generated-word value.
    VarGetElement,
    /// Join evaluated word parts into one owned Tcl value.
    WordConcat,
    /// Materialise a native `f64` as an owned Tcl double.
    ValueNewDouble,
    /// Materialise a native boolean as an owned Tcl boolean (an integer `0`/`1`).
    ValueNewBool,
    /// Read a Tcl value as a native `i64`, writing it through an out pointer.
    ///
    /// Parameters are the borrowed value handle and an `i64` out pointer; the
    /// i32 result is `0` on success, `1` when the runtime set a Tcl error on
    /// the interpreter and left the out storage untouched. A successful read
    /// caches the parsed internal rep onto the value.
    ValueGetWideInt,
    /// Read a Tcl value as a native `f64` — [`Self::ValueGetWideInt`]'s
    /// contract over an `f64` out pointer.
    ValueGetDouble,
    /// Read a Tcl value in boolean context — [`Self::ValueGetWideInt`]'s
    /// contract over an `i32` (`0`/`1`) out pointer.
    ValueGetBool,
    /// Convert an already reached operand for its exact Boolean instruction.
    /// Parameters are the borrowed value handle, a native truth purpose and
    /// an i32 out pointer. The result is the typed getter status. This direct
    /// operand conversion does not normalise an expression result.
    ValueGetBoolForPurpose,
    /// Produce and convert an actual held expression result. Parameters are
    /// the borrowed result handle, its inline/public expression production and
    /// an i32 out pointer. The runtime retains result-normalisation ownership
    /// before the final conversion; no operand purpose can grant this stage.
    ValueGetExpressionBool,
    /// Bind an indexed compiled slot to a named frame cell and store a value.
    ///
    /// The ABI v2 spelling of [`Self::LocalBind`]; both address the same cell.
    SlotBind,
    /// Assign the variable an indexed compiled slot addresses.
    SlotSet,
    /// Read the variable an indexed compiled slot addresses.
    SlotGet,
    /// `incr` the variable an indexed compiled slot addresses by a native i64,
    /// writing the new value through an `i64` out pointer.
    ///
    /// The i32 result is `0` on success, `1` when the runtime set a Tcl error
    /// on the interpreter (a non-integer cell, a `const`, a write-trace error,
    /// or a new value past the wide range).
    SlotIncrI64,
    /// `append` a value onto the variable an indexed compiled slot addresses;
    /// the i32 result is the Tcl completion code.
    SlotAppend,
    /// `lappend` a value onto the variable an indexed compiled slot addresses;
    /// the i32 result is the Tcl completion code.
    SlotLappend,
    /// Whether any variable trace can observe accesses to a named variable —
    /// the runtime half of a guarded `TraceBarrier`.
    ///
    /// Parameters are the name pointer and length; the i32 result is `1` when
    /// traced and `0` when not. `0` is a promise that nothing observes the
    /// cell, so generated code may take its native path.
    VarTraced,
    /// [`Self::VarTraced`] for the variable an indexed compiled slot addresses.
    SlotTraced,
    /// Enter a compiled activation, so compiled code counts as an eval-loop
    /// activation for the outermost-eval error-publication rule.
    ///
    /// Takes no parameters and returns an i32 status: zero on success (the
    /// caller owes exactly one [`Self::ActivationLeave`]), non-zero when the
    /// activation was refused and **no** activation is held.
    ActivationEnter,
    /// Leave a compiled activation, passing the activation's Tcl completion
    /// code so the outermost one publishes an uncaught error's trace.
    ActivationLeave,
    /// Store an owned value into the array element `name(key)`; the element
    /// half of [`Self::VarSet`]. Parameters are the name pointer and length,
    /// the key pointer and length, and the adopted value; the i32 result is
    /// the Tcl completion code.
    VarSetElement,
    /// Tcl `incr` on a named variable by a borrowed boxed delta, returning
    /// the cell's new value with one owned reference, or null with the
    /// interpreter carrying the Tcl error.
    VarIncr,
    /// Tcl `append` (`list` = 0) or `lappend` (`list` = 1) of `argc`
    /// borrowed values (an argv pointer) onto a named variable; the i32
    /// result is the Tcl completion code.
    VarUpdate,
    /// Read a boxed value as a native `i64` when it has that representation,
    /// writing it through an `i64` out pointer. The i32 result is `1` when
    /// the value was written and `0` when it is not a wide integer — and
    /// then **no** interpreter error is set, unlike [`Self::ValueGetWideInt`].
    ValueTryWideInt,
    /// [`Self::ValueTryWideInt`] over an `f64` out pointer.
    ValueTryDouble,
    /// Evaluate a borrowed boxed expression object with the runtime's
    /// expression evaluator, writing the completion triple to the
    /// caller-owned completion storage. The i32 result is ABI status.
    ExprEval,
    /// Apply the `expr` operator spelled by a name pointer and length to
    /// `argc` borrowed operands (an argv pointer) through the runtime's
    /// `::tcl::mathop` implementation, writing the completion triple.
    MathOp,
    /// Call `::tcl::mathfunc::<name>` (name pointer and length) through
    /// ordinary command dispatch over `argc` borrowed arguments (an argv
    /// pointer), writing the completion triple.
    MathFunc,
    /// Define a Tcl procedure whose body may run natively.
    ///
    /// Parameters are the name pointer and length, the formal-parameter text
    /// pointer and length, the body-source pointer and length, and `entry` —
    /// a wasm32 function-table index for the compiled body, or `0` for none.
    /// The i32 result is a Tcl completion code.
    ///
    /// `entry = 0` is exactly [`Self::ProcRegister`]: a source-only proc. The
    /// two spellings define the same proc, and this one subsumes the older,
    /// which stays for already-emitted legacy-tier modules.
    ProcDefineNative,
    /// Log one `while executing` / `invoked from within` `errorInfo` frame for
    /// a compiled statement that completed with an error.
    ///
    /// Parameters are the statement's 1-based line relative to the enclosing
    /// body and the statement's exact source text (pointer and length). No
    /// result: the runtime owns the `already_logged` protocol, so a statement
    /// already logged deeper in the same body is a no-op.
    LogCommand,
    /// Record the pending `return -level`/`-code` state, exactly as the
    /// `return` command records it. Parameters are the level and the Tcl
    /// completion code. No result.
    ///
    /// A compiled `return` completes with code `2` without dispatching the
    /// `return` command, so nothing else would write this state and the
    /// procedure's return boundary (`Interp::settle_return`) would consume
    /// whatever an earlier `return -level N` left behind.
    ReturnState,
    /// The number of proc bodies dispatched through a native entry.
    ///
    /// A test boundary, like [`Self::CallFrameAlloc`]'s outstanding-frame
    /// counter: it lets a linked test prove the native entry actually ran
    /// rather than the source body, which is otherwise unobservable because
    /// both produce the same Tcl result.
    NativeProcDispatches,
    /// Whether a host-only refusal is retained after a reached operation.
    /// This does not consume the refusal or publish a Tcl completion.
    HostRefusalPending,
}

impl CodegenAbiImportId {
    /// Check the retained host channel before using this call's result or
    /// entering Tcl completion handling. New operation imports require the
    /// check unless their descriptor explicitly proves a transport-only role.
    #[must_use]
    pub const fn requires_host_refusal_check(self) -> bool {
        !matches!(
            self,
            Self::CallFrameAlloc
                | Self::CallFrameFree
                | Self::NewOwnedString
                | Self::CompletionRelease
                | Self::ObjectRetain
                | Self::ObjectRelease
                | Self::ObjectNewString
                | Self::ValueNewString
                | Self::ValueNewWideInt
                | Self::ValueNewDouble
                | Self::ValueNewBool
                | Self::FramePush
                | Self::FramePop
                | Self::GuardRelease
                | Self::ActivationEnter
                | Self::ActivationLeave
                | Self::LogCommand
                | Self::ReturnState
                | Self::NativeProcDispatches
                | Self::HostRefusalPending
        )
    }
    /// Every import, in declaration order: the table [`CODEGEN_ABI_VERSION`] is
    /// derived from.
    pub const ALL: &'static [Self] = &[
        Self::CallFrameAlloc,
        Self::CallFrameFree,
        Self::NewOwnedString,
        Self::InvokeArgv,
        Self::CompletionRelease,
        Self::ObjectRetain,
        Self::ObjectRelease,
        Self::ObjectNewString,
        Self::EvalCode,
        Self::ExprBool,
        Self::ValueNewString,
        Self::ValueNewWideInt,
        Self::FramePush,
        Self::FramePop,
        Self::LocalBind,
        Self::LocalSet,
        Self::LocalGet,
        Self::VarSet,
        Self::VarGet,
        Self::ExprAdd,
        Self::Puts,
        Self::ProcRegister,
        Self::RuntimeCreateInterp,
        Self::RuntimeSetCurrentInterp,
        Self::RuntimeInitLibrary,
        Self::RuntimeIdentity,
        Self::GuardPrepare,
        Self::GuardCheck,
        Self::GuardRelease,
        Self::InvokeIntrinsicArgv,
        Self::VarGetElement,
        Self::WordConcat,
        Self::ValueNewDouble,
        Self::ValueNewBool,
        Self::ValueGetWideInt,
        Self::ValueGetDouble,
        Self::ValueGetBool,
        Self::ValueGetBoolForPurpose,
        Self::ValueGetExpressionBool,
        Self::SlotBind,
        Self::SlotSet,
        Self::SlotGet,
        Self::SlotIncrI64,
        Self::SlotAppend,
        Self::SlotLappend,
        Self::VarTraced,
        Self::SlotTraced,
        Self::ActivationEnter,
        Self::ActivationLeave,
        Self::VarSetElement,
        Self::VarIncr,
        Self::VarUpdate,
        Self::ValueTryWideInt,
        Self::ValueTryDouble,
        Self::ExprEval,
        Self::MathOp,
        Self::MathFunc,
        Self::ProcDefineNative,
        Self::LogCommand,
        Self::ReturnState,
        Self::NativeProcDispatches,
        Self::HostRefusalPending,
    ];

    /// Return this import's shared ABI descriptor.
    #[must_use]
    pub const fn descriptor(self) -> CodegenAbiImport {
        match self {
            Self::CallFrameAlloc => tcl_import("tcl_codegen_call_frame_alloc", I32_I32, I32),
            Self::CallFrameFree => tcl_import("tcl_codegen_call_frame_free", I32, I32),
            Self::NewOwnedString => tcl_import("tcl_obj_new_string_owned", I32_I32, I32),
            Self::InvokeArgv => tcl_import("tcl_invoke_argv", I32_I32_I32, I32),
            Self::CompletionRelease => tcl_import("tcl_completion_release", I32, NONE),
            Self::ObjectRetain => tcl_import("tcl_obj_retain", I32, I32),
            Self::ObjectRelease => tcl_import("tcl_obj_release", I32, NONE),
            Self::ObjectNewString => tcl_import("tcl_obj_new_string", I32_I32, I32),
            Self::EvalCode => tcl_import("tcl_eval_code", I32, I32),
            Self::ExprBool => tcl_import("tcl_expr_bool", I32, I32),
            Self::ValueNewString => tcl_import("tcl_value_new_string", I32_I32, I32),
            Self::ValueNewWideInt => tcl_import("tcl_value_new_wide_int", I64, I32),
            Self::FramePush => tcl_import("tcl_codegen_frame_push", NONE, NONE),
            Self::FramePop => tcl_import("tcl_codegen_frame_pop", NONE, NONE),
            Self::LocalBind => tcl_import("tcl_codegen_local_bind", I32_I32_I32_I32, I32),
            Self::LocalSet => tcl_import("tcl_codegen_local_set", I32_I32, I32),
            Self::LocalGet => tcl_import("tcl_codegen_local_get", I32, I32),
            Self::VarSet => tcl_import("tcl_codegen_var_set", I32_I32_I32, I32),
            Self::VarGet => tcl_import("tcl_codegen_var_get", I32_I32, I32),
            Self::ExprAdd => tcl_import("tcl_codegen_expr_add", I32_I32, I32),
            Self::Puts => tcl_import("tcl_codegen_puts", I32, I32),
            Self::ProcRegister => {
                tcl_import("tcl_codegen_proc_register", I32_I32_I32_I32_I32_I32, I32)
            }
            Self::RuntimeCreateInterp => tcl_import("tcl_runtime_create_interp", NONE, I32),
            Self::RuntimeSetCurrentInterp => {
                tcl_import("tcl_runtime_set_current_interp", I32, NONE)
            }
            Self::RuntimeInitLibrary => tcl_import("tcl_runtime_init_library", NONE, I32),
            Self::RuntimeIdentity => tcl_import("tcl_runtime_identity", I32_I32, I32),
            Self::GuardPrepare => {
                tcl_import("tcl_codegen_guard_prepare", I32_I32_I32_I32_I64_I32, I64)
            }
            Self::GuardCheck => tcl_import("tcl_codegen_guard_check", I64_I32_I32_I32, I32),
            Self::GuardRelease => tcl_import("tcl_codegen_guard_release", I64, NONE),
            Self::InvokeIntrinsicArgv => {
                tcl_import("tcl_intrinsic_invoke_argv", I32_I32_I32_I32, I32)
            }
            Self::VarGetElement => CodegenAbiImport {
                module: "tcl",
                name: "tcl_codegen_var_get_element",
                parameters: I32_I32_I32_I32,
                results: I32,
            },
            Self::WordConcat => CodegenAbiImport {
                module: "tcl",
                name: "tcl_codegen_word_concat",
                parameters: I32_I32,
                results: I32,
            },
            Self::ValueNewDouble => tcl_import("tcl_value_new_double", F64, I32),
            Self::ValueNewBool => tcl_import("tcl_value_new_bool", I32, I32),
            Self::ValueGetWideInt => tcl_import("tcl_value_get_wide_int", I32_I32, I32),
            Self::ValueGetDouble => tcl_import("tcl_value_get_double", I32_I32, I32),
            Self::ValueGetBool => tcl_import("tcl_value_get_bool", I32_I32, I32),
            Self::ValueGetBoolForPurpose => {
                tcl_import("tcl_value_get_bool_for_purpose", I32_I32_I32, I32)
            }
            Self::ValueGetExpressionBool => {
                tcl_import("tcl_value_get_expression_bool", I32_I32_I32, I32)
            }
            Self::SlotBind => tcl_import("tcl_codegen_slot_bind", I32_I32_I32_I32, I32),
            Self::SlotSet => tcl_import("tcl_codegen_slot_set", I32_I32, I32),
            Self::SlotGet => tcl_import("tcl_codegen_slot_get", I32, I32),
            Self::SlotIncrI64 => tcl_import("tcl_codegen_slot_incr_i64", I32_I64_I32, I32),
            Self::SlotAppend => tcl_import("tcl_codegen_slot_append", I32_I32, I32),
            Self::SlotLappend => tcl_import("tcl_codegen_slot_lappend", I32_I32, I32),
            Self::VarTraced => tcl_import("tcl_codegen_var_traced", I32_I32, I32),
            Self::SlotTraced => tcl_import("tcl_codegen_slot_traced", I32, I32),
            Self::ActivationEnter => tcl_import("tcl_codegen_activation_enter", NONE, I32),
            Self::ActivationLeave => tcl_import("tcl_codegen_activation_leave", I32, NONE),
            Self::VarSetElement => {
                tcl_import("tcl_codegen_var_set_element", I32_I32_I32_I32_I32, I32)
            }
            Self::VarIncr => tcl_import("tcl_codegen_var_incr", I32_I32_I32, I32),
            Self::VarUpdate => tcl_import("tcl_codegen_var_update", I32_I32_I32_I32_I32, I32),
            Self::ValueTryWideInt => tcl_import("tcl_codegen_value_try_wide_int", I32_I32, I32),
            Self::ValueTryDouble => tcl_import("tcl_codegen_value_try_double", I32_I32, I32),
            Self::ExprEval => tcl_import("tcl_codegen_expr_eval", I32_I32, I32),
            Self::MathOp => tcl_import("tcl_codegen_mathop", I32_I32_I32_I32_I32, I32),
            Self::MathFunc => tcl_import("tcl_codegen_mathfunc", I32_I32_I32_I32_I32, I32),
            Self::ProcDefineNative => tcl_import(
                "tcl_codegen_proc_define_native",
                I32_I32_I32_I32_I32_I32_I32,
                I32,
            ),
            Self::LogCommand => tcl_import("tcl_codegen_log_command", I32_I32_I32, NONE),
            Self::ReturnState => tcl_import("tcl_codegen_return_state", I32_I32, NONE),
            Self::NativeProcDispatches => {
                tcl_import("tcl_codegen_native_proc_dispatches", NONE, I32)
            }
            Self::HostRefusalPending => tcl_import("tcl_codegen_host_refusal_pending", NONE, I32),
        }
    }
}

/// The runtime's exported wasm32 indirect function table.
///
/// A wasm32 function pointer is an index into this table, so a module that
/// wants the runtime to call one of its own functions installs a `ref.func`
/// here and hands the runtime the index. `wasm-ld` picks the name; the runtime
/// publishes it with `--export-table` (`runtime/rust/build.rs`). This constant
/// is the single spelling both sides use, so a toolchain rename is one edit.
pub const WASM32_FUNCTION_TABLE_IMPORT: &str = "__indirect_function_table";

/// A native proc entry ran and wrote its completion output.
///
/// The body's own Tcl error is reported as `code == 1` in that completion, not
/// through this status.
pub const NATIVE_PROC_STATUS_RAN: i32 = 0;

/// A native proc entry declined, **before any observable effect**, and left
/// its completion output untouched.
///
/// The runtime then runs the proc's source body in the call frame it has
/// already prepared. That is only observationally identical because nothing
/// happened yet, so an entry must decline before its first ABI call that
/// writes a cell, dispatches a command, or sets a result — never part-way
/// through a body.
pub const NATIVE_PROC_STATUS_DECLINED: i32 = 1;

/// A reached operation retained a host-only refusal. Completion output is
/// untouched; the caller unwinds without guest capture or source fallback.
/// Effects that precede the refusal must never execute again.
pub const NATIVE_PROC_STATUS_HOST_REFUSED: i32 = 2;

/// wasm32 linear-memory pointer width.
pub const WASM32_POINTER_BYTES: i32 = 4;
/// First byte of the immutable data window reserved for generated wasm32 code.
///
/// The runtime's downward-growing shadow stack occupies the preceding MiB.
pub const WASM32_CODEGEN_DATA_START: i64 = 0x10_0000;
/// Exclusive end of the immutable data window reserved for generated wasm32 code.
///
/// Linked runtime data starts here (`wasm-ld --global-base=0x20_0000`), so the
/// complete generated constant pool must stay below this address.
pub const WASM32_CODEGEN_DATA_END: i64 = 0x20_0000;
/// Offset of `TclCompletionAbi.code`.
pub const WASM32_COMPLETION_CODE_OFFSET: i32 = 0;
/// Offset of `TclCompletionAbi.result`.
pub const WASM32_COMPLETION_RESULT_OFFSET: i32 = 4;
/// Offset of `TclCompletionAbi.options`.
pub const WASM32_COMPLETION_OPTIONS_OFFSET: i32 = 8;
/// Size of the wasm32 `TclCompletionAbi` transport layout.
pub const WASM32_COMPLETION_SIZE: i32 = 12;
/// Alignment of the wasm32 `TclCompletionAbi` transport layout.
pub const WASM32_COMPLETION_ALIGN: i32 = 4;
/// Width of an opaque runtime-issued guard token in the wasm32 ABI.
pub const WASM32_GUARD_TOKEN_SIZE: i32 = 8;
/// Alignment of an opaque runtime-issued guard token in wasm32 linear memory.
pub const WASM32_GUARD_TOKEN_ALIGN: i32 = 8;
/// Width of a registry intrinsic stable scalar in the wasm32 ABI.
pub const WASM32_INTRINSIC_ID_SIZE: i32 = 4;
/// Width of a zero-extended [`GuardDomains`](crate::guard::GuardDomains) mask in the wasm32 ABI.
pub const WASM32_GUARD_DOMAINS_SIZE: i32 = 4;
/// Offset of the u32 identity-vocabulary namespace in a materialised guard identity.
pub const WASM32_GUARD_IDENTITY_NAMESPACE_OFFSET: i32 = 0;
/// Offset of the u64 stable identity value in a materialised guard identity.
pub const WASM32_GUARD_IDENTITY_VALUE_OFFSET: i32 = 8;
/// Size of a materialised guard identity, including the four-byte alignment gap.
pub const WASM32_GUARD_IDENTITY_SIZE: i32 = 16;
/// Alignment of a materialised guard identity.
pub const WASM32_GUARD_IDENTITY_ALIGN: i32 = 8;

/// The version of this code-generation ABI: a fingerprint of every import's
/// module, name and signature (in [`CodegenAbiImportId::ALL`] order) and of the
/// wasm32 transport constants in [`LAYOUT`].
///
/// Derived rather than counted, so an import that changes shape, appears or
/// goes cannot leave the version where it was. An artefact records the version
/// it was emitted against (`ArtefactIdentityManifest::abi_version`) and a
/// runtime refuses one that names another.
pub const CODEGEN_ABI_VERSION: u32 = fingerprint(&DESCRIPTORS, &LAYOUT);

const IMPORT_COUNT: usize = CodegenAbiImportId::ALL.len();

/// Every import's descriptor, in [`CodegenAbiImportId::ALL`] order.
const DESCRIPTORS: [CodegenAbiImport; IMPORT_COUNT] = {
    let mut descriptors = [CodegenAbiImportId::ALL[0].descriptor(); IMPORT_COUNT];
    let mut index = 0;
    while index < IMPORT_COUNT {
        descriptors[index] = CodegenAbiImportId::ALL[index].descriptor();
        index += 1;
    }
    descriptors
};

const FNV_OFFSET: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

/// The wasm32 transport constants [`CODEGEN_ABI_VERSION`] folds after the
/// imports: the name of the function table a native entry is called through,
/// the 32-bit statuses, sizes and offsets, and the 64-bit data window.
struct Layout<'a> {
    table_import: &'a str,
    words: &'a [i32],
    wide: &'a [i64],
}

const LAYOUT: Layout<'static> = Layout {
    table_import: WASM32_FUNCTION_TABLE_IMPORT,
    words: &[
        NATIVE_PROC_STATUS_RAN,
        NATIVE_PROC_STATUS_DECLINED,
        NATIVE_PROC_STATUS_HOST_REFUSED,
        WASM32_POINTER_BYTES,
        WASM32_COMPLETION_CODE_OFFSET,
        WASM32_COMPLETION_RESULT_OFFSET,
        WASM32_COMPLETION_OPTIONS_OFFSET,
        WASM32_COMPLETION_SIZE,
        WASM32_COMPLETION_ALIGN,
        WASM32_GUARD_TOKEN_SIZE,
        WASM32_GUARD_TOKEN_ALIGN,
        WASM32_INTRINSIC_ID_SIZE,
        WASM32_GUARD_DOMAINS_SIZE,
        WASM32_GUARD_IDENTITY_NAMESPACE_OFFSET,
        WASM32_GUARD_IDENTITY_VALUE_OFFSET,
        WASM32_GUARD_IDENTITY_SIZE,
        WASM32_GUARD_IDENTITY_ALIGN,
    ],
    wide: &[WASM32_CODEGEN_DATA_START, WASM32_CODEGEN_DATA_END],
};

const fn fold(mut hash: u32, bytes: &[u8]) -> u32 {
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
        index += 1;
    }
    hash
}

const fn fold_types(mut hash: u32, types: &[CodegenAbiValueType]) -> u32 {
    let mut index = 0;
    while index < types.len() {
        let code = match types[index] {
            CodegenAbiValueType::I32 => 1,
            CodegenAbiValueType::I64 => 2,
            CodegenAbiValueType::F64 => 3,
        };
        hash = fold(hash, &[code]);
        index += 1;
    }
    hash
}

/// The fingerprint of `imports` and the layout.
const fn fingerprint(imports: &[CodegenAbiImport], layout: &Layout) -> u32 {
    let mut hash = FNV_OFFSET;
    let mut index = 0;
    while index < imports.len() {
        let import = imports[index];
        hash = fold(hash, import.module.as_bytes());
        hash = fold(hash, &[0]);
        hash = fold(hash, import.name.as_bytes());
        hash = fold(hash, &[0]);
        hash = fold_types(hash, import.parameters);
        hash = fold(hash, &[0xff]);
        hash = fold_types(hash, import.results);
        hash = fold(hash, &[0xfe]);
        index += 1;
    }
    hash = fold(hash, layout.table_import.as_bytes());
    let mut word = 0;
    while word < layout.words.len() {
        hash = fold(hash, &layout.words[word].to_le_bytes());
        word += 1;
    }
    word = 0;
    while word < layout.wide.len() {
        hash = fold(hash, &layout.wide[word].to_le_bytes());
        word += 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{
        CODEGEN_ABI_VERSION, CodegenAbiImport, CodegenAbiImportId, CodegenAbiValueType,
        DESCRIPTORS, I32, I64, LAYOUT, Layout, NATIVE_PROC_STATUS_DECLINED,
        NATIVE_PROC_STATUS_HOST_REFUSED, NATIVE_PROC_STATUS_RAN, WASM32_COMPLETION_ALIGN,
        WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
        WASM32_COMPLETION_RESULT_OFFSET, WASM32_COMPLETION_SIZE, WASM32_FUNCTION_TABLE_IMPORT,
        WASM32_GUARD_DOMAINS_SIZE, WASM32_GUARD_IDENTITY_ALIGN,
        WASM32_GUARD_IDENTITY_NAMESPACE_OFFSET, WASM32_GUARD_IDENTITY_SIZE,
        WASM32_GUARD_IDENTITY_VALUE_OFFSET, WASM32_GUARD_TOKEN_ALIGN, WASM32_GUARD_TOKEN_SIZE,
        WASM32_INTRINSIC_ID_SIZE, fingerprint,
    };

    #[test]
    fn legacy_and_general_import_descriptors_preserve_the_wasm_abi() {
        let expected = [
            (
                CodegenAbiImportId::ObjectNewString,
                "tcl_obj_new_string",
                2,
                1,
            ),
            (CodegenAbiImportId::EvalCode, "tcl_eval_code", 1, 1),
            (CodegenAbiImportId::ExprBool, "tcl_expr_bool", 1, 1),
            (
                CodegenAbiImportId::ValueNewString,
                "tcl_value_new_string",
                2,
                1,
            ),
            (
                CodegenAbiImportId::FramePush,
                "tcl_codegen_frame_push",
                0,
                0,
            ),
            (CodegenAbiImportId::FramePop, "tcl_codegen_frame_pop", 0, 0),
            (
                CodegenAbiImportId::LocalBind,
                "tcl_codegen_local_bind",
                4,
                1,
            ),
            (CodegenAbiImportId::LocalSet, "tcl_codegen_local_set", 2, 1),
            (CodegenAbiImportId::LocalGet, "tcl_codegen_local_get", 1, 1),
            (CodegenAbiImportId::VarSet, "tcl_codegen_var_set", 3, 1),
            (CodegenAbiImportId::VarGet, "tcl_codegen_var_get", 2, 1),
            (CodegenAbiImportId::ExprAdd, "tcl_codegen_expr_add", 2, 1),
            (CodegenAbiImportId::Puts, "tcl_codegen_puts", 1, 1),
            (
                CodegenAbiImportId::ProcRegister,
                "tcl_codegen_proc_register",
                6,
                1,
            ),
            (
                CodegenAbiImportId::RuntimeCreateInterp,
                "tcl_runtime_create_interp",
                0,
                1,
            ),
            (
                CodegenAbiImportId::RuntimeSetCurrentInterp,
                "tcl_runtime_set_current_interp",
                1,
                0,
            ),
            (
                CodegenAbiImportId::RuntimeInitLibrary,
                "tcl_runtime_init_library",
                0,
                1,
            ),
        ];

        for (id, name, parameter_count, result_count) in expected {
            let descriptor = id.descriptor();
            assert_eq!(descriptor.module, "tcl", "{id:?}");
            assert_eq!(descriptor.name, name, "{id:?}");
            assert_eq!(descriptor.parameters.len(), parameter_count, "{id:?}");
            assert_eq!(descriptor.results.len(), result_count, "{id:?}");
            assert!(
                descriptor
                    .parameters
                    .iter()
                    .chain(descriptor.results)
                    .all(|value| *value == CodegenAbiValueType::I32),
                "{id:?} must remain a wasm32 ABI import"
            );
        }
    }

    #[test]
    fn native_wide_int_materialisation_uses_the_owned_i64_to_object_abi() {
        let descriptor = CodegenAbiImportId::ValueNewWideInt.descriptor();
        assert_eq!(descriptor.module, "tcl");
        assert_eq!(descriptor.name, "tcl_value_new_wide_int");
        assert_eq!(descriptor.parameters, I64);
        assert_eq!(descriptor.results, I32);
    }

    #[test]
    fn guarded_intrinsic_imports_keep_identity_token_and_completion_abi() {
        use CodegenAbiValueType::{I32, I64};

        let expected = [
            (
                CodegenAbiImportId::GuardPrepare,
                "tcl_codegen_guard_prepare",
                &[I32, I32, I32, I32, I64, I32][..],
                &[I64][..],
            ),
            (
                CodegenAbiImportId::GuardCheck,
                "tcl_codegen_guard_check",
                &[I64, I32, I32, I32][..],
                &[I32][..],
            ),
            (
                CodegenAbiImportId::GuardRelease,
                "tcl_codegen_guard_release",
                &[I64][..],
                &[][..],
            ),
            (
                CodegenAbiImportId::InvokeIntrinsicArgv,
                "tcl_intrinsic_invoke_argv",
                &[I32, I32, I32, I32][..],
                &[I32][..],
            ),
        ];

        for (id, name, parameters, results) in expected {
            let descriptor = id.descriptor();
            assert_eq!(descriptor.module, "tcl", "{id:?}");
            assert_eq!(descriptor.name, name, "{id:?}");
            assert_eq!(descriptor.parameters, parameters, "{id:?}");
            assert_eq!(descriptor.results, results, "{id:?}");
        }
    }

    #[test]
    fn typed_value_imports_carry_native_scalars_and_an_out_pointer_status() {
        use CodegenAbiValueType::{F64, I32};

        let new_double = CodegenAbiImportId::ValueNewDouble.descriptor();
        assert_eq!(new_double.name, "tcl_value_new_double");
        assert_eq!(new_double.parameters, &[F64][..]);
        assert_eq!(new_double.results, &[I32][..]);

        let new_bool = CodegenAbiImportId::ValueNewBool.descriptor();
        assert_eq!(new_bool.name, "tcl_value_new_bool");
        assert_eq!(new_bool.parameters, &[I32][..]);
        assert_eq!(new_bool.results, &[I32][..]);

        // Every getter is (value handle, out pointer) -> status, so generated
        // code has one shape to lower for all three.
        for (id, name) in [
            (
                CodegenAbiImportId::ValueGetWideInt,
                "tcl_value_get_wide_int",
            ),
            (CodegenAbiImportId::ValueGetDouble, "tcl_value_get_double"),
            (CodegenAbiImportId::ValueGetBool, "tcl_value_get_bool"),
        ] {
            let descriptor = id.descriptor();
            assert_eq!(descriptor.module, "tcl", "{id:?}");
            assert_eq!(descriptor.name, name, "{id:?}");
            assert_eq!(descriptor.parameters, &[I32, I32][..], "{id:?}");
            assert_eq!(descriptor.results, &[I32][..], "{id:?}");
        }
    }

    #[test]
    fn original_boolean_imports_separate_operand_purpose_from_expression_production() {
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        use CodegenAbiValueType::I32;
        for (id, name) in [
            (
                CodegenAbiImportId::ValueGetBoolForPurpose,
                "tcl_value_get_bool_for_purpose",
            ),
            (
                CodegenAbiImportId::ValueGetExpressionBool,
                "tcl_value_get_expression_bool",
            ),
        ] {
            let descriptor = id.descriptor();
            assert_eq!(descriptor.module, "tcl");
            assert_eq!(descriptor.name, name);
            assert_eq!(descriptor.parameters, &[I32, I32, I32]);
            assert_eq!(descriptor.results, &[I32]);
            assert!(CodegenAbiImportId::ALL.contains(&id));
            assert!(id.requires_host_refusal_check());
        }
    }

    #[test]
    fn indexed_slot_imports_address_a_cell_and_keep_the_legacy_local_spellings() {
        use CodegenAbiValueType::{I32, I64};

        let expected = [
            (
                CodegenAbiImportId::SlotBind,
                "tcl_codegen_slot_bind",
                &[I32, I32, I32, I32][..],
            ),
            (
                CodegenAbiImportId::SlotSet,
                "tcl_codegen_slot_set",
                &[I32, I32][..],
            ),
            (
                CodegenAbiImportId::SlotGet,
                "tcl_codegen_slot_get",
                &[I32][..],
            ),
            (
                CodegenAbiImportId::SlotIncrI64,
                "tcl_codegen_slot_incr_i64",
                &[I32, I64, I32][..],
            ),
            (
                CodegenAbiImportId::SlotAppend,
                "tcl_codegen_slot_append",
                &[I32, I32][..],
            ),
            (
                CodegenAbiImportId::SlotLappend,
                "tcl_codegen_slot_lappend",
                &[I32, I32][..],
            ),
        ];
        for (id, name, parameters) in expected {
            let descriptor = id.descriptor();
            assert_eq!(descriptor.module, "tcl", "{id:?}");
            assert_eq!(descriptor.name, name, "{id:?}");
            assert_eq!(descriptor.parameters, parameters, "{id:?}");
            assert_eq!(descriptor.results, &[I32][..], "{id:?}");
        }

        // The `local_*` spellings an already-emitted module imports keep their
        // exact shape; the two families address the same indexed cell.
        for (slot_id, local_id) in [
            (CodegenAbiImportId::SlotBind, CodegenAbiImportId::LocalBind),
            (CodegenAbiImportId::SlotSet, CodegenAbiImportId::LocalSet),
            (CodegenAbiImportId::SlotGet, CodegenAbiImportId::LocalGet),
        ] {
            assert_eq!(
                slot_id.descriptor().parameters,
                local_id.descriptor().parameters,
                "{slot_id:?} / {local_id:?}"
            );
            assert_eq!(
                slot_id.descriptor().results,
                local_id.descriptor().results,
                "{slot_id:?} / {local_id:?}"
            );
        }
    }

    #[test]
    fn trace_barrier_imports_answer_a_boolean_for_a_name_or_a_slot() {
        use CodegenAbiValueType::I32;

        let by_name = CodegenAbiImportId::VarTraced.descriptor();
        assert_eq!(by_name.module, "tcl");
        assert_eq!(by_name.name, "tcl_codegen_var_traced");
        assert_eq!(by_name.parameters, &[I32, I32][..]);
        assert_eq!(by_name.results, &[I32][..]);

        let by_slot = CodegenAbiImportId::SlotTraced.descriptor();
        assert_eq!(by_slot.module, "tcl");
        assert_eq!(by_slot.name, "tcl_codegen_slot_traced");
        assert_eq!(by_slot.parameters, &[I32][..]);
        assert_eq!(by_slot.results, &[I32][..]);
    }

    #[test]
    fn compiled_activation_imports_bracket_generated_code_with_a_status_and_a_code() {
        let enter = CodegenAbiImportId::ActivationEnter.descriptor();
        assert_eq!(enter.module, "tcl");
        assert_eq!(enter.name, "tcl_codegen_activation_enter");
        assert_eq!(enter.parameters, []);
        assert_eq!(enter.results, I32);

        let leave = CodegenAbiImportId::ActivationLeave.descriptor();
        assert_eq!(leave.module, "tcl");
        assert_eq!(leave.name, "tcl_codegen_activation_leave");
        assert_eq!(leave.parameters, I32);
        assert_eq!(leave.results, []);
    }

    #[test]
    fn native_tier_imports_carry_names_argv_and_completion_storage() {
        use CodegenAbiValueType::I32;
        let expected = [
            (
                CodegenAbiImportId::VarSetElement,
                "tcl_codegen_var_set_element",
                5,
            ),
            (CodegenAbiImportId::VarIncr, "tcl_codegen_var_incr", 3),
            (CodegenAbiImportId::VarUpdate, "tcl_codegen_var_update", 5),
            (
                CodegenAbiImportId::ValueTryWideInt,
                "tcl_codegen_value_try_wide_int",
                2,
            ),
            (
                CodegenAbiImportId::ValueTryDouble,
                "tcl_codegen_value_try_double",
                2,
            ),
            (CodegenAbiImportId::ExprEval, "tcl_codegen_expr_eval", 2),
            (CodegenAbiImportId::MathOp, "tcl_codegen_mathop", 5),
            (CodegenAbiImportId::MathFunc, "tcl_codegen_mathfunc", 5),
        ];
        for (id, name, parameters) in expected {
            let descriptor = id.descriptor();
            assert_eq!(descriptor.module, "tcl", "{id:?}");
            assert_eq!(descriptor.name, name, "{id:?}");
            assert_eq!(descriptor.parameters.len(), parameters, "{id:?}");
            assert!(descriptor.parameters.iter().all(|value| *value == I32));
            assert_eq!(descriptor.results, &[I32][..], "{id:?}");
        }
    }

    #[test]
    fn native_proc_dispatch_imports_carry_the_entry_index_and_a_test_counter() {
        use CodegenAbiValueType::I32;

        // The definition import is `proc_register` plus one trailing entry
        // index, so a module that binds a native body and one that does not
        // emit the same call with `entry = 0`.
        let define = CodegenAbiImportId::ProcDefineNative.descriptor();
        assert_eq!(define.module, "tcl");
        assert_eq!(define.name, "tcl_codegen_proc_define_native");
        assert_eq!(define.parameters, &[I32; 7][..]);
        assert_eq!(define.results, &[I32][..]);
        let register = CodegenAbiImportId::ProcRegister.descriptor();
        assert_eq!(
            define.parameters.len(),
            register.parameters.len() + 1,
            "proc_define_native is proc_register plus the entry index"
        );
        assert_eq!(define.results, register.results);

        // The error-edge logger returns nothing: the runtime owns the
        // `already_logged` protocol, so the emitter has no decision to make.
        let log = CodegenAbiImportId::LogCommand.descriptor();
        assert_eq!(log.module, "tcl");
        assert_eq!(log.name, "tcl_codegen_log_command");
        assert_eq!(log.parameters, &[I32; 3][..]);
        assert_eq!(log.results, []);

        // The pending-return-state writer takes the same (level, code) pair
        // the `return` command records, and answers nothing: it is a state
        // write, not a completion.
        let state = CodegenAbiImportId::ReturnState.descriptor();
        assert_eq!(state.module, "tcl");
        assert_eq!(state.name, "tcl_codegen_return_state");
        assert_eq!(state.parameters, &[I32; 2][..]);
        assert_eq!(state.results, []);

        let dispatches = CodegenAbiImportId::NativeProcDispatches.descriptor();
        assert_eq!(dispatches.module, "tcl");
        assert_eq!(dispatches.name, "tcl_codegen_native_proc_dispatches");
        assert_eq!(dispatches.parameters, []);
        assert_eq!(dispatches.results, &[I32][..]);
    }

    #[test]
    fn host_refusal_query_is_distinct_from_guest_completion_and_cleanup() {
        let query = CodegenAbiImportId::HostRefusalPending.descriptor();
        assert_eq!(query.module, "tcl");
        assert_eq!(query.name, "tcl_codegen_host_refusal_pending");
        assert_eq!(query.parameters, [] as [CodegenAbiValueType; 0]);
        assert_eq!(query.results, I32);
        for call in [
            CodegenAbiImportId::InvokeArgv,
            CodegenAbiImportId::ValueGetWideInt,
            CodegenAbiImportId::VarGet,
            CodegenAbiImportId::GuardCheck,
        ] {
            assert!(call.requires_host_refusal_check());
        }
        for call in [
            CodegenAbiImportId::HostRefusalPending,
            CodegenAbiImportId::ObjectRelease,
            CodegenAbiImportId::CallFrameFree,
        ] {
            assert!(!call.requires_host_refusal_check());
        }
    }

    #[test]
    fn the_runtime_identity_import_takes_a_buffer_and_answers_a_length() {
        use CodegenAbiValueType::I32;

        let identity = CodegenAbiImportId::RuntimeIdentity.descriptor();
        assert_eq!(identity.module, "tcl");
        assert_eq!(identity.name, "tcl_runtime_identity");
        assert_eq!(identity.parameters, &[I32, I32][..]);
        assert_eq!(identity.results, &[I32][..]);
    }

    #[test]
    fn native_proc_entry_statuses_and_the_table_import_name_have_one_spelling() {
        // `0` is "ran" so a zeroed status word is never mistaken for a
        // decline, matching every other ABI status in this file.
        assert_eq!(NATIVE_PROC_STATUS_RAN, 0);
        assert_eq!(NATIVE_PROC_STATUS_DECLINED, 1);
        assert_eq!(NATIVE_PROC_STATUS_HOST_REFUSED, 2);
        assert_ne!(NATIVE_PROC_STATUS_HOST_REFUSED, NATIVE_PROC_STATUS_DECLINED);
        assert_ne!(NATIVE_PROC_STATUS_HOST_REFUSED, NATIVE_PROC_STATUS_RAN);
        assert_ne!(NATIVE_PROC_STATUS_RAN, NATIVE_PROC_STATUS_DECLINED);
        // wasm-ld's name for the table `--export-table` publishes.
        assert_eq!(WASM32_FUNCTION_TABLE_IMPORT, "__indirect_function_table");
    }

    #[test]
    fn guard_and_completion_layouts_are_explicit_for_wasm32_transport() {
        assert_eq!(WASM32_GUARD_TOKEN_SIZE, 8);
        assert_eq!(WASM32_GUARD_TOKEN_ALIGN, 8);
        assert_eq!(WASM32_INTRINSIC_ID_SIZE, 4);
        assert_eq!(WASM32_GUARD_DOMAINS_SIZE, 4);
        assert_eq!(WASM32_GUARD_IDENTITY_NAMESPACE_OFFSET, 0);
        assert_eq!(WASM32_GUARD_IDENTITY_VALUE_OFFSET, 8);
        assert_eq!(WASM32_GUARD_IDENTITY_SIZE, 16);
        assert_eq!(WASM32_GUARD_IDENTITY_ALIGN, 8);

        assert_eq!(WASM32_COMPLETION_CODE_OFFSET, 0);
        assert_eq!(WASM32_COMPLETION_RESULT_OFFSET, 4);
        assert_eq!(WASM32_COMPLETION_OPTIONS_OFFSET, 8);
        assert_eq!(WASM32_COMPLETION_SIZE, 12);
        assert_eq!(WASM32_COMPLETION_ALIGN, 4);
    }

    #[test]
    fn every_import_is_in_the_abi_table() {
        // The version is derived from `ALL`, so an import missing from it would
        // leave the version where it was. The declaration is the list the table
        // must equal, read from this file so a variant added without a row here
        // fails rather than passes.
        let source = include_str!("codegen_abi.rs");
        let body = source
            .split("pub enum CodegenAbiImportId {")
            .nth(1)
            .and_then(|rest| rest.split("\n}\n").next())
            .expect("the enum's body is in this file");
        let declared: Vec<&str> = body
            .lines()
            .filter_map(|line| line.strip_prefix("    "))
            .filter(|line| {
                line.ends_with(',') && line.starts_with(|c: char| c.is_ascii_uppercase())
            })
            .map(|line| line.trim_end_matches(','))
            .collect();
        let listed: Vec<String> = CodegenAbiImportId::ALL
            .iter()
            .map(|id| format!("{id:?}"))
            .collect();
        assert_eq!(listed, declared);
    }

    #[test]
    fn the_abi_version_moves_with_the_table() {
        assert_eq!(fingerprint(&DESCRIPTORS, &LAYOUT), CODEGEN_ABI_VERSION);

        let mut dropped = DESCRIPTORS.to_vec();
        dropped.pop();
        assert_ne!(
            fingerprint(&dropped, &LAYOUT),
            CODEGEN_ABI_VERSION,
            "an import goes"
        );

        let mut reordered = DESCRIPTORS.to_vec();
        reordered.swap(0, 1);
        assert_ne!(
            fingerprint(&reordered, &LAYOUT),
            CODEGEN_ABI_VERSION,
            "order"
        );

        let with = |edit: fn(CodegenAbiImport) -> CodegenAbiImport| {
            let mut edited = DESCRIPTORS.to_vec();
            edited[3] = edit(edited[3]);
            fingerprint(&edited, &LAYOUT)
        };
        assert_ne!(
            with(|import| CodegenAbiImport {
                name: "tcl_invoke_argv2",
                ..import
            }),
            CODEGEN_ABI_VERSION,
            "a name"
        );
        assert_ne!(
            with(|import| CodegenAbiImport {
                module: "other",
                ..import
            }),
            CODEGEN_ABI_VERSION,
            "a module"
        );
        assert_ne!(
            with(|import| CodegenAbiImport {
                parameters: &[CodegenAbiValueType::I64],
                ..import
            }),
            CODEGEN_ABI_VERSION,
            "a parameter type"
        );
        assert_ne!(
            with(|import| CodegenAbiImport {
                results: &[],
                ..import
            }),
            CODEGEN_ABI_VERSION,
            "a result"
        );

        let mut words = LAYOUT.words.to_vec();
        words[0] += 1;
        assert_ne!(
            fingerprint(
                &DESCRIPTORS,
                &Layout {
                    words: &words,
                    ..LAYOUT
                }
            ),
            CODEGEN_ABI_VERSION,
            "a 32-bit layout constant"
        );
        let mut wide = LAYOUT.wide.to_vec();
        wide[1] += 1;
        assert_ne!(
            fingerprint(
                &DESCRIPTORS,
                &Layout {
                    wide: &wide,
                    ..LAYOUT
                }
            ),
            CODEGEN_ABI_VERSION,
            "the data window"
        );
        assert_ne!(
            fingerprint(
                &DESCRIPTORS,
                &Layout {
                    table_import: "table",
                    ..LAYOUT
                }
            ),
            CODEGEN_ABI_VERSION,
            "the function table import"
        );
    }

    #[test]
    fn every_layout_constant_is_in_the_abi_fingerprint() {
        // A constant missing from `LAYOUT` could change without the version
        // moving, so the declarations are read out of this file and each is held
        // to appear in the table the version folds.
        let source = include_str!("codegen_abi.rs");
        let table = source
            .split("const LAYOUT: Layout<'static> = Layout {")
            .nth(1)
            .and_then(|rest| rest.split("\n};\n").next())
            .expect("the layout table is in this file");
        let declared: Vec<&str> = source
            .lines()
            .filter_map(|line| line.strip_prefix("pub const "))
            .filter_map(|line| line.split(':').next())
            .filter(|name| name.starts_with("WASM32_") || name.starts_with("NATIVE_PROC_STATUS_"))
            .collect();
        assert!(declared.len() >= 10, "the scan found the declarations");
        for name in declared {
            assert!(table.contains(name), "{name} is not in the ABI fingerprint");
        }
    }
}
