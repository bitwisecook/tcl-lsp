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

//! Bytecode artifact types for the Tcl 9 bytecode ("TCLVM") backend.
//!
//! The lean, `forbid(unsafe)` leaf crate that owns the shared bytecode
//! *artifact*: the opcode set ([`Op`], matching Tcl 9.0.2), the instruction
//! and assembly types ([`Instruction`], [`FunctionAsm`], [`ModuleAsm`]), the
//! interning tables ([`LiteralTable`], [`LocalVarTable`]), plus instruction
//! [`layout`] and disassembly [`format`].
//!
//! The compiler's emitter (`tcl_compiler::codegen`) produces these types and
//! re-exports them; the bytecode VM consumes them. Keeping them in a leaf crate
//! (whose only dependency is `tcl-syntax`, for [`BinOp`]/[`UnaryOp`]) lets the
//! VM depend on the artifact without pulling in the whole compiler.

pub mod format;
pub mod layout;

use std::collections::HashMap;
use std::fmt;

use tcl_lexer::Span;
use tcl_syntax::expr::ast::{BinOp, UnaryOp};

/// Index sentinel for "end"-based Tcl index notation.
///
/// `end` → `INDEX_END`, `end-N` → `INDEX_END - N`.
pub const INDEX_END: i32 = -(1 << 30);

/// Convert a non-negative count or index to an `i32` bytecode operand.
///
/// Bytecode `Imm` operands are `i32`; call-site usage always comes from
/// `usize` counts (argument lists, indices, slot numbers) that cannot
/// plausibly exceed `i32::MAX` — a compiler invariant. A program that
/// does exceed this limit is malformed and panicking is the correct
/// outcome rather than silent truncation.
#[inline]
#[must_use]
pub fn bytecode_imm(n: usize) -> i32 {
    i32::try_from(n).expect("bytecode operand exceeds i32::MAX")
}

/// Parse a Tcl index string to an integer suitable for IMM instructions.
///
/// Plain integers compile directly. `end`-based indices are encoded
/// relative to [`INDEX_END`] so the VM can resolve them at runtime.
#[must_use]
pub fn parse_tcl_index(s: &str) -> Option<i32> {
    let s = s.trim();
    if s == "end" {
        return Some(INDEX_END);
    }
    if let Some(rest) = s.strip_prefix("end-") {
        return rest.parse::<i32>().ok().map(|n| INDEX_END - n);
    }
    if let Some(rest) = s.strip_prefix("end+") {
        return rest.parse::<i32>().ok().map(|n| INDEX_END + n);
    }
    s.parse::<i32>().ok()
}

/// `string is` class name → numeric index.
#[must_use]
pub fn str_class_id(name: &str) -> Option<u8> {
    Some(match name {
        "alnum" => 0,
        "alpha" => 1,
        "ascii" => 2,
        "control" => 3,
        "digit" => 4,
        "graph" => 5,
        "lower" => 6,
        "print" => 7,
        "punct" => 8,
        "space" => 9,
        "upper" => 10,
        "wordchar" => 11,
        "xdigit" => 12,
        _ => return None,
    })
}

/// Numeric index → `string is` class name.
#[must_use]
pub fn str_class_name(id: u8) -> Option<&'static str> {
    Some(match id {
        0 => "alnum",
        1 => "alpha",
        2 => "ascii",
        3 => "control",
        4 => "digit",
        5 => "graph",
        6 => "lower",
        7 => "print",
        8 => "punct",
        9 => "space",
        10 => "upper",
        11 => "wordchar",
        12 => "xdigit",
        _ => return None,
    })
}

