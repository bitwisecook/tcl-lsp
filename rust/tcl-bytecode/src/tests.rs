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

#[test]
fn private_original_literal_has_ordered_slot_without_string_receipt() {
    let mut table = super::LiteralTable::new();
    let string = table.intern_bytes(b"VALUE");
    let original = table.register_private_original();
    assert_eq!((string, original), (0, 1));
    assert_eq!(
        table.entries()[string].byte_payload(),
        Some(b"VALUE".as_slice())
    );
    assert_eq!(table.entries()[original].byte_payload(), None);
    assert_ne!(table.entries()[original], "");
    assert_eq!(
        table.entries()[original].allocation(),
        &super::NativeLiteralAllocation::PrivateOriginal
    );
    assert_eq!(
        table.native_actions(),
        &[
            super::NativeLiteralAction::Register(0),
            super::NativeLiteralAction::Register(1)
        ]
    );
}
use super::*;
use tcl_lexer::Span;
use tcl_syntax::expr::ast::{BinOp, UnaryOp};

#[test]
fn replay_context_rejects_inconsistent_component_geometry() {
    let path = tcl_runtime_api::ByteNamespacePath::from_segments(["a:", "b"]);
    let other = tcl_runtime_api::ByteNamespacePath::from_segments(["a", ":b"]);
    let mut instruction = Instruction::new(Op::NOP, vec![]);
    instruction.source_command_namespace = path.clone();
    instruction.source_command_namespace_context = Some(
        tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(other),
    );
    let mut function = FunctionAsm {
        instructions: vec![instruction],
        ..FunctionAsm::default()
    };
    assert!(function.validate_native_compilation_entry().is_err());
    function.instructions[0].source_command_namespace_context = Some(
        tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(path.clone()),
    );
    assert_eq!(function.validate_native_compilation_entry(), Ok(()));
    assert_eq!(
        function.instructions[0].source_namespace_context().path(),
        &path
    );
}

#[test]
fn operation_replay_accepts_neutral_root_and_rejects_invalid_source_extent() {
    let mut instruction = Instruction::new(Op::NOP, vec![]);
    instruction.offset = 0;
    instruction
        .native_operation_selections
        .push(NativeOperationSelectionSite {
            compiler_prerequisite: None,
            requirements: vec![
                tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
                    "::", "string", "string",
                ),
            ],
            guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
            end: "end".to_owned(),
            source: tcl_lexer::SourceImage::document("string length abc"),
            span: Span::new(0, 17),
            namespace: tcl_runtime_api::ByteNamespacePath::root(),
            namespace_context: None,
        });
    let mut function = FunctionAsm {
        instructions: vec![instruction],
        labels: [("end".to_owned(), 1)].into(),
        ..FunctionAsm::default()
    };
    assert_eq!(function.validate_native_compilation_entry(), Ok(()));
    assert_eq!(
        function.instructions[0].native_operation_selections[0].namespace,
        tcl_runtime_api::ByteNamespacePath::root(),
    );
    function.instructions[0].native_operation_selections[0].span = Span::new(0, 16);
    assert!(function.validate_native_compilation_entry().is_err());
}

#[test]
fn parse_tcl_index_plain() {
    assert_eq!(parse_tcl_index("0"), Some(0));
    assert_eq!(parse_tcl_index("42"), Some(42));
    assert_eq!(parse_tcl_index("-1"), Some(-1));
}

#[test]
fn parse_tcl_index_end() {
    assert_eq!(parse_tcl_index("end"), Some(INDEX_END));
    assert_eq!(parse_tcl_index("end-1"), Some(INDEX_END - 1));
    assert_eq!(parse_tcl_index("end+2"), Some(INDEX_END + 2));
}

#[test]
fn parse_tcl_index_invalid() {
    assert_eq!(parse_tcl_index("foo"), None);
    assert_eq!(parse_tcl_index("end-abc"), None);
}

#[test]
fn end_relative_imm_stays_in_the_band() {
    assert_eq!(end_relative_imm(0), Some(INDEX_END));
    assert_eq!(end_relative_imm(-1), Some(INDEX_END - 1));
    assert_eq!(end_relative_imm(2), Some(INDEX_END + 2));
    assert_eq!(
        end_relative_imm(1 - i64::from(INDEX_END_BAND)),
        Some(INDEX_END - INDEX_END_BAND + 1)
    );
    assert_eq!(end_relative_imm(-i64::from(INDEX_END_BAND)), None);
    assert_eq!(end_relative_imm(-2_147_483_648), None);
    assert_eq!(end_relative_imm(-2_147_483_649), None);
}

#[test]
fn str_class_roundtrip() {
    for name in [
        "alnum", "alpha", "ascii", "control", "digit", "graph", "lower", "print", "punct", "space",
        "upper", "wordchar", "xdigit",
    ] {
        let id = str_class_id(name).unwrap();
        assert_eq!(str_class_name(id), Some(name));
    }
}

#[test]
fn op_mnemonic() {
    assert_eq!(Op::PUSH1.mnemonic(), "push1");
    assert_eq!(Op::ADD.mnemonic(), "add");
    assert_eq!(Op::IRULE_CONTAINS.mnemonic(), "iruleContains");
}

#[test]
fn portable_fixed_function_call_requires_its_original_registration() {
    assert_eq!(Op::CALL_FUNC1.mnemonic(), "callFunc1");
    assert_eq!(Op::CALL_FUNC1.size(), 2);
    assert!(!Op::CALL_FUNC1.is_one_byte());
    let function = FunctionAsm {
        instructions: vec![Instruction::new(Op::CALL_FUNC1, vec![Operand::Imm(1)])],
        ..Default::default()
    };
    assert_eq!(
        function.validate_native_compilation_entry(),
        Err(tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired)
    );
}

#[test]
fn op_family_routing_and_size() {
    // Spot-check one opcode from each `mnemonic_*` family so every routing
    // branch (core / arith / string / list_var / coro_oo / dict_misc) is
    // exercised and none reaches the `unreachable!` fallback; plus one
    // representative per size class for `size`/`is_one_byte`.
    assert_eq!(Op::PUSH1.mnemonic(), "push1");
    assert_eq!(Op::ADD.mnemonic(), "add");
    assert_eq!(Op::STR_EQ.mnemonic(), "streq");
    assert_eq!(Op::LIST.mnemonic(), "list");
    assert_eq!(Op::YIELD.mnemonic(), "yield");
    assert_eq!(Op::DICT_GET.mnemonic(), "dictGet");
    // size classes
    assert!(Op::ADD.is_one_byte());
    assert_eq!(Op::ADD.size(), 1);
    assert_eq!(Op::PUSH1.size(), 2);
    assert_eq!(Op::INCR_SCALAR1_IMM.size(), 3);
    assert_eq!(Op::PUSH4.size(), 5);
    assert_eq!(Op::INVOKE_REPLACE.size(), 6);
    assert_eq!(Op::RETURN_IMM.size(), 9);
    assert!(!Op::PUSH4.is_one_byte());
}

#[test]
fn op_display() {
    assert_eq!(format!("{}", Op::JUMP4), "jump4");
}

