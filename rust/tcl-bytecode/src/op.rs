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

//! The opcode set: mnemonics per release, encoded sizes and the operator mapping.

use std::fmt;
use tcl_syntax::expr::ast::{BinOp, UnaryOp};

/// Portable Tcl bytecode instruction opcodes.
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
    /// Portable pre-TIP-232 fixed-function call. The operand counts value
    /// arguments; the exact registration is carried outside the value stack.
    CALL_FUNC1,
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
    /// Tcl 9.1 tailcall with a four-byte namespace-inclusive argument count.
    TAILCALL4,
    /// Replace the procedure using a list containing its retained namespace,
    /// command and arguments. A namespace-only list schedules an empty return.
    TAILCALL_LIST,
    CONCAT_STK,
    TRY_CVT_TO_NUMERIC,
    VERIFY_DICT,
    DICT_GET,
    /// C9.1 single-member original Dictionary update, without variable lookup.
    DICT_PUT,
    /// C9.1 original error-code List prefix comparison; operand is prefix length.
    ERROR_PREFIX_EQ,
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
    /// C Tcl 9.1 selected frame evaluation: stack holds level then script.
    UPLEVEL,
    NSUPVAR,
    LREPLACE4,
    /// C9.1 original List replacement with counted operands and flags.
    LREPLACE,
    OVER,
    LSET_FLAT,
    LSET_LIST,
    LIST_CONCAT,
    PUSH_RETURN_OPTS,
    RETURN_STK,
    /// Swap the top two original objects; a native C9.1 one-byte opcode.
    /// Portable control recipes also use this stack operation independently
    /// of older native reverse instruction encodings.
    SWAP,
    /// C9.1 `isEmpty`: pop a value, push whether its string is empty without
    /// generating one for a dictionary or a length-known representation.
    IS_EMPTY,
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
    /// The authored F5 bare `matches` operator: case-sensitive whole-string
    /// glob matching, admitted by the independently installed F5 parser policy.
    /// The discriminators are measured on BIG-IP 21.1.0.1 build 0.0.26.
    IRULE_MATCHES,
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
    /// Produce the original object's wide creation epoch (C9.1 `INST_TCLOO_ID`).
    TCLOO_ID,
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
    /// C9.1 direct next invocation with a four-byte complete-word count.
    TCLOO_NEXT4,
    /// C9.1 direct nextto invocation with a four-byte complete-word count.
    TCLOO_NEXT_CLASS4,
    /// C9.1 next consumes one genuinely constructed original argument List.
    TCLOO_NEXT_LIST,
    /// C9.1 nextto consumes one genuinely constructed original argument List.
    TCLOO_NEXT_CLASS_LIST,
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
    /// The name a release whose compiler emits only the wide operand forms
    /// (Tcl 9.1) gives the instruction it emits in this one's place: the
    /// width suffix is gone, and the narrow forms are replaced by wide ones.
    #[must_use]
    pub const fn wide_mnemonic(self) -> &'static str {
        match self {
            Self::PUSH1 | Self::PUSH4 => "push",
            Self::INVOKE_STK1 | Self::INVOKE_STK4 => "invokeStk",
            Self::JUMP1 | Self::JUMP4 => "jump",
            Self::JUMP_TRUE1 | Self::JUMP_TRUE4 => "jumpTrue",
            Self::JUMP_FALSE1 | Self::JUMP_FALSE4 => "jumpFalse",
            Self::BEGIN_CATCH4 => "beginCatch",
            Self::RETURN_CODE_BRANCH => "returnCodeBranch1",
            Self::LREPLACE4 => "lreplace",
            Self::LOAD_SCALAR1 | Self::LOAD_SCALAR4 => "loadScalar",
            Self::STORE_SCALAR1 | Self::STORE_SCALAR4 => "storeScalar",
            Self::LOAD_ARRAY1 | Self::LOAD_ARRAY4 => "loadArray",
            Self::STORE_ARRAY1 | Self::STORE_ARRAY4 => "storeArray",
            Self::INCR_SCALAR1 => "incrScalar",
            Self::INCR_SCALAR1_IMM => "incrScalarImm",
            Self::INCR_ARRAY1 => "incrArray",
            Self::INCR_ARRAY1_IMM => "incrArrayImm",
            Self::APPEND_SCALAR1 | Self::APPEND_SCALAR4 => "appendScalar",
            Self::APPEND_ARRAY1 | Self::APPEND_ARRAY4 => "appendArray",
            Self::LAPPEND_SCALAR1 | Self::LAPPEND_SCALAR4 => "lappendScalar",
            Self::LAPPEND_ARRAY1 | Self::LAPPEND_ARRAY4 => "lappendArray",
            other => other.mnemonic(),
        }
    }

    const fn mnemonic_core(self) -> Option<&'static str> {
        Some(match self {
            Self::PUSH1 => "push1",
            Self::PUSH4 => "push4",
            Self::POP => "pop",
            Self::DUP => "dup",
            Self::INVOKE_STK1 => "invokeStk1",
            Self::INVOKE_STK4 => "invokeStk4",
            Self::CALL_FUNC1 => "callFunc1",
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
            Self::TAILCALL | Self::TAILCALL4 => "tailcall",
            Self::TAILCALL_LIST => "tailcallList",
            Self::INVOKE_REPLACE => "invokeReplace",
            Self::UPLEVEL => "uplevel",
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
            Self::IRULE_MATCHES => "iruleMatches",
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
            Self::LREPLACE => "lreplace",
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
            Self::TCLOO_ID => "tclooId",
            Self::TCLOO_NEXT => "tclooNext",
            Self::TCLOO_NEXT_CLASS => "tclooNextClass",
            Self::TCLOO_NEXT4 => "tclooNext4",
            Self::TCLOO_NEXT_CLASS4 => "tclooNextClass4",
            Self::TCLOO_NEXT_LIST => "tclooNextList",
            Self::TCLOO_NEXT_CLASS_LIST => "tclooNextClassList",
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
            Self::SWAP => "swap",
            Self::IS_EMPTY => "isEmpty",
            Self::VERIFY_DICT => "verifyDict",
            Self::DICT_GET => "dictGet",
            Self::DICT_PUT => "dictPut",
            Self::ERROR_PREFIX_EQ => "errorPrefixEq",
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
                | Self::UPLEVEL
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
                | Self::SWAP
                | Self::IS_EMPTY
                | Self::FOREACH_STEP
                | Self::FOREACH_END
                | Self::LMAP_COLLECT
                | Self::NOP
                | Self::EXPAND_START
                | Self::INVOKE_EXPANDED
                | Self::EXPAND_DROP
                | Self::YIELD
                | Self::YIELD_TO_INVOKE
                | Self::TAILCALL_LIST
                | Self::CORO_NAME
        )
    }

    /// The operand-free string, list, variable, dict and introspection opcodes
    /// — the other half of [`Op::is_one_byte`].
    const fn is_one_byte_access(self) -> bool {
        matches!(
            self,
            Self::DICT_PUT
                | Self::STR_EQ
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
                | Self::IRULE_MATCHES
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
                | Self::TCLOO_NEXT_LIST
                | Self::TCLOO_NEXT_CLASS_LIST
                | Self::TCLOO_SELF
                | Self::TCLOO_CLASS
                | Self::TCLOO_NS
                | Self::TCLOO_IS_OBJECT
                | Self::TCLOO_ID
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
            | Self::CALL_FUNC1
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
            | Self::TCLOO_NEXT4
            | Self::TCLOO_NEXT_CLASS4
            | Self::TAILCALL4
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
            | Self::ERROR_PREFIX_EQ
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
            Self::INVOKE_REPLACE
            | Self::LREPLACE4
            | Self::LREPLACE
            | Self::UNSET_SCALAR
            | Self::UNSET_ARRAY => 6,

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
            BinOp::Matches => Self::IRULE_MATCHES,
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