/// Tcl 9.0.2 bytecode instruction opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types, missing_docs)]
pub enum Op {
    PUSH1,
    PUSH4,
    POP,
    DUP,
    LOAD_SCALAR1,
    LOAD_SCALAR4,
    STORE_SCALAR1,
    STORE_SCALAR4,
    INCR_SCALAR1,
    INCR_SCALAR1_IMM,
    INVOKE_STK1,
    INVOKE_STK4,
    EVAL_STK,
    EXPR_STK,
    JUMP1,
    JUMP4,
    JUMP_TRUE1,
    JUMP_TRUE4,
    JUMP_FALSE1,
    JUMP_FALSE4,
    ADD,
    SUB,
    MULT,
    DIV,
    MOD,
    EXPON,
    LSHIFT,
    RSHIFT,
    BITOR,
    BITXOR,
    BITAND,
    EQ,
    NEQ,
    LT,
    GT,
    LE,
    GE,
    STR_EQ,
    STR_NEQ,
    STR_CMP,
    STR_LT,
    STR_GT,
    STR_LE,
    STR_GE,
    STR_CONCAT1,
    STR_LEN,
    STR_INDEX,
    LIST,
    LIST_LENGTH,
    LIST_INDEX,
    LIST_INDEX_IMM,
    LIST_RANGE_IMM,
    LINDEX_MULTI,
    APPEND_SCALAR1,
    APPEND_SCALAR4,
    LAPPEND_SCALAR1,
    LAPPEND_SCALAR4,
    APPEND_ARRAY1,
    APPEND_ARRAY4,
    LAPPEND_ARRAY1,
    LAPPEND_ARRAY4,
    RETURN_IMM,
    DONE,
    START_CMD,
    BREAK,
    CONTINUE,
    BEGIN_CATCH4,
    END_CATCH,
    PUSH_RESULT,
    PUSH_RETURN_CODE,
    /// `returnCodeBranch` — pop a return code (1–5) and jump forward
    /// `2*code − 1` bytes past this instruction: the compiler lays a table of
    /// five 2-byte `jump1` stubs right after it, one per code, error first
    /// (C Tcl's `INST_RETURN_CODE_BRANCH`, the compiled `try` handler
    /// dispatch). A code outside `error..continue` lands on the fifth stub.
    RETURN_CODE_BRANCH,
    FOREACH_START,
    FOREACH_STEP,
    FOREACH_END,
    /// `lmap_collect` — pop the top of stack and append it to the *collecting*
    /// `foreach`/`lmap` loop's VM-side accumulator (`ForeachState.accum`). Emitted
    /// on the body's fall-through path only (a `break`/`continue` redirect skips
    /// it), so the accumulator survives break/continue with the operand stack at a
    /// statement boundary. The paired `FOREACH_END` pushes `list(accum)` as the
    /// loop result. A real C Tcl 9.0 instruction (`lmap_collect`,
    /// `tclCompile.c`), though C keeps the accumulator in a temp local where the
    /// VM keeps it in the loop state.
    LMAP_COLLECT,
    /// `dictFirst <slot>` — begin iterating a dict (top of stack), storing the
    /// iterator state in local `<slot>`; pushes value, key, and a done flag
    /// (done on top). The C-Tcl compiled `dict for` / `dict map` primitive.
    DICT_FIRST,
    /// `dictNext <slot>` — advance the iterator in local `<slot>`; pushes the
    /// next value, key, and done flag.
    DICT_NEXT,
    /// `dictUpdateStart <dictSlot> <auxIdx>` — read the dict in local
    /// `<dictSlot>` and, for each key in the list on top of stack, store its
    /// value into the corresponding target local (from the out-of-band
    /// `dict_vars` list), unsetting the target when the key is absent. The key
    /// list stays on the stack for the paired `dictUpdateEnd`. C Tcl's compiled
    /// `dict update` prologue.
    DICT_UPDATE_START,
    /// `dictUpdateEnd <dictSlot> <auxIdx>` — pop the key list (top of stack) and
    /// write each target local (`dict_vars`) back into the dict in local
    /// `<dictSlot>` under its key, removing the key when its local is unset. C
    /// Tcl's compiled `dict update` epilogue.
    DICT_UPDATE_END,
    /// `dictExpand` — pop a key-path and a dict (path below, dict below that);
    /// create a local variable for every key of the (sub-)dict named by the
    /// path and push a "state" value (the snapshot key list) for the paired
    /// `dictRecombineImm`. C Tcl's compiled `dict with` prologue.
    DICT_EXPAND,
    /// `dictRecombineImm <dictSlot>` — pop a state value (top) and a key-path
    /// and write each state key's local back into the dict in local
    /// `<dictSlot>`, removing keys whose local was unset. C Tcl's compiled
    /// `dict with` epilogue.
    DICT_RECOMBINE_IMM,
    /// `dictRecombineStk` — [`DICT_RECOMBINE_IMM`](Op::DICT_RECOMBINE_IMM) with
    /// the dict variable's *name* taken from the stack (deepest of
    /// `varName path state`) rather than an LVT slot.
    DICT_RECOMBINE_STK,
    /// `dictGetDef <numKeys>` — like [`DICT_GET`](Op::DICT_GET) but with a
    /// default value on top of the keys: a key missing at any depth yields the
    /// default instead of an error (`dict getdef`/`dict getwithdefault`).
    DICT_GET_DEF,
    JUMP_TABLE,
    NOP,
    UMINUS,
    UPLUS,
    BITNOT,
    LNOT,
    NOT,
    LAND,
    LOR,
    LIST_IN,
    LIST_NOT_IN,
    STR_MAP,
    STR_FIND,
    STR_RFIND,
    STR_REPLACE,
    STR_TRIM,
    STR_TRIM_LEFT,
    STR_TRIM_RIGHT,
    STR_MATCH,
    STR_UPPER,
    STR_LOWER,
    STR_TITLE,
    STR_RANGE,
    STR_RANGE_IMM,
    STR_REVERSE,
    STR_REPEAT,
    REGEXP,
    STORE_STK,
    LOAD_STK,
    STORE_ARRAY_STK,
    LOAD_ARRAY_STK,
    /// `loadScalarStk` — load the scalar named by the popped name. C Tcl shares
    /// its `TEBCresume` case with `INST_LOAD_STK`; the compiler emits it for a
    /// name known not to carry an `(index)` part.
    LOAD_SCALAR_STK,
    /// `storeScalarStk` — store into the scalar named by the popped name (the
    /// `INST_STORE_STK` case in C Tcl, for an index-free name).
    STORE_SCALAR_STK,
    INCR_STK,
    INCR_STK_IMM,
    INCR_ARRAY_STK_IMM,
    /// `incrScalarStk` — increment the scalar named on the stack by the popped
    /// amount.
    INCR_SCALAR_STK,
    /// `incrScalarStkImm` — increment the scalar named on the stack by the
    /// 1-byte immediate.
    INCR_SCALAR_STK_IMM,
    /// `incrArray1` — increment the element (key on the stack) of the array in
    /// local `<slot>` by the popped amount.
    INCR_ARRAY1,
    /// `incrArray1Imm` — increment the element (key on the stack) of the array
    /// in local `<slot>` by the 1-byte immediate.
    INCR_ARRAY1_IMM,
    /// `incrArrayStk` — increment the array element named by the popped
    /// array-name/key pair by the popped amount.
    INCR_ARRAY_STK,
    APPEND_STK,
    LAPPEND_STK,
    /// `appendArrayStk` — string-append to the array element named by the
    /// popped array-name/key pair.
    APPEND_ARRAY_STK,
    /// `lappendArrayStk` — list-append to the array element named by the popped
    /// array-name/key pair.
    LAPPEND_ARRAY_STK,
    LAPPEND_LIST,
    LAPPEND_LIST_STK,
    LAPPEND_LIST_ARRAY_STK,
    STORE_ARRAY1,
    LOAD_ARRAY1,
    /// `storeArray4` — the 4-byte-slot form of `storeArray1` (arrays past slot
    /// 255).
    STORE_ARRAY4,
    /// `loadArray4` — the 4-byte-slot form of `loadArray1`.
    LOAD_ARRAY4,
    LAPPEND_LIST_ARRAY,
    ARRAY_EXISTS_IMM,
    /// `arrayExistsStk` — whether the variable named on the stack is an array.
    ARRAY_EXISTS_STK,
    /// `arrayMakeImm` — force local `<slot>` to be an array (`array set a {}`'s
    /// materialising half); a no-op when it already is one.
    ARRAY_MAKE_IMM,
    /// `arrayMakeStk` — force the variable named on the stack to be an array.
    ARRAY_MAKE_STK,
    UNSET_STK,
    UNSET_SCALAR,
    UNSET_ARRAY,
    /// `unsetArrayStk` — unset the array element named by the popped
    /// array-name/key pair; operand bit 0 is C's `TCL_LEAVE_ERR_MSG` (complain
    /// when absent).
    UNSET_ARRAY_STK,
    /// `existArray` — whether the element (key on the stack) of the array in
    /// local `<slot>` exists. Never errors.
    EXIST_ARRAY,
    /// `existArrayStk` — whether the array element named by the popped
    /// array-name/key pair exists. Never errors.
    EXIST_ARRAY_STK,
    /// `constImm` — define local `<slot>` as a constant holding the popped
    /// value (TIP 677). Re-defining an existing constant silently drops the
    /// value.
    CONST_IMM,
    /// `constStk` — define the variable named on the stack as a constant
    /// holding the popped value.
    CONST_STK,
    /// `variable` — link local `<slot>` to the namespace variable named by the
    /// popped (possibly qualified) name — the compiled `variable` command.
    VARIABLE,
    TAILCALL,
    CONCAT_STK,
    TRY_CVT_TO_NUMERIC,
    VERIFY_DICT,
    DICT_GET,
    DICT_EXISTS,
    INVOKE_REPLACE,
    EXIST_STK,
    EXIST_SCALAR,
    DICT_SET,
    DICT_UNSET,
    DICT_INCR_IMM,
    DICT_APPEND,
    DICT_LAPPEND,
    UPVAR,
    NSUPVAR,
    LREPLACE4,
    OVER,
    LSET_FLAT,
    LSET_LIST,
    LIST_CONCAT,
    PUSH_RETURN_OPTS,
    RETURN_STK,
    REVERSE,
    NUMERIC_TYPE,
    TRY_CVT_TO_BOOLEAN,
    STR_CLASS,
    SYNTAX,
    IRULE_CONTAINS,
    IRULE_STARTS_WITH,
    IRULE_ENDS_WITH,
    IRULE_EQUALS,
    IRULE_MATCHES_GLOB,
    IRULE_MATCHES_REGEX,
    IRULE_WORD_AND,
    IRULE_WORD_OR,
    IRULE_WORD_NOT,
    EXPAND_START,
    EXPAND_STKTOP,
    INVOKE_EXPANDED,
    /// `expandDrop` — abandon the innermost in-progress `{*}` expansion,
    /// truncating the operand stack back to the depth its `EXPAND_START`
    /// recorded (C `INST_EXPAND_DROP`).
    EXPAND_DROP,
    /// `currentNamespace` — push the current namespace's fully-qualified name
    /// (`::` at global scope).
    CURRENT_NAMESPACE,
    /// `infoLevelNumber` — push the current call-frame depth (`info level`).
    INFO_LEVEL_NUM,
    /// `infoLevelArgs` — push the invoking words of the level popped from the
    /// stack (`info level $n`).
    INFO_LEVEL_ARGS,
    /// `resolveCmd` — push the fully-qualified name the popped command name
    /// resolves to, or the empty string when it resolves to nothing. Never
    /// errors.
    RESOLVE_CMD,
    /// `originCmd` — push the fully-qualified name of the origin (through the
    /// import chain) of the popped command name; errors when it names no
    /// command.
    ORIGIN_CMD,
    /// `clockRead <which>` — push a wall-clock reading: `0` clicks,
    /// `1` microseconds, `2` milliseconds, `3` seconds.
    CLOCK_READ,
    /// `yield` — suspend the running coroutine, yielding the value on top of
    /// the stack; the resume value replaces it. Outside a coroutine this is the
    /// error `yield can only be called in a coroutine` (C `INST_YIELD`).
    YIELD,
    /// `yieldToInvoke` — suspend the running coroutine and, once it is parked,
    /// run the command list on top of the stack in the *resuming* context; its
    /// result replaces the list (the `yieldto` builtin's handoff, C
    /// `INST_YIELD_TO_INVOKE`).
    YIELD_TO_INVOKE,
    /// `coroName` — push the fully-qualified name of the current coroutine's
    /// command, or the empty string outside a coroutine (C
    /// `INST_COROUTINE_NAME`).
    CORO_NAME,
    /// `tclooSelf` — push the current object's command name (`self` /
    /// `self object`). Outside a method: `self may only be called from inside a
    /// method` (C `INST_TCLOO_SELF`).
    TCLOO_SELF,
    /// `tclooClass` — pop an object's command name and push its class's command
    /// name (`info object class`, C `INST_TCLOO_CLASS`).
    TCLOO_CLASS,
    /// `tclooNamespace` — pop an object's command name and push its instance
    /// namespace (`info object namespace`, C `INST_TCLOO_NS`).
    TCLOO_NS,
    /// `tclooIsObject` — pop a name and push whether it names a `TclOO` object
    /// (`info object isa object`). Never errors (C `INST_TCLOO_IS_OBJECT`).
    TCLOO_IS_OBJECT,
    /// `tclooNext <numWords>` — invoke the next implementation on the method
    /// chain (`next`). The operand counts the words on the stack: the first is
    /// the `next` command word itself (C's `skip = 1`), the rest are the
    /// arguments (C `INST_TCLOO_NEXT`).
    TCLOO_NEXT,
    /// `tclooNextClass <numWords>` — [`TCLOO_NEXT`](Op::TCLOO_NEXT) for
    /// `nextto`: of the `numWords` stack words the first is the command word and
    /// the second names the class to resume from (C's `skip = 2`, C
    /// `INST_TCLOO_NEXT_CLASS`).
    TCLOO_NEXT_CLASS,
}