/// The variable/array/dict/introspection opcodes the VM implements without
/// codegen support yet: mnemonic and encoded size must match
/// `tclInstructionTable` (`tclCompile.c`) exactly, since the disassembler
/// and the byte layout are both derived from them.
#[test]
fn op_table_matches_c_instruction_table() {
    for (op, mnemonic, size) in [
        (Op::LOAD_SCALAR_STK, "loadScalarStk", 1),
        (Op::STORE_SCALAR_STK, "storeScalarStk", 1),
        (Op::LOAD_ARRAY4, "loadArray4", 5),
        (Op::STORE_ARRAY4, "storeArray4", 5),
        (Op::INCR_SCALAR_STK, "incrScalarStk", 1),
        (Op::INCR_SCALAR_STK_IMM, "incrScalarStkImm", 2),
        (Op::INCR_ARRAY1, "incrArray1", 2),
        (Op::INCR_ARRAY1_IMM, "incrArray1Imm", 3),
        (Op::INCR_ARRAY_STK, "incrArrayStk", 1),
        (Op::APPEND_ARRAY_STK, "appendArrayStk", 1),
        (Op::LAPPEND_ARRAY_STK, "lappendArrayStk", 1),
        (Op::EXIST_ARRAY, "existArray", 5),
        (Op::EXIST_ARRAY_STK, "existArrayStk", 1),
        (Op::UNSET_ARRAY_STK, "unsetArrayStk", 2),
        (Op::ARRAY_EXISTS_STK, "arrayExistsStk", 1),
        (Op::ARRAY_MAKE_IMM, "arrayMakeImm", 5),
        (Op::ARRAY_MAKE_STK, "arrayMakeStk", 1),
        (Op::VARIABLE, "variable", 5),
        (Op::CONST_IMM, "constImm", 5),
        (Op::CONST_STK, "constStk", 1),
        (Op::CURRENT_NAMESPACE, "currentNamespace", 1),
        (Op::INFO_LEVEL_NUM, "infoLevelNumber", 1),
        (Op::INFO_LEVEL_ARGS, "infoLevelArgs", 1),
        (Op::RESOLVE_CMD, "resolveCmd", 1),
        (Op::ORIGIN_CMD, "originCmd", 1),
        (Op::CLOCK_READ, "clockRead", 2),
        (Op::DICT_GET_DEF, "dictGetDef", 5),
        (Op::DICT_RECOMBINE_STK, "dictRecombineStk", 1),
        (Op::EXPAND_DROP, "expandDrop", 1),
        (Op::YIELD, "yield", 1),
        (Op::YIELD_TO_INVOKE, "yieldToInvoke", 1),
        (Op::CORO_NAME, "coroName", 1),
        (Op::TCLOO_SELF, "tclooSelf", 1),
        (Op::TCLOO_CLASS, "tclooClass", 1),
        (Op::TCLOO_NS, "tclooNamespace", 1),
        (Op::TCLOO_IS_OBJECT, "tclooIsObject", 1),
        (Op::TCLOO_ID, "tclooId", 1),
        (Op::TCLOO_NEXT, "tclooNext", 2),
        (Op::TCLOO_NEXT_CLASS, "tclooNextClass", 2),
        (Op::TCLOO_NEXT4, "tclooNext4", 5),
        (Op::TCLOO_NEXT_CLASS4, "tclooNextClass4", 5),
        (Op::TCLOO_NEXT_LIST, "tclooNextList", 1),
        (Op::TCLOO_NEXT_CLASS_LIST, "tclooNextClassList", 1),
    ] {
        assert_eq!(op.mnemonic(), mnemonic, "mnemonic of {op:?}");
        assert_eq!(op.size(), size, "size of {op:?}");
        assert_eq!(
            op.is_one_byte(),
            size == 1,
            "is_one_byte must agree with size for {op:?}"
        );
    }
}

/// Every opcode whose *first* operand is an LVT slot must say so, or the
/// disassembler renders the slot as a bare integer instead of `%vN`.
#[test]
fn op_is_lvt_covers_the_new_slot_operands() {
    for op in [
        Op::LOAD_ARRAY4,
        Op::STORE_ARRAY4,
        Op::INCR_ARRAY1,
        Op::INCR_ARRAY1_IMM,
        Op::EXIST_ARRAY,
        Op::ARRAY_MAKE_IMM,
        Op::VARIABLE,
        Op::CONST_IMM,
        Op::LAPPEND_LIST_ARRAY,
    ] {
        assert!(op.is_lvt_op(), "{op:?} takes an LVT slot as operand 0");
    }
    // The stack forms name their variable at run time — no slot operand.
    for op in [
        Op::LOAD_SCALAR_STK,
        Op::STORE_SCALAR_STK,
        Op::INCR_ARRAY_STK,
        Op::UNSET_ARRAY_STK,
        Op::ARRAY_MAKE_STK,
        Op::CONST_STK,
        Op::CLOCK_READ,
        Op::DICT_GET_DEF,
        // The coroutine/`TclOO` ops name nothing in the LVT: the
        // `tclooNext`/`tclooNextClass` operand is a stack word count.
        Op::TCLOO_NEXT,
        Op::TCLOO_NEXT_CLASS,
        Op::TCLOO_NEXT4,
        Op::TCLOO_NEXT_CLASS4,
        Op::TCLOO_NEXT_LIST,
        Op::TCLOO_NEXT_CLASS_LIST,
        Op::YIELD,
        Op::CORO_NAME,
        Op::TCLOO_SELF,
    ] {
        assert!(!op.is_lvt_op(), "{op:?} has no LVT operand");
    }
}

#[test]
fn op_is_lvt() {
    assert!(Op::LOAD_SCALAR1.is_lvt_op());
    assert!(!Op::ADD.is_lvt_op());
}

#[test]
fn op_is_jump() {
    assert!(Op::JUMP1.is_jump());
    assert!(Op::JUMP_FALSE4.is_jump());
    assert!(!Op::ADD.is_jump());
}

#[test]
fn op_from_binop() {
    assert_eq!(Op::from_binop(BinOp::Add), Some(Op::ADD));
    assert_eq!(Op::from_binop(BinOp::Contains), Some(Op::IRULE_CONTAINS));
}

