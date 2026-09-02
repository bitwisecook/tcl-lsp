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
const I32_I32_I32_I32_I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 6];
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
}

impl CodegenAbiImportId {
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
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::{
        CodegenAbiImportId, CodegenAbiValueType, I32, I64, WASM32_COMPLETION_ALIGN,
        WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
        WASM32_COMPLETION_RESULT_OFFSET, WASM32_COMPLETION_SIZE, WASM32_GUARD_DOMAINS_SIZE,
        WASM32_GUARD_IDENTITY_ALIGN, WASM32_GUARD_IDENTITY_NAMESPACE_OFFSET,
        WASM32_GUARD_IDENTITY_SIZE, WASM32_GUARD_IDENTITY_VALUE_OFFSET, WASM32_GUARD_TOKEN_ALIGN,
        WASM32_GUARD_TOKEN_SIZE, WASM32_INTRINSIC_ID_SIZE,
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
        assert!(enter.parameters.is_empty());
        assert_eq!(enter.results, I32);

        let leave = CodegenAbiImportId::ActivationLeave.descriptor();
        assert_eq!(leave.module, "tcl");
        assert_eq!(leave.name, "tcl_codegen_activation_leave");
        assert_eq!(leave.parameters, I32);
        assert!(leave.results.is_empty());
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
}