impl Op {
    /// Disassembly mnemonic.
    ///
    /// The opcode→mnemonic table is a flat 1:1 map partitioned into cohesive
    /// per-family helpers (stack/control, arithmetic, string, list/var,
    /// coroutine/`TclOO`, and the dict/misc remainder) so no single function
    /// carries the whole table. Each helper returns `Some` only for the opcodes
    /// it owns; exactly one helper matches each opcode. The
    /// `op_family_routing_and_size` test spot-checks every routing branch, so a
    /// newly added opcode that is not routed into a family helper is caught
    /// before it reaches the final `unreachable!`.
    #[must_use]
    pub const fn mnemonic(self) -> &'static str {
        if let Some(m) = self.mnemonic_core() {
            m
        } else if let Some(m) = self.mnemonic_arith() {
            m
        } else if let Some(m) = self.mnemonic_string() {
            m
        } else if let Some(m) = self.mnemonic_list_var() {
            m
        } else if let Some(m) = self.mnemonic_coro_oo() {
            m
        } else {
            self.mnemonic_dict_misc()
        }
    }

    /// Stack, control-flow and invocation mnemonics.
    const fn mnemonic_core(self) -> Option<&'static str> {
        Some(match self {
            Self::PUSH1 => "push1",
            Self::PUSH4 => "push4",
            Self::POP => "pop",
            Self::DUP => "dup",
            Self::INVOKE_STK1 => "invokeStk1",
            Self::INVOKE_STK4 => "invokeStk4",
            Self::EVAL_STK => "evalStk",
            Self::EXPR_STK => "exprStk",
            Self::JUMP1 => "jump1",
            Self::JUMP4 => "jump4",
            Self::JUMP_TRUE1 => "jumpTrue1",
            Self::JUMP_TRUE4 => "jumpTrue4",
            Self::JUMP_FALSE1 => "jumpFalse1",
            Self::JUMP_FALSE4 => "jumpFalse4",
            Self::JUMP_TABLE => "jumpTable",
            Self::RETURN_IMM => "returnImm",
            Self::DONE => "done",
            Self::START_CMD => "startCommand",
            Self::BREAK => "break",
            Self::CONTINUE => "continue",
            Self::BEGIN_CATCH4 => "beginCatch4",
            Self::END_CATCH => "endCatch",
            Self::PUSH_RESULT => "pushResult",
            Self::PUSH_RETURN_CODE => "pushReturnCode",
            Self::RETURN_CODE_BRANCH => "returnCodeBranch",
            Self::FOREACH_START => "foreach_start",
            Self::FOREACH_STEP => "foreach_step",
            Self::FOREACH_END => "foreach_end",
            Self::LMAP_COLLECT => "lmap_collect",
            Self::DICT_FIRST => "dictFirst",
            Self::DICT_NEXT => "dictNext",
            Self::DICT_UPDATE_START => "dictUpdateStart",
            Self::DICT_UPDATE_END => "dictUpdateEnd",
            Self::DICT_EXPAND => "dictExpand",
            Self::DICT_RECOMBINE_IMM => "dictRecombineImm",
            Self::NOP => "nop",
            Self::TAILCALL => "tailcall",
            Self::INVOKE_REPLACE => "invokeReplace",
            Self::EXPAND_START => "expandStart",
            Self::EXPAND_STKTOP => "expandStkTop",
            Self::INVOKE_EXPANDED => "invokeExpanded",
            Self::EXPAND_DROP => "expandDrop",
            Self::CURRENT_NAMESPACE => "currentNamespace",
            Self::INFO_LEVEL_NUM => "infoLevelNumber",
            Self::INFO_LEVEL_ARGS => "infoLevelArgs",
            Self::RESOLVE_CMD => "resolveCmd",
            Self::ORIGIN_CMD => "originCmd",
            Self::CLOCK_READ => "clockRead",
            _ => return None,
        })
    }

    /// Arithmetic, bitwise, comparison and logical mnemonics.
    const fn mnemonic_arith(self) -> Option<&'static str> {
        Some(match self {
            Self::ADD => "add",
            Self::SUB => "sub",
            Self::MULT => "mult",
            Self::DIV => "div",
            Self::MOD => "mod",
            Self::EXPON => "expon",
            Self::LSHIFT => "lshift",
            Self::RSHIFT => "rshift",
            Self::BITOR => "bitor",
            Self::BITXOR => "bitxor",
            Self::BITAND => "bitand",
            Self::EQ => "eq",
            Self::NEQ => "neq",
            Self::LT => "lt",
            Self::GT => "gt",
            Self::LE => "le",
            Self::GE => "ge",
            Self::UMINUS => "uminus",
            Self::UPLUS => "uplus",
            Self::BITNOT => "bitnot",
            Self::LNOT => "lnot",
            Self::NOT => "not",
            Self::LAND => "land",
            Self::LOR => "lor",
            Self::NUMERIC_TYPE => "numericType",
            Self::TRY_CVT_TO_NUMERIC => "tryCvtToNumeric",
            Self::TRY_CVT_TO_BOOLEAN => "tryCvtToBoolean",
            _ => return None,
        })
    }

    /// String-operation and iRule string-test mnemonics.
    const fn mnemonic_string(self) -> Option<&'static str> {
        Some(match self {
            Self::STR_EQ => "streq",
            Self::STR_NEQ => "strneq",
            Self::STR_CMP => "strcmp",
            Self::STR_LT => "strlt",
            Self::STR_GT => "strgt",
            Self::STR_LE => "strle",
            Self::STR_GE => "strge",
            Self::STR_CONCAT1 => "strcat",
            Self::STR_LEN => "strlen",
            Self::STR_INDEX => "strindex",
            Self::STR_MAP => "strmap",
            Self::STR_FIND => "strfind",
            Self::STR_RFIND => "strrfind",
            Self::STR_REPLACE => "strreplace",
            Self::STR_TRIM => "strtrim",
            Self::STR_TRIM_LEFT => "strtrimLeft",
            Self::STR_TRIM_RIGHT => "strtrimRight",
            Self::STR_MATCH => "strmatch",
            Self::STR_UPPER => "strcaseUpper",
            Self::STR_LOWER => "strcaseLower",
            Self::STR_TITLE => "strcaseTitle",
            Self::STR_RANGE => "strrange",
            Self::STR_RANGE_IMM => "strrangeImm",
            Self::STR_REVERSE => "strreverse",
            Self::STR_REPEAT => "strrepeat",
            Self::STR_CLASS => "strclass",
            Self::REGEXP => "regexp",
            Self::IRULE_CONTAINS => "iruleContains",
            Self::IRULE_STARTS_WITH => "iruleStartsWith",
            Self::IRULE_ENDS_WITH => "iruleEndsWith",
            Self::IRULE_EQUALS => "iruleEquals",
            Self::IRULE_MATCHES_GLOB => "iruleMatchesGlob",
            Self::IRULE_MATCHES_REGEX => "iruleMatchesRegex",
            Self::IRULE_WORD_AND => "iruleAnd",
            Self::IRULE_WORD_OR => "iruleOr",
            Self::IRULE_WORD_NOT => "iruleNot",
            _ => return None,
        })
    }

    /// List, scalar-variable and array mnemonics.
    const fn mnemonic_list_var(self) -> Option<&'static str> {
        Some(match self {
            Self::LIST => "list",
            Self::LIST_LENGTH => "listLength",
            Self::LIST_INDEX => "listIndex",
            Self::LIST_INDEX_IMM => "listIndexImm",
            Self::LIST_RANGE_IMM => "listRangeImm",
            Self::LINDEX_MULTI => "lindexMulti",
            Self::LIST_IN => "listIn",
            Self::LIST_NOT_IN => "listNotIn",
            Self::LIST_CONCAT => "listConcat",
            Self::LSET_FLAT => "lsetFlat",
            Self::LSET_LIST => "lsetList",
            Self::LREPLACE4 => "lreplace4",
            Self::LOAD_SCALAR1 => "loadScalar1",
            Self::LOAD_SCALAR4 => "loadScalar4",
            Self::STORE_SCALAR1 => "storeScalar1",
            Self::STORE_SCALAR4 => "storeScalar4",
            Self::INCR_SCALAR1 => "incrScalar1",
            Self::INCR_SCALAR1_IMM => "incrScalar1Imm",
            Self::APPEND_SCALAR1 => "appendScalar1",
            Self::APPEND_SCALAR4 => "appendScalar4",
            Self::LAPPEND_SCALAR1 => "lappendScalar1",
            Self::LAPPEND_SCALAR4 => "lappendScalar4",
            Self::EXIST_SCALAR => "existScalar",
            Self::UNSET_SCALAR => "unsetScalar",
            Self::APPEND_ARRAY1 => "appendArray1",
            Self::APPEND_ARRAY4 => "appendArray4",
            Self::LAPPEND_ARRAY1 => "lappendArray1",
            Self::LAPPEND_ARRAY4 => "lappendArray4",
            Self::STORE_ARRAY1 => "storeArray1",
            Self::LOAD_ARRAY1 => "loadArray1",
            Self::STORE_ARRAY4 => "storeArray4",
            Self::LOAD_ARRAY4 => "loadArray4",
            Self::INCR_ARRAY1 => "incrArray1",
            Self::INCR_ARRAY1_IMM => "incrArray1Imm",
            Self::LAPPEND_LIST_ARRAY => "lappendListArray",
            Self::ARRAY_EXISTS_IMM => "arrayExistsImm",
            Self::ARRAY_MAKE_IMM => "arrayMakeImm",
            Self::UNSET_ARRAY => "unsetArray",
            Self::EXIST_ARRAY => "existArray",
            Self::CONST_IMM => "constImm",
            _ => return None,
        })
    }

    /// Coroutine (`yield`/`yieldto`/`coroName`) and `TclOO` mnemonics.
    const fn mnemonic_coro_oo(self) -> Option<&'static str> {
        Some(match self {
            Self::YIELD => "yield",
            Self::YIELD_TO_INVOKE => "yieldToInvoke",
            Self::CORO_NAME => "coroName",
            Self::TCLOO_SELF => "tclooSelf",
            Self::TCLOO_CLASS => "tclooClass",
            Self::TCLOO_NS => "tclooNamespace",
            Self::TCLOO_IS_OBJECT => "tclooIsObject",
            Self::TCLOO_NEXT => "tclooNext",
            Self::TCLOO_NEXT_CLASS => "tclooNextClass",
            _ => return None,
        })
    }

    /// Stack-variable, dict, upvar and remaining mnemonics.
    const fn mnemonic_dict_misc(self) -> &'static str {
        match self {
            Self::STORE_STK => "storeStk",
            Self::LOAD_STK => "loadStk",
            Self::STORE_SCALAR_STK => "storeScalarStk",
            Self::LOAD_SCALAR_STK => "loadScalarStk",
            Self::STORE_ARRAY_STK => "storeArrayStk",
            Self::LOAD_ARRAY_STK => "loadArrayStk",
            Self::INCR_STK => "incrStk",
            Self::INCR_STK_IMM => "incrStkImm",
            Self::INCR_ARRAY_STK_IMM => "incrArrayStkImm",
            Self::INCR_SCALAR_STK => "incrScalarStk",
            Self::INCR_SCALAR_STK_IMM => "incrScalarStkImm",
            Self::INCR_ARRAY_STK => "incrArrayStk",
            Self::APPEND_STK => "appendStk",
            Self::LAPPEND_STK => "lappendStk",
            Self::APPEND_ARRAY_STK => "appendArrayStk",
            Self::LAPPEND_ARRAY_STK => "lappendArrayStk",
            Self::LAPPEND_LIST => "lappendList",
            Self::LAPPEND_LIST_STK => "lappendListStk",
            Self::LAPPEND_LIST_ARRAY_STK => "lappendListArrayStk",
            Self::UNSET_STK => "unsetStk",
            Self::UNSET_ARRAY_STK => "unsetArrayStk",
            Self::CONCAT_STK => "concatStk",
            Self::EXIST_STK => "existStk",
            Self::EXIST_ARRAY_STK => "existArrayStk",
            Self::ARRAY_EXISTS_STK => "arrayExistsStk",
            Self::ARRAY_MAKE_STK => "arrayMakeStk",
            Self::CONST_STK => "constStk",
            Self::RETURN_STK => "returnStk",
            Self::VERIFY_DICT => "verifyDict",
            Self::DICT_GET => "dictGet",
            Self::DICT_GET_DEF => "dictGetDef",
            Self::DICT_EXISTS => "dictExists",
            Self::DICT_SET => "dictSet",
            Self::DICT_UNSET => "dictUnset",
            Self::DICT_INCR_IMM => "dictIncrImm",
            Self::DICT_APPEND => "dictAppend",
            Self::DICT_LAPPEND => "dictLappend",
            Self::DICT_RECOMBINE_STK => "dictRecombineStk",
            Self::UPVAR => "upvar",
            Self::NSUPVAR => "nsupvar",
            Self::VARIABLE => "variable",
            Self::OVER => "over",
            Self::REVERSE => "reverse",
            Self::PUSH_RETURN_OPTS => "pushReturnOpts",
            Self::SYNTAX => "syntax",
            // Every other opcode is routed into a family helper above; the
            // `opcode_family_partition_total` test proves this by construction.
            _ => unreachable!(),
        }
    }

    /// Whether this opcode takes an LVT (local variable table) operand.
    #[must_use]
    pub const fn is_lvt_op(self) -> bool {
        matches!(
            self,
            Self::LOAD_SCALAR1
                | Self::LOAD_SCALAR4
                | Self::STORE_SCALAR1
                | Self::STORE_SCALAR4
                | Self::INCR_SCALAR1
                | Self::INCR_SCALAR1_IMM
                | Self::APPEND_SCALAR1
                | Self::APPEND_SCALAR4
                | Self::LAPPEND_SCALAR1
                | Self::LAPPEND_SCALAR4
                | Self::APPEND_ARRAY1
                | Self::APPEND_ARRAY4
                | Self::LAPPEND_ARRAY1
                | Self::LAPPEND_ARRAY4
                | Self::LAPPEND_LIST
                | Self::STORE_ARRAY1
                | Self::LOAD_ARRAY1
                | Self::STORE_ARRAY4
                | Self::LOAD_ARRAY4
                | Self::INCR_ARRAY1
                | Self::INCR_ARRAY1_IMM
                | Self::LAPPEND_LIST_ARRAY
                | Self::ARRAY_EXISTS_IMM
                | Self::ARRAY_MAKE_IMM
                | Self::EXIST_SCALAR
                | Self::EXIST_ARRAY
                | Self::CONST_IMM
                | Self::DICT_APPEND
                | Self::DICT_LAPPEND
                | Self::UPVAR
                | Self::NSUPVAR
                | Self::VARIABLE
        )
    }

    /// Whether this opcode is a jump instruction.
    #[must_use]
    pub const fn is_jump(self) -> bool {
        matches!(
            self,
            Self::JUMP1
                | Self::JUMP4
                | Self::JUMP_TRUE1
                | Self::JUMP_TRUE4
                | Self::JUMP_FALSE1
                | Self::JUMP_FALSE4
        )
    }

    /// True for opcodes encoded as a bare byte with no operands (size 1).
    ///
    /// This is by far the largest size class, so it lives in its own helper to
    /// keep [`Op::size`] small, and is itself split in two — value/control ops
    /// and data-access ops — so neither match outgrows the crate's
    /// function-length limit. The three together still match every opcode and
    /// are covered by `opcode_family_partition_total`.
    #[must_use]
    pub const fn is_one_byte(self) -> bool {
        self.is_one_byte_value() || self.is_one_byte_access()
    }

    /// The operand-free arithmetic, comparison, coercion and stack/control
    /// opcodes — half of [`Op::is_one_byte`].
    const fn is_one_byte_value(self) -> bool {
        matches!(
            self,
            Self::POP
                | Self::DUP
                | Self::EVAL_STK
                | Self::EXPR_STK
                | Self::ADD
                | Self::SUB
                | Self::MULT
                | Self::DIV
                | Self::MOD
                | Self::EXPON
                | Self::LSHIFT
                | Self::RSHIFT
                | Self::BITOR
                | Self::BITXOR
                | Self::BITAND
                | Self::EQ
                | Self::NEQ
                | Self::LT
                | Self::GT
                | Self::LE
                | Self::GE
                | Self::UMINUS
                | Self::UPLUS
                | Self::BITNOT
                | Self::LNOT
                | Self::NOT
                | Self::LAND
                | Self::LOR
                | Self::NUMERIC_TYPE
                | Self::TRY_CVT_TO_NUMERIC
                | Self::TRY_CVT_TO_BOOLEAN
                | Self::DONE
                | Self::BREAK
                | Self::CONTINUE
                | Self::END_CATCH
                | Self::PUSH_RESULT
                | Self::PUSH_RETURN_CODE
                | Self::RETURN_CODE_BRANCH
                | Self::PUSH_RETURN_OPTS
                | Self::RETURN_STK
                | Self::FOREACH_STEP
                | Self::FOREACH_END
                | Self::LMAP_COLLECT
                | Self::NOP
                | Self::EXPAND_START
                | Self::INVOKE_EXPANDED
                | Self::EXPAND_DROP
                | Self::YIELD
                | Self::YIELD_TO_INVOKE
                | Self::CORO_NAME
        )
    }

    /// The operand-free string, list, variable, dict and introspection opcodes
    /// — the other half of [`Op::is_one_byte`].
    const fn is_one_byte_access(self) -> bool {
        matches!(
            self,
            Self::STR_EQ
                | Self::STR_NEQ
                | Self::STR_CMP
                | Self::STR_LT
                | Self::STR_GT
                | Self::STR_LE
                | Self::STR_GE
                | Self::STR_LEN
                | Self::STR_INDEX
                | Self::STR_MAP
                | Self::STR_FIND
                | Self::STR_RFIND
                | Self::STR_REPLACE
                | Self::STR_TRIM
                | Self::STR_TRIM_LEFT
                | Self::STR_TRIM_RIGHT
                | Self::STR_UPPER
                | Self::STR_LOWER
                | Self::STR_TITLE
                | Self::STR_RANGE
                | Self::STR_REVERSE
                | Self::STR_REPEAT
                | Self::IRULE_CONTAINS
                | Self::IRULE_STARTS_WITH
                | Self::IRULE_ENDS_WITH
                | Self::IRULE_EQUALS
                | Self::IRULE_MATCHES_GLOB
                | Self::IRULE_MATCHES_REGEX
                | Self::IRULE_WORD_AND
                | Self::IRULE_WORD_OR
                | Self::IRULE_WORD_NOT
                | Self::LIST_LENGTH
                | Self::LIST_INDEX
                | Self::LIST_IN
                | Self::LIST_NOT_IN
                | Self::LIST_CONCAT
                | Self::LSET_LIST
                | Self::STORE_STK
                | Self::LOAD_STK
                | Self::STORE_SCALAR_STK
                | Self::LOAD_SCALAR_STK
                | Self::STORE_ARRAY_STK
                | Self::LOAD_ARRAY_STK
                | Self::INCR_STK
                | Self::INCR_SCALAR_STK
                | Self::INCR_ARRAY_STK
                | Self::APPEND_STK
                | Self::LAPPEND_STK
                | Self::APPEND_ARRAY_STK
                | Self::LAPPEND_ARRAY_STK
                | Self::LAPPEND_LIST_STK
                | Self::LAPPEND_LIST_ARRAY_STK
                | Self::EXIST_STK
                | Self::EXIST_ARRAY_STK
                | Self::ARRAY_EXISTS_STK
                | Self::ARRAY_MAKE_STK
                | Self::CONST_STK
                | Self::VERIFY_DICT
                | Self::DICT_EXPAND
                | Self::DICT_RECOMBINE_STK
                | Self::CURRENT_NAMESPACE
                | Self::INFO_LEVEL_NUM
                | Self::INFO_LEVEL_ARGS
                | Self::RESOLVE_CMD
                | Self::ORIGIN_CMD
                | Self::TCLOO_SELF
                | Self::TCLOO_CLASS
                | Self::TCLOO_NS
                | Self::TCLOO_IS_OBJECT
        )
    }

    /// Instruction size in bytes (opcode + operands).
    ///
    /// Opcodes are grouped by size class. The single-byte class (the largest)
    /// is delegated to [`Op::is_one_byte`]; the remaining classes are matched
    /// here. The `opcode_family_partition_total` test exhaustively matches
    /// every variant, so a newly added opcode missing from both this match and
    /// `is_one_byte` is caught at test-compile time before the `unreachable!`.
    #[must_use]
    pub const fn size(self) -> u8 {
        if self.is_one_byte() {
            return 1;
        }
        match self {
            // 2-byte: opcode + 1-byte operand
            Self::PUSH1
            | Self::LOAD_SCALAR1
            | Self::STORE_SCALAR1
            | Self::INCR_SCALAR1
            | Self::INVOKE_STK1
            | Self::JUMP1
            | Self::JUMP_TRUE1
            | Self::JUMP_FALSE1
            | Self::STR_CONCAT1
            | Self::APPEND_SCALAR1
            | Self::LAPPEND_SCALAR1
            | Self::APPEND_ARRAY1
            | Self::LAPPEND_ARRAY1
            | Self::STORE_ARRAY1
            | Self::LOAD_ARRAY1
            | Self::INCR_ARRAY1
            | Self::INCR_STK_IMM
            | Self::INCR_SCALAR_STK_IMM
            | Self::INCR_ARRAY_STK_IMM
            | Self::UNSET_STK
            | Self::UNSET_ARRAY_STK
            | Self::TAILCALL
            | Self::STR_MATCH
            | Self::REGEXP
            | Self::CLOCK_READ
            | Self::TCLOO_NEXT
            | Self::TCLOO_NEXT_CLASS
            | Self::STR_CLASS => 2,

            // 3-byte: opcode + 2 1-byte operands
            Self::INCR_SCALAR1_IMM | Self::INCR_ARRAY1_IMM => 3,

            // 5-byte: opcode + 4-byte operand
            Self::PUSH4
            | Self::LOAD_SCALAR4
            | Self::STORE_SCALAR4
            | Self::INVOKE_STK4
            | Self::JUMP4
            | Self::JUMP_TRUE4
            | Self::JUMP_FALSE4
            | Self::LIST
            | Self::LIST_INDEX_IMM
            | Self::LINDEX_MULTI
            | Self::BEGIN_CATCH4
            | Self::FOREACH_START
            | Self::DICT_FIRST
            | Self::DICT_NEXT
            | Self::DICT_RECOMBINE_IMM
            | Self::JUMP_TABLE
            | Self::LAPPEND_LIST
            | Self::APPEND_SCALAR4
            | Self::LAPPEND_SCALAR4
            | Self::APPEND_ARRAY4
            | Self::LAPPEND_ARRAY4
            | Self::STORE_ARRAY4
            | Self::LOAD_ARRAY4
            | Self::LAPPEND_LIST_ARRAY
            | Self::ARRAY_EXISTS_IMM
            | Self::ARRAY_MAKE_IMM
            | Self::CONCAT_STK
            | Self::DICT_GET
            | Self::DICT_GET_DEF
            | Self::DICT_EXISTS
            | Self::EXIST_SCALAR
            | Self::EXIST_ARRAY
            | Self::CONST_IMM
            | Self::DICT_APPEND
            | Self::DICT_LAPPEND
            | Self::UPVAR
            | Self::NSUPVAR
            | Self::VARIABLE
            | Self::LSET_FLAT
            | Self::REVERSE
            | Self::OVER
            | Self::EXPAND_STKTOP => 5,

            // 6-byte
            Self::INVOKE_REPLACE | Self::LREPLACE4 | Self::UNSET_SCALAR | Self::UNSET_ARRAY => 6,

            // 9-byte: opcode + 2× 4-byte operands
            Self::LIST_RANGE_IMM
            | Self::STR_RANGE_IMM
            | Self::RETURN_IMM
            | Self::START_CMD
            | Self::SYNTAX
            | Self::DICT_SET
            | Self::DICT_UNSET
            | Self::DICT_INCR_IMM
            | Self::DICT_UPDATE_START
            | Self::DICT_UPDATE_END => 9,

            // All remaining opcodes are single-byte, handled above by the
            // early return; `opcode_family_partition_total` proves this.
            _ => unreachable!(),
        }
    }

    /// Map a [`BinOp`] to its bytecode opcode.
    #[must_use]
    pub fn from_binop(op: BinOp) -> Option<Self> {
        Some(match op {
            BinOp::Add => Self::ADD,
            BinOp::Sub => Self::SUB,
            BinOp::Mul => Self::MULT,
            BinOp::Div => Self::DIV,
            BinOp::Mod => Self::MOD,
            BinOp::Pow => Self::EXPON,
            BinOp::LShift => Self::LSHIFT,
            BinOp::RShift => Self::RSHIFT,
            BinOp::BitAnd => Self::BITAND,
            BinOp::BitOr => Self::BITOR,
            BinOp::BitXor => Self::BITXOR,
            BinOp::And => Self::LAND,
            BinOp::Or => Self::LOR,
            BinOp::Eq => Self::EQ,
            BinOp::Ne => Self::NEQ,
            BinOp::Lt => Self::LT,
            BinOp::Gt => Self::GT,
            BinOp::Le => Self::LE,
            BinOp::Ge => Self::GE,
            BinOp::StrEq => Self::STR_EQ,
            BinOp::StrNe => Self::STR_NEQ,
            BinOp::StrLt => Self::STR_LT,
            BinOp::StrGt => Self::STR_GT,
            BinOp::StrLe => Self::STR_LE,
            BinOp::StrGe => Self::STR_GE,
            BinOp::In => Self::LIST_IN,
            BinOp::Ni => Self::LIST_NOT_IN,
            BinOp::WordAnd => Self::IRULE_WORD_AND,
            BinOp::WordOr => Self::IRULE_WORD_OR,
            BinOp::Contains => Self::IRULE_CONTAINS,
            BinOp::StartsWith => Self::IRULE_STARTS_WITH,
            BinOp::EndsWith => Self::IRULE_ENDS_WITH,
            BinOp::StrEquals => Self::IRULE_EQUALS,
            BinOp::MatchesGlob => Self::IRULE_MATCHES_GLOB,
            BinOp::MatchesRegex => Self::IRULE_MATCHES_REGEX,
        })
    }

    /// Map a [`UnaryOp`] to its bytecode opcode.
    #[must_use]
    pub fn from_unaryop(op: UnaryOp) -> Option<Self> {
        Some(match op {
            UnaryOp::Neg => Self::UMINUS,
            UnaryOp::Pos => Self::UPLUS,
            UnaryOp::BitNot => Self::BITNOT,
            UnaryOp::Not => Self::NOT,
            UnaryOp::WordNot => Self::IRULE_WORD_NOT,
        })
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// Instruction operand

/// An instruction operand: either an immediate integer or a label reference.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Integer immediate (literal index, LVT slot, jump offset, etc.).
    Imm(i32),
    /// Symbolic label reference (resolved during layout).
    Label(String),
}