/// The exhaustive routing gate the family helpers' doc comments promise:
/// every `Op` variant reaches exactly one `mnemonic_*` family and one
/// size class without hitting the `unreachable!` fallbacks. The `match`
/// below lists every variant, so adding an opcode without extending it —
/// and therefore without deciding its family and size — fails to compile
/// here first.
#[test]
#[allow(clippy::too_many_lines)] // one arm per Op variant — the length IS the exhaustiveness gate
fn opcode_family_partition_total() {
    #[allow(clippy::too_many_lines)] // ditto: the match must list every variant
    fn touch(op: Op) -> (&'static str, u8) {
        match op {
            Op::PUSH1 => (Op::PUSH1.mnemonic(), Op::PUSH1.size()),
            Op::PUSH4 => (Op::PUSH4.mnemonic(), Op::PUSH4.size()),
            Op::POP => (Op::POP.mnemonic(), Op::POP.size()),
            Op::DUP => (Op::DUP.mnemonic(), Op::DUP.size()),
            Op::LOAD_SCALAR1 => (Op::LOAD_SCALAR1.mnemonic(), Op::LOAD_SCALAR1.size()),
            Op::LOAD_SCALAR4 => (Op::LOAD_SCALAR4.mnemonic(), Op::LOAD_SCALAR4.size()),
            Op::STORE_SCALAR1 => (Op::STORE_SCALAR1.mnemonic(), Op::STORE_SCALAR1.size()),
            Op::STORE_SCALAR4 => (Op::STORE_SCALAR4.mnemonic(), Op::STORE_SCALAR4.size()),
            Op::INCR_SCALAR1 => (Op::INCR_SCALAR1.mnemonic(), Op::INCR_SCALAR1.size()),
            Op::INCR_SCALAR1_IMM => (Op::INCR_SCALAR1_IMM.mnemonic(), Op::INCR_SCALAR1_IMM.size()),
            Op::INVOKE_STK1 => (Op::INVOKE_STK1.mnemonic(), Op::INVOKE_STK1.size()),
            Op::INVOKE_STK4 => (Op::INVOKE_STK4.mnemonic(), Op::INVOKE_STK4.size()),
            Op::CALL_FUNC1 => (Op::CALL_FUNC1.mnemonic(), Op::CALL_FUNC1.size()),
            Op::EVAL_STK => (Op::EVAL_STK.mnemonic(), Op::EVAL_STK.size()),
            Op::EXPR_STK => (Op::EXPR_STK.mnemonic(), Op::EXPR_STK.size()),
            Op::JUMP1 => (Op::JUMP1.mnemonic(), Op::JUMP1.size()),
            Op::JUMP4 => (Op::JUMP4.mnemonic(), Op::JUMP4.size()),
            Op::JUMP_TRUE1 => (Op::JUMP_TRUE1.mnemonic(), Op::JUMP_TRUE1.size()),
            Op::JUMP_TRUE4 => (Op::JUMP_TRUE4.mnemonic(), Op::JUMP_TRUE4.size()),
            Op::JUMP_FALSE1 => (Op::JUMP_FALSE1.mnemonic(), Op::JUMP_FALSE1.size()),
            Op::JUMP_FALSE4 => (Op::JUMP_FALSE4.mnemonic(), Op::JUMP_FALSE4.size()),
            Op::ADD => (Op::ADD.mnemonic(), Op::ADD.size()),
            Op::SUB => (Op::SUB.mnemonic(), Op::SUB.size()),
            Op::MULT => (Op::MULT.mnemonic(), Op::MULT.size()),
            Op::DIV => (Op::DIV.mnemonic(), Op::DIV.size()),
            Op::MOD => (Op::MOD.mnemonic(), Op::MOD.size()),
            Op::EXPON => (Op::EXPON.mnemonic(), Op::EXPON.size()),
            Op::LSHIFT => (Op::LSHIFT.mnemonic(), Op::LSHIFT.size()),
            Op::RSHIFT => (Op::RSHIFT.mnemonic(), Op::RSHIFT.size()),
            Op::BITOR => (Op::BITOR.mnemonic(), Op::BITOR.size()),
            Op::BITXOR => (Op::BITXOR.mnemonic(), Op::BITXOR.size()),
            Op::BITAND => (Op::BITAND.mnemonic(), Op::BITAND.size()),
            Op::EQ => (Op::EQ.mnemonic(), Op::EQ.size()),
            Op::NEQ => (Op::NEQ.mnemonic(), Op::NEQ.size()),
            Op::LT => (Op::LT.mnemonic(), Op::LT.size()),
            Op::GT => (Op::GT.mnemonic(), Op::GT.size()),
            Op::LE => (Op::LE.mnemonic(), Op::LE.size()),
            Op::GE => (Op::GE.mnemonic(), Op::GE.size()),
            Op::STR_EQ => (Op::STR_EQ.mnemonic(), Op::STR_EQ.size()),
            Op::STR_NEQ => (Op::STR_NEQ.mnemonic(), Op::STR_NEQ.size()),
            Op::STR_CMP => (Op::STR_CMP.mnemonic(), Op::STR_CMP.size()),
            Op::STR_LT => (Op::STR_LT.mnemonic(), Op::STR_LT.size()),
            Op::STR_GT => (Op::STR_GT.mnemonic(), Op::STR_GT.size()),
            Op::STR_LE => (Op::STR_LE.mnemonic(), Op::STR_LE.size()),
            Op::STR_GE => (Op::STR_GE.mnemonic(), Op::STR_GE.size()),
            Op::STR_CONCAT1 => (Op::STR_CONCAT1.mnemonic(), Op::STR_CONCAT1.size()),
            Op::STR_LEN => (Op::STR_LEN.mnemonic(), Op::STR_LEN.size()),
            Op::STR_INDEX => (Op::STR_INDEX.mnemonic(), Op::STR_INDEX.size()),
            Op::LIST => (Op::LIST.mnemonic(), Op::LIST.size()),
            Op::LIST_LENGTH => (Op::LIST_LENGTH.mnemonic(), Op::LIST_LENGTH.size()),
            Op::LIST_INDEX => (Op::LIST_INDEX.mnemonic(), Op::LIST_INDEX.size()),
            Op::LIST_INDEX_IMM => (Op::LIST_INDEX_IMM.mnemonic(), Op::LIST_INDEX_IMM.size()),
            Op::LIST_RANGE_IMM => (Op::LIST_RANGE_IMM.mnemonic(), Op::LIST_RANGE_IMM.size()),
            Op::LINDEX_MULTI => (Op::LINDEX_MULTI.mnemonic(), Op::LINDEX_MULTI.size()),
            Op::APPEND_SCALAR1 => (Op::APPEND_SCALAR1.mnemonic(), Op::APPEND_SCALAR1.size()),
            Op::APPEND_SCALAR4 => (Op::APPEND_SCALAR4.mnemonic(), Op::APPEND_SCALAR4.size()),
            Op::LAPPEND_SCALAR1 => (Op::LAPPEND_SCALAR1.mnemonic(), Op::LAPPEND_SCALAR1.size()),
            Op::LAPPEND_SCALAR4 => (Op::LAPPEND_SCALAR4.mnemonic(), Op::LAPPEND_SCALAR4.size()),
            Op::APPEND_ARRAY1 => (Op::APPEND_ARRAY1.mnemonic(), Op::APPEND_ARRAY1.size()),
            Op::APPEND_ARRAY4 => (Op::APPEND_ARRAY4.mnemonic(), Op::APPEND_ARRAY4.size()),
            Op::LAPPEND_ARRAY1 => (Op::LAPPEND_ARRAY1.mnemonic(), Op::LAPPEND_ARRAY1.size()),
            Op::LAPPEND_ARRAY4 => (Op::LAPPEND_ARRAY4.mnemonic(), Op::LAPPEND_ARRAY4.size()),
            Op::RETURN_IMM => (Op::RETURN_IMM.mnemonic(), Op::RETURN_IMM.size()),
            Op::DONE => (Op::DONE.mnemonic(), Op::DONE.size()),
            Op::START_CMD => (Op::START_CMD.mnemonic(), Op::START_CMD.size()),
            Op::BREAK => (Op::BREAK.mnemonic(), Op::BREAK.size()),
            Op::CONTINUE => (Op::CONTINUE.mnemonic(), Op::CONTINUE.size()),
            Op::BEGIN_CATCH4 => (Op::BEGIN_CATCH4.mnemonic(), Op::BEGIN_CATCH4.size()),
            Op::END_CATCH => (Op::END_CATCH.mnemonic(), Op::END_CATCH.size()),
            Op::PUSH_RESULT => (Op::PUSH_RESULT.mnemonic(), Op::PUSH_RESULT.size()),
            Op::PUSH_RETURN_CODE => (Op::PUSH_RETURN_CODE.mnemonic(), Op::PUSH_RETURN_CODE.size()),
            Op::RETURN_CODE_BRANCH => (
                Op::RETURN_CODE_BRANCH.mnemonic(),
                Op::RETURN_CODE_BRANCH.size(),
            ),
            Op::FOREACH_START => (Op::FOREACH_START.mnemonic(), Op::FOREACH_START.size()),
            Op::FOREACH_STEP => (Op::FOREACH_STEP.mnemonic(), Op::FOREACH_STEP.size()),
            Op::FOREACH_END => (Op::FOREACH_END.mnemonic(), Op::FOREACH_END.size()),
            Op::LMAP_COLLECT => (Op::LMAP_COLLECT.mnemonic(), Op::LMAP_COLLECT.size()),
            Op::DICT_FIRST => (Op::DICT_FIRST.mnemonic(), Op::DICT_FIRST.size()),
            Op::DICT_NEXT => (Op::DICT_NEXT.mnemonic(), Op::DICT_NEXT.size()),
            Op::DICT_UPDATE_START => (
                Op::DICT_UPDATE_START.mnemonic(),
                Op::DICT_UPDATE_START.size(),
            ),
            Op::DICT_UPDATE_END => (Op::DICT_UPDATE_END.mnemonic(), Op::DICT_UPDATE_END.size()),
            Op::DICT_EXPAND => (Op::DICT_EXPAND.mnemonic(), Op::DICT_EXPAND.size()),
            Op::DICT_RECOMBINE_IMM => (
                Op::DICT_RECOMBINE_IMM.mnemonic(),
                Op::DICT_RECOMBINE_IMM.size(),
            ),
            Op::DICT_RECOMBINE_STK => (
                Op::DICT_RECOMBINE_STK.mnemonic(),
                Op::DICT_RECOMBINE_STK.size(),
            ),
            Op::DICT_GET_DEF => (Op::DICT_GET_DEF.mnemonic(), Op::DICT_GET_DEF.size()),
            Op::JUMP_TABLE => (Op::JUMP_TABLE.mnemonic(), Op::JUMP_TABLE.size()),
            Op::NOP => (Op::NOP.mnemonic(), Op::NOP.size()),
            Op::UMINUS => (Op::UMINUS.mnemonic(), Op::UMINUS.size()),
            Op::UPLUS => (Op::UPLUS.mnemonic(), Op::UPLUS.size()),
            Op::BITNOT => (Op::BITNOT.mnemonic(), Op::BITNOT.size()),
            Op::LNOT => (Op::LNOT.mnemonic(), Op::LNOT.size()),
            Op::NOT => (Op::NOT.mnemonic(), Op::NOT.size()),
            Op::LAND => (Op::LAND.mnemonic(), Op::LAND.size()),
            Op::LOR => (Op::LOR.mnemonic(), Op::LOR.size()),
            Op::LIST_IN => (Op::LIST_IN.mnemonic(), Op::LIST_IN.size()),
            Op::LIST_NOT_IN => (Op::LIST_NOT_IN.mnemonic(), Op::LIST_NOT_IN.size()),
            Op::STR_MAP => (Op::STR_MAP.mnemonic(), Op::STR_MAP.size()),
            Op::STR_FIND => (Op::STR_FIND.mnemonic(), Op::STR_FIND.size()),
            Op::STR_RFIND => (Op::STR_RFIND.mnemonic(), Op::STR_RFIND.size()),
            Op::STR_REPLACE => (Op::STR_REPLACE.mnemonic(), Op::STR_REPLACE.size()),
            Op::STR_TRIM => (Op::STR_TRIM.mnemonic(), Op::STR_TRIM.size()),
            Op::STR_TRIM_LEFT => (Op::STR_TRIM_LEFT.mnemonic(), Op::STR_TRIM_LEFT.size()),
            Op::STR_TRIM_RIGHT => (Op::STR_TRIM_RIGHT.mnemonic(), Op::STR_TRIM_RIGHT.size()),
            Op::STR_MATCH => (Op::STR_MATCH.mnemonic(), Op::STR_MATCH.size()),
            Op::STR_UPPER => (Op::STR_UPPER.mnemonic(), Op::STR_UPPER.size()),
            Op::STR_LOWER => (Op::STR_LOWER.mnemonic(), Op::STR_LOWER.size()),
            Op::STR_TITLE => (Op::STR_TITLE.mnemonic(), Op::STR_TITLE.size()),
            Op::STR_RANGE => (Op::STR_RANGE.mnemonic(), Op::STR_RANGE.size()),
            Op::STR_RANGE_IMM => (Op::STR_RANGE_IMM.mnemonic(), Op::STR_RANGE_IMM.size()),
            Op::STR_REVERSE => (Op::STR_REVERSE.mnemonic(), Op::STR_REVERSE.size()),
            Op::STR_REPEAT => (Op::STR_REPEAT.mnemonic(), Op::STR_REPEAT.size()),
            Op::REGEXP => (Op::REGEXP.mnemonic(), Op::REGEXP.size()),
            Op::STORE_STK => (Op::STORE_STK.mnemonic(), Op::STORE_STK.size()),
            Op::LOAD_STK => (Op::LOAD_STK.mnemonic(), Op::LOAD_STK.size()),
            Op::STORE_ARRAY_STK => (Op::STORE_ARRAY_STK.mnemonic(), Op::STORE_ARRAY_STK.size()),
            Op::LOAD_ARRAY_STK => (Op::LOAD_ARRAY_STK.mnemonic(), Op::LOAD_ARRAY_STK.size()),
            Op::LOAD_SCALAR_STK => (Op::LOAD_SCALAR_STK.mnemonic(), Op::LOAD_SCALAR_STK.size()),
            Op::STORE_SCALAR_STK => (Op::STORE_SCALAR_STK.mnemonic(), Op::STORE_SCALAR_STK.size()),
            Op::INCR_STK => (Op::INCR_STK.mnemonic(), Op::INCR_STK.size()),
            Op::INCR_STK_IMM => (Op::INCR_STK_IMM.mnemonic(), Op::INCR_STK_IMM.size()),
            Op::INCR_ARRAY_STK_IMM => (
                Op::INCR_ARRAY_STK_IMM.mnemonic(),
                Op::INCR_ARRAY_STK_IMM.size(),
            ),
            Op::INCR_SCALAR_STK => (Op::INCR_SCALAR_STK.mnemonic(), Op::INCR_SCALAR_STK.size()),
            Op::INCR_SCALAR_STK_IMM => (
                Op::INCR_SCALAR_STK_IMM.mnemonic(),
                Op::INCR_SCALAR_STK_IMM.size(),
            ),
            Op::INCR_ARRAY1 => (Op::INCR_ARRAY1.mnemonic(), Op::INCR_ARRAY1.size()),
            Op::INCR_ARRAY1_IMM => (Op::INCR_ARRAY1_IMM.mnemonic(), Op::INCR_ARRAY1_IMM.size()),
            Op::INCR_ARRAY_STK => (Op::INCR_ARRAY_STK.mnemonic(), Op::INCR_ARRAY_STK.size()),
            Op::APPEND_STK => (Op::APPEND_STK.mnemonic(), Op::APPEND_STK.size()),
            Op::LAPPEND_STK => (Op::LAPPEND_STK.mnemonic(), Op::LAPPEND_STK.size()),
            Op::APPEND_ARRAY_STK => (Op::APPEND_ARRAY_STK.mnemonic(), Op::APPEND_ARRAY_STK.size()),
            Op::LAPPEND_ARRAY_STK => (
                Op::LAPPEND_ARRAY_STK.mnemonic(),
                Op::LAPPEND_ARRAY_STK.size(),
            ),
            Op::LAPPEND_LIST => (Op::LAPPEND_LIST.mnemonic(), Op::LAPPEND_LIST.size()),
            Op::LAPPEND_LIST_STK => (Op::LAPPEND_LIST_STK.mnemonic(), Op::LAPPEND_LIST_STK.size()),
            Op::LAPPEND_LIST_ARRAY_STK => (
                Op::LAPPEND_LIST_ARRAY_STK.mnemonic(),
                Op::LAPPEND_LIST_ARRAY_STK.size(),
            ),
            Op::STORE_ARRAY1 => (Op::STORE_ARRAY1.mnemonic(), Op::STORE_ARRAY1.size()),
            Op::LOAD_ARRAY1 => (Op::LOAD_ARRAY1.mnemonic(), Op::LOAD_ARRAY1.size()),
            Op::STORE_ARRAY4 => (Op::STORE_ARRAY4.mnemonic(), Op::STORE_ARRAY4.size()),
            Op::LOAD_ARRAY4 => (Op::LOAD_ARRAY4.mnemonic(), Op::LOAD_ARRAY4.size()),
            Op::LAPPEND_LIST_ARRAY => (
                Op::LAPPEND_LIST_ARRAY.mnemonic(),
                Op::LAPPEND_LIST_ARRAY.size(),
            ),
            Op::ARRAY_EXISTS_IMM => (Op::ARRAY_EXISTS_IMM.mnemonic(), Op::ARRAY_EXISTS_IMM.size()),
            Op::ARRAY_EXISTS_STK => (Op::ARRAY_EXISTS_STK.mnemonic(), Op::ARRAY_EXISTS_STK.size()),
            Op::ARRAY_MAKE_IMM => (Op::ARRAY_MAKE_IMM.mnemonic(), Op::ARRAY_MAKE_IMM.size()),
            Op::ARRAY_MAKE_STK => (Op::ARRAY_MAKE_STK.mnemonic(), Op::ARRAY_MAKE_STK.size()),
            Op::UNSET_STK => (Op::UNSET_STK.mnemonic(), Op::UNSET_STK.size()),
            Op::UNSET_SCALAR => (Op::UNSET_SCALAR.mnemonic(), Op::UNSET_SCALAR.size()),
            Op::UNSET_ARRAY => (Op::UNSET_ARRAY.mnemonic(), Op::UNSET_ARRAY.size()),
            Op::UNSET_ARRAY_STK => (Op::UNSET_ARRAY_STK.mnemonic(), Op::UNSET_ARRAY_STK.size()),
            Op::EXIST_ARRAY => (Op::EXIST_ARRAY.mnemonic(), Op::EXIST_ARRAY.size()),
            Op::EXIST_ARRAY_STK => (Op::EXIST_ARRAY_STK.mnemonic(), Op::EXIST_ARRAY_STK.size()),
            Op::CONST_IMM => (Op::CONST_IMM.mnemonic(), Op::CONST_IMM.size()),
            Op::CONST_STK => (Op::CONST_STK.mnemonic(), Op::CONST_STK.size()),
            Op::VARIABLE => (Op::VARIABLE.mnemonic(), Op::VARIABLE.size()),
            Op::TAILCALL => (Op::TAILCALL.mnemonic(), Op::TAILCALL.size()),
            Op::TAILCALL4 => (Op::TAILCALL4.mnemonic(), Op::TAILCALL4.size()),
            Op::TAILCALL_LIST => (Op::TAILCALL_LIST.mnemonic(), Op::TAILCALL_LIST.size()),
            Op::CONCAT_STK => (Op::CONCAT_STK.mnemonic(), Op::CONCAT_STK.size()),
            Op::TRY_CVT_TO_NUMERIC => (
                Op::TRY_CVT_TO_NUMERIC.mnemonic(),
                Op::TRY_CVT_TO_NUMERIC.size(),
            ),
            Op::VERIFY_DICT => (Op::VERIFY_DICT.mnemonic(), Op::VERIFY_DICT.size()),
            Op::DICT_GET => (Op::DICT_GET.mnemonic(), Op::DICT_GET.size()),
            Op::DICT_PUT => (Op::DICT_PUT.mnemonic(), Op::DICT_PUT.size()),
            Op::ERROR_PREFIX_EQ => (Op::ERROR_PREFIX_EQ.mnemonic(), Op::ERROR_PREFIX_EQ.size()),
            Op::DICT_EXISTS => (Op::DICT_EXISTS.mnemonic(), Op::DICT_EXISTS.size()),
            Op::INVOKE_REPLACE => (Op::INVOKE_REPLACE.mnemonic(), Op::INVOKE_REPLACE.size()),
            Op::EXIST_STK => (Op::EXIST_STK.mnemonic(), Op::EXIST_STK.size()),
            Op::EXIST_SCALAR => (Op::EXIST_SCALAR.mnemonic(), Op::EXIST_SCALAR.size()),
            Op::DICT_SET => (Op::DICT_SET.mnemonic(), Op::DICT_SET.size()),
            Op::DICT_UNSET => (Op::DICT_UNSET.mnemonic(), Op::DICT_UNSET.size()),
            Op::DICT_INCR_IMM => (Op::DICT_INCR_IMM.mnemonic(), Op::DICT_INCR_IMM.size()),
            Op::DICT_APPEND => (Op::DICT_APPEND.mnemonic(), Op::DICT_APPEND.size()),
            Op::DICT_LAPPEND => (Op::DICT_LAPPEND.mnemonic(), Op::DICT_LAPPEND.size()),
            Op::UPVAR => (Op::UPVAR.mnemonic(), Op::UPVAR.size()),
            Op::UPLEVEL => (Op::UPLEVEL.mnemonic(), Op::UPLEVEL.size()),
            Op::NSUPVAR => (Op::NSUPVAR.mnemonic(), Op::NSUPVAR.size()),
            Op::LREPLACE4 => (Op::LREPLACE4.mnemonic(), Op::LREPLACE4.size()),
            Op::LREPLACE => (Op::LREPLACE.mnemonic(), Op::LREPLACE.size()),
            Op::OVER => (Op::OVER.mnemonic(), Op::OVER.size()),
            Op::LSET_FLAT => (Op::LSET_FLAT.mnemonic(), Op::LSET_FLAT.size()),
            Op::LSET_LIST => (Op::LSET_LIST.mnemonic(), Op::LSET_LIST.size()),
            Op::LIST_CONCAT => (Op::LIST_CONCAT.mnemonic(), Op::LIST_CONCAT.size()),
            Op::PUSH_RETURN_OPTS => (Op::PUSH_RETURN_OPTS.mnemonic(), Op::PUSH_RETURN_OPTS.size()),
            Op::RETURN_STK => (Op::RETURN_STK.mnemonic(), Op::RETURN_STK.size()),
            Op::REVERSE => (Op::REVERSE.mnemonic(), Op::REVERSE.size()),
            Op::SWAP => (Op::SWAP.mnemonic(), Op::SWAP.size()),
            Op::IS_EMPTY => (Op::IS_EMPTY.mnemonic(), Op::IS_EMPTY.size()),
            Op::NUMERIC_TYPE => (Op::NUMERIC_TYPE.mnemonic(), Op::NUMERIC_TYPE.size()),
            Op::TRY_CVT_TO_BOOLEAN => (
                Op::TRY_CVT_TO_BOOLEAN.mnemonic(),
                Op::TRY_CVT_TO_BOOLEAN.size(),
            ),
            Op::STR_CLASS => (Op::STR_CLASS.mnemonic(), Op::STR_CLASS.size()),
            Op::SYNTAX => (Op::SYNTAX.mnemonic(), Op::SYNTAX.size()),
            Op::IRULE_CONTAINS => (Op::IRULE_CONTAINS.mnemonic(), Op::IRULE_CONTAINS.size()),
            Op::IRULE_STARTS_WITH => (
                Op::IRULE_STARTS_WITH.mnemonic(),
                Op::IRULE_STARTS_WITH.size(),
            ),
            Op::IRULE_ENDS_WITH => (Op::IRULE_ENDS_WITH.mnemonic(), Op::IRULE_ENDS_WITH.size()),
            Op::IRULE_EQUALS => (Op::IRULE_EQUALS.mnemonic(), Op::IRULE_EQUALS.size()),
            Op::IRULE_MATCHES_GLOB => (
                Op::IRULE_MATCHES_GLOB.mnemonic(),
                Op::IRULE_MATCHES_GLOB.size(),
            ),
            Op::IRULE_MATCHES_REGEX => (
                Op::IRULE_MATCHES_REGEX.mnemonic(),
                Op::IRULE_MATCHES_REGEX.size(),
            ),
            Op::IRULE_MATCHES => (Op::IRULE_MATCHES.mnemonic(), Op::IRULE_MATCHES.size()),
            Op::IRULE_WORD_AND => (Op::IRULE_WORD_AND.mnemonic(), Op::IRULE_WORD_AND.size()),
            Op::IRULE_WORD_OR => (Op::IRULE_WORD_OR.mnemonic(), Op::IRULE_WORD_OR.size()),
            Op::IRULE_WORD_NOT => (Op::IRULE_WORD_NOT.mnemonic(), Op::IRULE_WORD_NOT.size()),
            Op::EXPAND_START => (Op::EXPAND_START.mnemonic(), Op::EXPAND_START.size()),
            Op::EXPAND_STKTOP => (Op::EXPAND_STKTOP.mnemonic(), Op::EXPAND_STKTOP.size()),
            Op::INVOKE_EXPANDED => (Op::INVOKE_EXPANDED.mnemonic(), Op::INVOKE_EXPANDED.size()),
            Op::EXPAND_DROP => (Op::EXPAND_DROP.mnemonic(), Op::EXPAND_DROP.size()),
            Op::CURRENT_NAMESPACE => (
                Op::CURRENT_NAMESPACE.mnemonic(),
                Op::CURRENT_NAMESPACE.size(),
            ),
            Op::INFO_LEVEL_NUM => (Op::INFO_LEVEL_NUM.mnemonic(), Op::INFO_LEVEL_NUM.size()),
            Op::INFO_LEVEL_ARGS => (Op::INFO_LEVEL_ARGS.mnemonic(), Op::INFO_LEVEL_ARGS.size()),
            Op::RESOLVE_CMD => (Op::RESOLVE_CMD.mnemonic(), Op::RESOLVE_CMD.size()),
            Op::ORIGIN_CMD => (Op::ORIGIN_CMD.mnemonic(), Op::ORIGIN_CMD.size()),
            Op::CLOCK_READ => (Op::CLOCK_READ.mnemonic(), Op::CLOCK_READ.size()),
            Op::YIELD => (Op::YIELD.mnemonic(), Op::YIELD.size()),
            Op::YIELD_TO_INVOKE => (Op::YIELD_TO_INVOKE.mnemonic(), Op::YIELD_TO_INVOKE.size()),
            Op::CORO_NAME => (Op::CORO_NAME.mnemonic(), Op::CORO_NAME.size()),
            Op::TCLOO_NEXT4 => (Op::TCLOO_NEXT4.mnemonic(), Op::TCLOO_NEXT4.size()),
            Op::TCLOO_NEXT_CLASS4 => (
                Op::TCLOO_NEXT_CLASS4.mnemonic(),
                Op::TCLOO_NEXT_CLASS4.size(),
            ),
            Op::TCLOO_NEXT_LIST => (Op::TCLOO_NEXT_LIST.mnemonic(), Op::TCLOO_NEXT_LIST.size()),
            Op::TCLOO_NEXT_CLASS_LIST => (
                Op::TCLOO_NEXT_CLASS_LIST.mnemonic(),
                Op::TCLOO_NEXT_CLASS_LIST.size(),
            ),
            Op::TCLOO_SELF => (Op::TCLOO_SELF.mnemonic(), Op::TCLOO_SELF.size()),
            Op::TCLOO_CLASS => (Op::TCLOO_CLASS.mnemonic(), Op::TCLOO_CLASS.size()),
            Op::TCLOO_NS => (Op::TCLOO_NS.mnemonic(), Op::TCLOO_NS.size()),
            Op::TCLOO_IS_OBJECT => (Op::TCLOO_IS_OBJECT.mnemonic(), Op::TCLOO_IS_OBJECT.size()),
            Op::TCLOO_ID => (Op::TCLOO_ID.mnemonic(), Op::TCLOO_ID.size()),
            Op::TCLOO_NEXT => (Op::TCLOO_NEXT.mnemonic(), Op::TCLOO_NEXT.size()),
            Op::TCLOO_NEXT_CLASS => (Op::TCLOO_NEXT_CLASS.mnemonic(), Op::TCLOO_NEXT_CLASS.size()),
        }
    }
    let all = [
        Op::PUSH1,
        Op::PUSH4,
        Op::POP,
        Op::DUP,
        Op::LOAD_SCALAR1,
        Op::LOAD_SCALAR4,
        Op::STORE_SCALAR1,
        Op::STORE_SCALAR4,
        Op::INCR_SCALAR1,
        Op::INCR_SCALAR1_IMM,
        Op::INVOKE_STK1,
        Op::INVOKE_STK4,
        Op::CALL_FUNC1,
        Op::EVAL_STK,
        Op::EXPR_STK,
        Op::JUMP1,
        Op::JUMP4,
        Op::JUMP_TRUE1,
        Op::JUMP_TRUE4,
        Op::JUMP_FALSE1,
        Op::JUMP_FALSE4,
        Op::ADD,
        Op::SUB,
        Op::MULT,
        Op::DIV,
        Op::MOD,
        Op::EXPON,
        Op::LSHIFT,
        Op::RSHIFT,
        Op::BITOR,
        Op::BITXOR,
        Op::BITAND,
        Op::EQ,
        Op::NEQ,
        Op::LT,
        Op::GT,
        Op::LE,
        Op::GE,
        Op::STR_EQ,
        Op::STR_NEQ,
        Op::STR_CMP,
        Op::STR_LT,
        Op::STR_GT,
        Op::STR_LE,
        Op::STR_GE,
        Op::STR_CONCAT1,
        Op::STR_LEN,
        Op::STR_INDEX,
        Op::LIST,
        Op::LIST_LENGTH,
        Op::LIST_INDEX,
        Op::LIST_INDEX_IMM,
        Op::LIST_RANGE_IMM,
        Op::LINDEX_MULTI,
        Op::APPEND_SCALAR1,
        Op::APPEND_SCALAR4,
        Op::LAPPEND_SCALAR1,
        Op::LAPPEND_SCALAR4,
        Op::APPEND_ARRAY1,
        Op::APPEND_ARRAY4,
        Op::LAPPEND_ARRAY1,
        Op::LAPPEND_ARRAY4,
        Op::RETURN_IMM,
        Op::DONE,
        Op::START_CMD,
        Op::BREAK,
        Op::CONTINUE,
        Op::BEGIN_CATCH4,
        Op::END_CATCH,
        Op::PUSH_RESULT,
        Op::PUSH_RETURN_CODE,
        Op::RETURN_CODE_BRANCH,
        Op::FOREACH_START,
        Op::FOREACH_STEP,
        Op::FOREACH_END,
        Op::LMAP_COLLECT,
        Op::DICT_FIRST,
        Op::DICT_NEXT,
        Op::DICT_UPDATE_START,
        Op::DICT_UPDATE_END,
        Op::DICT_EXPAND,
        Op::DICT_RECOMBINE_IMM,
        Op::DICT_RECOMBINE_STK,
        Op::DICT_GET_DEF,
        Op::JUMP_TABLE,
        Op::NOP,
        Op::UMINUS,
        Op::UPLUS,
        Op::BITNOT,
        Op::LNOT,
        Op::NOT,
        Op::LAND,
        Op::LOR,
        Op::LIST_IN,
        Op::LIST_NOT_IN,
        Op::STR_MAP,
        Op::STR_FIND,
        Op::STR_RFIND,
        Op::STR_REPLACE,
        Op::STR_TRIM,
        Op::STR_TRIM_LEFT,
        Op::STR_TRIM_RIGHT,
        Op::STR_MATCH,
        Op::STR_UPPER,
        Op::STR_LOWER,
        Op::STR_TITLE,
        Op::STR_RANGE,
        Op::STR_RANGE_IMM,
        Op::STR_REVERSE,
        Op::STR_REPEAT,
        Op::REGEXP,
        Op::STORE_STK,
        Op::LOAD_STK,
        Op::STORE_ARRAY_STK,
        Op::LOAD_ARRAY_STK,
        Op::LOAD_SCALAR_STK,
        Op::STORE_SCALAR_STK,
        Op::INCR_STK,
        Op::INCR_STK_IMM,
        Op::INCR_ARRAY_STK_IMM,
        Op::INCR_SCALAR_STK,
        Op::INCR_SCALAR_STK_IMM,
        Op::INCR_ARRAY1,
        Op::INCR_ARRAY1_IMM,
        Op::INCR_ARRAY_STK,
        Op::APPEND_STK,
        Op::LAPPEND_STK,
        Op::APPEND_ARRAY_STK,
        Op::LAPPEND_ARRAY_STK,
        Op::LAPPEND_LIST,
        Op::LAPPEND_LIST_STK,
        Op::LAPPEND_LIST_ARRAY_STK,
        Op::STORE_ARRAY1,
        Op::LOAD_ARRAY1,
        Op::STORE_ARRAY4,
        Op::LOAD_ARRAY4,
        Op::LAPPEND_LIST_ARRAY,
        Op::ARRAY_EXISTS_IMM,
        Op::ARRAY_EXISTS_STK,
        Op::ARRAY_MAKE_IMM,
        Op::ARRAY_MAKE_STK,
        Op::UNSET_STK,
        Op::UNSET_SCALAR,
        Op::UNSET_ARRAY,
        Op::UNSET_ARRAY_STK,
        Op::EXIST_ARRAY,
        Op::EXIST_ARRAY_STK,
        Op::CONST_IMM,
        Op::CONST_STK,
        Op::VARIABLE,
        Op::TAILCALL,
        Op::TAILCALL4,
        Op::TAILCALL_LIST,
        Op::CONCAT_STK,
        Op::TRY_CVT_TO_NUMERIC,
        Op::VERIFY_DICT,
        Op::DICT_GET,
        Op::DICT_PUT,
        Op::ERROR_PREFIX_EQ,
        Op::DICT_EXISTS,
        Op::INVOKE_REPLACE,
        Op::EXIST_STK,
        Op::EXIST_SCALAR,
        Op::DICT_SET,
        Op::DICT_UNSET,
        Op::DICT_INCR_IMM,
        Op::DICT_APPEND,
        Op::DICT_LAPPEND,
        Op::UPVAR,
        Op::UPLEVEL,
        Op::NSUPVAR,
        Op::LREPLACE4,
        Op::LREPLACE,
        Op::OVER,
        Op::LSET_FLAT,
        Op::LSET_LIST,
        Op::LIST_CONCAT,
        Op::PUSH_RETURN_OPTS,
        Op::RETURN_STK,
        Op::REVERSE,
        Op::SWAP,
        Op::IS_EMPTY,
        Op::NUMERIC_TYPE,
        Op::TRY_CVT_TO_BOOLEAN,
        Op::STR_CLASS,
        Op::SYNTAX,
        Op::IRULE_CONTAINS,
        Op::IRULE_STARTS_WITH,
        Op::IRULE_ENDS_WITH,
        Op::IRULE_EQUALS,
        Op::IRULE_MATCHES_GLOB,
        Op::IRULE_MATCHES_REGEX,
        Op::IRULE_WORD_AND,
        Op::IRULE_WORD_OR,
        Op::IRULE_WORD_NOT,
        Op::EXPAND_START,
        Op::EXPAND_STKTOP,
        Op::INVOKE_EXPANDED,
        Op::EXPAND_DROP,
        Op::CURRENT_NAMESPACE,
        Op::INFO_LEVEL_NUM,
        Op::INFO_LEVEL_ARGS,
        Op::RESOLVE_CMD,
        Op::ORIGIN_CMD,
        Op::CLOCK_READ,
        Op::YIELD,
        Op::YIELD_TO_INVOKE,
        Op::CORO_NAME,
        Op::TCLOO_SELF,
        Op::TCLOO_CLASS,
        Op::TCLOO_NS,
        Op::TCLOO_IS_OBJECT,
        Op::TCLOO_ID,
        Op::TCLOO_NEXT,
        Op::TCLOO_NEXT_CLASS,
        Op::TCLOO_NEXT4,
        Op::TCLOO_NEXT_CLASS4,
        Op::TCLOO_NEXT_LIST,
        Op::TCLOO_NEXT_CLASS_LIST,
    ];
    for op in all {
        let (mnemonic, size) = touch(op);
        assert_ne!(mnemonic, "");
        assert!((1..=9).contains(&size), "{mnemonic}: size {size}");
    }
}

