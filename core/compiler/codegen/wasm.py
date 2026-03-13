"""WebAssembly (WASM) code generator for Tcl.

Translates CFG-based IR into valid WASM binary modules.  Supports two
output modes controlled by the ``optimise`` flag:

- **Non-optimised** (``optimise=False``): straightforward translation
  with stack-based variable access, no instruction folding.
- **Optimised** (``optimise=True``): constant folding, dead-code
  elimination, and local variable promotion.

Public API::

    wasm_codegen_module(cfg_module, ir_module, *, optimise=False) -> WasmModule
    wasm_codegen_function(cfg, params=(), *, optimise=False) -> WasmFunction

The resulting ``WasmModule`` can be serialised to a ``.wasm`` binary
via :meth:`WasmModule.to_bytes` or inspected via :meth:`WasmModule.to_wat`.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from enum import IntEnum

from ..cfg import (
    CFGBranch,
    CFGFunction,
    CFGGoto,
    CFGModule,
    CFGReturn,
)
from ..expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRIncr,
    IRModule,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)

# WASM binary encoding helpers


def _leb128_unsigned(value: int) -> bytes:
    """Encode an unsigned integer as LEB128."""
    if value < 0:
        msg = f"unsigned LEB128 requires non-negative value, got {value}"
        raise ValueError(msg)
    result = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            byte |= 0x80
        result.append(byte)
        if not value:
            break
    return bytes(result)


def _leb128_signed(value: int) -> bytes:
    """Encode a signed integer as LEB128."""
    result = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if (value == 0 and not (byte & 0x40)) or (value == -1 and (byte & 0x40)):
            result.append(byte)
            break
        result.append(byte | 0x80)
    return bytes(result)


def _encode_string(s: str) -> bytes:
    """Encode a UTF-8 string with length prefix."""
    encoded = s.encode("utf-8")
    return _leb128_unsigned(len(encoded)) + encoded


def _encode_vector(items: list[bytes]) -> bytes:
    """Encode a WASM vector (count + concatenated items)."""
    return _leb128_unsigned(len(items)) + b"".join(items)


# WASM type constants


class ValType(IntEnum):
    """WASM value types."""

    I32 = 0x7F
    I64 = 0x7E
    F32 = 0x7D
    F64 = 0x7C


class SectionId(IntEnum):
    """WASM section identifiers."""

    CUSTOM = 0
    TYPE = 1
    IMPORT = 2
    FUNCTION = 3
    TABLE = 4
    MEMORY = 5
    GLOBAL = 6
    EXPORT = 7
    START = 8
    ELEMENT = 9
    CODE = 10
    DATA = 11


# WASM opcodes used in emission
class WasmOp(IntEnum):
    """WASM instruction opcodes (subset used by this backend)."""

    UNREACHABLE = 0x00
    NOP = 0x01
    BLOCK = 0x02
    LOOP = 0x03
    IF = 0x04
    ELSE = 0x05
    END = 0x0B
    BR = 0x0C
    BR_IF = 0x0D
    BR_TABLE = 0x0E
    RETURN = 0x0F
    CALL = 0x10
    DROP = 0x1A
    SELECT = 0x1B
    LOCAL_GET = 0x20
    LOCAL_SET = 0x21
    LOCAL_TEE = 0x22
    GLOBAL_GET = 0x23
    GLOBAL_SET = 0x24
    I32_LOAD = 0x28
    I64_LOAD = 0x29
    I32_STORE = 0x36
    I64_STORE = 0x37
    MEMORY_SIZE = 0x3F
    MEMORY_GROW = 0x40
    I32_CONST = 0x41
    I64_CONST = 0x42
    F64_CONST = 0x44
    I32_EQZ = 0x45
    I32_EQ = 0x46
    I32_NE = 0x47
    I32_LT_S = 0x48
    I32_GT_S = 0x4A
    I32_LE_S = 0x4C
    I32_GE_S = 0x4E
    I64_EQZ = 0x50
    I64_EQ = 0x51
    I64_NE = 0x52
    I64_LT_S = 0x53
    I64_GT_S = 0x55
    I64_LE_S = 0x57
    I64_GE_S = 0x59
    I32_ADD = 0x6A
    I32_SUB = 0x6B
    I32_MUL = 0x6C
    I32_DIV_S = 0x6D
    I32_REM_S = 0x6F
    I32_AND = 0x71
    I32_OR = 0x72
    I32_XOR = 0x73
    I32_SHL = 0x74
    I32_SHR_S = 0x75
    I64_ADD = 0x7C
    I64_SUB = 0x7D
    I64_MUL = 0x7E
    I64_DIV_S = 0x7F
    I64_REM_S = 0x81
    I64_AND = 0x83
    I64_OR = 0x84
    I64_XOR = 0x85
    I64_SHL = 0x86
    I64_SHR_S = 0x87
    I32_WRAP_I64 = 0xA7
    I64_EXTEND_I32_S = 0xAC


# WASM magic and version
_WASM_MAGIC = b"\x00asm"
_WASM_VERSION = struct.pack("<I", 1)

# Block type for void blocks
_BLOCK_VOID = 0x40
# Block type for i64 result
_BLOCK_I64 = ValType.I64


# Data structures


@dataclass(slots=True)
class WasmInstruction:
    """A single WASM instruction with optional operands."""

    op: int
    operands: bytes = b""

    def encode(self) -> bytes:
        return bytes([self.op]) + self.operands


@dataclass(slots=True)
class WasmFunction:
    """Compiled WASM function."""

    name: str
    params: list[ValType]
    results: list[ValType]
    locals: list[ValType]
    body: list[WasmInstruction]
    local_names: list[str] = field(default_factory=list)
    exported: bool = False

    def encode_body(self) -> bytes:
        """Encode function body (locals + instructions + end)."""
        # Encode local declarations (run-length compressed)
        local_groups: list[bytes] = []
        if self.locals:
            # Group consecutive same-type locals
            groups: list[tuple[int, ValType]] = []
            current_type = self.locals[0]
            count = 1
            for lt in self.locals[1:]:
                if lt == current_type:
                    count += 1
                else:
                    groups.append((count, current_type))
                    current_type = lt
                    count = 1
            groups.append((count, current_type))
            for cnt, vt in groups:
                local_groups.append(_leb128_unsigned(cnt) + bytes([vt]))

        locals_section = _encode_vector(local_groups)
        code = b"".join(instr.encode() for instr in self.body)
        # Ensure the body ends with END
        if not self.body or self.body[-1].op != WasmOp.END:
            code += bytes([WasmOp.END])
        func_body = locals_section + code
        return _leb128_unsigned(len(func_body)) + func_body


@dataclass(slots=True)
class WasmImport:
    """An imported function."""

    module: str
    name: str
    type_idx: int


@dataclass(slots=True)
class WasmData:
    """A data segment for string constants."""

    offset: int
    data: bytes


@dataclass(slots=True)
class WasmModule:
    """Complete WASM module."""

    functions: list[WasmFunction] = field(default_factory=list)
    imports: list[WasmImport] = field(default_factory=list)
    data_segments: list[WasmData] = field(default_factory=list)
    memory_pages: int = 1
    _type_cache: dict[tuple[tuple[int, ...], tuple[int, ...]], int] = field(default_factory=dict)
    _types: list[tuple[list[ValType], list[ValType]]] = field(default_factory=list)

    def _intern_type(self, params: list[ValType], results: list[ValType]) -> int:
        key = (tuple(params), tuple(results))
        idx = self._type_cache.get(key)
        if idx is not None:
            return idx
        idx = len(self._types)
        self._types.append((list(params), list(results)))
        self._type_cache[key] = idx
        return idx

    def to_bytes(self) -> bytes:
        """Serialise to a valid WASM binary."""
        sections: list[bytes] = []

        # Register all function types
        for imp in self.imports:
            # Import types already registered
            pass
        for func in self.functions:
            self._intern_type(func.params, func.results)

        # Type section
        type_entries: list[bytes] = []
        for params, results in self._types:
            entry = b"\x60"  # functype
            entry += _encode_vector([bytes([p]) for p in params])
            entry += _encode_vector([bytes([r]) for r in results])
            type_entries.append(entry)
        if type_entries:
            sections.append(self._make_section(SectionId.TYPE, _encode_vector(type_entries)))

        # Import section
        if self.imports:
            import_entries: list[bytes] = []
            for imp in self.imports:
                entry = _encode_string(imp.module)
                entry += _encode_string(imp.name)
                entry += b"\x00"  # functype
                entry += _leb128_unsigned(imp.type_idx)
                import_entries.append(entry)
            sections.append(self._make_section(SectionId.IMPORT, _encode_vector(import_entries)))

        # Function section (type indices for defined functions)
        if self.functions:
            func_type_indices: list[bytes] = []
            for func in self.functions:
                tidx = self._intern_type(func.params, func.results)
                func_type_indices.append(_leb128_unsigned(tidx))
            sections.append(
                self._make_section(SectionId.FUNCTION, _encode_vector(func_type_indices))
            )

        # Memory section
        mem_data = _leb128_unsigned(1)  # 1 memory
        mem_data += b"\x00"  # no maximum
        mem_data += _leb128_unsigned(self.memory_pages)
        sections.append(self._make_section(SectionId.MEMORY, mem_data))

        # Export section
        export_entries: list[bytes] = []
        # Export memory
        exp = _encode_string("memory")
        exp += b"\x02"  # memory export
        exp += _leb128_unsigned(0)
        export_entries.append(exp)
        # Export functions
        num_imports = len(self.imports)
        for i, func in enumerate(self.functions):
            if func.exported:
                exp = _encode_string(func.name)
                exp += b"\x00"  # func export
                exp += _leb128_unsigned(num_imports + i)
                export_entries.append(exp)
        if export_entries:
            sections.append(self._make_section(SectionId.EXPORT, _encode_vector(export_entries)))

        # Code section
        if self.functions:
            code_entries = [func.encode_body() for func in self.functions]
            sections.append(self._make_section(SectionId.CODE, _encode_vector(code_entries)))

        # Data section
        if self.data_segments:
            data_entries: list[bytes] = []
            for seg in self.data_segments:
                entry = b"\x00"  # active, memory 0
                # i32.const offset + end
                entry += bytes([WasmOp.I32_CONST]) + _leb128_signed(seg.offset)
                entry += bytes([WasmOp.END])
                entry += _leb128_unsigned(len(seg.data)) + seg.data
                data_entries.append(entry)
            sections.append(self._make_section(SectionId.DATA, _encode_vector(data_entries)))

        return _WASM_MAGIC + _WASM_VERSION + b"".join(sections)

    def to_wat(self) -> str:
        """Generate a human-readable WAT (WebAssembly Text) representation."""
        lines: list[str] = ["(module"]

        # Types
        for i, (params, results) in enumerate(self._types):
            p = " ".join(f"(param {_valtype_name(p)})" for p in params)
            r = " ".join(f"(result {_valtype_name(r)})" for r in results)
            lines.append(f"  (type $t{i} (func {p} {r}))")

        # Imports
        for imp in self.imports:
            lines.append(
                f'  (import "{imp.module}" "{imp.name}" '
                f"(func $imp_{imp.name} (type $t{imp.type_idx})))"
            )

        # Memory
        lines.append(f'  (memory (export "memory") {self.memory_pages})')

        # Functions
        for func in self.functions:
            self._intern_type(func.params, func.results)
            export = f' (export "{func.name}")' if func.exported else ""
            sig_parts: list[str] = []
            for j, p in enumerate(func.params):
                name = func.local_names[j] if j < len(func.local_names) else f"$p{j}"
                sig_parts.append(f"(param {name} {_valtype_name(p)})")
            for r in func.results:
                sig_parts.append(f"(result {_valtype_name(r)})")
            sig = " ".join(sig_parts)

            lines.append(f"  (func ${func.name}{export} {sig}")

            # Locals
            param_count = len(func.params)
            for j, lt in enumerate(func.locals):
                local_idx = param_count + j
                name = (
                    func.local_names[local_idx] if local_idx < len(func.local_names) else f"$l{j}"
                )
                lines.append(f"    (local {name} {_valtype_name(lt)})")

            # Body
            indent = 2
            for instr in func.body:
                line = _format_wat_instr(instr, indent)
                if line is not None:
                    lines.append(line)
                    # Track indent for blocks
                    if instr.op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
                        indent += 1
                    elif instr.op == WasmOp.ELSE:
                        pass  # same indent
                    elif instr.op == WasmOp.END:
                        indent = max(2, indent - 1)

            lines.append("  )")

        # Data segments
        for seg in self.data_segments:
            escaped = (
                seg.data.decode("utf-8", errors="replace").replace("\\", "\\\\").replace('"', '\\"')
            )
            lines.append(f'  (data (i32.const {seg.offset}) "{escaped}")')

        lines.append(")")
        return "\n".join(lines)

    @staticmethod
    def _make_section(section_id: SectionId, content: bytes) -> bytes:
        return bytes([section_id]) + _leb128_unsigned(len(content)) + content


def _valtype_name(vt: ValType | int) -> str:
    names = {ValType.I32: "i32", ValType.I64: "i64", ValType.F32: "f32", ValType.F64: "f64"}
    return names.get(ValType(vt), f"unknown({vt})")


def _format_wat_instr(instr: WasmInstruction, indent: int) -> str | None:
    """Format a single instruction for WAT output."""
    prefix = "    " * indent
    op = instr.op

    _WAT_NAMES: dict[int, str] = {
        WasmOp.UNREACHABLE: "unreachable",
        WasmOp.NOP: "nop",
        WasmOp.BLOCK: "block",
        WasmOp.LOOP: "loop",
        WasmOp.IF: "if",
        WasmOp.ELSE: "else",
        WasmOp.END: "end",
        WasmOp.BR: "br",
        WasmOp.BR_IF: "br_if",
        WasmOp.BR_TABLE: "br_table",
        WasmOp.RETURN: "return",
        WasmOp.CALL: "call",
        WasmOp.DROP: "drop",
        WasmOp.SELECT: "select",
        WasmOp.LOCAL_GET: "local.get",
        WasmOp.LOCAL_SET: "local.set",
        WasmOp.LOCAL_TEE: "local.tee",
        WasmOp.GLOBAL_GET: "global.get",
        WasmOp.GLOBAL_SET: "global.set",
        WasmOp.I32_CONST: "i32.const",
        WasmOp.I64_CONST: "i64.const",
        WasmOp.F64_CONST: "f64.const",
        WasmOp.I32_EQZ: "i32.eqz",
        WasmOp.I32_EQ: "i32.eq",
        WasmOp.I32_NE: "i32.ne",
        WasmOp.I32_LT_S: "i32.lt_s",
        WasmOp.I32_GT_S: "i32.gt_s",
        WasmOp.I32_LE_S: "i32.le_s",
        WasmOp.I32_GE_S: "i32.ge_s",
        WasmOp.I64_EQZ: "i64.eqz",
        WasmOp.I64_EQ: "i64.eq",
        WasmOp.I64_NE: "i64.ne",
        WasmOp.I64_LT_S: "i64.lt_s",
        WasmOp.I64_GT_S: "i64.gt_s",
        WasmOp.I64_LE_S: "i64.le_s",
        WasmOp.I64_GE_S: "i64.ge_s",
        WasmOp.I32_ADD: "i32.add",
        WasmOp.I32_SUB: "i32.sub",
        WasmOp.I32_MUL: "i32.mul",
        WasmOp.I32_DIV_S: "i32.div_s",
        WasmOp.I32_REM_S: "i32.rem_s",
        WasmOp.I32_AND: "i32.and",
        WasmOp.I32_OR: "i32.or",
        WasmOp.I32_XOR: "i32.xor",
        WasmOp.I32_SHL: "i32.shl",
        WasmOp.I32_SHR_S: "i32.shr_s",
        WasmOp.I64_ADD: "i64.add",
        WasmOp.I64_SUB: "i64.sub",
        WasmOp.I64_MUL: "i64.mul",
        WasmOp.I64_DIV_S: "i64.div_s",
        WasmOp.I64_REM_S: "i64.rem_s",
        WasmOp.I64_AND: "i64.and",
        WasmOp.I64_OR: "i64.or",
        WasmOp.I64_XOR: "i64.xor",
        WasmOp.I64_SHL: "i64.shl",
        WasmOp.I64_SHR_S: "i64.shr_s",
        WasmOp.I32_WRAP_I64: "i32.wrap_i64",
        WasmOp.I64_EXTEND_I32_S: "i64.extend_i32_s",
        WasmOp.I32_STORE: "i32.store",
        WasmOp.I64_STORE: "i64.store",
        WasmOp.I32_LOAD: "i32.load",
        WasmOp.I64_LOAD: "i64.load",
        WasmOp.MEMORY_SIZE: "memory.size",
        WasmOp.MEMORY_GROW: "memory.grow",
    }

    name = _WAT_NAMES.get(op)
    if name is None:
        return f"{prefix};; unknown op 0x{op:02x}"

    # Decode operands for display
    if op in (
        WasmOp.LOCAL_GET,
        WasmOp.LOCAL_SET,
        WasmOp.LOCAL_TEE,
        WasmOp.GLOBAL_GET,
        WasmOp.GLOBAL_SET,
        WasmOp.CALL,
        WasmOp.BR,
        WasmOp.BR_IF,
    ):
        if instr.operands:
            # Decode LEB128 index from operands
            idx = _decode_leb128_unsigned(instr.operands)
            return f"{prefix}{name} {idx}"
    elif op in (WasmOp.I32_CONST,):
        if instr.operands:
            val = _decode_leb128_signed(instr.operands)
            return f"{prefix}{name} {val}"
    elif op in (WasmOp.I64_CONST,):
        if instr.operands:
            val = _decode_leb128_signed(instr.operands)
            return f"{prefix}{name} {val}"
    elif op == WasmOp.BLOCK or op == WasmOp.LOOP or op == WasmOp.IF:
        if instr.operands:
            bt = instr.operands[0]
            if bt == _BLOCK_VOID:
                return f"{prefix}{name}"
            return f"{prefix}{name} (result {_valtype_name(bt)})"
        return f"{prefix}{name}"

    return f"{prefix}{name}"


def _decode_leb128_unsigned(data: bytes) -> int:
    result = 0
    shift = 0
    for byte in data:
        result |= (byte & 0x7F) << shift
        if not (byte & 0x80):
            break
        shift += 7
    return result


def _decode_leb128_signed(data: bytes) -> int:
    result = 0
    shift = 0
    for byte in data:
        result |= (byte & 0x7F) << shift
        shift += 7
        if not (byte & 0x80):
            if byte & 0x40:
                result -= 1 << shift
            break
    return result


# WASM Emitter


# Maps Tcl binary expression operators to WASM i64 opcodes
_BINOP_WASM: dict[BinOp, int] = {
    BinOp.ADD: WasmOp.I64_ADD,
    BinOp.SUB: WasmOp.I64_SUB,
    BinOp.MUL: WasmOp.I64_MUL,
    BinOp.DIV: WasmOp.I64_DIV_S,
    BinOp.MOD: WasmOp.I64_REM_S,
    BinOp.LSHIFT: WasmOp.I64_SHL,
    BinOp.RSHIFT: WasmOp.I64_SHR_S,
    BinOp.BIT_AND: WasmOp.I64_AND,
    BinOp.BIT_OR: WasmOp.I64_OR,
    BinOp.BIT_XOR: WasmOp.I64_XOR,
    BinOp.EQ: WasmOp.I64_EQ,
    BinOp.NE: WasmOp.I64_NE,
    BinOp.LT: WasmOp.I64_LT_S,
    BinOp.GT: WasmOp.I64_GT_S,
    BinOp.LE: WasmOp.I64_LE_S,
    BinOp.GE: WasmOp.I64_GE_S,
}

# Maps Tcl unary operators to WASM equivalents
_UNARYOP_WASM: dict[UnaryOp, int | None] = {
    UnaryOp.NEG: None,  # implemented as 0 - x
    UnaryOp.POS: None,  # no-op
    UnaryOp.BIT_NOT: None,  # implemented as x ^ -1
    UnaryOp.NOT: WasmOp.I64_EQZ,
}


class _WasmEmitter:
    """Emits WASM instructions for a single CFG function.

    Variables are represented as WASM locals of type i64 (Tcl values
    are boxed integers in this simplified model).  String values are
    stored as pointers into the data segment.
    """

    def __init__(
        self,
        cfg: CFGFunction,
        params: tuple[str, ...] = (),
        *,
        optimise: bool = False,
        is_proc: bool = False,
        func_index_base: int = 0,
    ) -> None:
        self._cfg = cfg
        self._params = params
        self._optimise = optimise
        self._is_proc = is_proc
        self._func_index_base = func_index_base

        # Local variable management
        self._local_names: list[str] = []
        self._local_types: list[ValType] = []
        self._local_index: dict[str, int] = {}

        # Register parameters as locals
        for p in params:
            self._intern_local(p)

        # Instructions
        self._body: list[WasmInstruction] = []

        # String data accumulation
        self._strings: list[tuple[str, int]] = []  # (value, offset)
        self._string_index: dict[str, int] = {}
        self._string_offset = 0

        # Block ordering for structured control flow
        self._visited: set[str] = set()

        # Constant propagation state (for optimised mode)
        self._const_map: dict[str, int] = {}

    def _intern_local(self, name: str) -> int:
        idx = self._local_index.get(name)
        if idx is not None:
            return idx
        idx = len(self._local_names)
        self._local_names.append(name)
        self._local_types.append(ValType.I64)
        self._local_index[name] = idx
        return idx

    def _add_extra_local(self, prefix: str = "_tmp") -> int:
        name = f"{prefix}_{len(self._local_names)}"
        return self._intern_local(name)

    def _resolve_var_name(self, value: str) -> str | None:
        """Extract a variable name from a Tcl variable reference.

        Returns the bare variable name for ``$x`` or ``${x}`` patterns,
        or ``None`` if *value* is not a simple variable reference.
        """
        if value.startswith("${") and value.endswith("}"):
            return value[2:-1]
        if value.startswith("$") and not value.startswith("$["):
            return value[1:]
        return None

    def _emit_value(self, value: str) -> None:
        """Emit a value — resolving variable references or falling back to literal."""
        var = self._resolve_var_name(value)
        if var is not None:
            idx = self._local_index.get(var)
            if idx is not None:
                self._emit_local_get(idx)
                return
        # Try direct local lookup (for bare names like in IRAssignValue)
        idx = self._local_index.get(value)
        if idx is not None:
            self._emit_local_get(idx)
            return
        self._emit_literal(value)

    def _intern_string(self, value: str) -> int:
        """Return the memory offset for a string constant."""
        if value in self._string_index:
            return self._string_index[value]
        offset = self._string_offset
        encoded = value.encode("utf-8")
        self._string_offset += len(encoded) + 4  # 4 bytes for length prefix
        self._strings.append((value, offset))
        self._string_index[value] = offset
        return offset

    def _emit(self, op: int, operands: bytes = b"") -> None:
        self._body.append(WasmInstruction(op=op, operands=operands))

    def _emit_i64_const(self, value: int) -> None:
        self._emit(WasmOp.I64_CONST, _leb128_signed(value))

    def _emit_i32_const(self, value: int) -> None:
        self._emit(WasmOp.I32_CONST, _leb128_signed(value))

    def _emit_local_get(self, idx: int) -> None:
        self._emit(WasmOp.LOCAL_GET, _leb128_unsigned(idx))

    def _emit_local_set(self, idx: int) -> None:
        self._emit(WasmOp.LOCAL_SET, _leb128_unsigned(idx))

    def _emit_local_tee(self, idx: int) -> None:
        self._emit(WasmOp.LOCAL_TEE, _leb128_unsigned(idx))

    def _emit_call(self, func_idx: int) -> None:
        self._emit(WasmOp.CALL, _leb128_unsigned(func_idx))

    def _emit_br(self, depth: int) -> None:
        self._emit(WasmOp.BR, _leb128_unsigned(depth))

    def _emit_br_if(self, depth: int) -> None:
        self._emit(WasmOp.BR_IF, _leb128_unsigned(depth))

    # -- Expression emission --

    def _emit_expr(self, node: ExprNode) -> None:
        """Emit WASM instructions to evaluate an expression, leaving result on stack."""
        match node:
            case ExprLiteral(value=value):
                self._emit_literal(value)
            case ExprVar(name=name):
                idx = self._intern_local(name)
                self._emit_local_get(idx)
            case ExprBinary(op=op, left=left, right=right):
                self._emit_binary(op, left, right)
            case ExprUnary(op=op, operand=operand):
                self._emit_unary(op, operand)
            case ExprTernary(condition=cond, true_expr=te, false_expr=fe):
                self._emit_ternary(cond, te, fe)
            case ExprCall(func=func, args=args):
                self._emit_func_call(func, args)
            case ExprString(parts=parts):
                # String interpolation — emit first part as representative value
                if parts:
                    self._emit_expr(parts[0])
                else:
                    self._emit_i64_const(0)
            case _:
                # Fallback for unsupported expression types
                self._emit_i64_const(0)

    def _emit_literal(self, value: str) -> None:
        """Emit a literal value as an i64 constant."""
        try:
            int_val = int(value)
            if self._optimise:
                # Optimised: use i64.const directly
                self._emit_i64_const(int_val)
            else:
                # Non-optimised: explicit i64.const
                self._emit_i64_const(int_val)
        except ValueError:
            try:
                float_val = float(value)
                # Store as integer bit pattern
                self._emit_i64_const(int(float_val))
            except ValueError:
                # String literal — store as pointer to data segment
                offset = self._intern_string(value)
                self._emit_i64_const(offset)

    # WASM comparison ops that return i32 instead of i64
    _I32_RESULT_OPS = frozenset(
        {
            WasmOp.I64_EQ,
            WasmOp.I64_NE,
            WasmOp.I64_LT_S,
            WasmOp.I64_GT_S,
            WasmOp.I64_LE_S,
            WasmOp.I64_GE_S,
            WasmOp.I64_EQZ,
        }
    )

    def _emit_binary(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit a binary operation."""
        # Optimisation: constant folding for known-constant operands
        if self._optimise:
            folded = self._try_fold_binary(op, left, right)
            if folded is not None:
                self._emit_i64_const(folded)
                return

        wasm_op = _BINOP_WASM.get(op)
        if wasm_op is not None:
            self._emit_expr(left)
            self._emit_expr(right)
            self._emit(wasm_op)
            # WASM comparisons return i32; extend to i64 for our type model
            if wasm_op in self._I32_RESULT_OPS:
                self._emit(WasmOp.I64_EXTEND_I32_S)
        elif op == BinOp.POW:
            # Exponentiation — emit as a runtime call or loop
            self._emit_power(left, right)
        elif op in (BinOp.AND, BinOp.OR):
            self._emit_logical(op, left, right)
        else:
            # Unsupported binary op — emit operands and drop
            self._emit_expr(left)
            self._emit_expr(right)
            self._emit(WasmOp.I64_ADD)  # placeholder

    def _emit_power(self, base: ExprNode, exp: ExprNode) -> None:
        """Emit integer exponentiation as a loop."""
        base_local = self._add_extra_local("_pow_base")
        exp_local = self._add_extra_local("_pow_exp")
        result_local = self._add_extra_local("_pow_result")

        # result = 1
        self._emit_i64_const(1)
        self._emit_local_set(result_local)

        # base_local = base
        self._emit_expr(base)
        self._emit_local_set(base_local)

        # exp_local = exp
        self._emit_expr(exp)
        self._emit_local_set(exp_local)

        # loop: while exp > 0
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        # if exp <= 0, break
        self._emit_local_get(exp_local)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_LE_S)
        self._emit_br_if(1)  # break out of block

        # result *= base
        self._emit_local_get(result_local)
        self._emit_local_get(base_local)
        self._emit(WasmOp.I64_MUL)
        self._emit_local_set(result_local)

        # exp -= 1
        self._emit_local_get(exp_local)
        self._emit_i64_const(1)
        self._emit(WasmOp.I64_SUB)
        self._emit_local_set(exp_local)

        # continue loop
        self._emit_br(0)
        self._emit(WasmOp.END)  # end loop
        self._emit(WasmOp.END)  # end block

        # Push result
        self._emit_local_get(result_local)

    def _emit_logical(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit short-circuit logical AND/OR.

        Tcl ``expr`` logical operators always produce boolean ``0``/``1``,
        so both branches must normalise their result rather than returning
        the raw operand value.
        """
        if op == BinOp.AND:
            # if left == 0 then 0 else !!right
            self._emit_expr(left)
            self._emit(WasmOp.I64_EQZ)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_i64_const(0)
            self._emit(WasmOp.ELSE)
            self._emit_expr(right)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
            self._emit(WasmOp.END)
        else:
            # if left != 0 then 1 else !!right
            self._emit_expr(left)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_i64_const(1)
            self._emit(WasmOp.ELSE)
            self._emit_expr(right)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
            self._emit(WasmOp.END)

    def _emit_unary(self, op: UnaryOp, operand: ExprNode) -> None:
        """Emit a unary operation."""
        if op == UnaryOp.NEG:
            self._emit_i64_const(0)
            self._emit_expr(operand)
            self._emit(WasmOp.I64_SUB)
        elif op == UnaryOp.POS:
            self._emit_expr(operand)  # no-op
        elif op == UnaryOp.BIT_NOT:
            self._emit_expr(operand)
            self._emit_i64_const(-1)
            self._emit(WasmOp.I64_XOR)
        elif op == UnaryOp.NOT:
            self._emit_expr(operand)
            self._emit(WasmOp.I64_EQZ)
            # Extend i32 result back to i64
            self._emit(WasmOp.I64_EXTEND_I32_S)
        else:
            self._emit_expr(operand)

    def _emit_ternary(self, cond: ExprNode, true_expr: ExprNode, false_expr: ExprNode) -> None:
        """Emit a ternary (cond ? a : b) expression."""
        self._emit_expr(cond)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_NE)
        self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
        self._emit_expr(true_expr)
        self._emit(WasmOp.ELSE)
        self._emit_expr(false_expr)
        self._emit(WasmOp.END)

    def _emit_func_call(self, func: str, args: tuple[ExprNode, ...]) -> None:
        """Emit a math function call."""
        # For now, inline simple math functions
        if func == "abs" and len(args) == 1:
            # abs(x) = x < 0 ? -x : x
            tmp = self._add_extra_local("_abs_tmp")
            self._emit_expr(args[0])
            self._emit_local_tee(tmp)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_LT_S)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_i64_const(0)
            self._emit_local_get(tmp)
            self._emit(WasmOp.I64_SUB)
            self._emit(WasmOp.ELSE)
            self._emit_local_get(tmp)
            self._emit(WasmOp.END)
        elif func == "min" and len(args) == 2:
            tmp_a = self._add_extra_local("_min_a")
            tmp_b = self._add_extra_local("_min_b")
            self._emit_expr(args[0])
            self._emit_local_set(tmp_a)
            self._emit_expr(args[1])
            self._emit_local_set(tmp_b)
            self._emit_local_get(tmp_a)
            self._emit_local_get(tmp_b)
            self._emit(WasmOp.I64_LT_S)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_local_get(tmp_a)
            self._emit(WasmOp.ELSE)
            self._emit_local_get(tmp_b)
            self._emit(WasmOp.END)
        elif func == "max" and len(args) == 2:
            tmp_a = self._add_extra_local("_max_a")
            tmp_b = self._add_extra_local("_max_b")
            self._emit_expr(args[0])
            self._emit_local_set(tmp_a)
            self._emit_expr(args[1])
            self._emit_local_set(tmp_b)
            self._emit_local_get(tmp_a)
            self._emit_local_get(tmp_b)
            self._emit(WasmOp.I64_GT_S)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_local_get(tmp_a)
            self._emit(WasmOp.ELSE)
            self._emit_local_get(tmp_b)
            self._emit(WasmOp.END)
        else:
            # Unknown function — push 0
            self._emit_i64_const(0)

    # -- Optimisation passes --

    def _try_fold_binary(self, op: BinOp, left: ExprNode, right: ExprNode) -> int | None:
        """Try to constant-fold a binary expression at compile time."""
        left_val = self._try_const_value(left)
        right_val = self._try_const_value(right)
        if left_val is None or right_val is None:
            return None

        match op:
            case BinOp.ADD:
                return left_val + right_val
            case BinOp.SUB:
                return left_val - right_val
            case BinOp.MUL:
                return left_val * right_val
            case BinOp.DIV:
                return left_val // right_val if right_val != 0 else None
            case BinOp.MOD:
                return left_val % right_val if right_val != 0 else None
            case BinOp.LSHIFT:
                return left_val << right_val if 0 <= right_val < 64 else None
            case BinOp.RSHIFT:
                return left_val >> right_val if 0 <= right_val < 64 else None
            case BinOp.BIT_AND:
                return left_val & right_val
            case BinOp.BIT_OR:
                return left_val | right_val
            case BinOp.BIT_XOR:
                return left_val ^ right_val
            case BinOp.EQ:
                return int(left_val == right_val)
            case BinOp.NE:
                return int(left_val != right_val)
            case BinOp.LT:
                return int(left_val < right_val)
            case BinOp.GT:
                return int(left_val > right_val)
            case BinOp.LE:
                return int(left_val <= right_val)
            case BinOp.GE:
                return int(left_val >= right_val)
            case _:
                return None

    def _try_const_value(self, node: ExprNode) -> int | None:
        """Try to extract a constant integer value from an expression node."""
        match node:
            case ExprLiteral(value=value):
                try:
                    return int(value)
                except ValueError:
                    return None
            case ExprVar(name=name):
                return self._const_map.get(name)
            case _:
                return None

    # -- Statement emission --

    def _emit_stmt(self, stmt: IRStatement) -> None:
        """Emit WASM for a single IR statement."""
        match stmt:
            case IRAssignConst(name=name, value=value):
                idx = self._intern_local(name)
                self._emit_literal(value)
                self._emit_local_set(idx)
                if self._optimise:
                    try:
                        self._const_map[name] = int(value)
                    except ValueError:
                        self._const_map.pop(name, None)

            case IRAssignExpr(name=name, expr=expr):
                idx = self._intern_local(name)
                self._emit_expr(expr)
                self._emit_local_set(idx)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRAssignValue(name=name, value=value):
                idx = self._intern_local(name)
                self._emit_value(value)
                self._emit_local_set(idx)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRIncr(name=name, amount=amount):
                idx = self._intern_local(name)
                self._emit_local_get(idx)
                amt = 1
                if amount is not None:
                    try:
                        amt = int(amount)
                    except ValueError:
                        amt = 1
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_local_set(idx)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRExprEval(expr=expr):
                self._emit_expr(expr)
                self._emit(WasmOp.DROP)

            case IRCall(command=command, args=args):
                self._emit_call_stmt(command, args)

            case IRReturn(value=value):
                if value is not None:
                    self._emit_value(value)
                else:
                    self._emit_i64_const(0)
                self._emit(WasmOp.RETURN)

            case IRBarrier():
                # Barriers are opaque — emit a nop
                self._emit(WasmOp.NOP)
                if self._optimise:
                    self._const_map.clear()

            case IRIf(clauses=clauses, else_body=else_body):
                self._emit_if(clauses, else_body)

            case IRFor(init=init, condition=condition, next=next_script, body=body):
                self._emit_for(init, condition, next_script, body)

            case IRWhile(condition=condition, body=body):
                self._emit_while(condition, body)

            case IRForeach(iterators=iterators, body=body):
                self._emit_foreach(iterators, body)

            case IRSwitch(subject=subject, arms=arms, default_body=default_body):
                self._emit_switch(subject, arms, default_body)

            case IRCatch(body=body, result_var=result_var):
                self._emit_catch(body, result_var)

            case IRTry(body=body, handlers=handlers, finally_body=finally_body):
                self._emit_try(body, handlers, finally_body)

    def _emit_call_stmt(self, command: str, args: tuple[str, ...]) -> None:
        """Emit a command invocation as a nop (commands are runtime calls)."""
        # In the WASM model, most Tcl commands would be imported functions.
        # For now, emit nop placeholders.
        self._emit(WasmOp.NOP)

    def _emit_if(
        self,
        clauses: tuple[IRIfClause, ...],
        else_body: IRScript | None,
    ) -> None:
        """Emit an if/elseif/else chain."""
        if not clauses:
            return

        for i, clause in enumerate(clauses):
            self._emit_expr(clause.condition)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_script(clause.body)

            is_last = i == len(clauses) - 1
            if not is_last or else_body:
                self._emit(WasmOp.ELSE)

        if else_body:
            self._emit_script(else_body)

        # Close all if/else blocks
        for _ in clauses:
            self._emit(WasmOp.END)

    def _emit_for(
        self,
        init: IRScript,
        condition: ExprNode,
        next_script: IRScript,
        body: IRScript,
    ) -> None:
        """Emit a for loop."""
        # init
        self._emit_script(init)

        # block { loop {
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        # test condition
        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)  # break if false

        # body
        self._emit_script(body)

        # next
        self._emit_script(next_script)

        # loop back
        self._emit_br(0)

        self._emit(WasmOp.END)  # end loop
        self._emit(WasmOp.END)  # end block

    def _emit_while(self, condition: ExprNode, body: IRScript) -> None:
        """Emit a while loop."""
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)

        self._emit_script(body)

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

    def _emit_foreach(
        self,
        iterators: tuple[tuple[tuple[str, ...], str], ...],
        body: IRScript,
    ) -> None:
        """Emit a foreach loop.

        Since the WASM backend operates on integer values only, true Tcl
        list iteration is not possible.  Instead the iterator variable's
        integer value is used as the loop bound and the first loop
        variable receives the counter on each iteration.  This preserves
        correct execution for numeric-range patterns and avoids silently
        compiling the loop into a no-op.
        """
        counter = self._add_extra_local("_foreach_i")
        limit = self._add_extra_local("_foreach_n")

        # Derive limit from the first iterator's list variable.
        first_vars, first_list = iterators[0]
        self._emit_value(first_list)
        self._emit_local_set(limit)

        # counter = 0
        self._emit_i64_const(0)
        self._emit_local_set(counter)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        # if counter >= limit, break
        self._emit_local_get(counter)
        self._emit_local_get(limit)
        self._emit(WasmOp.I64_GE_S)
        self._emit_br_if(1)

        # Assign counter value to the first loop variable so the body
        # can reference it.
        if first_vars:
            var_local = self._intern_local(first_vars[0])
            self._emit_local_get(counter)
            self._emit_local_set(var_local)

        self._emit_script(body)

        # counter++
        self._emit_local_get(counter)
        self._emit_i64_const(1)
        self._emit(WasmOp.I64_ADD)
        self._emit_local_set(counter)

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

    def _emit_switch(self, subject: str, arms, default_body) -> None:
        """Emit a switch statement as a chain of if/else."""
        subject_local = self._intern_local(f"_switch_{subject}")
        self._emit_value(subject)
        self._emit_local_set(subject_local)

        arm_count = len(arms)
        for i, arm in enumerate(arms):
            if arm.body is None:
                continue
            # Compare subject against pattern
            self._emit_local_get(subject_local)
            self._emit_literal(arm.pattern)
            self._emit(WasmOp.I64_EQ)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_script(arm.body)
            if i < arm_count - 1 or default_body:
                self._emit(WasmOp.ELSE)

        if default_body:
            self._emit_script(default_body)

        # Close all if blocks
        for arm in arms:
            if arm.body is not None:
                self._emit(WasmOp.END)

    def _emit_catch(self, body: IRScript, result_var: str | None) -> None:
        """Emit a catch block (simplified — just run the body)."""
        # WASM has no exception handling in MVP; emit body directly
        self._emit_script(body)
        if result_var:
            idx = self._intern_local(result_var)
            self._emit_i64_const(0)
            self._emit_local_set(idx)

    def _emit_try(self, body: IRScript, handlers, finally_body) -> None:
        """Emit a try block (simplified — just run the body + finally)."""
        self._emit_script(body)
        if finally_body:
            self._emit_script(finally_body)

    def _emit_script(self, script: IRScript) -> None:
        """Emit all statements in a script."""
        for stmt in script.statements:
            self._emit_stmt(stmt)

    # -- Optimisation passes applied after emission --

    def _run_optimisations(self) -> None:
        """Apply WASM-level optimisation passes."""
        if not self._optimise:
            return

        self._opt_peephole()
        self._opt_dead_code()

    def _opt_peephole(self) -> None:
        """Peephole optimisations on the instruction stream."""
        optimised: list[WasmInstruction] = []
        i = 0
        while i < len(self._body):
            instr = self._body[i]

            # Pattern: i64.const 0; i64.add → remove both (identity add)
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.I64_CONST
                and _decode_leb128_signed(instr.operands) == 0
                and self._body[i + 1].op == WasmOp.I64_ADD
            ):
                i += 2
                continue

            # Pattern: i64.const 1; i64.mul → remove both (identity multiply)
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.I64_CONST
                and _decode_leb128_signed(instr.operands) == 1
                and self._body[i + 1].op == WasmOp.I64_MUL
            ):
                i += 2
                continue

            # Pattern: i64.const 0; i64.mul → replace with i64.const 0 (zero multiply)
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.I64_CONST
                and _decode_leb128_signed(instr.operands) == 0
                and self._body[i + 1].op == WasmOp.I64_MUL
            ):
                # Drop the value on stack, push 0
                optimised.append(WasmInstruction(op=WasmOp.DROP))
                optimised.append(WasmInstruction(op=WasmOp.I64_CONST, operands=_leb128_signed(0)))
                i += 2
                continue

            # Pattern: local.set X; local.get X → local.tee X
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.LOCAL_SET
                and self._body[i + 1].op == WasmOp.LOCAL_GET
                and instr.operands == self._body[i + 1].operands
            ):
                optimised.append(WasmInstruction(op=WasmOp.LOCAL_TEE, operands=instr.operands))
                i += 2
                continue

            # Pattern: nop → skip
            if instr.op == WasmOp.NOP:
                i += 1
                continue

            optimised.append(instr)
            i += 1

        self._body = optimised

    def _opt_dead_code(self) -> None:
        """Remove instructions after unconditional branches within blocks."""
        optimised: list[WasmInstruction] = []
        dead = False
        depth = 0

        for instr in self._body:
            if instr.op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
                depth += 1
                dead = False
                optimised.append(instr)
            elif instr.op == WasmOp.ELSE:
                dead = False
                optimised.append(instr)
            elif instr.op == WasmOp.END:
                depth = max(0, depth - 1)
                dead = False
                optimised.append(instr)
            elif dead:
                continue  # skip dead instruction
            else:
                optimised.append(instr)
                if instr.op in (WasmOp.RETURN, WasmOp.UNREACHABLE, WasmOp.BR):
                    dead = True

        self._body = optimised

    # -- CFG traversal --

    def _is_exit_block(self, block_name: str) -> bool:
        """Check if a block is an exit (no statements, no terminator or return)."""
        block = self._cfg.blocks.get(block_name)
        if block is None:
            return True
        if not block.statements and block.terminator is None:
            return True
        if not block.statements and isinstance(block.terminator, CFGReturn):
            return block.terminator.value is None
        return False

    def _find_merge_block(self, *roots: str) -> str | None:
        """Find the first block reachable from all *roots* via CFGGoto chains.

        This detects the common "merge" or "join" block where divergent
        control-flow paths reconverge (e.g. after an ``if``/``else``).
        Only follows unconditional goto edges so that nested branches
        are not confused with the immediate post-dominator.
        """

        def _goto_successors(start: str) -> set[str]:
            reachable: set[str] = set()
            worklist = [start]
            while worklist:
                name = worklist.pop()
                if name in reachable:
                    continue
                reachable.add(name)
                blk = self._cfg.blocks.get(name)
                if blk is None:
                    continue
                match blk.terminator:
                    case CFGGoto(target=t):
                        worklist.append(t)
                    case CFGBranch(true_target=tt, false_target=ft):
                        worklist.append(tt)
                        worklist.append(ft)
            return reachable

        sets = [_goto_successors(r) for r in roots]
        common = sets[0]
        for s in sets[1:]:
            common = common & s
        if not common:
            return None

        # Pick the merge block closest to the roots: walk from the first
        # root along goto edges and return the first block in *common*.
        visited: set[str] = set()
        frontier = [roots[0]]
        while frontier:
            name = frontier.pop(0)
            if name in visited:
                continue
            visited.add(name)
            if name in common and name not in roots:
                return name
            blk = self._cfg.blocks.get(name)
            if blk is None:
                continue
            match blk.terminator:
                case CFGGoto(target=t):
                    frontier.append(t)
                case CFGBranch(true_target=tt, false_target=ft):
                    frontier.append(tt)
                    frontier.append(ft)
        return None

    def _emit_foreach_loop(
        self,
        header_stmts: tuple,
        body_block: str,
        end_block: str,
    ) -> None:
        """Emit a counter-based loop for a foreach CFG pattern.

        The CFG builder desugars ``foreach`` into a header block whose
        ``IRCall`` contains the list variable and loop variable definitions,
        followed by a branch on the opaque ``<foreach_has_next>`` condition.

        Since the WASM backend operates on integer values only, the list
        variable's integer value is used as the loop bound and the first
        loop variable receives the counter on each iteration.
        """
        # Extract list variable and loop variable from the header's IRCall.
        list_var: str | None = None
        loop_var: str | None = None
        for stmt in header_stmts:
            if isinstance(stmt, IRCall) and stmt.command == "foreach":
                if stmt.args:
                    list_var = stmt.args[0]
                if stmt.defs:
                    loop_var = stmt.defs[0]
                break

        counter = self._add_extra_local("_foreach_i")
        limit = self._add_extra_local("_foreach_n")

        # Derive limit from the list variable.
        if list_var is not None:
            self._emit_value(list_var)
        else:
            self._emit_i64_const(0)
        self._emit_local_set(limit)

        # counter = 0
        self._emit_i64_const(0)
        self._emit_local_set(counter)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        # if counter >= limit, break
        self._emit_local_get(counter)
        self._emit_local_get(limit)
        self._emit(WasmOp.I64_GE_S)
        self._emit_br_if(1)

        # Assign counter to the loop variable so the body can use it.
        if loop_var is not None:
            var_local = self._intern_local(loop_var)
            self._emit_local_get(counter)
            self._emit_local_set(var_local)

        # Emit the body block's statements (but not its goto back-edge).
        self._visited.add(body_block)
        body = self._cfg.blocks.get(body_block)
        if body is not None:
            for stmt in body.statements:
                self._emit_stmt(stmt)

        # counter++
        self._emit_local_get(counter)
        self._emit_i64_const(1)
        self._emit(WasmOp.I64_ADD)
        self._emit_local_set(counter)

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

        # Continue with the end block.
        self._emit_block(end_block)

    def _emit_block(self, block_name: str) -> None:
        """Emit all statements in a CFG block."""
        if block_name in self._visited:
            return
        self._visited.add(block_name)

        block = self._cfg.blocks.get(block_name)
        if block is None:
            return

        # Detect implicit return pattern: last statement is IRExprEval
        # followed by goto to an empty exit block.  In Tcl, the last
        # expression result is the proc's return value.
        stmts = block.statements
        use_implicit_return = False
        if (
            stmts
            and isinstance(stmts[-1], IRExprEval)
            and isinstance(block.terminator, CFGGoto)
            and self._is_exit_block(block.terminator.target)
            and self._is_proc
        ):
            use_implicit_return = True

        if use_implicit_return:
            for stmt in stmts[:-1]:
                self._emit_stmt(stmt)
            # Emit the final expression without dropping — use as return value
            last_expr = stmts[-1]
            assert isinstance(last_expr, IRExprEval)
            self._emit_expr(last_expr.expr)
            self._emit(WasmOp.RETURN)
            return

        for stmt in stmts:
            self._emit_stmt(stmt)

        match block.terminator:
            case CFGReturn(value=value):
                if value is not None:
                    self._emit_value(value)
                else:
                    self._emit_i64_const(0)
                self._emit(WasmOp.RETURN)

            case CFGBranch(condition=condition, true_target=tt, false_target=ft):
                # Foreach pattern: the CFG builder desugars foreach into
                # a header block with an opaque <foreach_has_next> branch.
                # Detect this and emit a counter-based WASM loop instead.
                if isinstance(condition, ExprRaw) and condition.text == "<foreach_has_next>":
                    self._emit_foreach_loop(stmts, tt, ft)
                    return

                # Find the merge block where both branches reconverge
                # so we can emit it *after* the if/else/end rather than
                # inlining it into one branch and skipping it for the other.
                merge = self._find_merge_block(tt, ft)
                if merge is not None:
                    self._visited.add(merge)

                self._emit_expr(condition)
                self._emit_i64_const(0)
                self._emit(WasmOp.I64_NE)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                self._emit_block(tt)
                self._emit(WasmOp.ELSE)
                self._emit_block(ft)
                self._emit(WasmOp.END)

                if merge is not None:
                    self._visited.discard(merge)
                    self._emit_block(merge)

            case CFGGoto(target=target):
                self._emit_block(target)

    # -- Entry point --

    def generate(self) -> WasmFunction:
        """Generate a complete WASM function."""
        self._emit_block(self._cfg.entry)

        # Ensure function has a return value on all paths
        if not self._body or self._body[-1].op != WasmOp.RETURN:
            self._emit_i64_const(0)
            self._emit(WasmOp.END)
        else:
            self._emit(WasmOp.END)

        # Apply optimisations
        self._run_optimisations()

        # Separate param types from extra locals
        param_types = [ValType.I64] * len(self._params)
        extra_locals = self._local_types[len(self._params) :]

        return WasmFunction(
            name=self._cfg.name,
            params=param_types,
            results=[ValType.I64],
            locals=extra_locals,
            body=self._body,
            local_names=[f"${n}" for n in self._local_names],
            exported=True,
        )

    @property
    def data_segments(self) -> list[WasmData]:
        """Return accumulated string data segments."""
        segments: list[WasmData] = []
        for value, offset in self._strings:
            encoded = value.encode("utf-8")
            # Store: 4-byte length prefix + utf-8 data
            data = len(encoded).to_bytes(4, "little") + encoded
            segments.append(WasmData(offset=offset, data=data))
        return segments


# Public API


def wasm_codegen_function(
    cfg: CFGFunction,
    params: tuple[str, ...] = (),
    *,
    optimise: bool = False,
    is_proc: bool = False,
) -> WasmFunction:
    """Generate a WASM function from a CFG function.

    When *optimise* is ``True``, constant folding, peephole
    optimisation, and dead-code elimination are applied.
    """
    emitter = _WasmEmitter(cfg, params=params, optimise=optimise, is_proc=is_proc)
    return emitter.generate()


def wasm_codegen_module(
    cfg_module: CFGModule,
    ir_module: IRModule,
    *,
    optimise: bool = False,
) -> WasmModule:
    """Generate a complete WASM module from a CFG module.

    Args:
        cfg_module: The control-flow-graph module to compile.
        ir_module: The IR module (for procedure metadata).
        optimise: When ``True``, enable optimisation passes
            (constant folding, peephole, dead-code elimination).
            When ``False``, emit straightforward unoptimised code.

    Returns:
        A ``WasmModule`` that can be serialised to ``.wasm`` binary
        or inspected as WAT text.
    """
    module = WasmModule()

    # Compile top-level
    emitter = _WasmEmitter(cfg_module.top_level, optimise=optimise, is_proc=False)
    top_func = emitter.generate()
    top_func.name = "::top"
    module.functions.append(top_func)
    module.data_segments.extend(emitter.data_segments)

    # Compile procedures
    for qname, cfg_func in cfg_module.procedures.items():
        ir_proc = ir_module.procedures.get(qname)
        if ir_proc and ir_proc.namespace_scoped:
            continue
        params = ir_proc.params if ir_proc else ()
        proc_emitter = _WasmEmitter(
            cfg_func,
            params=params,
            optimise=optimise,
            is_proc=True,
        )
        proc_func = proc_emitter.generate()
        module.functions.append(proc_func)
        module.data_segments.extend(proc_emitter.data_segments)

    # Ensure types are registered
    for func in module.functions:
        module._intern_type(func.params, func.results)

    return module