// Instruction

/// A single bytecode instruction (labels unresolved until layout).
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    /// Opcode.
    pub op: Op,
    /// Operands.
    pub operands: Vec<Operand>,
    /// Human-readable comment for disassembly.
    pub comment: String,
    /// Byte offset (filled by the layout pass; -1 before layout).
    pub offset: i32,
    /// Pattern → label map for `JUMP_TABLE` only.
    pub jump_table: Option<HashMap<String, String>>,
    /// Prevent push-pop folding (jump target result).
    pub no_fold: bool,
    /// 1-based source line for `errorInfo`.
    pub source_line: u32,
    /// Original command text for `errorInfo`.
    pub source_cmd_text: String,
    /// Byte span of the source construct this instruction was lowered
    /// from, when known. `None` for synthetic instructions with no
    /// direct source (loop-result pushes, fallthrough jumps, padding
    /// NOPs). Stamped at emission time from `CodegenCtx::current_span`;
    /// the explorer maps it to a line:col `range` for click-to-source.
    pub source_span: Option<Span>,
    /// Loop-variable groups for `FOREACH_START`/`lmap` only — the analogue of
    /// C Tcl's `ForeachInfo.varLists` (`tclExecute.c` `INST_FOREACH_*`). One
    /// inner `Vec<String>` per iterator group; the value lists are pushed on the
    /// stack before the opcode. Not rendered in disassembly (keeps identity
    /// stable).
    pub foreach_vars: Option<Vec<Vec<String>>>,
    /// `FOREACH_START` only: this is a *collecting* loop (`lmap`), so the VM
    /// initialises a per-loop accumulator that `LMAP_COLLECT` appends to and the
    /// paired `FOREACH_END` materialises as `list(accum)`. Carried out-of-band
    /// alongside `foreach_vars` so the 5-byte operand form and disassembly stay
    /// byte-stable. `false` for a plain `foreach` and every other opcode.
    pub foreach_collect: bool,
    /// Target variable names for `DICT_UPDATE_START`/`DICT_UPDATE_END` only — the
    /// analogue of C Tcl's `DictUpdateInfo.varIndices`. One name per key in the
    /// key list on the stack; the VM stores/reads each named local. Carried
    /// out-of-band (like `foreach_vars`) so the 9-byte on-disk operand form and
    /// disassembly stay byte-stable. `None` for every other opcode.
    pub dict_vars: Option<Vec<String>>,
    /// `PUSH1`/`PUSH4` only: the literal is a *verbatim* (braced / constant)
    /// word and must be pushed exactly as-is, suppressing the runtime word
    /// substitution that the VM otherwise applies to `${…}` / `[…]` markers
    /// (see `tcl-vm::subst::subst_word`). This is the Rust analogue of the
    /// reference VM's brace/`_RAW_PREFIX` `PUSH` handling, carried out-of-band
    /// so the literal pool and disassembly stay byte-stable (keeps identity
    /// stable).
    pub push_verbatim: bool,
    /// `BEGIN_CATCH4` only: the label of the range's handler — where the VM
    /// resumes (stack trimmed, caught completion recorded) when an exceptional
    /// completion unwinds into the range. This is the analogue of C Tcl's
    /// `ExceptionRange.catchOffset`, carried out-of-band (like `foreach_vars`)
    /// so the 4-byte operand keeps C's meaning (the range index) and the
    /// disassembly stays byte-stable. `None` marks a *decorative* range — the
    /// codegen emitted the C-faithful shape but relies on the VM's
    /// activation-stack unwinding instead, so `BEGIN_CATCH4` is inert.
    pub catch_target: Option<String>,
}