#[test]
fn op_from_unaryop() {
    assert_eq!(Op::from_unaryop(UnaryOp::Neg), Some(Op::UMINUS));
    assert_eq!(Op::from_unaryop(UnaryOp::WordNot), Some(Op::IRULE_WORD_NOT));
}

#[test]
fn literal_table_intern() {
    let mut lit = LiteralTable::new();
    let a = lit.intern("hello");
    let b = lit.intern("world");
    let c = lit.intern("hello"); // dedup
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(c, 0); // same as first
    assert_eq!(lit.len(), 2);
}

#[test]
fn literal_table_register() {
    let mut lit = LiteralTable::new();
    let a = lit.register("x");
    let b = lit.register("x"); // no dedup
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(lit.len(), 2);
}

#[test]
fn literal_pool_preserves_native_bytes_without_unicode_aliasing() {
    let mut table = LiteralTable::new();
    let raw = table.intern_bytes(&[0xff]);
    let unicode = table.intern("ÿ");
    assert_ne!(raw, unicode);
    assert_eq!(table.intern_bytes(&[0xff]), raw);
    assert_eq!(table.entries()[raw].bytes(), &[0xff]);
    assert!(table.entries()[raw].unicode().is_err());
    assert_eq!(table.entries()[unicode].unicode(), Ok("ÿ"));
}

