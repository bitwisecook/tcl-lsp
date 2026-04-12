"""Opcode metadata and expression opcode mapping tables."""

from __future__ import annotations

from enum import Enum, auto

from ..expr_ast import BinOp, UnaryOp

# Bytecode opcodes

# Legacy sentinel kept only for the disassembler formatter.
_INDEX_END = -(2**30)


def _parse_tcl_index(s: str) -> int | None:
    """Parse a Tcl index string to an int suitable for IMM instructions.

    Plain integers compile directly.  ``end``-based indices are encoded
    relative to ``_INDEX_END`` (matching C Tcl's ``INDEX_END`` sentinel)
    so the VM can resolve them against the actual collection length at
    runtime.  Returns ``None`` for unparseable strings.
    """
    s = s.strip()
    if s == "end":
        return _INDEX_END
    if s.startswith("end-"):
        try:
            return _INDEX_END - int(s[4:])
        except ValueError:
            return None
    if s.startswith("end+"):
        try:
            return _INDEX_END + int(s[4:])
        except ValueError:
            return None
    try:
        return int(s)
    except ValueError:
        return None


# strclass operand: maps Tcl "string is" class names to numeric indices.
_STR_CLASS_IDS: dict[str, int] = {
    "alnum": 0,
    "alpha": 1,
    "ascii": 2,
    "control": 3,
    "digit": 4,
    "graph": 5,
    "lower": 6,
    "print": 7,
    "punct": 8,
    "space": 9,
    "upper": 10,
    "wordchar": 11,
    "xdigit": 12,
}
_STR_CLASS_NAMES: dict[int, str] = {v: k for k, v in _STR_CLASS_IDS.items()}