impl Instruction {
    /// Create a new instruction with default metadata.
    #[must_use]
    pub fn new(op: Op, operands: Vec<Operand>) -> Self {
        Self {
            op,
            operands,
            comment: String::new(),
            offset: -1,
            jump_table: None,
            no_fold: false,
            source_line: 0,
            source_cmd_text: String::new(),
            source_span: None,
            foreach_vars: None,
            foreach_collect: false,
            dict_vars: None,
            push_verbatim: false,
            catch_target: None,
        }
    }
}

// Interning tables

/// Intern pool mapping literal strings to object-array indices.
#[derive(Debug, Clone, Default)]
pub struct LiteralTable {
    entries: Vec<String>,
    index: HashMap<String, usize>,
}

impl LiteralTable {
    /// Create a new empty literal table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a deduplicated index for `value`.
    pub fn intern(&mut self, value: &str) -> usize {
        if let Some(&idx) = self.index.get(value) {
            return idx;
        }
        let idx = self.entries.len();
        self.entries.push(value.to_owned());
        self.index.insert(value.to_owned(), idx);
        idx
    }

    /// Always append `value` (no deduplication).
    pub fn register(&mut self, value: &str) -> usize {
        let idx = self.entries.len();
        self.entries.push(value.to_owned());
        idx
    }