#[test]
fn local_var_table_intern() {
    let mut lvt = LocalVarTable::new(&["a", "b"]);
    assert_eq!(lvt.len(), 2);
    assert_eq!(lvt.intern("a"), 0); // pre-populated
    assert_eq!(lvt.intern("c"), 2); // new slot
    assert_eq!(lvt.entries(), &["a", "b", "c"]);
}

#[test]
fn instruction_new() {
    let instr = Instruction::new(Op::PUSH1, vec![Operand::Imm(5)]);
    assert_eq!(instr.op, Op::PUSH1);
    assert_eq!(instr.offset, -1);
    assert!(!instr.no_fold);
}

fn pack_stamp(pack: &str, content_hash: u64) -> tcl_runtime_api::PackFactStamp {
    tcl_runtime_api::PackFactStamp {
        pack: pack.to_owned(),
        content_hash,
        vocabulary_version: "2".to_owned(),
        overlay_generation: 9,
        evaluator_revision: 0,
    }
}

#[test]
fn a_functions_rungs_are_read_off_what_it_records() {
    use tcl_runtime_api::{
        BackingKind, CommandBindingIdentity, ProcedureBindingIdentity, Rung, RungSet, SiteClaim,
    };

    let generic = RungSet::of(Rung::Generic);
    assert_eq!(FunctionAsm::default().rungs(), generic);

    let mut claimed = FunctionAsm::default();
    claimed
        .site_claims
        .push(SiteClaim::PackFacts(pack_stamp("vendor", 1)));
    assert_eq!(claimed.rungs(), generic.with(Rung::PackFacts));

    let mut aliased = FunctionAsm::default();
    aliased.site_claims.push(SiteClaim::BuiltinAlias {
        binding: CommandBindingIdentity::new("vendor::nth", "lindex"),
        facts: pack_stamp("vendor", 1),
    });
    assert_eq!(aliased.rungs(), generic.with(Rung::BuiltinAlias));

    let mut inlined = FunctionAsm::default();
    inlined
        .procedure_bindings
        .push(ProcedureBindingIdentity::new(
            "target",
            "::target",
            "",
            "return OLD",
        ));
    assert_eq!(inlined.rungs(), generic.with(Rung::ReferenceBody));

    // A reference-body claim is rung 3 whatever else the function records.
    let mut referenced = FunctionAsm::default();
    referenced.site_claims.push(SiteClaim::ReferenceBody {
        procedure: ProcedureBindingIdentity::new("vendor::double", "::vendor::double", "x", "x"),
        backing: BackingKind::TclBody,
        facts: pack_stamp("vendor", 1),
    });
    assert_eq!(referenced.rungs(), generic.with(Rung::ReferenceBody));

    let mut specialised = FunctionAsm::default();
    specialised
        .command_bindings
        .push(CommandBindingIdentity::new("llength", "llength"));
    assert_eq!(specialised.rungs(), generic.with(Rung::ShippedBacking));

    let mut everything = claimed.clone();
    everything.site_claims.extend(aliased.site_claims);
    everything.procedure_bindings = inlined.procedure_bindings;
    everything.command_bindings = specialised.command_bindings;
    assert_eq!(everything.rungs(), RungSet::ALL);
}