class Op(Enum):
    """Tcl 9.0.2 bytecode instruction opcodes (subset)."""

    PUSH1 = auto()
    PUSH4 = auto()
    POP = auto()
    DUP = auto()
    LOAD_SCALAR1 = auto()
    LOAD_SCALAR4 = auto()
    STORE_SCALAR1 = auto()
    STORE_SCALAR4 = auto()
    INCR_SCALAR1 = auto()
    INCR_SCALAR1_IMM = auto()
    INVOKE_STK1 = auto()
    INVOKE_STK4 = auto()
    EVAL_STK = auto()
    EXPR_STK = auto()
    JUMP1 = auto()
    JUMP4 = auto()
    JUMP_TRUE1 = auto()
    JUMP_TRUE4 = auto()
    JUMP_FALSE1 = auto()
    JUMP_FALSE4 = auto()
    ADD = auto()
    SUB = auto()
    MULT = auto()
    DIV = auto()
    MOD = auto()
    EXPON = auto()
    LSHIFT = auto()
    RSHIFT = auto()
    BITOR = auto()
    BITXOR = auto()
    BITAND = auto()
    EQ = auto()
    NEQ = auto()
    LT = auto()
    GT = auto()
    LE = auto()
    GE = auto()
    STR_EQ = auto()
    STR_NEQ = auto()
    STR_CMP = auto()
    STR_LT = auto()
    STR_GT = auto()
    STR_LE = auto()
    STR_GE = auto()
    STR_CONCAT1 = auto()
    STR_LEN = auto()
    STR_INDEX = auto()
    LIST = auto()
    LIST_LENGTH = auto()
    LIST_INDEX = auto()
    LIST_INDEX_IMM = auto()
    LIST_RANGE_IMM = auto()
    LINDEX_MULTI = auto()
    APPEND_SCALAR1 = auto()
    LAPPEND_SCALAR1 = auto()
    RETURN_IMM = auto()
    DONE = auto()
    START_CMD = auto()
    BREAK = auto()
    CONTINUE = auto()
    BEGIN_CATCH4 = auto()
    END_CATCH = auto()
    PUSH_RESULT = auto()
    PUSH_RETURN_CODE = auto()
    FOREACH_START = auto()
    FOREACH_STEP = auto()
    FOREACH_END = auto()
    JUMP_TABLE = auto()
    NOP = auto()
    UMINUS = auto()
    UPLUS = auto()
    BITNOT = auto()
    LNOT = auto()
    NOT = auto()
    LAND = auto()
    LOR = auto()
    LIST_IN = auto()
    LIST_NOT_IN = auto()
    STR_MAP = auto()
    STR_FIND = auto()
    STR_RFIND = auto()
    STR_REPLACE = auto()
    STR_TRIM = auto()
    STR_TRIM_LEFT = auto()
    STR_TRIM_RIGHT = auto()
    STR_MATCH = auto()
    STR_UPPER = auto()
    STR_LOWER = auto()
    STR_TITLE = auto()
    STR_RANGE = auto()
    STR_RANGE_IMM = auto()
    STR_REVERSE = auto()
    STR_REPEAT = auto()
    REGEXP = auto()
    STORE_STK = auto()
    LOAD_STK = auto()
    STORE_ARRAY_STK = auto()
    LOAD_ARRAY_STK = auto()
    INCR_STK = auto()
    INCR_STK_IMM = auto()
    INCR_ARRAY_STK_IMM = auto()
    APPEND_STK = auto()
    LAPPEND_STK = auto()
    LAPPEND_LIST = auto()
    LAPPEND_LIST_STK = auto()
    LAPPEND_LIST_ARRAY_STK = auto()
    STORE_ARRAY1 = auto()
    LOAD_ARRAY1 = auto()
    LAPPEND_LIST_ARRAY = auto()
    ARRAY_EXISTS_IMM = auto()
    UNSET_STK = auto()
    TAILCALL = auto()
    CONCAT_STK = auto()
    TRY_CVT_TO_NUMERIC = auto()
    VERIFY_DICT = auto()
    DICT_GET = auto()
    DICT_EXISTS = auto()
    INVOKE_REPLACE = auto()
    EXIST_STK = auto()
    EXIST_SCALAR = auto()
    DICT_SET = auto()
    DICT_UNSET = auto()
    DICT_INCR_IMM = auto()
    DICT_APPEND = auto()
    DICT_LAPPEND = auto()
    UPVAR = auto()
    NSUPVAR = auto()
    LREPLACE4 = auto()
    OVER = auto()
    LSET_FLAT = auto()
    LSET_LIST = auto()
    LIST_CONCAT = auto()
    # Tcl 8.5+/8.6+ opcodes for inline catch/try, string-is, etc.
    # (defined but not yet emitted by our codegen)
    PUSH_RETURN_OPTS = auto()  # 8.5+
    RETURN_STK = auto()  # 8.6+
    REVERSE = auto()  # 8.6+
    NUMERIC_TYPE = auto()  # 8.6+
    TRY_CVT_TO_BOOLEAN = auto()  # 8.6+
    STR_CLASS = auto()  # 8.6+
    SYNTAX = auto()  # 8.6+ — compile-time error (like returnImm)
    # iRules engine extensions
    IRULE_CONTAINS = auto()
    IRULE_STARTS_WITH = auto()
    IRULE_ENDS_WITH = auto()
    IRULE_EQUALS = auto()
    IRULE_MATCHES_GLOB = auto()
    IRULE_MATCHES_REGEX = auto()
    IRULE_WORD_AND = auto()
    IRULE_WORD_OR = auto()
    IRULE_WORD_NOT = auto()
    # {*} expansion (Tcl 8.5+)
    EXPAND_START = auto()
    EXPAND_STKTOP = auto()
    INVOKE_EXPANDED = auto()