    /// Return all interned entries in order.
    #[must_use]
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Maps variable names to local variable table (LVT) slot indices.
#[derive(Debug, Clone, Default)]
pub struct LocalVarTable {
    slots: Vec<String>,
    index: HashMap<String, usize>,
}

impl LocalVarTable {
    /// Create a new LVT, optionally pre-populating with procedure parameters.
    #[must_use]
    pub fn new(params: &[&str]) -> Self {
        let mut lvt = Self::default();
        for &p in params {
            lvt.intern(p);
        }
        lvt
    }

    /// Get or create a slot index for `name`.
    pub fn intern(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.index.get(name) {
            return idx;
        }
        let idx = self.slots.len();
        self.slots.push(name.to_owned());
        self.index.insert(name.to_owned(), idx);
        idx
    }

    /// Return all variable names in slot order.
    #[must_use]
    pub fn entries(&self) -> &[String] {
        &self.slots
    }

    /// Number of slots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// Complete assembly for one CFG function.
///
/// `Default` yields an empty function (no instructions) — used as the placeholder
/// asm for a scanner-driven activation like the VM's `subst` frame, which never
/// executes bytecode.
#[derive(Debug, Clone, Default)]
pub struct FunctionAsm {
    /// Function name.
    pub name: String,
    /// Literal constant pool.
    pub literals: LiteralTable,
    /// Local variable table.
    pub lvt: LocalVarTable,
    /// Instruction stream.
    pub instructions: Vec<Instruction>,
    /// Label → byte offset (populated by the layout pass).
    pub labels: HashMap<String, usize>,
    /// For each instruction inside an inline loop body (by index), the loop's
    /// `(break, continue)` jump targets as **byte offsets**. Lets the executor
    /// catch a `break`/`continue` *returned by a command* (`if {…} $z`, `eval
    /// break`) — which the inline `JUMP` only covers for a *literal*
    /// `break`/`continue` — and jump to the loop's exit/continue point, as C
    /// Tcl's exception ranges do. Either target may be `None` (a `for`-step's
    /// `continue` propagates). Sparse: only loop-body instructions appear.
    pub loop_targets: HashMap<usize, (Option<i32>, Option<i32>)>,
    /// For a procedure body, the 1-based source line of its `proc` definition
    /// (the body's line 1). `errorInfo`'s `(procedure "name" line N)` reports
    /// `N = instruction_line − body_base_line + 1`, so the line is relative to
    /// the proc, not the whole module. `0` for the top level / hand-built asm.
    pub body_base_line: u32,
    /// Inline command-body regions (`eval {…}` / `while`/`for`/`foreach` bodies)
    /// folded into this function's instruction stream. As an error unwinds past
    /// a covering region the executor synthesises the body frame the *uncompiled*
    /// command would add to `errorInfo` (`("eval" body line N)` + `invoked from
    /// within "eval {…}"`), so an inlined body matches C's `CmdFrame` trace
    /// without de-inlining. Empty when the function holds no inlined bodies.
    pub error_regions: Vec<ErrorRegion>,
}

/// An inlined command body's instruction range and the `errorInfo` frame the
/// enclosing command (`eval`/`while`/`for`/`foreach`) would add when its body
/// errors — the compiled analogue of C's per-command `CmdFrame`. Populated by
/// codegen when it folds a literal body inline (so the LSP keeps its inlined
/// view), consumed by the executor as an error unwinds (see
/// `FunctionAsm::error_regions`).
#[derive(Debug, Clone)]
pub struct ErrorRegion {
    /// Start byte offset of the enclosing command's source span (inclusive). An
    /// instruction is covered when its `source_span` lies within `[start, end)`,
    /// so coverage is matched by source containment — robust to instruction
    /// reordering and exact across interleaved (non-body) instructions.
    pub start: u32,
    /// End byte offset of the enclosing command's source span (exclusive).
    pub end: u32,
    /// The body-frame label — `eval`/`while`/`for`/`foreach` — quoted in the
    /// `("LABEL" body line N)` frame.
    pub label: String,
    /// The enclosing command's surface text, for its `invoked from within "…"`
    /// frame (truncated to 150 bytes by the logger, as in C).
    pub cmd_text: String,
    /// One less than the body's first source line: a covered instruction's
    /// body-relative line is `instruction_line − line_base` (so the body frame
    /// reports a line relative to the body, not the whole module).
    pub line_base: u32,
    /// The enclosing command's own source line — the line its `invoked from
    /// within` frame contributes to any further-out (proc) frame.
    pub cmd_line: u32,
}

/// Assembly for an entire module.
#[derive(Debug, Clone)]
pub struct ModuleAsm {
    /// Top-level script assembly.
    pub top_level: FunctionAsm,
    /// Procedure assemblies keyed by qualified name.
    pub procedures: HashMap<String, FunctionAsm>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn str_class_roundtrip() {
        for name in [
            "alnum", "alpha", "ascii", "control", "digit", "graph", "lower", "print", "punct",
            "space", "upper", "wordchar", "xdigit",
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
            (Op::TCLOO_NEXT, "tclooNext", 2),
            (Op::TCLOO_NEXT_CLASS, "tclooNextClass", 2),
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
}