#[test]
fn a_modules_claimed_packs_are_every_functions_stamps_sorted_and_once() {
    use tcl_runtime_api::SiteClaim;

    let claiming = |stamps: &[(&str, u64)]| {
        let mut function = FunctionAsm::default();
        for (pack, hash) in stamps {
            function
                .site_claims
                .push(SiteClaim::PackFacts(pack_stamp(pack, *hash)));
        }
        function
    };
    let issuer = tcl_runtime_api::ProcedureIdIssuer::default();
    let module = ModuleAsm {
        profile: tcl_dialect::DialectProfile::plain_tcl(),
        source: tcl_lexer::SourceImage::document(""),
        source_namespace: tcl_runtime_api::ByteNamespacePath::root(),
        plain_command_dispatch: false,
        top_level: claiming(&[("b", 2), ("a", 1)]),
        top_level_body: claiming(&[("d", 4), ("b", 2)]),
        procedures: HashMap::from([
            (issuer.allocate(), claiming(&[("c", 3), ("a", 1)])),
            (issuer.allocate(), FunctionAsm::default()),
        ]),
        procedure_provenance: HashMap::new(),
        manifest: None,
    };
    assert_eq!(
        module.claimed_packs(),
        vec![
            pack_stamp("a", 1),
            pack_stamp("b", 2),
            pack_stamp("c", 3),
            pack_stamp("d", 4)
        ]
    );

    let bare = ModuleAsm {
        top_level: FunctionAsm::default(),
        top_level_body: FunctionAsm::default(),
        procedures: HashMap::new(),
        ..module
    };
    assert!(bare.claimed_packs().is_empty());
}