# Maps Op → (mnemonic as shown in disassembly, instruction size in bytes)
_OP_INFO: dict[Op, tuple[str, int]] = {
    Op.PUSH1: ("push1", 2),
    Op.PUSH4: ("push4", 5),
    Op.POP: ("pop", 1),
    Op.DUP: ("dup", 1),
    Op.LOAD_SCALAR1: ("loadScalar1", 2),
    Op.LOAD_SCALAR4: ("loadScalar4", 5),
    Op.STORE_SCALAR1: ("storeScalar1", 2),
    Op.STORE_SCALAR4: ("storeScalar4", 5),
    Op.INCR_SCALAR1: ("incrScalar1", 2),
    Op.INCR_SCALAR1_IMM: ("incrScalar1Imm", 3),
    Op.INVOKE_STK1: ("invokeStk1", 2),
    Op.INVOKE_STK4: ("invokeStk4", 5),
    Op.EVAL_STK: ("evalStk", 1),
    Op.EXPR_STK: ("exprStk", 1),
    Op.JUMP1: ("jump1", 2),
    Op.JUMP4: ("jump4", 5),
    Op.JUMP_TRUE1: ("jumpTrue1", 2),
    Op.JUMP_TRUE4: ("jumpTrue4", 5),
    Op.JUMP_FALSE1: ("jumpFalse1", 2),
    Op.JUMP_FALSE4: ("jumpFalse4", 5),
    Op.ADD: ("add", 1),
    Op.SUB: ("sub", 1),
    Op.MULT: ("mult", 1),
    Op.DIV: ("div", 1),
    Op.MOD: ("mod", 1),
    Op.EXPON: ("expon", 1),
    Op.LSHIFT: ("lshift", 1),
    Op.RSHIFT: ("rshift", 1),
    Op.BITOR: ("bitor", 1),
    Op.BITXOR: ("bitxor", 1),
    Op.BITAND: ("bitand", 1),
    Op.EQ: ("eq", 1),
    Op.NEQ: ("neq", 1),
    Op.LT: ("lt", 1),
    Op.GT: ("gt", 1),
    Op.LE: ("le", 1),
    Op.GE: ("ge", 1),
    Op.STR_EQ: ("streq", 1),
    Op.STR_NEQ: ("strneq", 1),
    Op.STR_CMP: ("strcmp", 1),
    Op.STR_LT: ("strlt", 1),
    Op.STR_GT: ("strgt", 1),
    Op.STR_LE: ("strle", 1),
    Op.STR_GE: ("strge", 1),
    Op.STR_CONCAT1: ("strcat", 2),
    Op.STR_LEN: ("strlen", 1),
    Op.STR_INDEX: ("strindex", 1),
    Op.LIST: ("list", 5),
    Op.LIST_LENGTH: ("listLength", 1),
    Op.LIST_INDEX: ("listIndex", 1),
    Op.LIST_INDEX_IMM: ("listIndexImm", 5),
    Op.LIST_RANGE_IMM: ("listRangeImm", 9),
    Op.LINDEX_MULTI: ("lindexMulti", 5),
    Op.APPEND_SCALAR1: ("appendScalar1", 2),
    Op.LAPPEND_SCALAR1: ("lappendScalar1", 2),
    Op.RETURN_IMM: ("returnImm", 9),
    Op.DONE: ("done", 1),
    Op.START_CMD: ("startCommand", 9),
    Op.BREAK: ("break", 1),
    Op.CONTINUE: ("continue", 1),
    Op.BEGIN_CATCH4: ("beginCatch4", 5),
    Op.END_CATCH: ("endCatch", 1),
    Op.PUSH_RESULT: ("pushResult", 1),
    Op.PUSH_RETURN_CODE: ("pushReturnCode", 1),
    Op.FOREACH_START: ("foreach_start", 5),
    Op.FOREACH_STEP: ("foreach_step", 1),
    Op.FOREACH_END: ("foreach_end", 1),
    Op.JUMP_TABLE: ("jumpTable", 5),
    Op.NOP: ("nop", 1),
    Op.UMINUS: ("uminus", 1),
    Op.UPLUS: ("uplus", 1),
    Op.BITNOT: ("bitnot", 1),
    Op.LNOT: ("lnot", 1),
    Op.NOT: ("not", 1),
    Op.LAND: ("land", 1),
    Op.LOR: ("lor", 1),
    Op.LIST_IN: ("listIn", 1),
    Op.LIST_NOT_IN: ("listNotIn", 1),
    Op.STR_MAP: ("strmap", 1),
    Op.STR_FIND: ("strfind", 1),
    Op.STR_RFIND: ("strrfind", 1),
    Op.STR_REPLACE: ("strreplace", 1),
    Op.STR_TRIM: ("strtrim", 1),
    Op.STR_TRIM_LEFT: ("strtrimLeft", 1),
    Op.STR_TRIM_RIGHT: ("strtrimRight", 1),
    Op.STR_MATCH: ("strmatch", 2),  # +nocase flag
    Op.STR_UPPER: ("strcaseUpper", 1),
    Op.STR_LOWER: ("strcaseLower", 1),
    Op.STR_TITLE: ("strcaseTitle", 1),
    Op.STR_RANGE: ("strrange", 1),
    Op.STR_RANGE_IMM: ("strrangeImm", 9),
    Op.STR_REVERSE: ("strreverse", 1),
    Op.STR_REPEAT: ("strrepeat", 1),
    Op.REGEXP: ("regexp", 2),  # +argc
    Op.STORE_STK: ("storeStk", 1),
    Op.LOAD_STK: ("loadStk", 1),
    Op.STORE_ARRAY_STK: ("storeArrayStk", 1),
    Op.LOAD_ARRAY_STK: ("loadArrayStk", 1),
    Op.INCR_STK: ("incrStk", 1),
    Op.INCR_STK_IMM: ("incrStkImm", 2),
    Op.INCR_ARRAY_STK_IMM: ("incrArrayStkImm", 2),
    Op.APPEND_STK: ("appendStk", 1),
    Op.LAPPEND_STK: ("lappendStk", 1),
    Op.LAPPEND_LIST: ("lappendList", 5),
    Op.LAPPEND_LIST_STK: ("lappendListStk", 1),
    Op.LAPPEND_LIST_ARRAY_STK: ("lappendListArrayStk", 1),
    Op.STORE_ARRAY1: ("storeArray1", 2),
    Op.LOAD_ARRAY1: ("loadArray1", 2),
    Op.LAPPEND_LIST_ARRAY: ("lappendListArray", 5),
    Op.ARRAY_EXISTS_IMM: ("arrayExistsImm", 5),
    Op.UNSET_STK: ("unsetStk", 2),
    Op.TAILCALL: ("tailcall", 2),
    Op.CONCAT_STK: ("concatStk", 5),
    Op.TRY_CVT_TO_NUMERIC: ("tryCvtToNumeric", 1),
    Op.VERIFY_DICT: ("verifyDict", 1),
    Op.DICT_GET: ("dictGet", 5),
    Op.DICT_EXISTS: ("dictExists", 5),
    Op.INVOKE_REPLACE: ("invokeReplace", 6),
    Op.EXIST_STK: ("existStk", 1),
    Op.EXIST_SCALAR: ("existScalar", 5),
    Op.DICT_SET: ("dictSet", 9),
    Op.DICT_UNSET: ("dictUnset", 9),
    Op.DICT_INCR_IMM: ("dictIncrImm", 9),
    Op.DICT_APPEND: ("dictAppend", 5),
    Op.DICT_LAPPEND: ("dictLappend", 5),
    Op.UPVAR: ("upvar", 5),
    Op.NSUPVAR: ("nsupvar", 5),
    Op.LREPLACE4: ("lreplace4", 6),
    Op.OVER: ("over", 5),
    Op.LSET_FLAT: ("lsetFlat", 5),
    Op.LSET_LIST: ("lsetList", 1),
    Op.LIST_CONCAT: ("listConcat", 1),
    # Tcl 8.5+/8.6+ opcodes (defined but not yet emitted by our codegen)
    Op.PUSH_RETURN_OPTS: ("pushReturnOpts", 1),
    Op.RETURN_STK: ("returnStk", 1),
    Op.REVERSE: ("reverse", 5),
    Op.NUMERIC_TYPE: ("numericType", 1),
    Op.TRY_CVT_TO_BOOLEAN: ("tryCvtToBoolean", 1),
    Op.STR_CLASS: ("strclass", 2),
    Op.SYNTAX: ("syntax", 9),  # same layout as returnImm: op(1) + code(4) + level(4)
    # iRules engine extensions
    Op.IRULE_CONTAINS: ("iruleContains", 1),
    Op.IRULE_STARTS_WITH: ("iruleStartsWith", 1),
    Op.IRULE_ENDS_WITH: ("iruleEndsWith", 1),
    Op.IRULE_EQUALS: ("iruleEquals", 1),
    Op.IRULE_MATCHES_GLOB: ("iruleMatchesGlob", 1),
    Op.IRULE_MATCHES_REGEX: ("iruleMatchesRegex", 1),
    Op.IRULE_WORD_AND: ("iruleAnd", 1),
    Op.IRULE_WORD_OR: ("iruleOr", 1),
    Op.IRULE_WORD_NOT: ("iruleNot", 1),
    # {*} expansion
    Op.EXPAND_START: ("expandStart", 1),
    Op.EXPAND_STKTOP: ("expandStkTop", 5),
    Op.INVOKE_EXPANDED: ("invokeExpanded", 1),
}

# Ops that take an LVT operand — used by the formatter to emit %vN notation
_LVT_OPS = frozenset(
    {
        Op.LOAD_SCALAR1,
        Op.LOAD_SCALAR4,
        Op.STORE_SCALAR1,
        Op.STORE_SCALAR4,
        Op.INCR_SCALAR1,
        Op.INCR_SCALAR1_IMM,
        Op.APPEND_SCALAR1,
        Op.LAPPEND_SCALAR1,
        Op.LAPPEND_LIST,
        Op.STORE_ARRAY1,
        Op.LOAD_ARRAY1,
        Op.LAPPEND_LIST_ARRAY,
        Op.ARRAY_EXISTS_IMM,
        Op.EXIST_SCALAR,
        Op.DICT_APPEND,
        Op.DICT_LAPPEND,
        Op.UPVAR,
        Op.NSUPVAR,
    }
)

# Ops whose operands are jump targets — formatted as relative offsets
_JUMP_OPS = frozenset(
    {
        Op.JUMP1,
        Op.JUMP4,
        Op.JUMP_TRUE1,
        Op.JUMP_TRUE4,
        Op.JUMP_FALSE1,
        Op.JUMP_FALSE4,
    }
)

# BinOp → bytecode op
_BINOP_MAP: dict[BinOp, Op] = {
    BinOp.ADD: Op.ADD,
    BinOp.SUB: Op.SUB,
    BinOp.MUL: Op.MULT,
    BinOp.DIV: Op.DIV,
    BinOp.MOD: Op.MOD,
    BinOp.POW: Op.EXPON,
    BinOp.LSHIFT: Op.LSHIFT,
    BinOp.RSHIFT: Op.RSHIFT,
    BinOp.BIT_AND: Op.BITAND,
    BinOp.BIT_OR: Op.BITOR,
    BinOp.BIT_XOR: Op.BITXOR,
    BinOp.AND: Op.LAND,
    BinOp.OR: Op.LOR,
    BinOp.EQ: Op.EQ,
    BinOp.NE: Op.NEQ,
    BinOp.LT: Op.LT,
    BinOp.GT: Op.GT,
    BinOp.LE: Op.LE,
    BinOp.GE: Op.GE,
    BinOp.STR_EQ: Op.STR_EQ,
    BinOp.STR_NE: Op.STR_NEQ,
    BinOp.STR_LT: Op.STR_LT,
    BinOp.STR_GT: Op.STR_GT,
    BinOp.STR_LE: Op.STR_LE,
    BinOp.STR_GE: Op.STR_GE,
    BinOp.IN: Op.LIST_IN,
    BinOp.NI: Op.LIST_NOT_IN,
    # iRules engine extensions
    BinOp.WORD_AND: Op.IRULE_WORD_AND,
    BinOp.WORD_OR: Op.IRULE_WORD_OR,
    BinOp.CONTAINS: Op.IRULE_CONTAINS,
    BinOp.STARTS_WITH: Op.IRULE_STARTS_WITH,
    BinOp.ENDS_WITH: Op.IRULE_ENDS_WITH,
    BinOp.STR_EQUALS: Op.IRULE_EQUALS,
    BinOp.MATCHES_GLOB: Op.IRULE_MATCHES_GLOB,
    BinOp.MATCHES_REGEX: Op.IRULE_MATCHES_REGEX,
}

# UnaryOp → bytecode op
_UNARYOP_MAP: dict[UnaryOp, Op] = {
    UnaryOp.NEG: Op.UMINUS,
    UnaryOp.POS: Op.UPLUS,
    UnaryOp.BIT_NOT: Op.BITNOT,
    UnaryOp.NOT: Op.NOT,
    UnaryOp.WORD_NOT: Op.IRULE_WORD_NOT,
}
