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
from collections.abc import Callable
from dataclasses import dataclass, field
from enum import IntEnum

from ...analysis.semantic_model import Range
from ...parsing.substitution import backslash_subst as _tcl_backslash_subst
from ...parsing.tokens import TokenType
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
    ExprCommand,
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
    CommandTokens,
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRBlock,
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
from ..var_escape import ProcEscapeSummary, analyse_var_escape

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


def _tcl_token_value(token: str) -> str:
    """Return the runtime VALUE of a Tcl source token (word).

    Mirrors the C Tcl parser's substitution rules:
    - Braced words ``{content}``: strip outer braces; content is literal
      (no substitution of any kind).
    - All other words: apply backslash substitution (``\\n`` → newline,
      ``\\\\`` → ``\\``, etc.).  Variable and command substitutions are
      left as-is because they must be resolved at runtime.

    Use this before ``_tcl_list_quote`` whenever *token* comes directly
    from IR source tokens rather than from a previously-evaluated value.

    Inverse: ``_tcl_list_quote(_tcl_token_value(token))`` gives the
    canonical list-element representation of the word's value.
    """
    if token.startswith("{") and token.endswith("}") and len(token) >= 2:
        return token[1:-1]
    if "\\" in token:
        return _tcl_backslash_subst(token)
    return token


_LIST_ELEM_WHITESPACE = frozenset(" \t\n\x0b\x0c\r")


# Flag bits for :func:`_tcl_scan_element` / :func:`_tcl_convert_element`.
# Names and semantics mirror ``enum ConvertFlags`` in ``tclUtil.c`` so the
# port is 1:1 with the Zig runtime (``runtime/zig/tcl_obj.zig``) and with
# reference Tcl 9.0's ``TclScanElement`` / ``TclConvertElement``.
_FLAG_CONVERT_NONE = 0
_FLAG_DONT_USE_BRACES = 1
_FLAG_CONVERT_BRACE = 2
_FLAG_CONVERT_ESCAPE = 4
_FLAG_DONT_QUOTE_HASH = 8
_FLAG_CONVERT_MASK = _FLAG_CONVERT_BRACE | _FLAG_CONVERT_ESCAPE


def _tcl_scan_element(s: str, flag_in: int = 0) -> int:
    """Port of ``TclScanElement`` (tclUtil.c, Tcl 9.0, COMPAT=1).

    Chooses the ``CONVERT_*`` mode appropriate for *s* so that
    :func:`_tcl_convert_element` produces a valid list-element
    representation.  Pass ``_FLAG_DONT_QUOTE_HASH`` to skip the
    leading-``#`` quoting rule — matches ``UpdateStringOfList``'s
    ``i ? TCL_DONT_QUOTE_HASH : 0`` convention for non-first elements.
    """
    if s == "":
        return (flag_in & _FLAG_DONT_QUOTE_HASH) | _FLAG_CONVERT_BRACE
    forbid_none = False
    require_escape = False
    prefer_escape = False
    prefer_brace = False
    nesting = 0
    if s[0] == "{" or s[0] == '"':
        forbid_none = True
        prefer_brace = True
    if s[0] == "#" and (flag_in & _FLAG_DONT_QUOTE_HASH) == 0:
        prefer_brace = True
    i = 0
    n = len(s)
    while i < n:
        ch = s[i]
        if ch == "{":
            nesting += 1
        elif ch == "}":
            nesting -= 1
            if nesting < 0:
                require_escape = True
        elif ch == "]" or ch == '"':
            forbid_none = True
            prefer_escape = True
        elif ch == "[" or ch == "$" or ch == ";":
            forbid_none = True
            prefer_brace = True
        elif ch == "\\":
            if i + 1 >= n:
                require_escape = True
            elif s[i + 1] == "\n":
                require_escape = True
                i += 1
            elif s[i + 1] == "{" or s[i + 1] == "}" or s[i + 1] == "\\":
                i += 1
            forbid_none = True
            prefer_brace = True
        elif ch in _LIST_ELEM_WHITESPACE:
            forbid_none = True
            prefer_brace = True
        i += 1
    if nesting > 0:
        require_escape = True
    out_hash = flag_in & _FLAG_DONT_QUOTE_HASH
    if require_escape:
        return out_hash | _FLAG_CONVERT_ESCAPE
    if forbid_none:
        if prefer_escape and not prefer_brace:
            return out_hash | _FLAG_CONVERT_MASK
        return out_hash | _FLAG_CONVERT_BRACE
    return out_hash | _FLAG_CONVERT_NONE


_ESCAPE_ONE_CHAR = frozenset('][$; \\"')
_ESCAPE_CTL_MAP = {
    "\n": "\\n",
    "\t": "\\t",
    "\r": "\\r",
    "\x0b": "\\v",
    "\x0c": "\\f",
}


def _tcl_convert_element(s: str, flags: int) -> str:
    """Port of ``TclConvertElement`` (tclUtil.c, Tcl 9.0, COMPAT=1)."""
    conversion = flags & _FLAG_CONVERT_MASK
    if (flags & _FLAG_DONT_USE_BRACES) and (conversion & _FLAG_CONVERT_BRACE):
        conversion = _FLAG_CONVERT_ESCAPE
    if s == "":
        return "{}"
    prefix = ""
    if s[0] == "#" and (flags & _FLAG_DONT_QUOTE_HASH) == 0:
        if conversion == _FLAG_CONVERT_ESCAPE:
            prefix = "\\#"
            s = s[1:]
        else:
            conversion = _FLAG_CONVERT_BRACE
    if conversion == _FLAG_CONVERT_NONE:
        return prefix + s
    if conversion == _FLAG_CONVERT_BRACE:
        return prefix + "{" + s + "}"
    # CONVERT_ESCAPE or CONVERT_MASK.
    out: list[str] = [prefix] if prefix else []
    for ch in s:
        if ch in _ESCAPE_ONE_CHAR:
            out.append("\\")
            out.append(ch)
        elif ch == "{" or ch == "}":
            if conversion == _FLAG_CONVERT_ESCAPE:
                out.append("\\")
            out.append(ch)
        elif ch in _ESCAPE_CTL_MAP:
            out.append(_ESCAPE_CTL_MAP[ch])
        else:
            out.append(ch)
    return "".join(out)


def _tcl_list_quote(s: str, first: bool = True) -> str:
    """Return *s* encoded as a single Tcl list element.

    ``first=True`` — the element is position 0 of the output list, so a
    leading ``#`` is quoted (``{#}`` or ``\\#``) to prevent later
    ``eval`` re-parses from treating it as a comment.
    ``first=False`` — adds ``TCL_DONT_QUOTE_HASH``, matching the
    second-and-later-element rule in ``UpdateStringOfList``.
    """
    flag_in = 0 if first else _FLAG_DONT_QUOTE_HASH
    flags = _tcl_scan_element(s, flag_in)
    return _tcl_convert_element(s, flags)


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
class DiagSite:
    """A single diagnostic call-site in the compiled module.

    Emitted by the codegen as a sidecar entry for every command invocation
    whose trap path surfaces the command to the user — the eval fallback,
    unsupported-command trap, and unknown-command dispatch.  At runtime
    the last-registered site's ID is prefixed onto the stderr trap line
    so a companion resolver (tcl-trap-resolve skill, or an in-process
    harness loader) can map ``site=1234`` back to ``(file, line, col,
    command)`` without embedding the strings in the .wasm.
    """

    id: int  # site_id emitted as the argument to tcl_diag_set
    file: str  # source filename (may be "<unknown>" when not threaded)
    line: int  # 1-based line number of the originating command
    col: int  # 1-based column
    end_line: int
    end_col: int
    command: str  # the Tcl command being dispatched
    args: tuple[str, ...] = ()  # raw argument strings (best-effort)
    kind: str = "fallback"  # "fallback" | "unsupported" | "unknown"
    proc: str | None = None  # fully-qualified proc name this site lives in


@dataclass(slots=True)
class DiagMap:
    """Sidecar source-location map for a compiled WASM module.

    Produced alongside a :class:`WasmModule` by
    :func:`wasm_codegen_module` when a filename is supplied.  The map
    resolves the opaque ``site_id`` values that the runtime writes on a
    trap back to concrete source locations.  :func:`to_json_dict` emits
    the wire format that the runner writes to ``<module>.wasm.map.json``.

    ``procs`` separately resolves WASM function indices to their proc
    qualified names so a wasm backtrace like ``<wasm function 73>`` can
    be rendered as ``::tcltest::Configure``.
    """

    sites: list[DiagSite] = field(default_factory=list)
    procs: list[tuple[int, str]] = field(default_factory=list)
    filename: str = "<unknown>"

    def add_site(self, site: DiagSite) -> int:
        """Register a site and return its id (1-based; 0 means 'unset')."""
        self.sites.append(site)
        return site.id

    def to_json_dict(self) -> dict:
        return {
            "filename": self.filename,
            "sites": [
                {
                    "id": s.id,
                    "file": s.file,
                    "line": s.line,
                    "col": s.col,
                    "end_line": s.end_line,
                    "end_col": s.end_col,
                    "command": s.command,
                    "args": list(s.args),
                    "kind": s.kind,
                    "proc": s.proc,
                }
                for s in self.sites
            ],
            "procs": [{"func_idx": f, "qname": q} for f, q in self.procs],
        }


@dataclass(slots=True)
class WasmModule:
    """Complete WASM module."""

    functions: list[WasmFunction] = field(default_factory=list)
    imports: list[WasmImport] = field(default_factory=list)
    data_segments: list[WasmData] = field(default_factory=list)
    memory_pages: int = 1
    import_memory: bool = True  # import memory from runtime instead of defining own
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
            # Import memory from runtime so data segments share the same
            # linear memory as TclObj allocations.
            if self.import_memory:
                mem_imp = _encode_string("tcl")
                mem_imp += _encode_string("memory")
                mem_imp += b"\x02"  # memory import
                mem_imp += b"\x00"  # limits: no maximum
                mem_imp += _leb128_unsigned(self.memory_pages)
                import_entries.append(mem_imp)
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

        # Memory section (only if not importing)
        if not self.import_memory:
            mem_data = _leb128_unsigned(1)  # 1 memory
            mem_data += b"\x00"  # no maximum
            mem_data += _leb128_unsigned(self.memory_pages)
            sections.append(self._make_section(SectionId.MEMORY, mem_data))

        # Export section
        export_entries: list[bytes] = []
        # Export memory (only if we define it, not if imported)
        if not self.import_memory:
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
        if self.import_memory:
            lines.append(f'  (import "tcl" "memory" (memory {self.memory_pages}))')
        else:
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


# Runtime function signatures imported from the Tcl runtime.
# Each entry maps an import key to (module, export_name, param_types, result_types).
#
# Values are represented as i32 TclObj pointers.  The TclObj lifecycle
# imports (obj_new_int, obj_new_string, obj_get_int) bridge between raw
# WASM integers (i64) used in expr arithmetic and the i32 object pointers
# passed to/from runtime functions and stored in locals.
_RUNTIME_IMPORTS: dict[str, tuple[str, str, list[ValType], list[ValType]]] = {
    # TclObj lifecycle — always imported
    "tcl_obj_new_int": ("tcl", "obj_new_int", [ValType.I64], [ValType.I32]),
    "tcl_obj_new_string": ("tcl", "obj_new_string", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_obj_get_int": ("tcl", "obj_get_int", [ValType.I32], [ValType.I64]),
    # Command runtime — all parameters/results are i32 TclObj pointers
    "tcl_puts": ("tcl", "tcl_cmd_puts", [ValType.I32], [ValType.I32]),
    "tcl_append": ("tcl", "tcl_cmd_append", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_length": ("tcl", "tcl_cmd_list_length", [ValType.I32], [ValType.I32]),
    "tcl_lappend": ("tcl", "tcl_cmd_lappend", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_length": ("tcl", "string_length", [ValType.I32], [ValType.I32]),
    "tcl_string_index": ("tcl", "string_index", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_range": (
        "tcl",
        "string_range",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_compare": (
        "tcl",
        "string_compare",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_expr_order_cmp": (
        "tcl",
        "tcl_expr_order_cmp",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_map": ("tcl", "string_map", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_match": ("tcl", "string_match", [ValType.I32, ValType.I32], [ValType.I32]),
    # ``string trim{,left,right} value ?chars?`` — a null (i32 0) chars arg
    # means "default whitespace"; otherwise the TclObj's string value is the
    # set of bytes to trim.
    "tcl_string_trim": ("tcl", "string_trim", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_trimleft": (
        "tcl",
        "string_trimleft",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_trimright": (
        "tcl",
        "string_trimright",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_equal": ("tcl", "string_equal", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_first": ("tcl", "string_first", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_last": ("tcl", "string_last", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_repeat": ("tcl", "string_repeat", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_reverse": ("tcl", "string_reverse", [ValType.I32], [ValType.I32]),
    "tcl_string_toupper": ("tcl", "string_toupper", [ValType.I32], [ValType.I32]),
    "tcl_string_tolower": ("tcl", "string_tolower", [ValType.I32], [ValType.I32]),
    "tcl_string_replace": (
        "tcl",
        "string_replace",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_is_integer": ("tcl", "string_is_integer", [ValType.I32], [ValType.I32]),
    "tcl_string_is_alpha": ("tcl", "string_is_alpha", [ValType.I32], [ValType.I32]),
    "tcl_string_is_digit": ("tcl", "string_is_digit", [ValType.I32], [ValType.I32]),
    "tcl_string_is_space": ("tcl", "string_is_space", [ValType.I32], [ValType.I32]),
    "tcl_list_create": ("tcl", "tcl_list", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_concat": ("tcl", "tcl_cmd_concat", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_index": ("tcl", "tcl_cmd_list_index", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_range": (
        "tcl",
        "tcl_cmd_list_range",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_tail": ("tcl", "list_tail", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_sort": ("tcl", "tcl_cmd_list_sort", [ValType.I32], [ValType.I32]),
    "tcl_list_search": ("tcl", "tcl_cmd_list_search", [ValType.I32, ValType.I32], [ValType.I32]),
    # Dict commands
    "tcl_dict_create": ("tcl", "dict_create", [], [ValType.I32]),
    "tcl_dict_get": ("tcl", "dict_get", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_dict_set": (
        "tcl",
        "dict_set",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_dict_exists": ("tcl", "dict_exists", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_dict_keys": ("tcl", "dict_keys", [ValType.I32], [ValType.I32]),
    "tcl_dict_values": ("tcl", "dict_values", [ValType.I32], [ValType.I32]),
    "tcl_dict_size": ("tcl", "dict_size", [ValType.I32], [ValType.I32]),
    "tcl_error": ("tcl", "tcl_cmd_error", [ValType.I32], []),
    "tcl_format": (
        "tcl",
        "tcl_cmd_format",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_regexp": ("tcl", "tcl_cmd_regexp", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_open": ("tcl", "tcl_cmd_open", [ValType.I32], [ValType.I32]),
    "tcl_close": ("tcl", "tcl_cmd_close", [ValType.I32], [ValType.I32]),
    "tcl_read": ("tcl", "tcl_cmd_read", [ValType.I32], [ValType.I32]),
    "tcl_gets": ("tcl", "tcl_cmd_gets", [ValType.I32], [ValType.I32]),
    # Global variable table
    "tcl_global_set": ("tcl", "global_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_global_get": ("tcl", "global_get", [ValType.I32], [ValType.I32]),
    "tcl_global_exists": ("tcl", "global_exists", [ValType.I32], [ValType.I32]),
    # Catch / error handling
    "tcl_catch_enter": ("tcl", "catch_enter", [], []),
    "tcl_catch_leave": ("tcl", "catch_leave", [], [ValType.I32]),
    "tcl_catch_result": ("tcl", "catch_result", [], [ValType.I32]),
    "tcl_catch_has_error": ("tcl", "catch_has_error", [], [ValType.I32]),
    "tcl_catch_set_ok_result": ("tcl", "catch_set_ok_result", [ValType.I32], []),
    # Interpreter fallback
    "tcl_eval": ("tcl", "tcl_eval", [ValType.I32], [ValType.I32]),
    # Namespace context for eval-fallback calls — compiled procs
    # set the current namespace before ``tcl_eval`` so dynamic
    # ``proc $name`` / ``variable $name`` inside the fallback
    # qualify into the enclosing namespace instead of the global
    # scope.  ``ns_set`` returns an opaque i64 save token;
    # ``ns_restore`` unwinds it.
    "tcl_ns_set": (
        "tcl",
        "ns_set",
        [ValType.I32, ValType.I32],
        [ValType.I64],
    ),
    "tcl_ns_restore": ("tcl", "ns_restore", [ValType.I64], []),
    # Frame stack (local variable scoping)
    "tcl_frame_push": ("tcl", "frame_push", [], [ValType.I32]),
    "tcl_frame_pop": ("tcl", "frame_pop", [], []),
    "tcl_var_resolve": ("tcl", "var_resolve", [ValType.I32], [ValType.I32]),
    "tcl_var_set": ("tcl", "var_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_var_exists": ("tcl", "var_exists", [ValType.I32], [ValType.I32]),
    "tcl_local_set": ("tcl", "local_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_local_get": ("tcl", "local_get", [ValType.I32], [ValType.I32]),
    # Proc registry
    "tcl_proc_register": (
        "tcl",
        "proc_register",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_proc_register_compiled": (
        "tcl",
        "proc_register_compiled",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Info command
    "tcl_info_exists": ("tcl", "info_exists", [ValType.I32], [ValType.I32]),
    "tcl_info_dispatch": ("tcl", "info_dispatch", [ValType.I32, ValType.I32], [ValType.I32]),
    # Clock command — WASI clock_time_get wrappers, integer results.
    "tcl_clock_seconds": ("tcl", "clock_seconds", [], [ValType.I32]),
    "tcl_clock_clicks": ("tcl", "clock_clicks", [], [ValType.I32]),
    "tcl_clock_milliseconds": ("tcl", "clock_milliseconds", [], [ValType.I32]),
    # Frame-depth helpers for uplevel — temporarily shift frame_depth
    # so a called ``tcl_eval`` runs at a caller's scope.
    "tcl_frame_depth_stash": (
        "tcl",
        "frame_depth_stash",
        [ValType.I32],
        [ValType.I32],
    ),
    "tcl_frame_depth_restore": ("tcl", "frame_depth_restore", [ValType.I32], []),
    # Arrays — dedicated per-array hash tables.
    "tcl_array_set": (
        "tcl",
        "array_set",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_get": ("tcl", "array_get", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_array_exists": ("tcl", "array_exists", [ValType.I32], [ValType.I32]),
    "tcl_array_element_exists": (
        "tcl",
        "array_element_exists",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_size": ("tcl", "array_size", [ValType.I32], [ValType.I32]),
    "tcl_array_unset": ("tcl", "array_unset", [ValType.I32], [ValType.I32]),
    "tcl_array_unset_element": (
        "tcl",
        "array_unset_element",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_names": (
        "tcl",
        "array_names",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # String split/join
    "tcl_split": ("tcl", "tcl_cmd_split", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_join": ("tcl", "tcl_cmd_join", [ValType.I32, ValType.I32], [ValType.I32]),
    # Diagnostic — record the current source site so trap paths can
    # prefix stderr with ``site=<id>`` for sidecar-map resolution.
    "tcl_diag_set": ("tcl", "diag_set", [ValType.I32], []),
    # I/O + channel stubs (tcl_io_stubs.zig) — trap with "unsupported
    # command: <name>".  Each stub takes i32 args and returns i32.
    "tcl_eof": ("tcl", "tcl_cmd_eof", [ValType.I32], [ValType.I32]),
    "tcl_flush": ("tcl", "tcl_cmd_flush", [ValType.I32], [ValType.I32]),
    "tcl_fblocked": ("tcl", "tcl_cmd_fblocked", [ValType.I32], [ValType.I32]),
    "tcl_fconfigure": ("tcl", "tcl_cmd_fconfigure", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_tell": ("tcl", "tcl_cmd_tell", [ValType.I32], [ValType.I32]),
    "tcl_seek": ("tcl", "tcl_cmd_seek", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_chan": ("tcl", "tcl_cmd_chan", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_fcopy": ("tcl", "tcl_cmd_fcopy", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_fileevent": ("tcl", "tcl_cmd_fileevent", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_socket": ("tcl", "tcl_cmd_socket", [ValType.I32, ValType.I32], [ValType.I32]),
    # Filesystem / process stubs (tcl_fs_stubs.zig).
    "tcl_file": (
        "tcl",
        "tcl_cmd_file",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_glob": ("tcl", "tcl_cmd_glob", [ValType.I32], [ValType.I32]),
    "tcl_pwd": ("tcl", "tcl_cmd_pwd", [], [ValType.I32]),
    "tcl_cd": ("tcl", "tcl_cmd_cd", [ValType.I32], [ValType.I32]),
    "tcl_exec": ("tcl", "tcl_cmd_exec", [ValType.I32], [ValType.I32]),
    "tcl_source": ("tcl", "tcl_cmd_source", [ValType.I32], [ValType.I32]),
    "tcl_load": ("tcl", "tcl_cmd_load", [ValType.I32], [ValType.I32]),
    "tcl_unload": ("tcl", "tcl_cmd_unload", [ValType.I32], [ValType.I32]),
    # Format / regex / encoding stubs (tcl_fmt_stubs.zig).
    "tcl_scan": ("tcl", "tcl_cmd_scan", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_binary": ("tcl", "tcl_cmd_binary", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_regsub": ("tcl", "tcl_cmd_regsub", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_encoding": (
        "tcl",
        "tcl_cmd_encoding",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Time / event stubs (tcl_time_stubs.zig).  clock_format /
    # clock_scan / clock_add are the non-implemented clock
    # subcommands; the arithmetic ones (seconds / clicks /
    # milliseconds) are real runtime fns in tcl_clock.zig.
    "tcl_clock_format": ("tcl", "clock_format", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_clock_scan": ("tcl", "clock_scan", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_clock_add": ("tcl", "clock_add", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_after": ("tcl", "tcl_cmd_after", [ValType.I32], [ValType.I32]),
    "tcl_vwait": ("tcl", "tcl_cmd_vwait", [ValType.I32], [ValType.I32]),
    "tcl_update": ("tcl", "tcl_cmd_update", [], [ValType.I32]),
    "tcl_coroutine": ("tcl", "tcl_cmd_coroutine", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_yield": ("tcl", "tcl_cmd_yield", [ValType.I32], [ValType.I32]),
    "tcl_yieldto": ("tcl", "tcl_cmd_yieldto", [ValType.I32], [ValType.I32]),
    # Environment / metadata stubs (tcl_env_stubs.zig).  ``namespace``
    # and ``namespace eval`` are handled separately by the compiler;
    # this stub catches the other namespace subcommands falling
    # through.
    "tcl_namespace": ("tcl", "namespace", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_package": ("tcl", "tcl_cmd_package_cmd", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_trace": ("tcl", "tcl_cmd_trace_cmd", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_interp": ("tcl", "tcl_cmd_interp_cmd", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_apply": ("tcl", "tcl_cmd_apply", [ValType.I32, ValType.I32], [ValType.I32]),
}

# Import keys for the TclObj lifecycle functions — always registered
_OBJ_LIFECYCLE_IMPORTS = frozenset(
    {
        "tcl_obj_new_int",
        "tcl_obj_new_string",
        "tcl_obj_get_int",
    }
)

# Commands that map to runtime imports.  The value is
# (import_key, arg_count_or_None) where ``None`` means variadic.
#
# Stub-mapped commands (file, glob, encoding, trace, etc.) route
# through this dispatch table so the compiler calls the stub
# directly — producing a clean ``unsupported command: <name>`` trap
# — instead of generating a ``tcl_eval`` fallback that invokes the
# interpreter which then fails to find the command.
_CMD_RUNTIME: dict[str, tuple[str, int | None]] = {
    # Implemented runtime functions
    "puts": ("tcl_puts", 1),
    "append": ("tcl_append", 2),
    "llength": ("tcl_list_length", 1),
    "lappend": ("tcl_lappend", 2),
    "lindex": ("tcl_list_index", 2),
    "lrange": ("tcl_list_range", 3),
    "lsort": ("tcl_list_sort", 1),
    "lsearch": ("tcl_list_search", 2),
    "concat": ("tcl_concat", 2),
    "error": ("tcl_error", 1),
    "split": ("tcl_split", 2),
    "join": ("tcl_join", 2),
    # I/O stubs — open/close/read/gets/eof/flush/fblocked/fconfigure/
    # tell/seek/chan/fcopy/fileevent/socket all trap with
    # "unsupported command: <name>".
    "open": ("tcl_open", 1),
    "close": ("tcl_close", 1),
    "read": ("tcl_read", 1),
    "gets": ("tcl_gets", 1),
    "eof": ("tcl_eof", 1),
    "flush": ("tcl_flush", 1),
    "fblocked": ("tcl_fblocked", 1),
    "fconfigure": ("tcl_fconfigure", 2),
    "tell": ("tcl_tell", 1),
    "seek": ("tcl_seek", 2),
    "chan": ("tcl_chan", 2),
    "fcopy": ("tcl_fcopy", 2),
    "fileevent": ("tcl_fileevent", 2),
    "socket": ("tcl_socket", 2),
    # Filesystem / process stubs.
    "file": ("tcl_file", 3),
    "glob": ("tcl_glob", 1),
    "pwd": ("tcl_pwd", 0),
    "cd": ("tcl_cd", 1),
    "exec": ("tcl_exec", 1),
    "source": ("tcl_source", 1),
    "load": ("tcl_load", 1),
    "unload": ("tcl_unload", 1),
    # Format / regex / encoding stubs.
    "format": ("tcl_format", 4),
    "scan": ("tcl_scan", 2),
    "binary": ("tcl_binary", 2),
    "regexp": ("tcl_regexp", 2),
    "regsub": ("tcl_regsub", 2),
    "encoding": ("tcl_encoding", 3),
    # Event / coroutine stubs.  The arithmetic clock commands
    # (``clock seconds`` / ``clock clicks`` / ``clock milliseconds``)
    # are real runtime fns and route through a separate dispatcher
    # in ``_emit_cmd_clock``; only the formatting variants surface
    # here.
    "after": ("tcl_after", 1),
    "vwait": ("tcl_vwait", 1),
    "update": ("tcl_update", 0),
    "coroutine": ("tcl_coroutine", 2),
    "yield": ("tcl_yield", 1),
    "yieldto": ("tcl_yieldto", 1),
    # Environment / metadata stubs.  ``namespace eval`` is compiled
    # (it is a control-flow construct); the ``namespace`` entry here
    # only catches the other subcommands (current, qualifiers, which,
    # tail, code, delete, import, export, exists, parent, children,
    # inscope, origin, forget, path, ensemble) that route through
    # this dispatch.  Likewise ``interp`` covers the commands the
    # previous ``_UNSUPPORTED_COMMANDS`` entry used to hard-trap;
    # the stub trap is equivalent but gives a sourced site.
    "package": ("tcl_package", 2),
    "trace": ("tcl_trace", 2),
    "interp": ("tcl_interp", 2),
    "apply": ("tcl_apply", 2),
}

# String sub-command → import key
_STRING_SUBCMD_IMPORT: dict[str, str] = {
    "length": "tcl_string_length",
    "index": "tcl_string_index",
    "range": "tcl_string_range",
    "compare": "tcl_string_compare",
    "match": "tcl_string_match",
    "map": "tcl_string_map",
    "trim": "tcl_string_trim",
    "trimleft": "tcl_string_trimleft",
    "trimright": "tcl_string_trimright",
    "equal": "tcl_string_equal",
    "first": "tcl_string_first",
    "last": "tcl_string_last",
    "repeat": "tcl_string_repeat",
    "reverse": "tcl_string_reverse",
    "toupper": "tcl_string_toupper",
    "tolower": "tcl_string_tolower",
    "replace": "tcl_string_replace",
}

# ``string is <class> <value>`` sub-sub-command → import key
_STRING_IS_IMPORT: dict[str, str] = {
    "integer": "tcl_string_is_integer",
    "alpha": "tcl_string_is_alpha",
    "digit": "tcl_string_is_digit",
    "space": "tcl_string_is_space",
}

# Dict sub-command → (import key, additional_arg_count after dict_var)
_DICT_SUBCMD_IMPORT: dict[str, str] = {
    "get": "tcl_dict_get",
    "set": "tcl_dict_set",
    "exists": "tcl_dict_exists",
    "keys": "tcl_dict_keys",
    "values": "tcl_dict_values",
    "size": "tcl_dict_size",
    "create": "tcl_dict_create",
}

# ``clock <subcmd>`` → import key.  Only subcommands that map to a
# WASI-backed runtime hook are listed; ``format``/``scan`` fall through
# to the interpreter which itself traps in the sandbox.
_CLOCK_SUBCMD_IMPORT: dict[str, str] = {
    "seconds": "tcl_clock_seconds",
    "clicks": "tcl_clock_clicks",
    "milliseconds": "tcl_clock_milliseconds",
}

# Commands that are scope declarations, compile-time-only, or CFG
# placeholders — NOPs in WASM.
_SCOPE_NOP_COMMANDS = frozenset(
    {
        "namespace",
        "proc",
        "package",
        "rename",
        # CFG placeholders — the actual logic is emitted by the loop/foreach
        # emitters; these IRCall nodes are just iteration-setup markers.
        "foreach",
    }
)

# Commands that require capabilities unavailable in the WASM sandbox.
# The codegen emits a call to the runtime ``error`` function with a
# descriptive message so the module traps with a clear diagnostic
# rather than silently emitting a NOP.
# Commands that have no meaningful implementation in WASM and
# aren't covered by the ``_CMD_RUNTIME`` stub table.  Kept as an
# explicit trap path so (a) we reject them at compile time with a
# clear message and (b) new users see they're intentionally
# unimplemented rather than silently accepted.  Most commands that
# used to live here (``exec``, ``socket``, ``interp``, ``after``,
# etc.) have moved to the stub dispatch table — they still trap,
# but the diag machinery attributes the trap to a source site.
_UNSUPPORTED_COMMANDS: frozenset[str] = frozenset()


def _parse_array_ref(name: str) -> tuple[str, str] | None:
    """Return ``(array_name, key_text)`` if *name* looks like ``arr(key)``.

    Returns ``None`` for scalar names.  The key may be a literal, a
    variable reference (``$i``), an interpolated string
    (``counter-$tag``), or a command substitution — the caller is
    responsible for emitting it as a TclObj value via ``_emit_value``.

    Array names can themselves contain ``::`` (``::ns::arr(key)``) and
    the key may contain ``)``-balanced nested parens — we require
    properly nested parens with the final ``)`` matching the first
    unescaped ``(``.
    """
    # Strip Tcl's ``$=`` scalar-disambiguator prefix that the lowerer
    # sometimes applies to braced ``${x(1)}`` scalars so we don't
    # misidentify those as array refs.
    if not name or "(" not in name or not name.endswith(")"):
        return None
    if name.startswith("$={"):
        return None
    # Find the first '(' that isn't escaped.
    i = 0
    n = len(name)
    while i < n:
        if name[i] == "\\" and i + 1 < n:
            i += 2
            continue
        if name[i] == "(":
            break
        i += 1
    else:
        return None
    arr = name[:i]
    if not arr:
        return None
    # The key runs from i+1 to the matching ')', which is the last char.
    key = name[i + 1 : -1]
    return (arr, key)


def _derive_proc_namespace(qname: str) -> str:
    """Return the namespace containing *qname*.

    ``::counter::init`` → ``::counter``; ``::main`` → ``::``. Used to
    resolve ``variable x`` inside a proc to ``::counter::x``.
    """
    if not qname:
        return "::"
    # Strip leading ``::`` for the search, then re-add.
    bare = qname[2:] if qname.startswith("::") else qname
    idx = bare.rfind("::")
    if idx < 0:
        return "::"
    return "::" + bare[:idx]


def _has_embedded_subst_scan(value: str) -> bool:
    """Detect embedded $var / ${var} / [cmd] substitutions mixed with literal text.

    Returns True when *value* contains both literal characters and a
    substitution — a pure ``$x`` / ``${x}`` / ``[cmd]`` reference returns
    False (handled by the single-token paths in _emit_value).
    """
    # Strip off a pure variable or command substitution to see if there's
    # anything else present.
    if value.startswith("${") and value.endswith("}"):
        # A single ``${name}`` is pure only when the inner text has no
        # further substitutions or braces.  ``${a}${b}`` starts with
        # ``${`` and ends with ``}`` but is NOT pure.
        inner = value[2:-1]
        if "$" not in inner and "[" not in inner and "}" not in inner:
            return False
    if value.startswith("$") and not value.startswith("$["):
        # Check whether the entire string is just $name (possibly with
        # :: namespace separators, e.g. $::ns::var).
        j = 1
        n = len(value)
        while j < n:
            ch = value[j]
            if ch.isalnum() or ch == "_":
                j += 1
            elif ch == ":" and j + 1 < n and value[j + 1] == ":":
                j += 2
            else:
                break
        if j == n:
            return False
    if value.startswith("[") and value.endswith("]"):
        return False
    i = 0
    n = len(value)
    while i < n:
        c = value[i]
        if c == "\\" and i + 1 < n:
            i += 2
            continue
        if c == "$" and i + 1 < n:
            nxt = value[i + 1]
            if nxt == "{" or nxt.isalpha() or nxt == "_" or nxt == ":":
                return True
        if c == "[":
            return True
        i += 1
    return False


def _scan_expr_body_imports(expr_text: str, needed: set[str]) -> None:
    """Parse an expression body and add any runtime imports it needs."""
    from ...parsing.expr_parser import parse_expr
    from ..expr_ast import BinOp, ExprBinary, ExprCall, ExprTernary, ExprUnary

    try:
        node = parse_expr(expr_text)
    except Exception:
        return

    def _walk(n: object) -> None:
        match n:
            case ExprBinary(op=op, left=left, right=right):
                if op in (BinOp.STR_EQ, BinOp.STR_EQUALS, BinOp.STR_NE):
                    needed.add("tcl_string_equal")
                if op in (BinOp.STR_LT, BinOp.STR_GT, BinOp.STR_LE, BinOp.STR_GE):
                    needed.add("tcl_string_compare")
                if op in (BinOp.LT, BinOp.GT, BinOp.LE, BinOp.GE):
                    needed.add("tcl_expr_order_cmp")
                _walk(left)
                _walk(right)
            case ExprUnary(operand=operand):
                _walk(operand)
            case ExprTernary(cond=cond, true_expr=t, false_expr=f):
                _walk(cond)
                _walk(t)
                _walk(f)
            case ExprCall(args=args):
                for arg in args:
                    _walk(arg)

    _walk(node)


def _scan_text_for_cmd_subst(text: str, needed: set[str]) -> None:
    """Scan a text value for [cmd ...] command substitutions and add imports."""
    i = 0
    while i < len(text):
        if text[i] == "[":
            depth = 1
            j = i + 1
            while j < len(text) and depth > 0:
                if text[j] == "[":
                    depth += 1
                elif text[j] == "]":
                    depth -= 1
                j += 1
            cmd_text = text[i + 1 : j - 1].strip()
            parts = cmd_text.split(None, 2)
            if parts:
                cmd = parts[0]
                if cmd in _CMD_RUNTIME:
                    needed.add(_CMD_RUNTIME[cmd][0])
                elif cmd == "list":
                    # ``[list $a $b ...]`` with variable/command args uses
                    # tcl_lappend internally in _emit_list_value to properly
                    # quote each element.  Add it so the import is available.
                    needed.add("tcl_lappend")
                elif cmd == "dict" and len(parts) > 1:
                    subcmd = parts[1]
                    if subcmd in _DICT_SUBCMD_IMPORT:
                        needed.add(_DICT_SUBCMD_IMPORT[subcmd])
                elif cmd == "string" and len(parts) > 1:
                    subcmd = parts[1]
                    if subcmd in _STRING_SUBCMD_IMPORT:
                        needed.add(_STRING_SUBCMD_IMPORT[subcmd])
                    elif subcmd == "is":
                        # "string is <class> <value>" — extract class name
                        rest = parts[2] if len(parts) > 2 else ""
                        class_parts = rest.split(None, 1)
                        if class_parts and class_parts[0] in _STRING_IS_IMPORT:
                            needed.add(_STRING_IS_IMPORT[class_parts[0]])
                elif cmd == "info" and len(parts) > 1:
                    subcmd = parts[1]
                    if subcmd == "exists":
                        needed.add("tcl_global_exists")
                        needed.add("tcl_obj_new_int")
                        # FRAME-tagged vars (per var-escape analysis)
                        # route through this runtime helper for
                        # literal-name existence checks.
                        needed.add("tcl_info_exists")
                        # ``info exists arr(key)`` inside a substitution —
                        # parts[2] (if present) is the var text.
                        if len(parts) > 2:
                            raw = parts[2]
                            if raw.startswith("${") and raw.endswith("}"):
                                raw = raw[2:-1]
                            elif raw.startswith("$"):
                                raw = raw[1:]
                            if _parse_array_ref(raw) is not None:
                                needed.add("tcl_array_element_exists")
                    if subcmd in ("body", "args", "exists"):
                        needed.add("tcl_info_dispatch")
                    elif subcmd == "level":
                        # Inline ``info level 0`` in a compiled proc
                        # builds a list by ``tcl_append``-ing the
                        # proc name + each param; pull in the helper
                        # so the codegen's concat chain can reach it.
                        needed.add("tcl_append")
                elif cmd == "lassign":
                    needed.add("tcl_list_index")
                    needed.add("tcl_list_tail")
                elif cmd == "clock" and len(parts) > 1:
                    key = _CLOCK_SUBCMD_IMPORT.get(parts[1])
                    if key is not None:
                        needed.add(key)
                elif cmd == "uplevel":
                    needed.add("tcl_eval")
                    needed.add("tcl_frame_depth_stash")
                    needed.add("tcl_frame_depth_restore")
                elif cmd == "array" and len(parts) > 1:
                    sub = parts[1]
                    if sub == "exists":
                        needed.add("tcl_array_exists")
                    elif sub == "size":
                        needed.add("tcl_array_size")
                    elif sub == "unset":
                        needed.add("tcl_array_unset")
                    elif sub == "names":
                        needed.add("tcl_array_names")
                        # ``array names arr $pat*`` interpolates the
                        # pattern; pull in ``tcl_append`` so the
                        # codegen's interpolation emitter can build
                        # the pattern string at runtime instead of
                        # degrading to a raw literal (which would
                        # disable the glob filter).
                        needed.add("tcl_append")
                    elif sub == "set":
                        needed.add("tcl_array_set")
                    elif sub == "get":
                        needed.add("tcl_array_get")
                elif cmd == "namespace" and len(parts) > 1 and parts[1] == "eval":
                    # ``[namespace eval ns arg1 arg2 ...]`` with dynamic
                    # script args: needs tcl_eval to run the assembled
                    # script and tcl_append to join multiple args.
                    needed.add("tcl_eval")
                    needed.add("tcl_append")
                elif cmd == "expr":
                    if len(parts) > 1:
                        # Extract the whole expr body (not using split, which
                        # breaks braced content like {"a" < "b"}).
                        rest = cmd_text[len(cmd) :].lstrip()
                        if rest.startswith("{") and rest.endswith("}"):
                            rest = rest[1:-1]
                        _scan_expr_body_imports(rest, needed)
                elif cmd == "catch":
                    # ``[catch {body} ?var?]`` as a command substitution
                    # compiles via the real catch codegen path (see
                    # ``_emit_command_subst``/``_emit_command_subst_value``);
                    # that path needs the catch runtime imports whose
                    # top-level scan only adds them when ``catch`` is a
                    # statement command.
                    needed.add("tcl_catch_enter")
                    needed.add("tcl_catch_leave")
                    needed.add("tcl_catch_result")
                    needed.add("tcl_catch_has_error")
                    needed.add("tcl_catch_set_ok_result")
                # Recurse into the command text for nested substitutions
                if len(parts) > 1:
                    _scan_text_for_cmd_subst(parts[-1], needed)
            i = j
        else:
            i += 1


def _scan_needed_imports(
    cfg_module: CFGModule,
    ir_module: IRModule,
) -> set[str]:
    """Pre-scan IR to find which runtime imports the compiled module needs."""
    needed: set[str] = set()

    def _scan_script(script: IRScript) -> None:
        for stmt in script.statements:
            _scan_stmt(stmt)

    def _scan_value(value: str, *, is_body_text: bool = False) -> None:
        """Scan a value string for embedded command substitutions.

        ``is_body_text=True`` means the string is source text that the
        lowerer will re-parse (e.g. a proc body) rather than a value
        that the codegen will emit at runtime.  In that case we skip
        the interpolated-string scan — otherwise a proc body containing
        ``$a`` / ``$b`` would spuriously pull in ``tcl_append``, since
        the body is never emitted via the concat-chain path.
        """
        if "[" in value:
            _scan_text_for_cmd_subst(value, needed)
        if is_body_text:
            return
        # Interpolated strings ("hello $x" / "$x[foo]" etc.) need tcl_append
        # at codegen time to concatenate the parts.
        if _has_embedded_subst_scan(value):
            needed.add("tcl_append")
        # ``$arr(key)`` references need tcl_array_get at codegen time.
        _scan_value_for_array_refs(value)
        # Pure ``$::ns::var`` / ``${::ns::var}`` references read through
        # the global table.
        if "::" in value and ("$" in value or value.startswith("::")):
            # Conservative: request global_get whenever the value might
            # resolve to a ``::``-qualified name.  False positives just
            # over-import; they don't break correctness.
            inner = value
            if inner.startswith("${") and inner.endswith("}"):
                inner = inner[2:-1]
            elif inner.startswith("$"):
                inner = inner[1:]
            if inner.startswith("::"):
                needed.add("tcl_global_get")

    def _scan_expr(expr: object) -> None:
        """Walk an expression AST and scan ExprCommand nodes."""
        from ..expr_ast import (
            ExprBinary,
            ExprCall,
            ExprCommand,
            ExprTernary,
            ExprUnary,
            ExprVar,
        )

        match expr:
            case ExprCommand(text=text):
                _scan_text_for_cmd_subst(text, needed)
            case ExprVar(name=name, text=text):
                # ``$::ns::var`` in an expression reads from the global
                # table; ``$arr(key)`` reads via tcl_array_get.
                if name.startswith("::"):
                    needed.add("tcl_global_get")
                if "(" in text and text.endswith(")"):
                    needed.add("tcl_array_get")
            case ExprBinary(op=op, left=left, right=right):
                if op in (BinOp.STR_EQ, BinOp.STR_EQUALS, BinOp.STR_NE):
                    needed.add("tcl_string_equal")
                if op in (BinOp.STR_LT, BinOp.STR_GT, BinOp.STR_LE, BinOp.STR_GE):
                    needed.add("tcl_string_compare")
                if op in (BinOp.LT, BinOp.GT, BinOp.LE, BinOp.GE):
                    needed.add("tcl_expr_order_cmp")
                _scan_expr(left)
                _scan_expr(right)
            case ExprUnary(operand=operand):
                _scan_expr(operand)
            case ExprTernary(cond=cond, true_expr=t, false_expr=f):
                _scan_expr(cond)
                _scan_expr(t)
                _scan_expr(f)
            case ExprCall(args=args):
                for arg in args:
                    _scan_expr(arg)

    def _scan_qualified_name(name: str | None) -> None:
        """``::``-prefixed variable names route reads/writes through the
        global table.  Pull in the matching runtime hooks eagerly so the
        code emission paths can rely on them being registered.

        Also handles array-element writes: ``set arr(key) val`` lowers
        with ``name='arr(key)'``, which needs ``tcl_array_set``.
        """
        if not name:
            return
        if _parse_array_ref(name) is not None:
            needed.add("tcl_array_set")
            needed.add("tcl_array_get")
            return
        if name.startswith("::"):
            needed.add("tcl_global_get")
            needed.add("tcl_global_set")

    def _scan_value_for_array_refs(value: str | None) -> None:
        """Pull in array-get for ``$arr(key)`` references inside a value
        string.  The read path routes through tcl_array_get whenever the
        name parses as an array reference.
        """
        if not value or "(" not in value:
            return
        # Conservative: scan for ``$name(`` or ``${name(`` substrings.
        i = 0
        n = len(value)
        while i < n:
            if value[i] == "$" and i + 1 < n:
                # Advance over bare name or ${...} with parens.
                j = i + 1
                if value[j] == "{":
                    # ``${arr(key)}`` — the lowerer emits this form
                    # when the surface syntax was a quoted-string
                    # ``$arr(key)``.  Check for ``(`` *inside* the
                    # braced content, not just after the closing
                    # brace.  Without this, the import-scan misses
                    # ``tcl_array_get`` and ``_emit_array_element_read``
                    # silently emits ``i32.const 0`` at run time —
                    # making every ``puts "v=$arr(key)"`` read come
                    # back as empty string.
                    inner_start = j + 1
                    k = inner_start
                    while k < n and value[k] != "}":
                        k += 1
                    # scan the inner for ``(``
                    ii = inner_start
                    while ii < k:
                        if value[ii] == "(":
                            needed.add("tcl_array_get")
                            break
                        ii += 1
                    else:
                        pass
                    if "tcl_array_get" in needed:
                        break
                    j = k
                else:
                    while j < n and (
                        value[j].isalnum()
                        or value[j] == "_"
                        or (value[j] == ":" and j + 1 < n and value[j + 1] == ":")
                    ):
                        j += 1
                if j < n and value[j] == "(":
                    needed.add("tcl_array_get")
                    break
                i = j
            else:
                i += 1

    def _scan_array_subcmd(args: tuple[str, ...]) -> None:
        if not args:
            return
        sub = args[0]
        if sub == "exists":
            needed.add("tcl_array_exists")
        elif sub == "size":
            needed.add("tcl_array_size")
        elif sub == "unset":
            needed.add("tcl_array_unset")
        elif sub == "names":
            needed.add("tcl_array_names")
            # ``array names arr ?pattern?`` — the pattern may be an
            # interpolated string (``$option*``); scan it so
            # ``tcl_append`` / ``tcl_array_get`` make it into the
            # module imports and the pattern is emitted literally,
            # not as a null TclObj that disables the filter.
            if len(args) >= 3:
                _scan_value(args[2])
        elif sub == "set":
            needed.add("tcl_array_set")

    def _scan_stmt(stmt: IRStatement) -> None:
        match stmt:
            case IRCall(command=command, args=args):
                if command in _CMD_RUNTIME:
                    needed.add(_CMD_RUNTIME[command][0])
                elif command == "string" and args and args[0] in _STRING_SUBCMD_IMPORT:
                    needed.add(_STRING_SUBCMD_IMPORT[args[0]])
                elif command == "string" and args and args[0] == "is" and len(args) >= 3:
                    is_key = _STRING_IS_IMPORT.get(args[1])
                    if is_key:
                        needed.add(is_key)
                elif command == "list":
                    # ``list $a $b ...`` with variable args uses tcl_lappend
                    # internally in _emit_list_value to quote each element.
                    needed.add("tcl_lappend")
                elif command == "dict" and args and args[0] in _DICT_SUBCMD_IMPORT:
                    needed.add(_DICT_SUBCMD_IMPORT[args[0]])
                elif command == "global":
                    needed.add("tcl_global_get")
                    needed.add("tcl_global_set")
                elif command == "upvar":
                    # upvar #0 resolves to a global alias — reads/writes go
                    # through the global table. The target name is computed
                    # at runtime (may be an interpolated string).
                    needed.add("tcl_global_get")
                    needed.add("tcl_global_set")
                elif command == "variable":
                    # variable inside a proc aliases local → ::ns::local in
                    # the enclosing namespace. Same runtime path as upvar #0.
                    needed.add("tcl_global_get")
                    needed.add("tcl_global_set")
                    # Dynamic ``variable $name ?value?`` (tcltest's
                    # ``Default`` / ``Option``) needs runtime string
                    # concatenation + existence probe to build the
                    # qualified target and honour the Tcl semantics
                    # of initialising only when unset.
                    if args and (args[0].startswith("$") or args[0].startswith("[")):
                        needed.add("tcl_append")
                        needed.add("tcl_global_exists")
                        needed.add("tcl_obj_get_int")
                elif command == "info" and args:
                    # info exists — resolves via tcl_global_exists for
                    # aliased / ``::`` names; plain locals are boxed to
                    # TclObj via tcl_obj_new_int.  Other subcommands go
                    # through tcl_info_dispatch.
                    if args[0] == "exists":
                        needed.add("tcl_global_exists")
                        needed.add("tcl_obj_new_int")
                        # Literal-name existence in FRAME-tagged vars
                        # dispatches through the runtime helper.
                        needed.add("tcl_info_exists")
                        # ``info exists arr(key)`` — probe the array table.
                        if len(args) >= 2:
                            raw = args[1]
                            # ``info exists $dynamicName`` — dispatch
                            # to runtime ``info_exists`` which resolves
                            # the name's value at runtime.
                            if raw.startswith("$") or raw.startswith("["):
                                stripped = raw
                                if raw.startswith("${") and raw.endswith("}"):
                                    stripped = raw[2:-1]
                                elif raw.startswith("$"):
                                    stripped = raw[1:]
                                if _parse_array_ref(stripped) is None:
                                    needed.add("tcl_info_exists")
                            # Strip a ``$`` / ``${...}`` sigil if present.
                            if raw.startswith("${") and raw.endswith("}"):
                                raw = raw[2:-1]
                            elif raw.startswith("$"):
                                raw = raw[1:]
                            if _parse_array_ref(raw) is not None:
                                needed.add("tcl_array_element_exists")
                    if args[0] in ("body", "args", "exists"):
                        needed.add("tcl_info_dispatch")
                    elif args[0] == "level":
                        # Inline ``info level 0`` builds a list via
                        # the ``tcl_append`` concat-chain; see
                        # ``_emit_info_value``.
                        needed.add("tcl_append")
                elif command == "lassign":
                    needed.add("tcl_list_index")
                    needed.add("tcl_list_tail")
                elif command == "clock" and args:
                    key = _CLOCK_SUBCMD_IMPORT.get(args[0])
                    if key is not None:
                        needed.add(key)
                elif command == "array" and args:
                    _scan_array_subcmd(args)
                elif command == "uplevel":
                    needed.add("tcl_eval")
                    needed.add("tcl_frame_depth_stash")
                    needed.add("tcl_frame_depth_restore")
                    # Multi-word bodies concat with spaces; also each body
                    # part is run through _emit_value which may need
                    # tcl_append for interpolated strings.
                    for arg in args:
                        _scan_value(arg)
                    if len(args) > 2 or (
                        len(args) > 1
                        and not args[0].startswith(("#", "-"))
                        and not args[0].lstrip("-").isdigit()
                    ):
                        needed.add("tcl_append")
                elif command == "namespace" and args and args[0] == "eval" and len(args) > 2:
                    # ``namespace eval ns arg1 arg2 ...`` with dynamic
                    # script args: evaluated through WASM, not fallback.
                    needed.add("tcl_eval")
                    needed.add("tcl_append")
                    # Scan each script arg as a runtime value (not body text)
                    # so array-element refs ($arr($key)) pull in tcl_array_get.
                    for a in args[2:]:
                        _scan_value(a)
                elif command == "unset":
                    for a in args:
                        if _parse_array_ref(a) is not None:
                            needed.add("tcl_array_unset_element")
                            break
                elif command == "catch":
                    needed.add("tcl_catch_enter")
                    needed.add("tcl_catch_leave")
                    needed.add("tcl_catch_result")
                    needed.add("tcl_catch_has_error")
                    needed.add("tcl_catch_set_ok_result")
                # Scan all arguments for command substitutions.  For
                # body-taking commands (``proc``, ``namespace eval``,
                # ``catch``, etc.) the args are source text re-parsed
                # by the lowerer, not values emitted at runtime — mark
                # them so the scan doesn't over-import tcl_append.
                is_body = command in _SCOPE_NOP_COMMANDS or command == "catch"
                for arg in args:
                    _scan_value(arg, is_body_text=is_body)
            case IRAssignValue(name=name, value=value):
                _scan_qualified_name(name)
                _scan_value(value)
                _scan_value_for_array_refs(value)
            case IRAssignConst(name=name):
                _scan_qualified_name(name)
            case IRIncr(name=name, amount=amount):
                _scan_qualified_name(name)
                if isinstance(amount, str):
                    _scan_value(amount)
                    _scan_value_for_array_refs(amount)
            case IRExprEval(expr=expr):
                _scan_expr(expr)
            case IRAssignExpr(name=name, expr=expr):
                _scan_qualified_name(name)
                _scan_expr(expr)
            case IRReturn(value=value, expr=expr):
                if value is not None:
                    _scan_value(value)
                if expr is not None:
                    _scan_expr(expr)
            case IRBarrier(command=barrier_cmd, args=barrier_args):
                # Eval-fallback path always needs tcl_eval.
                needed.add("tcl_eval")
                if barrier_cmd == "uplevel":
                    needed.add("tcl_frame_depth_stash")
                    needed.add("tcl_frame_depth_restore")
                    # The body may contain ``$var``/``[cmd]`` references
                    # that have to be resolved BEFORE eval — scan for
                    # interpolation helpers.
                    for arg in barrier_args:
                        _scan_value(arg)
                elif (
                    barrier_cmd == "namespace"
                    and barrier_args
                    and barrier_args[0] == "eval"
                    and len(barrier_args) > 2
                ):
                    # ``namespace eval ns arg1 arg2 ...`` with dynamic script
                    # args: emitted via WASM assembly (not fallback), so each
                    # script arg is evaluated through ``_emit_value`` and may
                    # need ``tcl_append``, ``tcl_array_get``, etc.
                    needed.add("tcl_append")
                    for a in barrier_args[2:]:
                        _scan_value(a)
                elif (
                    barrier_cmd == "return"
                    and barrier_args
                    and len(barrier_args) == 3
                    and barrier_args[0] == "-code"
                    and barrier_args[1] == "error"
                ):
                    # ``return -code error <msg>`` emits the message
                    # inline via ``_emit_value``; scan it so the
                    # interpolation helpers (``tcl_append``, maybe
                    # ``tcl_array_get`` for ``$arr(key)`` references)
                    # show up in the module's import list.
                    _scan_value(barrier_args[2])
            case IRBlock(body=body):
                # ``namespace eval`` body inlined; recurse into its
                # statements so any runtime helpers they need (e.g.
                # tcl_append for interpolated strings) show up in
                # the module's import list.
                _scan_script(body)
            case IRIf(clauses=clauses, else_body=else_body):
                for clause in clauses:
                    _scan_expr(clause.condition)
                    _scan_script(clause.body)
                if else_body:
                    _scan_script(else_body)
            case IRFor(init=init, condition=condition, body=body, next=next_s):
                _scan_script(init)
                _scan_expr(condition)
                _scan_script(body)
                _scan_script(next_s)
            case IRWhile(condition=condition, body=body):
                _scan_expr(condition)
                _scan_script(body)
            case IRForeach(body=body):
                needed.add("tcl_list_length")
                needed.add("tcl_list_index")
                _scan_script(body)
            case IRSwitch(subject=subject, arms=arms, default_body=default_body, mode=mode):
                if mode == "glob":
                    needed.add("tcl_string_match")
                else:
                    needed.add("tcl_string_equal")
                # The subject may be a command substitution or
                # interpolated string — scan it so the codegen's
                # ``_emit_value`` call in ``_emit_str_value``
                # (which now routes ``ExprRaw`` through the full
                # value-emitter) can reach ``tcl_append`` /
                # ``tcl_list_length`` / etc.  Without this the
                # interpolation emitter degrades to
                # ``_emit_obj_literal`` and the subject compares
                # against its raw source text rather than its
                # runtime value.
                _scan_value(subject)
                for arm in arms:
                    if arm.body:
                        _scan_script(arm.body)
                if default_body:
                    _scan_script(default_body)
            case IRCatch(body=body):
                needed.add("tcl_catch_enter")
                needed.add("tcl_catch_leave")
                needed.add("tcl_catch_result")
                needed.add("tcl_catch_has_error")
                needed.add("tcl_catch_set_ok_result")
                _scan_script(body)
            case IRTry(body=body, finally_body=finally_body):
                _scan_script(body)
                if finally_body:
                    _scan_script(finally_body)

    # Scan top-level
    _scan_script(ir_module.top_level)

    # Scan procedures
    for proc in ir_module.procedures.values():
        _scan_script(proc.body)

    # Scan methods
    if ir_module.methods:
        for method in ir_module.methods.values():
            _scan_script(method.body)

    # Always include TclObj lifecycle imports — every non-trivial module
    # needs them for value creation, storage, and arithmetic unboxing.
    needed.update(_OBJ_LIFECYCLE_IMPORTS)

    # Always include error and eval — error for traps, eval for
    # interpreter fallback on uncompiled commands.  tcl_diag_set is
    # also always imported so trap messages can be resolved against
    # the sidecar source-location map.  tcl_global_set / get cover
    # the top-level-var-mirror path in ``_emit_var_write_obj_impl``
    # that keeps ``::top`` writes visible to eval fallbacks.
    needed.add("tcl_error")
    needed.add("tcl_eval")
    needed.add("tcl_ns_set")
    needed.add("tcl_ns_restore")
    needed.add("tcl_diag_set")
    needed.add("tcl_global_set")
    needed.add("tcl_global_get")
    # Register each compiled proc by name so the interpreter's
    # host-bridge dispatch can find it when an interpreted caller
    # (a Tcl-source proc body walked by eval_script) invokes a
    # compiled helper.  The frame helpers are also pulled in so
    # compiled procs can push a frame + bind params (necessary for
    # eval-fallbacks inside a proc body to resolve ``$var`` via
    # ``var_resolve``).  Only needed when the module has any procs.
    if ir_module.procedures:
        needed.add("tcl_proc_register_compiled")
        needed.add("tcl_frame_push")
        needed.add("tcl_frame_pop")
        needed.add("tcl_local_set")
        needed.add("tcl_local_get")

    return needed


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

    Tcl variables are stored as i32 TclObj pointers in WASM locals.
    Expression evaluation (``expr``) operates on raw i64 integers
    internally — variables are unboxed via ``tcl_obj_get_int`` before
    arithmetic and results are boxed via ``tcl_obj_new_int`` when
    stored back into locals.  String literals are boxed via
    ``tcl_obj_new_string`` from data-segment pointers.
    """

    def __init__(
        self,
        cfg: CFGFunction,
        params: tuple[str, ...] = (),
        *,
        optimise: bool = False,
        is_proc: bool = False,
        func_index_base: int = 0,
        shared_imports: dict[str, int] | None = None,
        proc_index: dict[str, tuple[int, int]] | None = None,
        proc_defaults: dict[str, tuple[str | None, ...]] | None = None,
        proc_args_tail: set[str] | None = None,
        shared_strings: list[tuple[str, int]] | None = None,
        shared_string_index: dict[str, int] | None = None,
        shared_string_offset: list[int] | None = None,
        proc_qname: str | None = None,
        diag_map: DiagMap | None = None,
        escape_summary: "ProcEscapeSummary | None" = None,
    ) -> None:
        self._cfg = cfg
        self._params = params
        self._optimise = optimise
        self._is_proc = is_proc
        self._func_index_base = func_index_base
        # Fully qualified proc name (e.g. ``::counter::init``) so the
        # ``variable`` command can resolve ``name`` → ``::counter::name``.
        # None for ``::top`` and other non-proc emitters.
        self._proc_qname = proc_qname
        self._proc_namespace = _derive_proc_namespace(proc_qname) if proc_qname else "::"

        # Shared state from module compilation
        self._shared_imports: dict[str, int] = shared_imports or {}
        # proc_index maps qualified name → (func_idx, n_params)
        self._proc_index: dict[str, tuple[int, int]] = proc_index or {}
        # Per-proc default-value strings, indexed by slot.  Consulted
        # by ``_emit_cmd_proc_call`` to pad missing call-site args
        # with the declared default rather than a boxed zero.
        self._proc_defaults: dict[str, tuple[str | None, ...]] = proc_defaults or {}
        # Set of proc qnames whose last formal parameter is ``args``
        # (Tcl's variadic catch-all).  When calling these procs,
        # surplus call-site args are packed into a single list TclObj
        # and passed as the last positional argument.
        self._proc_args_tail: set[str] = proc_args_tail or set()

        # Local variable management
        self._local_names: list[str] = []
        self._local_types: list[ValType] = []
        self._local_index: dict[str, int] = {}
        # Subset of _local_index: only Tcl user-visible variables (not
        # internal scratch locals created by _add_extra_local).  Used
        # by _emit_frame_sync to mirror proc locals into the call frame
        # before handing control to the Zig interpreter.
        self._tcl_var_locals: dict[str, int] = {}

        # Register parameters as locals
        for p in params:
            self._intern_local(p)

        # Instructions
        self._body: list[WasmInstruction] = []

        # String data accumulation — shared across all functions in the
        # module so data segments don't overlap in linear memory.
        # shared_string_offset is a single-element list so all emitters
        # mutate the same counter.
        self._strings: list[tuple[str, int]] = shared_strings if shared_strings is not None else []
        self._string_index: dict[str, int] = (
            shared_string_index if shared_string_index is not None else {}
        )
        self._string_offset_ref: list[int] = (
            shared_string_offset if shared_string_offset is not None else [0]
        )

        # Global variable tracking
        self._globals: set[str] = set()

        # Variable aliases — populated by ``upvar`` and ``variable``.
        # Maps a local Tcl variable name to an (kind, target_local_idx)
        # tuple where target_local_idx is a hidden WASM local holding the
        # resolved target name as an i32 TclObj pointer.
        #   kind == "global": reads/writes go through tcl_global_get/set
        #     using the TclObj name stashed at the alias site.
        # Caller-frame (``upvar 1 var local``) is not yet supported —
        # _emit_cmd_upvar traps at compile time for non-``#0`` levels.
        self._aliases: dict[str, tuple[str, int]] = {}

        # Catch depth — when > 0, unsupported commands call error()
        # without unreachable (error returns normally in catch context).
        self._catch_depth = 0

        # Block ordering for structured control flow
        self._visited: set[str] = set()

        # Active loop headers — used by _is_loop_header to distinguish
        # a nested-loop back-edge from an outer-loop back-edge. When we
        # walk from a body-start node looking for a cycle back to the
        # current branch block, reaching an active OUTER loop header
        # means we've followed the outer loop's back-edge, not found
        # a new nested loop; stop the walk in that case.
        self._active_loop_headers: set[str] = set()

        # Loop control — break/continue use WASM structured control flow.
        # The `_loop_depth` increments each time we enter a block{loop{}}
        # pair.  An `if` inside a loop body adds nesting that break/continue
        # must skip over.  We track the nesting depth in `_ctrl_depth` and
        # record the ctrl_depth at each loop entry in `_loop_ctrl_depths`.
        self._loop_depth = 0
        self._ctrl_depth = 0
        self._loop_ctrl_depths: list[int] = []

        # Constant propagation state (for optimised mode)
        self._const_map: dict[str, int] = {}

        # Source-location sidecar.  Shared across all emitters in a
        # module so a single monotonic ID space covers every trap-able
        # call site.  When ``None`` the emitter runs without diag
        # instrumentation (useful for tests that don't care about
        # source mapping); fallback / unsupported sites still work,
        # they just don't emit ``tcl_diag_set`` before the call.
        self._diag_map: DiagMap | None = diag_map

        # Active ``namespace eval`` block (if any).  When non-None,
        # unqualified command names inside the block are resolved to
        # ``<block_namespace>::<name>`` before proc-registry lookup,
        # matching Tcl's namespace path semantics for the common case
        # where a namespace body calls its own helper procs by bare
        # name (``Option -verbose …`` inside ``namespace eval
        # ::tcltest { … }`` really means ``::tcltest::Option``).
        self._block_namespace: str | None = None

        # Current statement's source range + command, updated at the
        # top of ``_emit_stmt`` and consulted by the fallback / trap
        # emitters to stamp a diag site.  Zero/empty until the first
        # statement is walked.
        self._current_range: Range | None = None
        self._current_command: str = ""
        self._current_args: tuple[str, ...] = ()

        # Per-proc var-escape summary.  When provided, _iter_sync_locals
        # uses it to skip variables the analysis proved cannot be seen by
        # name from the interpreter — trimming the frame-sync set below
        # what ``vars_used`` alone would allow.  ``None`` means "no
        # analysis run" and falls back to conservative behaviour.
        self._escape_summary: ProcEscapeSummary | None = escape_summary

    def _intern_local(self, name: str) -> int:
        """Register a Tcl variable as an i32 local (TclObj pointer)."""
        idx = self._local_index.get(name)
        if idx is not None:
            return idx
        idx = len(self._local_names)
        self._local_names.append(name)
        self._local_types.append(ValType.I32)
        self._local_index[name] = idx
        self._tcl_var_locals[name] = idx
        return idx

    def _add_extra_local(self, prefix: str = "_tmp", val_type: ValType = ValType.I64) -> int:
        """Create an internal temporary local with the given type.

        Internal locals default to i64 for arithmetic scratch space.
        """
        name = f"{prefix}_{len(self._local_names)}"
        idx = len(self._local_names)
        self._local_names.append(name)
        self._local_types.append(val_type)
        self._local_index[name] = idx
        return idx

    def _resolve_var_name(self, value: str) -> str | None:
        """Extract a variable name from a Tcl variable reference.

        Returns the bare variable name for ``$x`` or ``${x}`` patterns
        when *value* is the entire reference (nothing after it), or
        ``None`` otherwise.  Rejecting mixed forms like ``$x*`` or
        ``$x$y`` is essential — those are interpolated strings where
        the codegen has to emit a concat chain, not a single
        ``local.get`` of a variable named ``x*``.
        """
        if value.startswith("${") and value.endswith("}"):
            # ``${name}`` must not have anything past the closing
            # brace.  Check by scanning: the first ``}`` that
            # terminates the braced form must be at position
            # ``len - 1``.
            inner = value[2:-1]
            # ``${name}`` braces don't nest, so any inner ``}`` is
            # invalid Tcl — but we degrade gracefully by treating
            # it as non-simple (fall through to interpolation).
            if "}" in inner:
                return None
            return inner
        if value.startswith("$") and not value.startswith("$["):
            # Bare ``$name`` accepts only identifier characters in
            # *name* (letters, digits, underscore).  Anything else
            # — including ``*``, ``/``, ``-`` (used by tcltest
            # option names like ``-tmpdir``) — ends the variable
            # reference and starts literal-text territory, which
            # means the value is an interpolated string.
            #
            # Exception: ``$arr(key)`` is a single variable reference
            # (array element read) — once we see ``(`` after a valid
            # identifier head, consume through the matching ``)`` and
            # treat the whole thing as the variable name so that the
            # downstream ``_emit_var_read_obj`` path can dispatch it
            # to the array-element reader via ``_parse_array_ref``.
            i = 1
            while i < len(value):
                c = value[i]
                if c.isalnum() or c == "_":
                    i += 1
                    continue
                break
            if i < len(value) and value[i] == "(" and value.endswith(")"):
                # ``$arr(key)`` — the key may itself contain
                # substitutions; let downstream codegen handle it.
                return value[1:]
            if i != len(value):
                return None
            return value[1:]
        return None

    def _emit_value(self, value: str, *, was_braced: bool = False) -> None:
        """Emit an i32 TclObj pointer for *value*.

        Resolves ``$x`` and ``${x}`` variable references to local.get
        (already i32), detects ``[cmd ...]`` command substitution and
        dispatches through the proc-call mechanism, or creates a new
        TclObj for literals.

        For strings with embedded substitutions (e.g. ``"hello $x"`` or
        ``"fib(10) = [fib 10]"``), parses the string into chunks and
        emits a concat chain using the runtime ``tcl_append`` function.

        *was_braced* is set by callers that know the IR value came from
        a braced ``{…}`` token (the lexer strips the outer braces before
        storing the value, so we otherwise can't tell).  In that case,
        Tcl semantics say NO substitution applies — backslash
        substitution and ``$`` / ``[`` interpolation must be suppressed
        so ``\\{`` stays as the two-char sequence ``\\{`` instead of
        collapsing to ``{``.
        """
        if not was_braced:
            var = self._resolve_var_name(value)
            if var is not None:
                # Intern on first reference so reads before assignment
                # get a proper local (default-initialised to 0) instead
                # of falling through and boxing "$var" as a string literal.
                # Aliased variables (upvar/variable) route through the
                # runtime global table via _emit_var_read_obj.
                self._emit_var_read_obj(var)
                return
            # Try direct local lookup (for bare names like in IRAssignValue).
            # Alias-aware: checks _aliases before local_index.
            if value in self._aliases:
                self._emit_var_read_obj(value)
                return
            idx = self._local_index.get(value)
            if idx is not None:
                self._emit_local_get(idx)
                return
        # Braced literal — outer braces suppress all substitution in Tcl.
        # _split_command_subst preserves {…} so downstream code can
        # distinguish a braced word (literal) from a quoted one (allows
        # substitution).  Strip the braces and emit the content as-is.
        if value.startswith("{") and value.endswith("}"):
            self._emit_obj_literal(value[1:-1])
            return
        # Braced token whose outer braces the IR already stripped — emit
        # the raw content verbatim, no ``\\`` / ``$`` / ``[`` processing.
        if was_braced:
            self._emit_obj_literal(value)
            return
        # Command substitution: [cmd arg1 arg2 ...]
        if value.startswith("[") and value.endswith("]"):
            self._emit_command_subst_value(value)
            return
        # Interpolated string: contains embedded $var/${var}/[cmd]
        # mixed with literal text.  Emit a concat chain.
        if self._has_embedded_subst(value):
            self._emit_interpolated_value(value)
            return
        # Apply Tcl backslash substitution for non-braced literals.  Braced
        # words (already handled above) suppress all substitution; plain words
        # and double-quoted words allow ``\\`` → ``\``, ``\n`` → newline, etc.
        if "\\" in value:
            value = _tcl_backslash_subst(value)
        self._emit_obj_literal(value)

    def _has_embedded_subst(self, value: str) -> bool:
        """Check if *value* contains embedded $var, ${var}, or [cmd] substitutions."""
        i = 0
        n = len(value)
        while i < n:
            c = value[i]
            if c == "\\" and i + 1 < n:
                i += 2
                continue
            if c == "$" and i + 1 < n:
                nxt = value[i + 1]
                if nxt == "{" or nxt.isalpha() or nxt == "_":
                    return True
            if c == "[":
                return True
            i += 1
        return False

    def _parse_interpolated_parts(self, value: str) -> list[tuple[str, str]]:
        """Parse an interpolated string into ("lit", text) / ("var", name) /
        ("cmd", body) tuples.

        The returned list represents the string as a sequence of parts to
        concatenate.  Backslash escapes are preserved in literal parts.
        """
        parts: list[tuple[str, str]] = []
        buf: list[str] = []

        def flush() -> None:
            if buf:
                parts.append(("lit", "".join(buf)))
                buf.clear()

        i = 0
        n = len(value)
        while i < n:
            c = value[i]
            if c == "\\" and i + 1 < n:
                esc = value[i + 1]
                buf.append({"n": "\n", "t": "\t", "r": "\r"}.get(esc, esc))
                i += 2
                continue
            if c == "$" and i + 1 < n:
                nxt = value[i + 1]
                if nxt == "{":
                    # ${name}
                    j = value.find("}", i + 2)
                    if j == -1:
                        buf.append(c)
                        i += 1
                        continue
                    flush()
                    parts.append(("var", value[i + 2 : j]))
                    i = j + 1
                    continue
                if nxt.isalpha() or nxt == "_" or nxt == ":":
                    # $name — accepts [A-Za-z0-9_] and ``::`` namespace
                    # separators so names like ``$::counter::secsPerMinute``
                    # parse as a single variable reference rather than
                    # stopping at the first colon.  When the name is
                    # immediately followed by ``(``, consume through
                    # the matching ``)`` so ``$arr(key)`` is emitted
                    # as a single ("var", "arr(key)") part — the
                    # downstream `_emit_var_read_obj` dispatcher then
                    # routes it through ``_parse_array_ref`` to the
                    # array-element reader.  Without this, ``$a(x)``
                    # would split into ``$a`` + literal ``(x)`` and
                    # the array lookup would never happen.
                    j = i + 1
                    while j < n:
                        ch = value[j]
                        if ch.isalnum() or ch == "_":
                            j += 1
                        elif ch == ":" and j + 1 < n and value[j + 1] == ":":
                            j += 2
                        else:
                            break
                    if j < n and value[j] == "(":
                        # Scan for matching ')' — nested ``(..)``
                        # aren't standard in ``$arr(key)`` syntax
                        # but embedded ``$var`` / ``[cmd]`` may
                        # appear as part of the key; we look for
                        # the first unescaped ``)`` at depth 0
                        # relative to ``(``/``)`` nesting.
                        depth = 1
                        k = j + 1
                        while k < n and depth > 0:
                            ck = value[k]
                            if ck == "\\" and k + 1 < n:
                                k += 2
                                continue
                            if ck == "(":
                                depth += 1
                            elif ck == ")":
                                depth -= 1
                                if depth == 0:
                                    break
                            k += 1
                        if k < n and value[k] == ")":
                            flush()
                            parts.append(("var", value[i + 1 : k + 1]))
                            i = k + 1
                            continue
                    flush()
                    parts.append(("var", value[i + 1 : j]))
                    i = j
                    continue
            if c == "[":
                # [cmd ...] — find matching ] with nesting
                depth = 1
                j = i + 1
                while j < n and depth > 0:
                    if value[j] == "[":
                        depth += 1
                    elif value[j] == "]":
                        depth -= 1
                    if depth > 0:
                        j += 1
                    else:
                        break
                if depth != 0:
                    buf.append(c)
                    i += 1
                    continue
                flush()
                parts.append(("cmd", value[i + 1 : j]))
                i = j + 1
                continue
            buf.append(c)
            i += 1
        flush()
        return parts

    def _emit_interpolated_value(self, value: str) -> None:
        """Emit a TclObj for an interpolated string with $var/[cmd] chunks.

        Parses the string and emits each part (literal, variable, or
        command substitution), then concatenates them via tcl_append.
        """
        parts = self._parse_interpolated_parts(value)
        if not parts:
            self._emit_obj_literal("")
            return
        append_idx = self._shared_imports.get("tcl_append")
        if append_idx is None:
            # No append available — fall back to literal
            self._emit_obj_literal(value)
            return

        # Emit the first part
        self._emit_part(parts[0])
        # Concat remaining parts
        for part in parts[1:]:
            self._emit_part(part)
            self._emit_call(append_idx)

    def _emit_part(self, part: tuple[str, str]) -> None:
        kind, data = part
        if kind == "lit":
            self._emit_obj_literal(data)
        elif kind == "var":
            # Route through _emit_var_read_obj so array references
            # (``arr(key)``), aliases (upvar/variable), and
            # ``::``-qualified globals resolve through the correct
            # runtime path rather than all being treated as simple
            # WASM locals — otherwise ``"v=$arr(x)"`` would intern
            # a local named ``arr(x)`` and read 0 instead of
            # dispatching to ``tcl_array_get``.
            self._emit_var_read_obj(data)
        elif kind == "cmd":
            self._emit_command_subst_value("[" + data + "]")

    def _emit_command_subst_value(self, text: str) -> None:
        """Emit an i32 TclObj for a command substitution in value context.

        Dispatches ``[cmd ...]`` to a known proc (keeping its i32 result)
        or to ``[expr {...}]`` (compiling the expression and boxing).
        Falls back to a null TclObj for unknown commands.
        """
        cmd_text = text[1:-1].strip()
        if not cmd_text:
            self._emit_i32_const(0)
            return

        parts = self._split_command_subst(cmd_text)
        if not parts:
            self._emit_i32_const(0)
            return

        cmd_name = parts[0]
        cmd_args = parts[1:]

        # [expr {...}] — compile expression and box to TclObj.
        # Strip outer braces from expr_arg ({...} kept by splitter).
        if cmd_name == "expr" and len(cmd_args) == 1:
            expr_arg = cmd_args[0]
            if expr_arg.startswith("{") and expr_arg.endswith("}"):
                expr_arg = expr_arg[1:-1]
            from ...parsing.expr_parser import parse_expr

            try:
                nested_expr = parse_expr(expr_arg)
                self._emit_expr(nested_expr)
                self._emit_box_int()
                return
            except Exception:
                pass

        # [catch {body} ?varName?] in value context — re-parse the body
        # and emit via the real catch codegen so the body runs in the
        # compiled frame (with locals visible to eval-fallbacks).  Going
        # through ``_emit_eval_fallback`` would rebuild the script as
        # ``catch $v1 $v2 ...`` and lose the body's word structure.
        if cmd_name == "catch" and cmd_args:
            self._emit_catch_from_args(tuple(cmd_args), defs=(), keep_on_stack=True)
            return

        # ``[set varname]`` — 1-arg read form.  Without this shortcut
        # the call falls through to the eval fallback, which rebuilds
        # the script and evaluates ``set`` against the global table —
        # but compiled-frame locals and array elements live in
        # different storage, so ``[set a(x)]`` would return empty.
        # Route reads through the same ``_emit_var_read_obj`` path
        # that handles array refs, aliases, and qualified globals.
        if cmd_name == "set" and len(cmd_args) == 1:
            self._emit_var_read_obj(cmd_args[0])
            return
        # ``[set varname value]`` — 2-arg write: evaluate the value,
        # store it, and leave it on the stack (set returns the new value).
        if cmd_name == "set" and len(cmd_args) == 2:
            self._emit_value(cmd_args[1])
            self._emit_var_write_obj_keep(cmd_args[0])
            return

        # ``[namespace origin <name>]`` — return the canonical qualified
        # name of a command.  The runtime has no proc registry visible
        # to ``namespace origin``, so resolve at compile time: if the
        # bare name is known in the current proc's namespace, return its
        # qualified form; otherwise prepend ``::`` for global scope.
        # The tcltest pattern ``[list [namespace origin Eval] ...]``
        # depends on this producing a non-null string so uplevel
        # dispatches the right proc.
        if cmd_name == "namespace" and len(cmd_args) >= 2 and cmd_args[0] == "origin":
            bare_name = cmd_args[1]
            qualified = self._resolve_proc_qname(bare_name) or bare_name
            if not qualified.startswith("::"):
                ns = self._proc_namespace or "::"
                qualified = f"{ns}::{bare_name}" if ns != "::" else f"::{bare_name}"
            self._emit_obj_literal(qualified)
            return

        # Proc call — result is already i32 TclObj
        proc_info = self._resolve_proc(cmd_name)
        if proc_info is not None:
            func_idx, n_params = proc_info
            qname = self._resolve_proc_qname(cmd_name)
            defaults = self._proc_defaults.get(qname, ()) if qname else ()
            for i in range(min(n_params, len(cmd_args))):
                self._emit_value(cmd_args[i])
            # Missing args: emit the declared default (as a string
            # literal so ``{foo bar}`` reaches the callee as the
            # literal string), or fall back to a boxed-int 0 sentinel
            # when no default was declared.  ``[outputChannel]`` with
            # no args relies on this — without proper default
            # handling, ``filename`` arrives as the integer TclObj 0
            # rather than the ``""`` the ``proc outputChannel
            # {{filename ""}}`` spec declares, making ``info level
            # 0`` report a 2-element list and the no-args early-
            # return check fail.
            for slot in range(len(cmd_args), n_params):
                default = defaults[slot] if slot < len(defaults) else None
                if default is None:
                    self._emit_default_arg()
                else:
                    self._emit_obj_literal(default)
            self._emit_call(func_idx)
            return

        # dict sub-command in value context — returns i32 TclObj
        if cmd_name == "dict" and cmd_args:
            subcmd = cmd_args[0]
            import_key = _DICT_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = cmd_args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not spec[3]:
                    self._emit_i32_const(0)
                return

        # string sub-command in value context
        if cmd_name == "string" and cmd_args:
            subcmd = cmd_args[0]
            import_key = _STRING_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = cmd_args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not spec[3]:
                    self._emit_i32_const(0)
                return

        # info sub-command in value context — leaves i32 TclObj on stack.
        if cmd_name == "info" and cmd_args:
            self._emit_info_value(tuple(cmd_args))
            return

        # lassign in value context — return leftover list.
        if cmd_name == "lassign" and cmd_args:
            self._emit_cmd_lassign(tuple(cmd_args), defs=(), keep_on_stack=True)
            return

        # clock in value context — i32 TclObj of the timer value.
        if cmd_name == "clock" and cmd_args:
            self._emit_clock_value(tuple(cmd_args))
            return

        # array subcommand in value context.
        if cmd_name == "array" and cmd_args:
            self._emit_array_subcmd_value(tuple(cmd_args))
            return

        # uplevel in value context.
        if cmd_name == "uplevel" and cmd_args:
            self._emit_cmd_uplevel(tuple(cmd_args))
            return

        # ``list`` in value context — variadic list builder, same
        # space-joined compile-time shape as the statement-context
        # path.
        if cmd_name == "list":
            self._emit_list_value(tuple(cmd_args))
            return

        # ``concat`` in value context — variadic, trim+join semantics.
        if cmd_name == "concat":
            concat_idx = self._shared_imports.get("tcl_concat")
            if concat_idx is not None:
                if not cmd_args:
                    self._emit_obj_literal("")
                elif len(cmd_args) == 1:
                    # Single-arg concat: trim whitespace (concat(x, "") returns trimmed x).
                    self._emit_value(cmd_args[0])
                    self._emit_obj_literal("")
                    self._emit_call(concat_idx)
                else:
                    self._emit_value(cmd_args[0])
                    for a in cmd_args[1:]:
                        self._emit_value(a)
                        self._emit_call(concat_idx)
                return

        # Runtime command in value context (llength, lindex, etc.)
        if cmd_name in _CMD_RUNTIME:
            import_key, _ = _CMD_RUNTIME[cmd_name]
            func_idx = self._shared_imports.get(import_key)
            if func_idx is not None:
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                for i in range(min(param_count, len(cmd_args))):
                    self._emit_value(cmd_args[i])
                for _ in range(param_count - len(cmd_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not spec[3]:
                    self._emit_i32_const(0)
                return

        # ``[namespace eval ns arg1 arg2 ...]`` with dynamic script args.
        # The eval-fallback path would embed the literal source text (e.g.
        # ``$CustomMatch($mode)``) and let the interpreter re-evaluate it.
        # But the interpreter can't see compiled-frame aliases (e.g.
        # ``CustomMatch`` aliased to ``::tcltest::CustomMatch`` via
        # ``variable CustomMatch``), so array-element references like
        # ``$CustomMatch($mode)`` resolve to the wrong value (or nothing).
        # Fix: evaluate each script arg through the WASM compiled path
        # (which honours aliases) to get the actual string values, join
        # them with spaces at runtime, then pass the assembled script to
        # ``tcl_eval``.
        if cmd_name == "namespace" and cmd_args and cmd_args[0] == "eval" and len(cmd_args) > 2:
            if self._emit_namespace_eval_bridge(cmd_args[2:], drop_result=False):
                return
            # Required runtime imports missing — fall through to the
            # generic eval fallback below so we still produce something.
            self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)
            return

        # Unknown command in value context — fall back to interpreter.
        # Use the original command text (script_override) so braced
        # words like {[^a-z]+} are not re-interpreted as command
        # substitutions during fallback script reconstruction.
        self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)

    def _emit_obj_literal(self, value: str) -> None:
        """Create a TclObj for a literal value — pushes i32 pointer.

        Integers are boxed via ``tcl_obj_new_int``; non-numeric strings
        are boxed via ``tcl_obj_new_string`` from the data segment.

        Requires TclObj lifecycle imports; raises if they are missing
        to avoid silently emitting raw i32 values that would be
        misinterpreted as TclObj pointers.
        """
        new_int_idx = self._shared_imports.get("tcl_obj_new_int")
        new_str_idx = self._shared_imports.get("tcl_obj_new_string")
        if new_int_idx is None or new_str_idx is None:
            msg = (
                "WASM TclObj literal emission requires shared imports "
                "'tcl_obj_new_int' and 'tcl_obj_new_string'"
            )
            raise RuntimeError(msg)

        try:
            int_val = int(value)
            self._emit_i64_const(int_val)
            self._emit_call(new_int_idx)
        except ValueError:
            try:
                float_val = float(value)
                self._emit_i64_const(int(float_val))
                self._emit_call(new_int_idx)
            except ValueError:
                offset = self._intern_string(value)
                encoded = value.encode("utf-8")
                # data_ptr = segment offset + 4 (skip length prefix)
                self._emit_i32_const(offset + 4)
                self._emit_i32_const(len(encoded))
                self._emit_call(new_str_idx)

    def _emit_box_int(self) -> None:
        """Convert i64 on stack to i32 TclObj pointer via tcl_obj_new_int."""
        self._emit_call(self._shared_imports["tcl_obj_new_int"])

    def _emit_unbox_int(self) -> None:
        """Convert i32 TclObj pointer on stack to i64 via tcl_obj_get_int."""
        self._emit_call(self._shared_imports["tcl_obj_get_int"])

    def _emit_default_arg(self) -> None:
        """Emit a boxed integer zero as a default/missing argument.

        Unlike ``i32.const 0`` (null pointer), this creates a real
        TclObj so that ``tcl_obj_get_int`` in the callee reads 0
        instead of arbitrary memory.
        """
        self._emit_i64_const(0)
        self._emit_box_int()

    def _emit_args_list(
        self,
        tail_args: tuple[str, ...],
        *,
        was_braced_fn: "Callable[[int], bool] | None" = None,
    ) -> None:
        """Emit a Tcl list TclObj containing *tail_args*.

        Used when calling a proc whose last formal parameter is ``args``
        (Tcl's variadic catch-all): all surplus call-site arguments must
        be packed into a single list before being passed as that slot.

        If the list is empty (no surplus args) emit an empty string
        TclObj (Tcl's empty list).  If all elements are pure literals
        (no ``$var`` / ``[cmd]`` substitutions) build the list string
        at compile time and emit it as a single object literal.
        Otherwise build the list at runtime by starting with an empty
        list and calling ``tcl_cmd_lappend`` for each element.

        *was_braced_fn* tells us for each tail index whether the source
        token was a braced ``{…}`` word.  Braced content is protected
        from backslash substitution per Tcl semantics, so when the
        call-site word was braced the arg's bytes must pass through
        unchanged even if they contain ``\\{`` / ``\\n`` / ``\\$`` /
        etc.  Without this flag a test body like
        ``{set x "a\\{"; lappend x abc}`` would have its ``\\{`` folded
        to ``{`` before reaching the proc's ``args``, breaking later
        reparsing.
        """
        if not tail_args:
            # ``args`` formal receives the empty list ``{}``
            self._emit_obj_literal("")
            return

        def _was_braced(i: int) -> bool:
            return was_braced_fn(i) if was_braced_fn is not None else False

        # Check whether all elements are plain literals.
        def _is_literal(a: str) -> bool:
            return (
                not self._has_embedded_subst(a)
                and not a.startswith("$")
                and not a.startswith("[")
                and a not in self._aliases
                and a not in self._local_index
            )

        all_literals = all(_is_literal(a) for a in tail_args)
        if all_literals:
            # Build the list string at compile time.  IR values have outer
            # braces already stripped by the lexer, so treat brace-looking
            # values (e.g. "{}" from source "{{}}") as literal data and let
            # _tcl_list_quote encode them correctly.  Non-braced values may
            # have raw backslash sequences that need expansion first; braced
            # values carry their exact source bytes and must pass through
            # unchanged.
            def _prep(a: str, braced: bool) -> str:
                if braced:
                    return a
                if a.startswith("{") and a.endswith("}"):
                    return a  # brace chars are part of the value, not quoting
                return _tcl_backslash_subst(a) if "\\" in a else a

            list_str = " ".join(
                _tcl_list_quote(_prep(a, _was_braced(i)), first=(i == 0))
                for i, a in enumerate(tail_args)
            )
            self._emit_obj_literal(list_str)
            return

        # Runtime path: start with empty list, lappend each arg
        lappend_idx = self._shared_imports.get("tcl_lappend")
        if lappend_idx is not None:
            self._emit_obj_literal("")  # empty list seed
            for i, a in enumerate(tail_args):
                self._emit_value(a, was_braced=_was_braced(i))
                self._emit_call(lappend_idx)
        else:
            # No lappend available — fall back to compile-time join.
            # IR values are already de-braced by the lexer; _tcl_list_quote
            # handles proper list encoding.
            def _prep2(a: str, braced: bool) -> str:
                if braced:
                    return a
                return _tcl_backslash_subst(a) if "\\" in a else a

            list_str = " ".join(
                _tcl_list_quote(_prep2(a, _was_braced(i)), first=(i == 0))
                for i, a in enumerate(tail_args)
            )
            self._emit_obj_literal(list_str)

    def _intern_string(self, value: str) -> int:
        """Return the memory offset for a string constant."""
        if value in self._string_index:
            return self._string_index[value]
        offset = self._string_offset_ref[0]
        encoded = value.encode("utf-8")
        self._string_offset_ref[0] += len(encoded) + 4  # 4 bytes for length prefix
        self._strings.append((value, offset))
        self._string_index[value] = offset
        return offset

    def _emit(self, op: int, operands: bytes = b"") -> None:
        if op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
            self._ctrl_depth += 1
        elif op == WasmOp.END:
            self._ctrl_depth -= 1
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
        """Emit WASM instructions to evaluate an expression, leaving i64 on stack.

        Expression evaluation operates on raw i64 integers.  Variables
        (i32 TclObj pointers) are unboxed via ``tcl_obj_get_int`` before
        use; the caller is responsible for boxing the result back to i32
        when storing into a local.
        """
        match node:
            case ExprLiteral(text=value):
                self._emit_literal(value)
            case ExprVar(name=name, text=text):
                # Alias-aware: reads through _emit_var_read_obj so upvar'd
                # and variable-declared vars resolve via the global table.
                # For array refs (``$a(key)``) the parser reports only the
                # base name; the ``text`` field carries the full token
                # including the ``(key)`` suffix, which _emit_var_read_obj
                # re-parses via _parse_array_ref.
                resolved = name
                if "(" in text and text.endswith(")"):
                    # Drop the leading ``$``/``${`` so _parse_array_ref
                    # sees ``name(key)``.
                    bare = text[1:] if text.startswith("$") else text
                    if bare.startswith("{") and bare.endswith("}"):
                        bare = bare[1:-1]
                    resolved = bare
                self._emit_var_read_obj(resolved)
                self._emit_unbox_int()
            case ExprBinary(op=op, left=left, right=right):
                self._emit_binary(op, left, right)
            case ExprUnary(op=op, operand=operand):
                self._emit_unary(op, operand)
            case ExprTernary(condition=cond, true_branch=te, false_branch=fe):
                self._emit_ternary(cond, te, fe)
            case ExprCall(function=func, args=args):
                self._emit_func_call(func, args)
            case ExprCommand(text=text):
                self._emit_command_subst(text)
            case ExprString(text=text):
                # Quoted string in expr context — try numeric parse
                self._emit_literal(text)
            case _:
                # Fallback for unsupported expression types (ExprRaw)
                self._emit_i64_const(0)

    def _emit_command_subst(self, text: str) -> None:
        """Emit WASM for a command substitution in expression context.

        Parses ``[cmd arg1 arg2 ...]`` text.  If *cmd* is a known proc,
        emits a direct call and unboxes the i32 result to i64.  If it's
        ``[expr {...}]``, recursively compiles the nested expression.
        Otherwise falls back to i64.const 0.

        The result is always i64 (consistent with expression context).
        """
        # Strip surrounding brackets if present
        cmd_text = text
        if cmd_text.startswith("[") and cmd_text.endswith("]"):
            cmd_text = cmd_text[1:-1]
        cmd_text = cmd_text.strip()
        if not cmd_text:
            self._emit_i64_const(0)
            return

        # Simple split — handles most Tcl command forms.
        # For nested brackets, we rely on the expression parser having
        # already extracted the outermost command boundary.
        parts = self._split_command_subst(cmd_text)
        if not parts:
            self._emit_i64_const(0)
            return

        cmd_name = parts[0]
        cmd_args = parts[1:]

        # Handle [expr {...}] — recursively compile the expression.
        # Strip outer braces from expr_arg ({...} kept by splitter).
        if cmd_name == "expr" and len(cmd_args) == 1:
            expr_arg = cmd_args[0]
            if expr_arg.startswith("{") and expr_arg.endswith("}"):
                expr_arg = expr_arg[1:-1]
            # Parse and emit the nested expression
            from ...parsing.expr_parser import parse_expr

            try:
                nested_expr = parse_expr(expr_arg)
                self._emit_expr(nested_expr)
                return
            except Exception:
                self._emit_i64_const(0)
                return

        # [catch {body} ?varName?] in expression context — emit the
        # real catch codegen and unbox the i32 return code to i64 so
        # ``if {[catch { ... } msg]} { ... }`` tests OK vs ERROR via
        # integer comparison.  Falling through to the eval fallback
        # here would rebuild ``catch`` as a space-joined script,
        # losing the body's word boundaries when the first body word
        # starts with ``$``.
        if cmd_name == "catch" and cmd_args:
            self._emit_catch_from_args(tuple(cmd_args), defs=(), keep_on_stack=True)
            self._emit_unbox_int()
            return

        # ``[set varname]`` — 1-arg read form in expression context
        # (e.g. ``if {[set a(x)] == 1} ...``).  Mirror the value-
        # context special-case: route through ``_emit_var_read_obj``
        # and unbox to i64.  See comment in
        # ``_emit_command_subst_value``.
        if cmd_name == "set" and len(cmd_args) == 1:
            self._emit_var_read_obj(cmd_args[0])
            self._emit_unbox_int()
            return
        # ``[set varname value]`` — 2-arg write form in expression
        # context (e.g. ``[set scriptCompare [catch {...} var]] == 0``).
        # Evaluate the value, store it keeping it on the stack, then
        # unbox to i64 for the surrounding expression comparison.
        if cmd_name == "set" and len(cmd_args) == 2:
            self._emit_value(cmd_args[1])
            self._emit_var_write_obj_keep(cmd_args[0])
            self._emit_unbox_int()
            return

        # ``[namespace origin <name>]`` — compile-time qualified name;
        # mirrors the value-context handler.
        if cmd_name == "namespace" and len(cmd_args) >= 2 and cmd_args[0] == "origin":
            bare_name = cmd_args[1]
            qualified = self._resolve_proc_qname(bare_name) or bare_name
            if not qualified.startswith("::"):
                ns = self._proc_namespace or "::"
                qualified = f"{ns}::{bare_name}" if ns != "::" else f"::{bare_name}"
            self._emit_obj_literal(qualified)
            self._emit_unbox_int()
            return

        # Handle proc calls — resolve command to a known procedure
        proc_info = self._resolve_proc(cmd_name)
        if proc_info is not None:
            func_idx, n_params = proc_info
            qname = self._resolve_proc_qname(cmd_name)
            defaults = self._proc_defaults.get(qname, ()) if qname else ()
            for i in range(min(n_params, len(cmd_args))):
                self._emit_value(cmd_args[i])
            # Pad missing args with declared defaults (see the mirror
            # comment in ``_emit_command_subst_value``).
            for slot in range(len(cmd_args), n_params):
                default = defaults[slot] if slot < len(defaults) else None
                if default is None:
                    self._emit_default_arg()
                else:
                    self._emit_obj_literal(default)
            self._emit_call(func_idx)
            # Result is i32 TclObj — unbox to i64 for expression context
            self._emit_unbox_int()
            return

        # dict sub-command — returns i32 TclObj, unbox to i64
        if cmd_name == "dict" and cmd_args:
            subcmd = cmd_args[0]
            import_key = _DICT_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = cmd_args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if spec[3]:
                    self._emit_unbox_int()
                else:
                    self._emit_i64_const(0)
                return

        # string sub-command — returns i32 TclObj, unbox to i64
        if cmd_name == "string" and cmd_args:
            subcmd = cmd_args[0]
            import_key = _STRING_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = cmd_args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if spec[3]:
                    self._emit_unbox_int()
                else:
                    self._emit_i64_const(0)
                return

        # info sub-command — returns i32 TclObj, unbox to i64 for exprs.
        if cmd_name == "info" and cmd_args:
            self._emit_info_value(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # lassign in expression context — returns leftover list (i32 TclObj).
        if cmd_name == "lassign" and cmd_args:
            self._emit_cmd_lassign(tuple(cmd_args), defs=(), keep_on_stack=True)
            self._emit_unbox_int()
            return

        # clock in expression context — integer timer value.
        if cmd_name == "clock" and cmd_args:
            self._emit_clock_value(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # array subcommand in expression context.
        if cmd_name == "array" and cmd_args:
            self._emit_array_subcmd_value(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # uplevel in expression context.
        if cmd_name == "uplevel" and cmd_args:
            self._emit_cmd_uplevel(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # Runtime command — returns i32 TclObj, unbox to i64
        if cmd_name in _CMD_RUNTIME:
            import_key, _ = _CMD_RUNTIME[cmd_name]
            func_idx = self._shared_imports.get(import_key)
            if func_idx is not None:
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                for i in range(min(param_count, len(cmd_args))):
                    self._emit_value(cmd_args[i])
                for _ in range(param_count - len(cmd_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if spec[3]:
                    self._emit_unbox_int()
                else:
                    self._emit_i64_const(0)
                return

        # Unknown command in expression context — fall back to interpreter,
        # unbox result to i64 for expression arithmetic.  Use the original
        # command text so braced words are not mangled during reconstruction.
        self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)
        self._emit_unbox_int()

    def _split_command_subst(self, text: str) -> list[str]:
        """Split a command substitution body into command name + args.

        Handles braced arguments ``{...}`` and nested brackets ``[...]``
        as atomic units.  Braces and quotes are stripped from the
        resulting word so downstream emission sees the raw content
        (e.g. ``{5}`` becomes ``5``).
        """
        parts: list[str] = []
        i = 0
        n = len(text)
        while i < n:
            # Skip whitespace
            while i < n and text[i] in " \t\n":
                i += 1
            if i >= n:
                break
            # Braced argument — keep outer braces so _emit_value can
            # recognise {…} words as literals (no substitution in Tcl).
            if text[i] == "{":
                depth = 1
                start = i
                i += 1
                while i < n and depth > 0:
                    # Per Tcl spec: \{ and \} inside a braced word are
                    # backslash-escaped and do NOT count for brace depth.
                    if text[i] == "\\" and i + 1 < n and text[i + 1] in "{}":
                        i += 2  # skip backslash + brace
                        continue
                    if text[i] == "{":
                        depth += 1
                    elif text[i] == "}":
                        depth -= 1
                    i += 1
                # Keep outer { } intact; _emit_value strips them.
                parts.append(text[start:i])
            # Bracketed argument (nested command) — keep brackets
            elif text[i] == "[":
                depth = 1
                start = i
                i += 1
                while i < n and depth > 0:
                    if text[i] == "[":
                        depth += 1
                    elif text[i] == "]":
                        depth -= 1
                    i += 1
                parts.append(text[start:i])
            # Quoted argument — strip outer quotes
            elif text[i] == '"':
                start = i
                i += 1
                while i < n and text[i] != '"':
                    if text[i] == "\\":
                        i += 1  # skip escaped char
                    i += 1
                i += 1  # skip closing quote
                # Strip outer " "
                parts.append(text[start + 1 : i - 1])
            # Bare word (delimited by whitespace)
            else:
                start = i
                while i < n and text[i] not in " \t\n":
                    # Handle nested brackets within bare words
                    if text[i] == "[":
                        depth = 1
                        i += 1
                        while i < n and depth > 0:
                            if text[i] == "[":
                                depth += 1
                            elif text[i] == "]":
                                depth -= 1
                            i += 1
                    else:
                        i += 1
                parts.append(text[start:i])
        return parts

    def _emit_literal(self, value: str) -> None:
        """Emit a literal as a raw i64 constant (for expression context)."""
        try:
            int_val = int(value)
            self._emit_i64_const(int_val)
        except ValueError:
            try:
                float_val = float(value)
                self._emit_i64_const(int(float_val))
            except ValueError:
                # String literal in expr context — no meaningful integer value
                self._emit_i64_const(0)

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

        # Ordering comparisons: use tcl_expr_order_cmp which tries
        # numeric first then falls back to string comparison.
        #
        # Tcl 9 semantics: ``"a" < "b"`` → 1 (lexicographic).
        # Tcl 8.x semantics: same expression raises ``expected floating-
        # point number but got "a"``.  The WASM runtime currently
        # targets Tcl 9.0 only — no dialect gating is threaded through
        # the emitter.  If 8.x compatibility is needed in future,
        # switch this branch based on ``self._target_dialect`` (not
        # yet plumbed) so the emitter falls through to the plain
        # ``I64_LT_S``/... path that errors on non-numeric operands.
        if op in (BinOp.LT, BinOp.GT, BinOp.LE, BinOp.GE):
            self._emit_expr_order_cmp(op, left, right)
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
        elif op in (BinOp.STR_EQ, BinOp.STR_EQUALS):
            self._emit_str_eq(left, right)
        elif op == BinOp.STR_NE:
            self._emit_str_eq(left, right)
            # Negate: result XOR 1
            self._emit_i64_const(1)
            self._emit(WasmOp.I64_XOR)
        elif op in (BinOp.STR_LT, BinOp.STR_GT, BinOp.STR_LE, BinOp.STR_GE):
            self._emit_str_cmp(op, left, right)
        else:
            # Unsupported binary op — emit operands and drop
            self._emit_expr(left)
            self._emit_expr(right)
            self._emit(WasmOp.I64_ADD)  # placeholder

    def _emit_str_eq(self, left: ExprNode, right: ExprNode) -> None:
        """Emit string equality comparison — result is i64 (0 or 1).

        Uses the ``string_equal`` runtime import.  Both operands are
        emitted as i32 TclObj values, the runtime returns an i32 TclObj
        wrapping 0 or 1, which is then unboxed to i64.
        """
        seq_idx = self._shared_imports.get("tcl_string_equal")
        if seq_idx is not None:
            # Emit left and right as i32 TclObj values
            self._emit_str_value(left)
            self._emit_str_value(right)
            self._emit_call(seq_idx)
            # string_equal returns TclObj wrapping 0/1 — unbox to i64
            self._emit_unbox_int()
        else:
            # Fallback: integer comparison
            self._emit_expr(left)
            self._emit_expr(right)
            self._emit(WasmOp.I64_EQ)
            self._emit(WasmOp.I64_EXTEND_I32_S)

    def _emit_str_cmp(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit a string ordering comparison — result is i64 (0 or 1).

        Calls ``string_compare(a, b)`` which returns a TclObj wrapping
        -1, 0, or 1, unboxes to i64, then compares with 0.
        """
        cmp_idx = self._shared_imports.get("tcl_string_compare")
        if cmp_idx is None:
            self._emit_i64_const(0)
            return
        self._emit_str_value(left)
        self._emit_str_value(right)
        self._emit_call(cmp_idx)
        self._emit_unbox_int()  # i64: -1, 0, or 1
        self._emit_i64_const(0)
        match op:
            case BinOp.STR_LT:
                self._emit(WasmOp.I64_LT_S)
            case BinOp.STR_GT:
                self._emit(WasmOp.I64_GT_S)
            case BinOp.STR_LE:
                self._emit(WasmOp.I64_LE_S)
            case BinOp.STR_GE:
                self._emit(WasmOp.I64_GE_S)
        self._emit(WasmOp.I64_EXTEND_I32_S)

    def _emit_expr_order_cmp(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit an ordering comparison using tcl_expr_order_cmp.

        Tries numeric comparison first; falls back to string comparison when
        either operand is non-numeric (Tcl 9 semantics for < > <= >=).
        """
        cmp_idx = self._shared_imports.get("tcl_expr_order_cmp")
        if cmp_idx is None:
            # Fallback: plain integer comparison
            self._emit_expr(left)
            self._emit_expr(right)
            wasm_op = _BINOP_WASM.get(op)
            if wasm_op is not None:
                self._emit(wasm_op)
                self._emit(WasmOp.I64_EXTEND_I32_S)
            else:
                self._emit_i64_const(0)
            return
        self._emit_str_value(left)
        self._emit_str_value(right)
        self._emit_call(cmp_idx)
        self._emit_unbox_int()  # i64: -1, 0, or 1
        self._emit_i64_const(0)
        match op:
            case BinOp.LT:
                self._emit(WasmOp.I64_LT_S)
            case BinOp.GT:
                self._emit(WasmOp.I64_GT_S)
            case BinOp.LE:
                self._emit(WasmOp.I64_LE_S)
            case BinOp.GE:
                self._emit(WasmOp.I64_GE_S)
        self._emit(WasmOp.I64_EXTEND_I32_S)

    def _emit_str_value(self, node: ExprNode) -> None:
        """Emit an expression as an i32 TclObj pointer (for string ops).

        For ExprVar and ExprRaw, emits as a variable read.
        For ExprLiteral, creates a boxed string/int.
        For other nodes, evaluates as i64 and boxes.
        """
        from ..expr_ast import ExprCommand, ExprLiteral, ExprRaw, ExprString, ExprVar

        match node:
            case ExprVar(text=text):
                # Array ref in value context: ``$a(key)`` — strip the
                # leading ``$`` / ``${`` / trailing ``}`` and route
                # through the array-aware _emit_var_read_obj path.
                if "(" in text and text.endswith(")"):
                    bare = text[1:] if text.startswith("$") else text
                    if bare.startswith("{") and bare.endswith("}"):
                        bare = bare[1:-1]
                    self._emit_var_read_obj(bare)
                    return
                var = self._resolve_var_name(text)
                if var is not None:
                    self._emit_var_read_obj(var)
                    return
            case ExprCommand(text=text):
                # Command substitution ``[cmd ...]`` — evaluate via
                # _emit_value to get a TclObj string pointer directly.
                # Going through the i64 fallback path loses the string
                # representation (empty string → int 0 → "0").
                self._emit_value(text)
                return
            case ExprRaw(text=text):
                # ``ExprRaw`` carries the literal source text of a
                # subject / operand that the expression parser
                # couldn't classify further — commonly the subject
                # of a ``switch`` whose CFG builder wraps it as
                # ``ExprBinary(STR_EQ, ExprRaw(subject), ...)``.
                # Route through ``_emit_value`` so ``[cmd …]``
                # command substitutions and ``"text $var"``
                # interpolated strings get evaluated, not emitted
                # as the raw literal source.  The old path only
                # handled bare ``$var`` references via
                # ``_resolve_var_name``, so ``switch -- [llength
                # $m] {…}`` compared ``"[llength $m]"`` (the
                # literal string) against each arm's pattern and
                # always fell through to the ``default`` branch.
                self._emit_value(text)
                return
            case ExprLiteral(text=text):
                self._emit_obj_literal(text)
                return
            case ExprString(text=text):
                # Quoted string literal — strip outer quotes/braces and
                # apply backslash substitution for double-quoted strings.
                inner = text
                if len(inner) >= 2:
                    if inner[0] == '"' and inner[-1] == '"':
                        inner = inner[1:-1]
                        if "\\" in inner:
                            inner = _tcl_backslash_subst(inner)
                    elif inner[0] == "{" and inner[-1] == "}":
                        inner = inner[1:-1]
                self._emit_obj_literal(inner)
                return
        # Fallback: evaluate expr to i64 and box
        self._emit_expr(node)
        self._emit_box_int()

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
            case ExprLiteral(text=value):
                try:
                    return int(value)
                except ValueError:
                    return None
            case ExprVar(name=name, text=text):
                # Array refs and aliased vars have time-varying values — don't
                # treat them as constants even if _const_map was populated
                # before the alias was established.
                if "(" in text:
                    return None
                if name in self._aliases:
                    return None
                return self._const_map.get(name)
            case _:
                return None

    # -- Statement emission --

    def set_compiled_proc_registrations(self, ir_module: IRModule) -> None:
        """Queue a ``tcl_proc_register_compiled`` call per compiled proc.

        Emitted at the top of ``generate()`` (before normal ::top
        code runs) so the runtime's proc table knows every compiled
        proc by name, marked with ``func_idx=1`` so
        ``eval_proc_call`` takes the host-bridge path instead of
        looking for a Tcl source body.  The bridge
        (``call_compiled_proc`` in the host embedder) then
        dispatches by proc name to the compiled WASM function.

        ``func_idx`` is used as a marker here, not an actual index
        — the host looks procs up by *name*.  Any non-zero value
        works; we pick 1 for clarity.

        ``has_args_tail`` — 1 when the last declared parameter is
        named ``args`` (Tcl's variadic marker).  The runtime's
        host-bridge dispatcher reads this flag to pack excess
        call arguments into a list TclObj so the compiled
        function's fixed-arity signature always sees the right
        number of parameters.  Without this bit, a
        ``proc foo {args} {}`` compiled to a single-i32-param
        function traps with ``too many parameters provided: given
        N, expected 1`` on any N ≥ 2 invocation (e.g. tcltest's
        ``useLocal counter.tcl counter`` bundle-runner stub).
        """
        self._compiled_proc_queue = []
        reg_idx = self._shared_imports.get("tcl_proc_register_compiled")
        if reg_idx is None:
            return
        for qname, proc in ir_module.procedures.items():
            if proc.body_source is None:
                continue
            has_args_tail = bool(proc.params and proc.params[-1] == "args")
            self._compiled_proc_queue.append((qname, len(proc.params), has_args_tail))

    def _record_stmt_context(self, stmt: IRStatement) -> None:
        """Update the 'current statement' tracking the diag machinery reads.

        Called at the top of ``_emit_stmt`` and — importantly — from
        ``_emit_block``'s implicit-return tail path which invokes
        ``_emit_call_stmt_tail`` directly without going through
        ``_emit_stmt``.  Every IR statement carries a ``range`` (see
        core/compiler/ir.py); IRCall additionally carries ``command``
        and ``args``.
        """
        rng = getattr(stmt, "range", None)
        if rng is not None:
            self._current_range = rng
        cmd = getattr(stmt, "command", None)
        self._current_command = cmd if isinstance(cmd, str) else ""
        raw_args = getattr(stmt, "args", ())
        self._current_args = tuple(raw_args) if isinstance(raw_args, tuple | list) else ()

    def _emit_stmt(self, stmt: IRStatement) -> None:
        """Emit WASM for a single IR statement."""
        # Record this statement's source location so any nested trap
        # emission (eval fallback, unsupported-command trap) can stamp
        # a sidecar diag site with a meaningful file:line:col.
        self._record_stmt_context(stmt)
        match stmt:
            case IRAssignConst(name=name, value=value):
                self._emit_obj_literal(value)
                self._emit_var_write_obj(name)
                if self._optimise and name not in self._aliases:
                    # Aliased writes go to a global under a possibly-dynamic
                    # name — constant-tracking the local Tcl name would
                    # short-circuit reads that ought to see cross-proc
                    # updates to the global.
                    try:
                        self._const_map[name] = int(value)
                    except ValueError:
                        self._const_map.pop(name, None)

            case IRAssignExpr(name=name, expr=expr):
                self._emit_expr(expr)
                self._emit_box_int()
                self._emit_var_write_obj(name)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRAssignValue(name=name, value=value):
                # value_needs_backsubst is already handled by _emit_value's
                # general backslash substitution path for non-braced literals.
                self._emit_value(value)
                self._emit_var_write_obj(name)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRIncr(name=name, amount=amount):
                self._emit_var_read_obj(name)
                self._emit_unbox_int()
                amt = 1
                if amount is not None:
                    try:
                        amt = int(amount)
                    except ValueError:
                        # Variable increment — unbox the amount variable
                        self._emit_value(amount)
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        self._emit_box_int()
                        self._emit_var_write_obj(name)
                        if self._optimise:
                            self._const_map.pop(name, None)
                        return
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                self._emit_var_write_obj(name)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRExprEval(expr=expr):
                self._emit_expr(expr)
                self._emit(WasmOp.DROP)

            case IRCall(command=command, args=args, defs=defs, tokens=tokens):
                if (
                    tokens is not None
                    and tokens.expand_word is not None
                    and any(tokens.expand_word)
                    and tokens.argv_texts
                ):
                    # ``{*}`` argument expansion — reconstruct the
                    # original command text with ``{*}`` prefixes in
                    # front of each expanded word so the runtime can
                    # handle the expansion.  Using argv_texts directly
                    # without the prefix would miss the expansion.
                    ew = tokens.expand_word
                    parts = [
                        (f"{{*}}{t}" if (i < len(ew) and ew[i]) else t)
                        for i, t in enumerate(tokens.argv_texts)
                    ]
                    script = " ".join(parts)
                    self._emit_eval_fallback(command, args, script_override=script)
                    self._emit(WasmOp.DROP)
                    if self._optimise:
                        self._const_map.clear()
                else:
                    self._emit_call_stmt(command, args, defs, tokens=tokens)

            case IRReturn(value=value, expr=expr):
                if expr is not None:
                    self._emit_expr(expr)
                    self._emit_box_int()
                elif value is not None:
                    self._emit_value(value)
                else:
                    self._emit_i32_const(0)
                self._emit(WasmOp.RETURN)

            case IRBarrier(command=barrier_cmd, args=barrier_args, reason=reason):
                # Barriers are dynamic commands (eval, uplevel, trace,
                # etc.) that defeat static *analysis* but may still
                # have concrete runtime implementations we can call
                # directly.  Dispatch in priority order:
                #   1. ``uplevel`` has a dedicated emitter that shifts
                #      the frame-depth around a tcl_eval call so the
                #      body runs at a caller's scope.
                #   2. Commands in ``_CMD_RUNTIME`` have real (or stub-
                #      trapping) runtime fns — dispatch through that
                #      table so ``trace add variable …`` becomes a
                #      direct ``tcl_trace`` call with precise diag
                #      attribution instead of a generic tcl_eval
                #      fallback that would lose arity info.
                #   3. Anything else really is a black-box barrier:
                #      fall back to the interpreter.
                if barrier_cmd == "uplevel" and barrier_args:
                    self._emit_cmd_uplevel(barrier_args)
                    self._emit(WasmOp.DROP)
                elif (
                    barrier_cmd == "return"
                    and barrier_args
                    and len(barrier_args) == 3
                    and barrier_args[0] == "-code"
                    and barrier_args[1] == "error"
                ):
                    # ``return -code error <msg>`` — evaluate the
                    # message value inline so embedded ``$var`` /
                    # ``[cmd]`` substitutions resolve against the
                    # current frame, then call ``tcl_cmd_error``.
                    # Going through the eval fallback would
                    # brace-wrap the message and block the
                    # substitutions the error text needs (tcltest's
                    # error strings embed ``$option``/``$values``
                    # everywhere).  Other ``return -code …`` forms
                    # (dynamic code, break/continue, numeric) reach
                    # the fallback below where quoting is safe.
                    self._emit_cmd_return(barrier_args)
                elif barrier_cmd and barrier_cmd in _CMD_RUNTIME:
                    self._emit_cmd_runtime(barrier_cmd, barrier_args, ())
                elif barrier_cmd:
                    self._emit_eval_fallback(barrier_cmd, barrier_args)
                    self._emit(WasmOp.DROP)
                else:
                    self._emit_eval_fallback(reason)
                    self._emit(WasmOp.DROP)
                if self._optimise:
                    self._const_map.clear()

            case IRBlock(body=body, namespace=block_ns):
                # ``namespace eval`` body inlined into the enclosing
                # script.  Procs inside were already lifted to
                # ``module.procedures`` with qualified names; the
                # remaining statements run as plain code.  We stash
                # the block's namespace so ``_emit_call_stmt`` can
                # resolve unqualified command names (``Option …`` →
                # ``::tcltest::Option``) for the duration of the body.
                prev_ns = self._block_namespace
                self._block_namespace = block_ns
                try:
                    for stmt in body.statements:
                        self._emit_stmt(stmt)
                finally:
                    self._block_namespace = prev_ns
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

            case IRSwitch(subject=subject, arms=arms, default_body=default_body, mode=mode):
                self._emit_switch(subject, arms, default_body, mode=mode)

            case IRCatch(body=body, result_var=result_var):
                self._emit_catch(body, result_var)

            case IRTry(body=body, handlers=handlers, finally_body=finally_body):
                self._emit_try(body, handlers, finally_body)

    def _resolve_proc_qname(self, command: str) -> str | None:
        """Resolve ``command`` to the qualified proc name if it matches."""
        if command.startswith("::"):
            return command if command in self._proc_index else None
        if self._block_namespace and self._block_namespace != "::":
            qn = f"{self._block_namespace}::{command}"
            if qn in self._proc_index:
                return qn
        if self._proc_namespace and self._proc_namespace != "::":
            qn = f"{self._proc_namespace}::{command}"
            if qn in self._proc_index:
                return qn
        if f"::{command}" in self._proc_index:
            return f"::{command}"
        if command in self._proc_index:
            return command
        return None

    def _resolve_proc(self, command: str) -> tuple[int, int] | None:
        """Look up a user-defined proc by name, returning (func_idx, n_params) or None.

        Resolution order for an *unqualified* name (Tcl namespace-path
        semantics, simplified):
          1. ``<active block namespace>::<name>`` — inside a
             ``namespace eval ::ns { … }`` body.
          2. ``<enclosing proc namespace>::<name>`` — when compiling
             ``::tcltest::workingDirectory``'s body, a bare call to
             ``AcceptAbsolutePath`` must first try
             ``::tcltest::AcceptAbsolutePath``.
          3. ``::<name>`` — the global namespace.
          4. Bare ``<name>`` — defensive fallback for non-standard
             proc-index entries.
        """
        if command.startswith("::"):
            return self._proc_index.get(command)
        # Inside a namespace-eval block, try the block's namespace first.
        if self._block_namespace and self._block_namespace != "::":
            ns_qname = f"{self._block_namespace}::{command}"
            hit = self._proc_index.get(ns_qname)
            if hit is not None:
                return hit
        # Try the enclosing proc's own namespace — captured in
        # ``_proc_namespace`` at emitter construction time.
        if self._proc_namespace and self._proc_namespace != "::":
            ns_qname = f"{self._proc_namespace}::{command}"
            hit = self._proc_index.get(ns_qname)
            if hit is not None:
                return hit
        return self._proc_index.get(f"::{command}") or self._proc_index.get(command)

    def _emit_call_stmt(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...] = (),
        tokens: "CommandTokens | None" = None,
    ) -> None:
        """Emit a command invocation.

        Dispatches known commands to inline WASM or imported runtime
        functions.  Unknown commands emit NOP.

        Proc calls are resolved first so that a user-defined proc
        named ``puts`` shadows the built-in runtime import, matching
        Tcl's command resolution semantics.

        *tokens* — original parsed tokens for the command invocation.
        Threaded through to user-proc calls so they can distinguish
        braced (``{…}``) args from unbraced ones (braced args must
        skip backslash / interpolation substitution, per Tcl
        semantics).
        """
        # <upvar-invalidate> — synthetic IRCall emitted by the CFG builder
        # to invalidate caller-side SSA defs around a call that modifies
        # variables via upvar.  No code to emit at the WASM level.
        if command == "<upvar-invalidate>":
            return

        # <cond> — synthetic IRCall emitted by the CFG builder in front
        # of an ``if`` dispatch whose condition contains a command
        # substitution that defines variables (``[catch { ... } result]``
        # is the canonical example).  The actual condition evaluation
        # happens via the block's branch terminator — this placeholder
        # only exists to carry the ``defs`` list for SSA reasoning, and
        # emits no code.
        if command == "<cond>":
            return

        # global varName — register variable as global-scoped
        if command == "global":
            for var_name in args:
                self._globals.add(var_name)
                # Pre-load the global value into the local
                gget_idx = self._shared_imports.get("tcl_global_get")
                if gget_idx is not None:
                    local_idx = self._intern_local(var_name)
                    self._emit_obj_literal(var_name)
                    self._emit_call(gget_idx)
                    self._emit_local_set(local_idx)
            return

        # upvar ?level? otherVar myVar ?otherVar myVar ...? — register
        # a local alias so subsequent reads/writes route through the
        # target scope.  Only ``#0`` (global alias) is currently supported.
        if command == "upvar":
            self._emit_cmd_upvar(args)
            return

        # variable name ?value? ?name value ...? — in a namespace proc,
        # aliases local ``name`` to ``::ns::name`` in the enclosing
        # namespace and optionally initialises it.
        if command == "variable":
            self._emit_cmd_variable(args)
            return

        # Scope declarations are NOPs in the WASM model — EXCEPT when
        # the proc / namespace command has a dynamic name that must be
        # resolved at runtime (tcltest's ``Option`` does ``proc
        # $varName body`` to install accessor procs, and the compile-
        # time registry never learns about them).  Route those through
        # the eval fallback so the interpreter's ``proc`` handler
        # registers under the current namespace.
        if command in _SCOPE_NOP_COMMANDS:
            if command == "proc" and args and (args[0].startswith("$") or args[0].startswith("[")):
                self._emit_eval_fallback(command, args)
                self._emit(WasmOp.DROP)
                return
            # ``namespace eval ns arg1 arg2 ...`` in statement context
            # with dynamic script args: build the script at WASM level
            # (so compiled-frame aliases like $arr($key) are resolved
            # correctly) and call tcl_eval for side effects.
            if command == "namespace" and args and args[0] == "eval" and len(args) > 2:
                # Bridge drops the result since we're in statement context.
                # If imports are missing the bridge returns False and we
                # silently skip (statement context has no stack commitments).
                self._emit_namespace_eval_bridge(args[2:], drop_result=True)
                return
            return

        # break/continue — emit WASM br to exit/restart the enclosing loop.
        # Loop structure: block{ loop{ block{ <body> }; <next>; br 0 } }
        # _loop_ctrl_depths records ctrl_depth at the inner (continue) block.
        # From inside the body at ctrl_depth D, with loop_ctrl C:
        #   continue: br(D - C) exits the continue block → runs <next>
        #   break:    br(D - C + 2) exits continue block + loop + outer block
        if command == "break" and self._loop_ctrl_depths:
            loop_ctrl = self._loop_ctrl_depths[-1]
            br_depth = self._ctrl_depth - loop_ctrl + 2
            self._emit_br(br_depth)
            return
        if command == "continue" and self._loop_ctrl_depths:
            loop_ctrl = self._loop_ctrl_depths[-1]
            br_depth = self._ctrl_depth - loop_ctrl
            self._emit_br(br_depth)
            return

        # catch {body} ?resultVar? — re-parse body text and emit with
        # error-flag semantics.  The CFG builder converts IRCatch into
        # IRCall("catch", (body_text, ...)) with defs listing modified vars.
        if command == "catch" and args:
            self._emit_catch_from_args(args, defs)
            return

        # User-defined proc call takes priority over built-ins
        proc_info = self._resolve_proc(command)
        if proc_info is not None:
            qname = self._resolve_proc_qname(command)
            self._emit_cmd_proc_call(
                proc_info[0],
                proc_info[1],
                args,
                defs,
                qname=qname,
                tokens=tokens,
            )
            return

        # set/incr: only inline the canonical 1-or-2 arg forms that the
        # lowering emits as IRCall fallbacks.  Non-canonical shapes
        # ({*} expansion, dict set, wrong arity) fall through to NOP.
        if command == "set" and 1 <= len(args) <= 2:
            self._emit_cmd_set(args)
            return

        if command == "incr" and 1 <= len(args) <= 2:
            self._emit_cmd_incr(args)
            return

        # return: emit WASM return
        if command == "return":
            self._emit_cmd_return(args)
            return

        # string sub-commands
        if command == "string" and args:
            self._emit_cmd_string(args, defs)
            return

        # dict sub-commands
        if command == "dict" and args:
            self._emit_cmd_dict(args, defs)
            return

        # info sub-commands
        if command == "info" and args:
            self._emit_cmd_info(args, defs)
            return

        # lassign list ?varName ...? — destructure a list.
        if command == "lassign" and args:
            self._emit_cmd_lassign(args, defs, keep_on_stack=False)
            return

        # clock seconds / clicks / milliseconds — WASI-backed timer.
        if command == "clock" and args:
            self._emit_clock_value(args)
            if defs:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            else:
                self._emit(WasmOp.DROP)
            return

        # uplevel ?level? body — run script in a caller's scope.
        if command == "uplevel" and args:
            self._emit_cmd_uplevel(args)
            if defs:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            else:
                self._emit(WasmOp.DROP)
            return

        # array subcommand dispatch — reads/writes route through the
        # dedicated array hash tables.
        if command == "array" and args:
            self._emit_array_subcmd_value(args)
            if defs:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            else:
                self._emit(WasmOp.DROP)
            return

        # unset — handle array elements specially so ``unset arr(key)``
        # actually removes the element from the array storage.  Plain
        # scalar unset is fine to fall through for now (compiled procs
        # don't need to clear WASM locals).
        if command == "unset" and args:
            if self._emit_unset_array_elems(args):
                return

        # ``list`` — variadic list builder.  Bypass _CMD_RUNTIME's
        # 2-arg tcl_list (which would silently drop the remaining
        # elements) and emit a flat space-joined string instead.
        if command == "list":
            self._emit_list_value(args)
            self._emit(WasmOp.DROP)
            return

        # Commands backed by runtime imports
        if command in _CMD_RUNTIME:
            self._emit_cmd_runtime(command, args, defs)
            return

        # Unsupported commands — emit runtime error trap
        if command in _UNSUPPORTED_COMMANDS:
            self._emit_unsupported_trap(command)
            return

        # Unknown command — fall back to interpreter
        self._emit_eval_fallback(command, args, tokens=tokens)
        self._emit(WasmOp.DROP)  # statement context — discard result

    def _emit_diag_site(
        self,
        command: str,
        *,
        args: tuple[str, ...] = (),
        kind: str = "fallback",
    ) -> None:
        """Register a diagnostic site and emit a ``tcl_diag_set`` call.

        The site ID is a monotonic counter scoped to the module's
        :class:`DiagMap`; ID 0 is reserved for "unset", so the first
        emitted site is ID 1.  The site stores the current statement's
        source range (populated by ``_emit_stmt``) and the supplied
        command + kind so a sidecar resolver can print a useful
        location on trap.

        When no ``diag_map`` is attached to this emitter (standalone
        tests that don't care about source mapping) the call is a
        no-op — the ``tcl_diag_set`` import is still registered, but
        we simply don't emit a call for it.
        """
        if self._diag_map is None:
            return
        diag_idx = self._shared_imports.get("tcl_diag_set")
        if diag_idx is None:
            return
        rng = self._current_range
        if rng is None:
            # Happens when a trap is emitted outside a statement context
            # (rare — e.g. the upvar preamble pre-scan).  Skip rather
            # than pointing at an unrelated site.
            return
        site_id = len(self._diag_map.sites) + 1
        site = DiagSite(
            id=site_id,
            file=self._diag_map.filename,
            line=rng.start.line + 1,  # IR is 0-based; sidecar is 1-based
            col=rng.start.character + 1,
            end_line=rng.end.line + 1,
            end_col=rng.end.character + 1,
            command=command,
            args=args,
            kind=kind,
            proc=self._proc_qname,
        )
        self._diag_map.add_site(site)
        self._emit_i32_const(site_id)
        self._emit_call(diag_idx)

    def _emit_unsupported_trap(self, command: str) -> None:
        """Emit hard trap for commands that cannot work in WASM at all.

        Used for exec, socket, coroutine, etc. — these can't be
        handled by the interpreter either.  Inside catch, error()
        sets the flag without trapping.
        """
        self._emit_diag_site(command, kind="unsupported")
        fidx = self._shared_imports.get("tcl_error")
        if fidx is not None:
            msg = f"unsupported in WASM: {command}"
            self._emit_obj_literal(msg)
            self._emit_call(fidx)
        if self._catch_depth == 0:
            self._emit(WasmOp.UNREACHABLE)

    def _emit_eval_fallback(
        self,
        command: str,
        args: tuple[str, ...] | list[str] = (),
        *,
        script_override: str | None = None,
        tokens: "CommandTokens | None" = None,
    ) -> None:
        """Fall back to the Zig interpreter for an uncompiled command.

        Builds a command string from *command* and *args* and calls
        ``tcl_eval(script)``.  The result (i32 TclObj) is left on
        the WASM stack — caller must drop or use it as needed.

        When *script_override* is given it is used verbatim as the
        eval script instead of reconstructing it from *command* / *args*.
        Use this when the original source text must be preserved (e.g.
        for ``{*}`` argument expansion whose syntax cannot be round-tripped
        through the normal quoting path).

        *tokens* carries the original parsed word tokens when available.
        Used to distinguish braced (``{…}``, STR) arguments — whose IR
        value is the literal content — from plain / double-quoted
        (ESC) arguments whose IR value still contains raw backslash
        sequences that must be substituted before list-quoting.
        Without this distinction ``"a\\\\{b"`` (source → IR value
        ``a\\{b``) would round-trip through list-quote as though the
        two backslashes were the *final* value, producing ``a\\{b``
        instead of ``a\\{b`` after the interpreter re-parses the word.
        """
        self._emit_diag_site(command, args=tuple(args), kind="fallback")
        eval_idx = self._shared_imports.get("tcl_eval")
        if eval_idx is not None:
            if script_override is not None:
                script = script_override
            else:
                # Build command string: "command arg1 arg2 ..."
                # For literal args, concatenate them. For $var refs,
                # include the dollar sign so the interpreter can resolve them.
                def _arg_was_braced(i: int) -> bool:
                    """Return True if call-site arg *i* came from a ``{…}`` token.

                    *i* is 0-based into *args*, so ``tokens.argv[i + 1]``
                    is the corresponding parsed word (argv[0] is the
                    command name).  Requires a single-token word —
                    a concatenated word like ``{a}b`` is not purely
                    braced and needs normal processing.
                    """
                    if tokens is None or tokens.argv is None:
                        return False
                    tok_idx = i + 1
                    if tok_idx >= len(tokens.argv):
                        return False
                    if tokens.single_token_word is not None:
                        if tok_idx >= len(tokens.single_token_word):
                            return False
                        if not tokens.single_token_word[tok_idx]:
                            return False
                    return tokens.argv[tok_idx].type == TokenType.STR

                parts = [command]
                for i, a in enumerate(args):
                    if a.startswith("$") or a.startswith("["):
                        # Substitution words pass through unquoted so
                        # the interpreter can resolve them at eval time.
                        parts.append(a)
                    elif _arg_was_braced(i):
                        # Braced token — IR holds the literal content
                        # with outer ``{}`` stripped.  Re-wrap in braces
                        # so the interpreter sees the exact same word
                        # without applying any substitution.  Fall back
                        # to list-quote when the value contains an
                        # unbalanced brace (rare — ``{a{b}`` style).
                        if "{" in a or "}" in a:
                            # Use list-quote to get balanced/escaped form.
                            # Args are never at command-start so a leading
                            # ``#`` does not need quoting.
                            parts.append(_tcl_list_quote(a, first=False))
                        else:
                            parts.append("{" + a + "}")
                    else:
                        # Literal IR value from an ESC token (plain or
                        # double-quoted word).  The IR stores the RAW
                        # text: source-level ``\\{`` is still two bytes
                        # ``\`` + ``{``.  Apply backslash substitution
                        # so the value we embed in the script reflects
                        # what the original word would have evaluated
                        # to.  _tcl_list_quote then encodes it as a
                        # safe Tcl word that round-trips through the
                        # interpreter's word parser.
                        #
                        # Guard: if compile-time substitution would
                        # produce a Python string with isolated
                        # surrogates (``\uD83D\uDE02`` in a Tcl 9 test)
                        # we can't safely UTF-8-encode it into the WASM
                        # data segment.  Fall back to embedding the raw
                        # value verbatim so the interpreter itself
                        # handles the escape sequences at eval time.
                        prepped: str | None = None
                        if "\\" in a:
                            candidate = _tcl_backslash_subst(a)
                            try:
                                candidate.encode("utf-8")
                            except UnicodeEncodeError:
                                prepped = None  # signal: use raw path
                            else:
                                prepped = candidate
                        else:
                            prepped = a
                        # Script args never sit at command-start — pass
                        # ``first=False`` so a leading ``#`` is left alone.
                        if prepped is None:
                            parts.append(_tcl_list_quote(a, first=False))
                        else:
                            parts.append(_tcl_list_quote(prepped, first=False))
                script = " ".join(parts)
            # Sync all live proc-locals into the frame so the
            # interpreter can see them via var_resolve.  We sync
            # conservatively (every local, not just ``$name`` refs
            # in the script) because Tcl commands like ``info exists
            # x`` / ``unset x`` / ``upvar 1 x y`` take BARE identifiers
            # that we can't reliably distinguish from literal strings
            # by a simple scan.  Narrowing is a future optimisation;
            # requires a per-command argument-kind analysis.
            self._emit_frame_sync()
            # Stamp the current namespace into the interpreter's ns
            # register so dynamic ``proc $name body`` / ``variable
            # $name`` inside the fallback qualify into the enclosing
            # namespace instead of falling through to ``::``.  Only
            # applies inside compiled procs that have a namespace
            # other than global.
            ns_saved_idx: int | None = None
            if self._is_proc and self._proc_namespace and self._proc_namespace != "::":
                ns_set_idx = self._shared_imports.get("tcl_ns_set")
                if ns_set_idx is not None:
                    ns_saved_idx = self._add_extra_local(prefix="_ns_saved", val_type=ValType.I64)
                    # ns name without the leading ``::`` — the
                    # interpreter's qualify_name prepends ``::`` if
                    # needed; keep the full ``::ns`` form for
                    # consistency with how the compiler emits
                    # qualified names elsewhere.
                    ns_literal = self._proc_namespace
                    # Stash namespace bytes in the data section and
                    # push (ptr, len).  Reuse the string-constant
                    # pool so multiple fallbacks in the same proc
                    # share the bytes.
                    offset = self._intern_string(ns_literal)
                    encoded = ns_literal.encode("utf-8")
                    self._emit_i32_const(offset + 4)
                    self._emit_i32_const(len(encoded))
                    self._emit_call(ns_set_idx)
                    self._emit_local_set(ns_saved_idx)
            self._emit_obj_literal(script)
            self._emit_call(eval_idx)
            # Reload locals from frame — eval may have modified them (e.g.
            # ``set x 99``, writes through ``upvar`` aliases, ``unset``).
            self._emit_frame_readback()
            # Restore the caller's namespace context.
            if ns_saved_idx is not None:
                ns_restore_idx = self._shared_imports.get("tcl_ns_restore")
                if ns_restore_idx is not None:
                    # Stash eval result before calling ns_restore
                    # (which returns void) and put it back after.
                    result_tmp = self._add_extra_local(prefix="_eval_result", val_type=ValType.I32)
                    self._emit_local_set(result_tmp)
                    self._emit_local_get(ns_saved_idx)
                    self._emit_call(ns_restore_idx)
                    self._emit_local_get(result_tmp)
            return
        # No interpreter available — hard trap
        fidx = self._shared_imports.get("tcl_error")
        if fidx is not None:
            msg = f"unsupported in WASM: {command}"
            self._emit_obj_literal(msg)
            self._emit_call(fidx)
        self._emit(WasmOp.UNREACHABLE)

    def _emit_call_stmt_tail(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...] = (),
    ) -> None:
        """Emit a command invocation in tail position, keeping its i32 result on the stack.

        Used for implicit return: the last command's result becomes the
        proc's return value.  Dispatch order matches ``_emit_call_stmt``
        (proc calls first, then built-ins) to ensure consistent behaviour.
        """
        # Scope declarations produce no value — return null TclObj
        if command in _SCOPE_NOP_COMMANDS:
            # ``namespace eval ns arg1 arg2 ...`` in tail position with dynamic
            # script args: assemble the script at WASM level and call tcl_eval
            # so the result becomes the proc's return value.
            if command == "namespace" and args and args[0] == "eval" and len(args) > 2:
                if self._emit_namespace_eval_bridge(args[2:], drop_result=False):
                    return
                # Runtime imports missing — push null TclObj as fallback.
                self._emit_i32_const(0)
                return
            self._emit_i32_const(0)
            return

        # <upvar-invalidate> — synthetic CFG-builder invalidation node.
        # In tail position (rare, but possible if placed last by optimisation)
        # we still need to push a null TclObj.
        if command == "<upvar-invalidate>":
            self._emit_i32_const(0)
            return

        # upvar/variable in tail position — run the alias-setup side effects,
        # then push null (upvar returns empty string).
        if command == "upvar":
            self._emit_cmd_upvar(args)
            self._emit_i32_const(0)
            return
        if command == "variable":
            self._emit_cmd_variable(args)
            self._emit_i32_const(0)
            return

        # catch in tail position — emit body with error handling,
        # leave the catch return code (0 or 1) on the stack.
        if command == "catch" and args:
            self._emit_catch_from_args(args, defs, keep_on_stack=True)
            return

        # User-defined proc call — keep i32 result
        proc_info = self._resolve_proc(command)
        if proc_info is not None:
            func_idx, n_params = proc_info
            # Push exactly n_params args (truncate surplus, pad missing)
            for i in range(min(n_params, len(args))):
                self._emit_value(args[i])
            for _ in range(n_params - len(args)):
                self._emit_i32_const(0)
            self._emit_call(func_idx)
            # i32 result stays on the stack
            return

        # set: inline local operations (returns the i32 TclObj)
        if command == "set" and 1 <= len(args) <= 2:
            var = args[0]
            # Inside ``namespace eval ::ns { ... }`` the
            # fast-local-path is wrong — unqualified writes must
            # land in ``::ns::<name>``.  Route through the full
            # write path (which consults ``_block_namespace``
            # and emits ``tcl_global_set``).
            in_ns_block = (
                not self._is_proc
                and self._block_namespace is not None
                and self._block_namespace != "::"
                and _parse_array_ref(var) is None
            )
            if var in self._aliases and len(args) >= 2:
                # Aliased write: route through global set, which returns the value.
                self._emit_value(args[1])
                self._emit_var_write_obj_keep(var)
                return
            if var in self._aliases:
                # Aliased read: tcl_global_get leaves value on stack.
                self._emit_var_read_obj(var)
                return
            if in_ns_block:
                if len(args) >= 2:
                    self._emit_value(args[1])
                    self._emit_var_write_obj_keep(var)
                else:
                    self._emit_var_read_obj(var)
                return
            idx = self._intern_local(var)
            if len(args) >= 2:
                self._emit_value(args[1])
                self._emit_local_tee(idx)
            else:
                self._emit_local_get(idx)
            return

        # incr: returns the new value as i32 TclObj (Tcl semantics)
        if command == "incr" and 1 <= len(args) <= 2:
            var = args[0]
            in_ns_block = (
                not self._is_proc
                and self._block_namespace is not None
                and self._block_namespace != "::"
                and _parse_array_ref(var) is None
            )
            if var in self._aliases or in_ns_block:
                # Aliased or namespace-scoped incr: load via the
                # global table, add, store back.
                self._emit_var_read_obj(var)
                self._emit_unbox_int()
                amt = 1
                if len(args) >= 2:
                    try:
                        amt = int(args[1])
                    except ValueError:
                        self._emit_value(args[1])
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        self._emit_box_int()
                        self._emit_var_write_obj_keep(var)
                        return
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                self._emit_var_write_obj_keep(var)
                return
            idx = self._intern_local(var)
            self._emit_local_get(idx)
            self._emit_unbox_int()
            amt = 1
            if len(args) >= 2:
                try:
                    amt = int(args[1])
                except ValueError:
                    self._emit_value(args[1])
                    self._emit_unbox_int()
                    self._emit(WasmOp.I64_ADD)
                    self._emit_box_int()
                    self._emit_local_tee(idx)
                    return
            self._emit_i64_const(amt)
            self._emit(WasmOp.I64_ADD)
            self._emit_box_int()
            self._emit_local_tee(idx)
            return

        # return: emit WASM return (handled at call site, but can appear in tail)
        if command == "return":
            if args:
                self._emit_value(args[0])
            else:
                self._emit_i32_const(0)
            return

        # info sub-commands — keep result on stack
        if command == "info" and args:
            self._emit_info_value(args)
            return

        # lassign — keep leftover-list result on stack
        if command == "lassign" and args:
            self._emit_cmd_lassign(args, defs, keep_on_stack=True)
            return

        # clock — keep timer result on stack
        if command == "clock" and args:
            self._emit_clock_value(args)
            return

        # array subcommand in tail position — keep result on stack
        if command == "array" and args:
            self._emit_array_subcmd_value(args)
            return

        # uplevel in tail position — keep eval result on stack.
        if command == "uplevel" and args:
            self._emit_cmd_uplevel(args)
            return

        # string sub-commands — keep result on stack
        if command == "string" and args:
            subcmd = args[0]
            # Handle "string is <class> <value>" in tail position
            if subcmd == "is" and len(args) >= 3:
                is_key = _STRING_IS_IMPORT.get(args[1])
                if is_key is not None and is_key in self._shared_imports:
                    func_idx = self._shared_imports[is_key]
                    self._emit_value(args[-1])
                    self._emit_call(func_idx)
                    return
            import_key = _STRING_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not spec[3]:
                    self._emit_i32_const(0)
                return
            self._emit_i32_const(0)
            return

        # dict sub-commands — keep result on stack
        if command == "dict" and args:
            subcmd = args[0]
            import_key = _DICT_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = args[1:]
                # For dict set, first arg is the variable name
                if subcmd == "set" and len(sub_args) >= 3:
                    var_idx = self._intern_local(sub_args[0])
                    self._emit_local_get(var_idx)  # dict value
                    self._emit_value(sub_args[1])  # key
                    self._emit_value(sub_args[2])  # value
                    self._emit_call(func_idx)
                    self._emit_local_tee(var_idx)
                else:
                    for i in range(min(param_count, len(sub_args))):
                        self._emit_value(sub_args[i])
                    for _ in range(param_count - len(sub_args)):
                        self._emit_i32_const(0)
                    self._emit_call(func_idx)
                    if not spec[3]:
                        self._emit_i32_const(0)
                return
            self._emit_i32_const(0)
            return

        # Runtime command — use the same dispatch logic as non-tail,
        # but keep the return value on the stack instead of dropping it.
        if command in _CMD_RUNTIME:
            import_key, _ = _CMD_RUNTIME[command]
            fidx = self._shared_imports.get(import_key)
            if fidx is not None:
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                mutates_var = command in ("append", "lappend")
                if mutates_var and len(args) >= 2:
                    var_name = args[0]
                    var_idx = self._intern_local(var_name)
                    # Loop: each value_arg gets concatenated / appended
                    # in order.  After the last one we tee — the final
                    # updated value is left on the stack for implicit
                    # return.
                    last = len(args) - 1
                    for i, value_arg in enumerate(args[1:], start=1):
                        self._emit_local_get(var_idx)
                        self._emit_value(value_arg)
                        self._emit_call(fidx)
                        if i == last:
                            self._emit_local_tee(var_idx)
                        else:
                            self._emit_local_set(var_idx)
                elif command == "puts":
                    if args:
                        self._emit_value(args[-1])
                    else:
                        self._emit_i32_const(0)
                    self._emit_call(fidx)
                    if not spec[3]:
                        self._emit_i32_const(0)
                else:
                    for i in range(min(param_count, len(args))):
                        self._emit_value(args[i])
                    for _ in range(param_count - len(args)):
                        self._emit_i32_const(0)
                    self._emit_call(fidx)
                    if not spec[3]:
                        self._emit_i32_const(0)
                return

        # Unknown command in tail position — fall back to interpreter,
        # leaving the result on the stack for implicit return.
        self._emit_eval_fallback(command, args)

    # -- Global variable write-through --

    def _emit_var_read_obj(self, name: str) -> None:
        """Push the current TclObj value of local Tcl variable *name* on the stack.

        For aliased variables (``upvar``/``variable``), this reads through
        ``tcl_global_get`` using the target name stashed at alias-setup time.
        For ``::``-qualified names (e.g. ``::ns::var``), this reads the
        global directly.  Array references (``arr(key)``) dispatch to
        ``tcl_array_get`` with the element's stored value.  Otherwise
        this is a plain WASM ``local.get`` of the interned local.
        """
        array_ref = _parse_array_ref(name)
        if array_ref is not None:
            self._emit_array_element_read(*array_ref)
            return
        binding = self._aliases.get(name)
        if binding is not None:
            kind, target_idx = binding
            if kind == "global":
                gget_idx = self._shared_imports.get("tcl_global_get")
                if gget_idx is not None:
                    self._emit_local_get(target_idx)
                    self._emit_call(gget_idx)
                    return
        if name.startswith("::"):
            gget_idx = self._shared_imports.get("tcl_global_get")
            if gget_idx is not None:
                self._emit_obj_literal(name)
                self._emit_call(gget_idx)
                return
        # Mirror of the write-side fix: inside a ``namespace eval
        # ::ns { … }`` body, unqualified reads must look up the
        # ``::ns::<name>`` global rather than a bare local — because
        # we wrote it through the global table with the qualified
        # name, and a bare-local read would find 0.
        if not self._is_proc and self._block_namespace and self._block_namespace != "::":
            gget_idx = self._shared_imports.get("tcl_global_get")
            if gget_idx is not None:
                qname = f"{self._block_namespace}::{name}"
                self._emit_obj_literal(qname)
                self._emit_call(gget_idx)
                return
        # Var-escape FRAME branch: the escape analysis proved this
        # name is observed by the interpreter or a callee upvar, so
        # its authoritative value lives in the runtime frame.  Read
        # through ``tcl_local_get`` (frame-scoped; does NOT fall through
        # to globals — that would alias with top-level vars of the same
        # name).
        if self._is_frame_only_var(name):
            lget_idx = self._shared_imports.get("tcl_local_get")
            if lget_idx is not None:
                self._emit_obj_literal(name)
                self._emit_call(lget_idx)
                return
        idx = self._intern_local(name)
        self._emit_local_get(idx)

    def _emit_var_write_obj(self, name: str) -> None:
        """Consume a TclObj value on the stack and write it to local Tcl variable *name*.

        For aliased variables, the value is routed to ``tcl_global_set``
        using the stashed target name.  For plain variables, the value is
        stored in the interned WASM local; if the name was declared
        ``global`` it's also written back to the global table.
        """
        self._emit_var_write_obj_impl(name, keep_on_stack=False)

    def _emit_var_write_obj_keep(self, name: str) -> None:
        """Like _emit_var_write_obj but leaves the written value on the stack.

        Used in tail-position ``set``/``incr`` emissions where the command's
        return value is the proc's return value.
        """
        self._emit_var_write_obj_impl(name, keep_on_stack=True)

    def _emit_var_write_obj_impl(self, name: str, *, keep_on_stack: bool) -> None:
        array_ref = _parse_array_ref(name)
        if array_ref is not None:
            self._emit_array_element_write(array_ref[0], array_ref[1], keep_on_stack=keep_on_stack)
            return
        binding = self._aliases.get(name)
        if binding is not None:
            kind, target_idx = binding
            if kind == "global":
                self._emit_global_set_via_local(target_idx, keep_on_stack=keep_on_stack)
                return
        if name.startswith("::"):
            # ``::``-qualified names always refer to globals.  Route the
            # write directly without creating an unused WASM local.
            self._emit_global_set_via_literal(name, keep_on_stack=keep_on_stack)
            return
        # Inside a ``namespace eval ::ns { … }`` body, unqualified
        # writes must land in the ``::ns::`` table, not in a local
        # or in ``::``.  Without this, ``namespace eval ns { set x
        # 5 }`` wrote ``::x`` (overwriting/creating the wrong
        # global) and reads of ``$::ns::x`` came back empty.  We
        # qualify and route through ``tcl_global_set`` with the
        # full name; no local mirror, because the block may be
        # re-entered in a different frame (tcltest's stage-2 body
        # is a sequence of ``namespace eval ::tcltest { … }``
        # blocks).
        if not self._is_proc and self._block_namespace and self._block_namespace != "::":
            qname = f"{self._block_namespace}::{name}"
            self._emit_global_set_via_literal(qname, keep_on_stack=keep_on_stack)
            return
        # Var-escape FRAME branch: the escape analysis proved this
        # name is observed by the interpreter or a callee upvar, so
        # its authoritative value must live in the runtime frame.
        # Route the write through ``tcl_local_set``.  Also mirror to a
        # WASM local so subsequent reads of this name in the same
        # basic block could fast-path — but we conservatively skip the
        # mirror today because ``_emit_var_read_obj`` already goes
        # through the frame for FRAME-tagged names.
        if self._is_frame_only_var(name):
            lset_idx = self._shared_imports.get("tcl_local_set")
            if lset_idx is not None:
                # Stack holds [value].  Need [name_obj, value] for
                # tcl_local_set.  Stash value in a scratch local so we
                # can push name_obj first.
                tmp = self._add_extra_local(prefix="_frame_set_tmp", val_type=ValType.I32)
                self._emit_local_set(tmp)
                self._emit_obj_literal(name)
                self._emit_local_get(tmp)
                self._emit_call(lset_idx)
                if keep_on_stack:
                    self._emit(WasmOp.DROP)  # drop tcl_local_set return
                    self._emit_local_get(tmp)  # leave value on stack
                else:
                    self._emit(WasmOp.DROP)
                return

        # At the top level (``::top``) there's no ``proc`` scope — every
        # variable is in the global namespace.  Route writes through
        # ``tcl_global_set`` AND keep a WASM local mirror so later reads
        # at the same level can use the fast path.  Without the global
        # mirror, an interpreter fallback at top level (e.g. a
        # dynamically-registered ``proc $varName`` call) can't see the
        # variable — eval-fallback always resolves via the global
        # table when no frame is active.
        if not self._is_proc:
            gset_idx = self._shared_imports.get("tcl_global_set")
            if gset_idx is not None:
                idx = self._intern_local(name)
                # Stack holds [value].  Need [name_obj, value] for gset.
                tmp = self._add_extra_local(prefix="_gset_tmp", val_type=ValType.I32)
                self._emit_local_set(tmp)
                # Also mirror to the WASM local for fast in-top reads.
                self._emit_local_get(tmp)
                self._emit_local_set(idx)
                self._emit_obj_literal(name)
                self._emit_local_get(tmp)
                self._emit_call(gset_idx)
                if keep_on_stack:
                    self._emit(WasmOp.DROP)  # drop gset return
                    self._emit_local_get(tmp)  # leave value on stack
                else:
                    self._emit(WasmOp.DROP)  # drop gset return
                return
        idx = self._intern_local(name)
        if keep_on_stack:
            self._emit_local_tee(idx)
        else:
            self._emit_local_set(idx)
        self._emit_global_writeback(name, idx)

    def _emit_array_name_obj(self, arr: str) -> None:
        """Push the TclObj containing the runtime name of array *arr*.

        Honours ``upvar`` / ``variable`` aliases so ``upvar #0
        counter::T-$tag counter; set counter(N) 0`` hits the correct
        per-tag array.  ``::``-qualified names pass through as
        literals.  Inside a ``namespace eval ::ns { ... }`` body
        unqualified names are prefixed with the block's namespace
        so ``set Option(-match) *`` lands in ``::ns::Option(-match)``
        rather than the bare global ``Option(-match)``.
        """
        binding = self._aliases.get(arr)
        if binding is not None and binding[0] == "global":
            self._emit_local_get(binding[1])
            return
        if arr.startswith("::"):
            self._emit_obj_literal(arr)
            return
        if (
            not self._is_proc
            and self._block_namespace is not None
            and self._block_namespace != "::"
        ):
            self._emit_obj_literal(f"{self._block_namespace}::{arr}")
            return
        self._emit_obj_literal(arr)

    def _emit_array_element_read(self, arr: str, key: str) -> None:
        """``$arr(key)`` — emit tcl_array_get(arr_name, key) and leave the
        i32 TclObj result on the stack.
        """
        func_idx = self._shared_imports.get("tcl_array_get")
        if func_idx is None:
            self._emit_i32_const(0)
            return
        self._emit_array_name_obj(arr)
        self._emit_value(key)
        self._emit_call(func_idx)

    def _emit_array_element_write(self, arr: str, key: str, *, keep_on_stack: bool) -> None:
        """``set arr(key) value`` — value is already on the stack.

        Stashes the value, pushes (arr_name, key, value) and calls
        tcl_array_set.
        """
        func_idx = self._shared_imports.get("tcl_array_set")
        if func_idx is None:
            if not keep_on_stack:
                self._emit(WasmOp.DROP)
            return
        tmp = self._add_extra_local(prefix="_arr_val", val_type=ValType.I32)
        self._emit_local_set(tmp)
        self._emit_array_name_obj(arr)
        self._emit_value(key)
        self._emit_local_get(tmp)
        self._emit_call(func_idx)
        if not keep_on_stack:
            self._emit(WasmOp.DROP)

    def _emit_global_set_via_local(self, target_local_idx: int, *, keep_on_stack: bool) -> None:
        """Consume a value on the stack, write it to the global whose name
        is held in WASM local *target_local_idx*.  Leaves the value on the
        stack if *keep_on_stack*.
        """
        gset_idx = self._shared_imports.get("tcl_global_set")
        if gset_idx is None:
            if not keep_on_stack:
                self._emit(WasmOp.DROP)
            return
        # Stack: [value].  We need [target_name, value] for tcl_global_set.
        tmp = self._add_extra_local(prefix="_gset_val", val_type=ValType.I32)
        self._emit_local_set(tmp)
        self._emit_local_get(target_local_idx)
        self._emit_local_get(tmp)
        self._emit_call(gset_idx)
        if not keep_on_stack:
            self._emit(WasmOp.DROP)

    def _emit_global_set_via_literal(self, target_name: str, *, keep_on_stack: bool) -> None:
        """Consume a value on the stack, write it to the global named *target_name*.

        Leaves the value on the stack if *keep_on_stack*.
        """
        gset_idx = self._shared_imports.get("tcl_global_set")
        if gset_idx is None:
            if not keep_on_stack:
                self._emit(WasmOp.DROP)
            return
        tmp = self._add_extra_local(prefix="_gset_val", val_type=ValType.I32)
        self._emit_local_set(tmp)
        self._emit_obj_literal(target_name)
        self._emit_local_get(tmp)
        self._emit_call(gset_idx)
        if not keep_on_stack:
            self._emit(WasmOp.DROP)

    def _emit_global_writeback(self, name: str, local_idx: int) -> None:
        """If *name* is a declared global, write the local value to the global table."""
        if name not in self._globals:
            return
        gset_idx = self._shared_imports.get("tcl_global_set")
        if gset_idx is None:
            return
        self._emit_obj_literal(name)
        self._emit_local_get(local_idx)
        self._emit_call(gset_idx)
        self._emit(WasmOp.DROP)

    def _emit_namespace_eval_bridge(
        self,
        script_parts: tuple[str, ...] | list[str],
        *,
        drop_result: bool,
    ) -> bool:
        """Emit a ``namespace eval <ns> <script-parts...>`` bridge.

        Assembles the script at WASM level (so compiled-frame aliases
        like ``$arr($key)`` resolve correctly) then calls ``tcl_eval``
        with a ``frame_sync`` / ``frame_readback`` pair around it so
        the interpreter sees the caller's locals.

        *drop_result* controls what happens to the eval result:
          - ``True`` (statement context): drop — stack returns to empty.
          - ``False`` (value / tail context): keep the i32 TclObj on
            the stack for the caller.

        Returns ``True`` when the bridge was emitted in full, ``False``
        when the required runtime imports aren't available — the
        caller should fall through to its normal fallback path
        (usually ``_emit_eval_fallback``) in that case.
        """
        eval_idx = self._shared_imports.get("tcl_eval")
        if eval_idx is None:
            return False
        if not script_parts:
            # No script → empty string is a valid no-op.
            if not drop_result:
                self._emit_obj_literal("")
            return True
        if len(script_parts) == 1:
            self._emit_value(script_parts[0])
        else:
            append_idx = self._shared_imports.get("tcl_append")
            if append_idx is None:
                return False
            self._emit_value(script_parts[0])
            for sa in script_parts[1:]:
                self._emit_obj_literal(" ")
                self._emit_call(append_idx)
                self._emit_value(sa)
                self._emit_call(append_idx)
        self._emit_frame_sync()
        self._emit_call(eval_idx)
        if drop_result:
            self._emit(WasmOp.DROP)
        self._emit_frame_readback()
        return True

    def _is_frame_only_var(self, name: str) -> bool:
        """True when the escape analysis says ``name`` must live in the runtime frame.

        Only returns True for non-pessimistic procs where analysis
        specifically proved the var escapes.  Pessimistic procs
        (``dynamic_barrier=True``) keep today's WASM-local-with-sync
        behaviour — routing every var through the frame primitives
        would be a much larger change and the sync path is already
        correct for that case.

        Vars already handled by other routing mechanisms are excluded:
        ``::``-qualified globals go through ``tcl_global_*``;
        ``global`` / ``variable`` declared names go through the globals
        table too; upvar aliases have their own binding.  Those paths
        own the interpreter-visible storage and must not be short-
        circuited by the frame-only branch.
        """
        summary = self._escape_summary
        if summary is None or summary.dynamic_barrier:
            return False
        if not self._is_proc:
            # At top level there's no frame; FRAME tags are meaningless
            # and the writer already routes ``::``-prefixed and
            # block-namespace writes through ``tcl_global_set``.
            return False
        if name in self._globals:
            return False
        if name in self._aliases:
            return False
        if name.startswith("::"):
            return False
        return summary.is_frame(name)

    def _iter_sync_locals(self, vars_used: set[str] | None = None) -> "list[tuple[str, int]]":
        """Return the ``(name, local_idx)`` pairs that should take part
        in a frame sync / readback at the current emit site, sorted by
        name for deterministic WASM output across Python-dict iteration
        orders.

        *vars_used* is the conservative set of local names the upcoming
        interpreter call could observe.  ``None`` means "unknown — sync
        every Tcl local".  An empty set also means the caller
        explicitly knows nothing is referenced (no sync needed).

        When a per-proc var-escape summary is available, the caller
        passes ``vars_used`` (the fallback's statically known reference
        set), and the proc is not in the dynamic-barrier pessimistic
        state, vars the analysis proved the interpreter cannot observe
        by name (``LOCAL`` tag) are excluded from the sync set.

        The ``vars_used is not None`` guard is a soundness gate: it
        keeps the narrow-sync off the "unknown fallback" path until an
        interprocedural extension propagates callee ``upvar 1`` source
        sets back into the caller's summary.
        """
        summary = self._escape_summary
        narrow_to_frame = (
            summary is not None and not summary.dynamic_barrier and vars_used is not None
        )

        pairs: list[tuple[str, int]] = []
        for name, idx in self._tcl_var_locals.items():
            if name in self._aliases:
                continue
            if name in self._globals:
                continue
            if name.startswith("::"):
                continue
            if vars_used is not None and name not in vars_used:
                continue
            if narrow_to_frame and not summary.is_frame(name):
                # Analysis proved this var cannot be observed by name
                # from the interpreter — no sync needed.
                continue
            pairs.append((name, idx))
        pairs.sort(key=lambda p: p[0])
        return pairs

    def _emit_frame_sync(self, vars_used: set[str] | None = None) -> None:
        """Mirror proc-locals into the call frame before the Zig
        interpreter takes control (e.g. at a ``tcl_eval`` call site).

        Only called at interpreter-entry boundary points — not after every
        variable write — so procs that never reach the interpreter pay zero
        frame-sync overhead.

        *vars_used* narrows the sync to locals the upcoming script is
        statically known to reference (``$name`` / ``${name}``).  When
        ``None`` (or script scanning failed) we fall back to syncing
        every Tcl local, matching the original conservative behaviour.
        An empty set skips the sync entirely.

        A null-guard (``if (local != 0) { … }``) around each
        ``tcl_local_set`` call ensures that WASM locals that were never
        assigned (still zero-initialised) do not create phantom frame entries
        that would make ``info exists`` return true for unset variables.

        Skipped when:
        - not inside a compiled proc (``_is_proc`` is False)
        - ``tcl_local_set`` is not imported (frame not active)
        """
        if not self._is_proc:
            return
        lset_idx = self._shared_imports.get("tcl_local_set")
        if lset_idx is None:
            return
        for name, idx in self._iter_sync_locals(vars_used):
            # Emit: if (local != 0) { tcl_local_set(name_obj, local) }
            # Skips zero-initialised locals that were never assigned so they
            # don't create spurious frame entries.
            self._emit_local_get(idx)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_obj_literal(name)
            self._emit_local_get(idx)
            self._emit_call(lset_idx)
            self._emit(WasmOp.DROP)
            self._emit(WasmOp.END)

    def _emit_frame_readback(self, vars_used: set[str] | None = None) -> None:
        """Reload proc-locals from the call frame after the interpreter
        returns.  See :func:`_emit_frame_sync` for the *vars_used*
        contract — the same set must be used here so writes by the
        interpreter (``set x 99`` through an aliased name) surface back
        into the compiled proc's WASM locals.
        """
        if not self._is_proc:
            return
        lget_idx = self._shared_imports.get("tcl_local_get")
        if lget_idx is None:
            return
        for name, idx in self._iter_sync_locals(vars_used):
            self._emit_obj_literal(name)
            self._emit_call(lget_idx)
            self._emit_local_set(idx)

    def _register_global_alias(self, local_name: str, target_value: str) -> None:
        """Register *local_name* as a global alias and stash the target name.

        Emits code that evaluates *target_value* (which may be an
        interpolated string like ``counter::T-$tag``) to a TclObj, stores
        it in a hidden WASM local, and records the binding so subsequent
        reads/writes of *local_name* route through tcl_global_{get,set}.
        """
        target_idx = self._add_extra_local(prefix=f"_alias_{local_name}_tgt", val_type=ValType.I32)
        self._emit_value(target_value)
        self._emit_local_set(target_idx)
        self._aliases[local_name] = ("global", target_idx)

    def _emit_cmd_upvar(self, args: tuple[str, ...]) -> None:
        """``upvar ?level? otherVar myVar ?otherVar myVar ...?``

        Only the ``#0`` (global alias) form is currently compiled.  For
        any other level the module traps at runtime with a descriptive
        message — a future enhancement will add caller-frame alias
        support via a new runtime hook.
        """
        if len(args) < 2:
            self._emit_unsupported_trap("upvar (too few args)")
            return

        # Determine whether args[0] is a level specifier.  A leading '#'
        # always marks one; a bare digit sequence does too.  Anything
        # else means the level defaults to 1 and args[0] is otherVar.
        first = args[0]
        if first.startswith("#"):
            level_spec = first
            pairs_start = 1
        elif first.lstrip("-").isdigit():
            level_spec = first
            pairs_start = 1
        else:
            level_spec = "1"
            pairs_start = 0

        pair_args = args[pairs_start:]
        if len(pair_args) & 1:
            self._emit_unsupported_trap("upvar (uneven var pairs)")
            return

        if level_spec != "#0":
            # Caller-frame aliasing not yet implemented — trap with a
            # clear message rather than silently mis-compiling.
            self._emit_unsupported_trap(
                f"upvar level {level_spec} — compiled procs only support #0 "
                "(global alias).  Caller-frame aliasing (upvar N) needs "
                "the frame-pushing variant of compiled procs; use uplevel "
                "to run code in the caller instead."
            )
            return

        for i in range(0, len(pair_args), 2):
            target = pair_args[i]
            local_name = pair_args[i + 1]
            self._register_global_alias(local_name, target)

    def _emit_cmd_variable(self, args: tuple[str, ...]) -> None:
        """``variable name ?value? ?name value ...?``

        Inside a namespace proc, aliases ``name`` to ``::ns::name`` in
        the enclosing namespace.  If a value is provided and the
        namespace variable does not already exist, it is initialised.

        At namespace-eval top-level (``namespace eval ::ns {
        variable debug 0 }``), the ``?value?`` form still needs to do
        the write — ``debug 0`` must land in ``::ns::debug``.  The
        previous implementation treated ns-eval variables as a no-op
        claiming assignments "already hit globals", but bare ``set x
        5`` in the body was writing to ``::x`` (the wrong global);
        reads of ``::ns::x`` came back empty, and tcltest's ``$debug
        >= 1`` compared ``""`` against ``1`` (empty-string comparisons
        pass as truthy here), forcing every ``DebugPuts`` to fire.
        """
        if not args:
            return
        if not self._is_proc:
            # Namespace-eval top-level — no frame to alias into,
            # but the ``?value?`` pairs still need their writes.
            # Route through ``_emit_var_write_obj`` so the
            # namespace-qualification logic in that path picks
            # ``_block_namespace`` (set by the enclosing ``IRBlock``)
            # and writes ``::ns::<name>`` to the global table.
            # Bare ``variable name`` with no initializer is a
            # declaration that does nothing in our compiled model
            # (reads are already auto-qualified by block-ns).
            i = 0
            while i < len(args):
                name = args[i]
                has_value = i + 1 < len(args)
                if has_value:
                    self._emit_value(args[i + 1])
                    self._emit_var_write_obj(name)
                    i += 2
                else:
                    i += 1
            return

        ns = self._proc_namespace or "::"
        i = 0
        while i < len(args):
            name = args[i]
            has_value = i + 1 < len(args)
            # Dynamic name — ``variable $varName ?value?`` as used by
            # tcltest's ``Default`` / ``Option``.  Build the qualified
            # target at runtime (``<ns>::<$name-value>``) and — when a
            # value is supplied — write it to the global table via
            # ``tcl_global_set``.  The compile-time alias registration
            # (used for later ``$local`` reads in the proc body) is
            # skipped because the name only exists at runtime; users
            # who want a local alias can follow up with an explicit
            # ``upvar``.
            is_dynamic = (name.startswith("$") or name.startswith("[")) and _parse_array_ref(
                name
            ) is None
            if is_dynamic:
                gset_idx = self._shared_imports.get("tcl_global_set")
                append_idx = self._shared_imports.get("tcl_append")
                gexist_idx = self._shared_imports.get("tcl_global_exists")
                if has_value and gset_idx is not None and append_idx is not None:
                    # Construct ``<ns>::<name>`` at runtime by
                    # concatenating the namespace prefix with the
                    # dynamic name value via ``tcl_append``.  Stash
                    # the result in a hidden local so the existence
                    # check and the write can reuse it without
                    # recomputing the concat.
                    ns_prefix = f"{ns}::" if ns != "::" else "::"
                    qname_idx = self._add_extra_local(prefix="_var_dyn_qname", val_type=ValType.I32)
                    self._emit_obj_literal(ns_prefix)
                    self._emit_value(name)
                    self._emit_call(append_idx)
                    self._emit_local_set(qname_idx)
                    # ``variable`` only initialises when the variable
                    # does not already exist — check via
                    # ``tcl_global_exists`` and skip the write on hit.
                    if gexist_idx is not None:
                        self._emit_local_get(qname_idx)
                        self._emit_call(gexist_idx)
                        self._emit_call(self._shared_imports["tcl_obj_get_int"])
                        self._emit(WasmOp.I64_EQZ)
                        self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                        self._emit_local_get(qname_idx)
                        self._emit_value(args[i + 1])
                        self._emit_call(gset_idx)
                        self._emit(WasmOp.DROP)
                        self._emit(WasmOp.END)
                    else:
                        self._emit_local_get(qname_idx)
                        self._emit_value(args[i + 1])
                        self._emit_call(gset_idx)
                        self._emit(WasmOp.DROP)
                    i += 2
                else:
                    # Bare ``variable $name`` declaration or missing
                    # runtime helpers — nothing to emit at compile time;
                    # reads of the local ``name`` won't route through a
                    # namespace alias, but that's the same degraded
                    # mode we had before dynamic support landed.
                    i += 2 if has_value else 1
                continue

            # Resolve the target name: ``::ns::name`` for unqualified
            # names, or the literal name if already qualified with ``::``.
            if name.startswith("::"):
                target = name
                # The local alias uses the final segment as its Tcl name.
                local_name = name.rsplit("::", 1)[-1]
            else:
                if ns == "::":
                    target = f"::{name}"
                else:
                    target = f"{ns}::{name}"
                local_name = name

            # Stash the target literal in a hidden local and register.
            target_idx = self._add_extra_local(
                prefix=f"_var_{local_name}_tgt", val_type=ValType.I32
            )
            self._emit_obj_literal(target)
            self._emit_local_set(target_idx)
            self._aliases[local_name] = ("global", target_idx)

            if has_value:
                # ``variable name value`` — initialise the namespace var
                # unconditionally (matches Tcl 8.x behaviour in practice
                # for scripted modules; upstream tcltest uses it idempotently).
                self._emit_value(args[i + 1])
                self._emit_var_write_obj(local_name)
                i += 2
            else:
                i += 1

    # -- Individual command emitters --

    def _emit_cmd_set(self, args: tuple[str, ...]) -> None:
        """``set varName ?value?`` — read or write a local/global/aliased variable."""
        if not args:
            self._emit_unsupported_trap("set (no args)")
            return
        var = args[0]
        if len(args) >= 2:
            self._emit_value(args[1])
            self._emit_var_write_obj(var)
        else:
            pass  # set x — read (result unused in statement context)

    def _emit_cmd_incr(self, args: tuple[str, ...]) -> None:
        """``incr varName ?increment?`` — unbox, i64 add, rebox."""
        if not args:
            self._emit_unsupported_trap("incr (no args)")
            return
        var = args[0]
        self._emit_var_read_obj(var)
        self._emit_unbox_int()
        amt = 1
        if len(args) >= 2:
            try:
                amt = int(args[1])
            except ValueError:
                # Variable increment — unbox the increment value
                self._emit_value(args[1])
                self._emit_unbox_int()
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                self._emit_var_write_obj(var)
                return
        self._emit_i64_const(amt)
        self._emit(WasmOp.I64_ADD)
        self._emit_box_int()
        self._emit_var_write_obj(var)

    def _emit_cmd_return(self, args: tuple[str, ...]) -> None:
        """``return ?value?`` or ``return -code code ?value?``.

        The simple single-value form compiles to a WASM return.
        ``return -code error <msg>`` is special-cased inline: evaluate
        *msg* via :meth:`_emit_value` (so embedded ``$var`` /
        ``[cmd]`` substitutions work), then call ``tcl_cmd_error``
        — which sets the catch's ``error_flag``/``error_msg`` when
        inside a ``catch`` or traps otherwise.  Going through
        :meth:`_emit_eval_fallback` would brace-wrap the message to
        preserve list structure, blocking the substitutions the
        error text needs — a real hazard because tcltest's error
        messages embed ``$option`` / ``$values`` everywhere.

        Other ``-code`` forms (``return -code break``, ``return -code
        continue``, numeric codes, ``-level N``, ``-errorinfo``
        ``-errorcode``) are rarer and fall through to the eval
        fallback, whose argument quoting is safe for them because
        their payloads are typically literal keywords or numeric
        values without interpolation.
        """
        if args and len(args) >= 3 and args[0] == "-code" and args[1] == "error" and len(args) == 3:
            # return -code error <msg>
            self._emit_value(args[2])
            # ``_RUNTIME_IMPORTS`` keys the error import as
            # ``tcl_error`` (internal key) → WASM name
            # ``tcl_cmd_error``; use the internal key to look up
            # the shared import slot.
            err_idx = self._shared_imports.get("tcl_error")
            if err_idx is None:
                self._emit_eval_fallback("return", args)
                return
            self._emit_call(err_idx)
            # tcl_cmd_error returns nothing; emit a null TclObj for
            # the WASM return value.  When inside a catch, error_flag
            # is now set and the catch body's has_error check will
            # trip on the next statement.  When outside a catch, the
            # runtime's tcl_cmd_error already traps and this return
            # is unreachable.
            self._emit_i32_const(0)
            self._emit(WasmOp.RETURN)
            return
        if args and args[0].startswith("-"):
            self._emit_eval_fallback("return", args)
            return
        if args:
            self._emit_value(args[0])
        else:
            self._emit_i32_const(0)
        self._emit(WasmOp.RETURN)

    def _emit_cmd_string(self, args: tuple[str, ...], defs: tuple[str, ...]) -> None:
        """``string subcommand ...`` — dispatch to runtime import (i32 args)."""
        if not args:
            self._emit_unsupported_trap("string (no subcommand)")
            return
        subcmd = args[0]
        # Handle "string is <class> ?-strict? <value>"
        if subcmd == "is" and len(args) >= 3:
            class_name = args[1]
            is_key = _STRING_IS_IMPORT.get(class_name)
            if is_key is not None and is_key in self._shared_imports:
                func_idx = self._shared_imports[is_key]
                spec = _RUNTIME_IMPORTS[is_key]
                # The value is the last argument (skip optional -strict)
                val_arg = args[-1]
                self._emit_value(val_arg)
                self._emit_call(func_idx)
                if defs:
                    def_idx = self._intern_local(defs[0])
                    self._emit_local_set(def_idx)
                elif spec[3]:
                    self._emit(WasmOp.DROP)
                return
        import_key = _STRING_SUBCMD_IMPORT.get(subcmd)
        if import_key is not None and import_key in self._shared_imports:
            func_idx = self._shared_imports[import_key]
            spec = _RUNTIME_IMPORTS[import_key]
            param_count = len(spec[2])
            # Push sub-command args (skip the sub-command name itself)
            sub_args = args[1:]
            for i in range(min(param_count, len(sub_args))):
                self._emit_value(sub_args[i])
            # Pad missing args with null TclObj
            for _ in range(param_count - len(sub_args)):
                self._emit_i32_const(0)
            self._emit_call(func_idx)
            # Store result in def variable if present
            if defs:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            elif spec[3]:
                # Has return value but no def — drop it
                self._emit(WasmOp.DROP)
        else:
            self._emit_unsupported_trap(f"string {subcmd}")

    def _emit_cmd_dict(self, args: tuple[str, ...], defs: tuple[str, ...]) -> None:
        """``dict subcommand ...`` — dispatch to runtime import (i32 args)."""
        if not args:
            self._emit_unsupported_trap("dict (no subcommand)")
            return
        subcmd = args[0]
        import_key = _DICT_SUBCMD_IMPORT.get(subcmd)
        if import_key is not None and import_key in self._shared_imports:
            func_idx = self._shared_imports[import_key]
            spec = _RUNTIME_IMPORTS[import_key]
            param_count = len(spec[2])
            sub_args = args[1:]
            # dict set dictVar key value — mutates the variable
            if subcmd == "set" and len(sub_args) >= 3:
                var_idx = self._intern_local(sub_args[0])
                self._emit_local_get(var_idx)  # current dict value
                self._emit_value(sub_args[1])  # key
                self._emit_value(sub_args[2])  # value
                self._emit_call(func_idx)
                self._emit_local_set(var_idx)
                return
            # dict exists/get/keys/values/size — read-only access
            for i in range(min(param_count, len(sub_args))):
                self._emit_value(sub_args[i])
            for _ in range(param_count - len(sub_args)):
                self._emit_i32_const(0)
            self._emit_call(func_idx)
            if defs:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            elif spec[3]:
                self._emit(WasmOp.DROP)
        else:
            self._emit_unsupported_trap(f"dict {subcmd}")

    def _emit_array_subcmd_value(self, args: tuple[str, ...]) -> None:
        """``array <subcmd> <arr> ?args?`` — leaves i32 TclObj on the stack.

        Supported subcommands:
          exists, size, unset, names, set, get.  Others fall back to
          the interpreter, which will see the compile-time snapshot
          of the array's state (via globals — interpreter doesn't
          touch per-array tables yet).
        """
        if not args:
            self._emit_i32_const(0)
            return
        subcmd = args[0]
        if subcmd == "exists" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_exists")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                self._emit_call(fidx)
                return
        elif subcmd == "size" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_size")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                self._emit_call(fidx)
                return
        elif subcmd == "unset" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_unset")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                self._emit_call(fidx)
                return
        elif subcmd == "names" and len(args) >= 2:
            fidx = self._shared_imports.get("tcl_array_names")
            if fidx is not None:
                self._emit_array_name_obj(args[1])
                # Optional glob pattern — ``array names arr`` → no
                # filter (null TclObj), ``array names arr pat`` →
                # use the supplied pattern.  ``-exact`` / ``-glob``
                # / ``-regexp`` modes fall through to the fallback
                # for now; the common case scripts use is positional.
                if len(args) >= 3:
                    self._emit_value(args[2])
                else:
                    self._emit_i32_const(0)
                self._emit_call(fidx)
                return
        elif subcmd == "set" and len(args) >= 3:
            # ``array set arr {key val key val ...}`` — iterate the list
            # literal at compile time when possible, otherwise fall back
            # to the interpreter.  Most real-world usage is literal.
            self._emit_array_set_list(args[1], args[2])
            return
        elif subcmd == "get" and len(args) >= 2:
            # ``array get arr`` — return a flat {key val key val ...}
            # list.  Not implemented in the compiled runtime yet; fall
            # back to tcl_eval so the interpreter handles it (returns
            # empty for now, which degrades rather than mis-computes).
            pass
        self._emit_eval_fallback("array", args)

    def _emit_array_set_list(self, arr: str, kv_text: str) -> None:
        """``array set arr {k v k v ...}`` — compile-time list literal.

        Parses the inline list and emits one ``array_set`` call per
        pair.  Falls back to the interpreter for non-literal inputs.
        Leaves an empty string TclObj on the stack as the command's
        return value.
        """
        from ._helpers import _split_list_simple

        fidx = self._shared_imports.get("tcl_array_set")
        if fidx is None:
            self._emit_eval_fallback("array", ("set", arr, kv_text))
            return
        # Strip an optional outer braces around the literal list.
        text = kv_text
        if text.startswith("{") and text.endswith("}"):
            text = text[1:-1]
        try:
            words = _split_list_simple(text)
        except Exception:
            self._emit_eval_fallback("array", ("set", arr, kv_text))
            return
        if len(words) & 1:
            self._emit_eval_fallback("array", ("set", arr, kv_text))
            return
        for i in range(0, len(words), 2):
            self._emit_array_name_obj(arr)
            self._emit_value(words[i])
            self._emit_value(words[i + 1])
            self._emit_call(fidx)
            self._emit(WasmOp.DROP)
        # ``array set`` returns empty string.
        self._emit_obj_literal("")

    def _emit_unset_array_elems(self, args: tuple[str, ...]) -> bool:
        """Handle ``unset arr(key)`` forms.  Returns True if at least one
        array-element unset was emitted (and scalar unsets, if mixed in,
        were emitted as NOPs) — caller should ``return`` from the
        dispatch.  Returns False if *args* contains no array-element
        references at all.
        """
        had_array = False
        fidx = self._shared_imports.get("tcl_array_unset_element")
        # Skip ``-nocomplain`` / ``--`` option prefix.
        i = 0
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "--":
                i += 1
                break
            i += 1
        for name in args[i:]:
            ref = _parse_array_ref(name)
            if ref is None:
                continue
            had_array = True
            arr, key = ref
            if fidx is None:
                continue
            self._emit_array_name_obj(arr)
            self._emit_value(key)
            self._emit_call(fidx)
            self._emit(WasmOp.DROP)
        return had_array

    def _emit_list_value(self, args: tuple[str, ...]) -> None:
        """Build a Tcl list from *args* and leave it on the stack as a TclObj.

        Each arg is emitted with proper Tcl list-element encoding —
        empty strings become ``{}``, words containing whitespace /
        braces / brackets / ``"`` / ``\\`` / ``$`` / ``;`` get wrapped
        in braces so re-parsers see one element per arg.  This is
        critical: ``[list "a b" c]`` must produce a two-element list
        ``{a b} c``, not the three-word string ``a b c``.

        When every arg is a literal the result is folded at compile
        time into a single ``obj_new_string``; otherwise we emit a
        ``tcl_concat`` chain with single-space separators.  Variable
        and command-substitution args skip the element-quoting step
        — their runtime value is used as-is, matching the pre-
        existing ``tcl_list`` behaviour for interpolated values
        (quoting them at compile time would brace-wrap whatever
        runtime value they hold, changing semantics).
        """
        if not args:
            self._emit_i32_const(0)
            return
        # Fast path: all literals → compile-time string.
        # _tcl_token_value expands each source token to its VALUE (strips
        # braces, applies backslash subst), then _tcl_list_quote encodes
        # the value as a list element.
        if all(not a.startswith("$") and not a.startswith("[") for a in args):
            self._emit_obj_literal(
                " ".join(
                    _tcl_list_quote(_tcl_token_value(a), first=(i == 0)) for i, a in enumerate(args)
                )
            )
            return
        # Mixed: start with an empty list, lappend each arg so that
        # runtime values containing spaces are properly quoted as
        # single list elements (tcl_cmd_lappend wraps in {} if needed).
        lappend_idx = self._shared_imports.get("tcl_lappend")
        if lappend_idx is None:
            # No lappend helper — fall back to first arg only.
            self._emit_value(args[0])
            return

        def _emit_elem(a: str) -> None:
            # Braced literal — strip outer braces and emit the raw content;
            # tcl_cmd_lappend will re-quote it correctly if needed.
            if a.startswith("{") and a.endswith("}"):
                self._emit_obj_literal(a[1:-1])
                return
            if a.startswith("$") or a.startswith("["):
                self._emit_value(a)
            else:
                self._emit_obj_literal(a)

        self._emit_obj_literal("")  # empty list seed
        for a in args:
            _emit_elem(a)
            self._emit_call(lappend_idx)

    def _emit_cmd_uplevel(self, args: tuple[str, ...]) -> None:
        """``uplevel ?level? body`` — evaluate script in a caller's frame.

        ``level`` defaults to ``1``.  ``#0`` means absolute global
        scope; ``#N`` means N frames above global (Tcl only really
        uses ``#0``); a bare integer N means N frames up relative to
        the current frame.

        We emit:
            saved = frame_depth_stash(up)
            result = tcl_eval(body)
            frame_depth_restore(saved)

        ``frame_depth_stash`` clamps the shift at frame 0 so invalid
        levels degrade to global-scope eval rather than trapping.  If
        the caller chain is entirely compiled (no frames pushed), this
        is effectively a no-op and the eval runs at global scope — the
        ``#0`` behaviour Tcl scripts most commonly use.

        Multiple bodies concatenated as separate words (``uplevel 1 a b c``)
        are joined with spaces to form the script, matching Tcl's
        semantics of "concat all remaining arguments into a single
        script".
        """
        if not args:
            self._emit_i32_const(0)
            return
        # Parse the level specifier.  ``#0`` → absolute 0 which clamps
        # to global; a bare integer N → relative N; anything else means
        # no level, default 1, and args[0] is the first body word.
        level_spec = args[0]
        up: int
        body_start = 1
        if level_spec.startswith("#"):
            try:
                abs_level = int(level_spec[1:])
                # "#N means N frames above global".  We approximate by
                # stashing all the way to that absolute depth; frame_depth_stash
                # subtracts, so ``up`` is (current_depth - abs_level), which
                # we can't know at compile time.  For the common #0 case
                # we stash "all the way" by passing a large relative shift.
                up = 0 if abs_level == 0 else 1
            except ValueError:
                up = 1
        else:
            try:
                up = int(level_spec)
                # Numeric → relative.
            except ValueError:
                # Not a level — arg0 is part of the body.
                up = 1
                body_start = 0
        if body_start >= len(args):
            self._emit_i32_const(0)
            return

        body_parts = list(args[body_start:])

        eval_idx = self._shared_imports.get("tcl_eval")
        stash_idx = self._shared_imports.get("tcl_frame_depth_stash")
        restore_idx = self._shared_imports.get("tcl_frame_depth_restore")
        if eval_idx is None or stash_idx is None or restore_idx is None:
            # Missing runtime helpers — fall back to plain eval, which
            # still works for compiled-to-compiled chains where
            # frame_depth is 0 throughout.
            self._emit_uplevel_body(body_parts)
            if eval_idx is not None:
                self._emit_call(eval_idx)
            else:
                self._emit_i32_const(0)
            return

        # For ``#0`` we pass a large shift (INT32_MAX / 2) so
        # frame_depth_stash clamps to zero regardless of the actual depth.
        shift = 0x3FFF_FFFF if level_spec == "#0" else up
        saved_local = self._add_extra_local(prefix="_uplevel_saved", val_type=ValType.I32)

        self._emit_i32_const(shift)
        self._emit_call(stash_idx)
        self._emit_local_set(saved_local)

        self._emit_uplevel_body(body_parts)
        self._emit_call(eval_idx)
        # Result TclObj is on stack; stash temporarily so we can restore.
        result_local = self._add_extra_local(prefix="_uplevel_result", val_type=ValType.I32)
        self._emit_local_set(result_local)

        self._emit_local_get(saved_local)
        self._emit_call(restore_idx)

        self._emit_local_get(result_local)

    def _emit_uplevel_body(self, parts: list[str]) -> None:
        """Push the uplevel body as a single TclObj, resolving any
        ``$var``/``[cmd]`` substitutions the parts contain before
        handing the final string to ``tcl_eval``.

        Typical cases:
          - ``uplevel #0 {set ::g 42}``  — parts = ["set ::g 42"], a
            literal script.  Emitted as an obj_literal.
          - ``uplevel "set ::$name $val"`` — parts = ["set ::$name $val"],
            an interpolated string.  Emitted via ``_emit_value`` so the
            concat chain resolves ``$name``/``$val`` before eval.
          - ``uplevel 1 a b c`` — parts = ["a","b","c"], Tcl concat-join
            with spaces.  Each part is resolved and joined.
        """
        if not parts:
            self._emit_obj_literal("")
            return
        if len(parts) == 1:
            self._emit_value(parts[0])
            return
        # Multi-part: emit each value obj and concat with spaces.
        # Simplest: use _emit_value for each and chain tcl_append.
        append_idx = self._shared_imports.get("tcl_append")
        if append_idx is None:
            # No concat helper — best-effort: fall back to the first part
            # (typical callers supply a single braced script).
            self._emit_value(parts[0])
            return
        self._emit_value(parts[0])
        for part in parts[1:]:
            self._emit_obj_literal(" ")
            self._emit_call(append_idx)
            self._emit_value(part)
            self._emit_call(append_idx)

    def _emit_clock_value(self, args: tuple[str, ...]) -> None:
        """Emit a ``clock <subcmd>`` expression; leaves i32 TclObj on stack.

        ``seconds``/``clicks``/``milliseconds`` call the WASI-backed
        runtime helpers directly.  Anything else falls through to the
        interpreter (which will likely trap for ``format``/``scan`` in
        the sandbox — that's fine as a clear diagnostic until we ship a
        timezone-aware formatter).
        """
        if not args:
            self._emit_i32_const(0)
            return
        subcmd = args[0]
        import_key = _CLOCK_SUBCMD_IMPORT.get(subcmd)
        if import_key is not None:
            func_idx = self._shared_imports.get(import_key)
            if func_idx is not None:
                self._emit_call(func_idx)
                return
        # Fall back to the interpreter for unsupported subcommands.
        self._emit_eval_fallback("clock", args)

    def _emit_cmd_lassign(
        self,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        *,
        keep_on_stack: bool,
    ) -> None:
        """``lassign list ?varName ...?`` — destructure a list into vars.

        For each ``varName`` at position *i*, assigns ``list_index(list, i)``
        to that variable (empty string if out of range).  Returns the
        leftover list (elements beyond the supplied variables) — computed
        at runtime via ``tcl_list_tail``.

        If *keep_on_stack* is True (value/expression context), the
        leftover-list i32 TclObj is left on the stack; otherwise the
        result is stored in ``defs[0]`` if given, or dropped.
        """
        if not args:
            self._emit_unsupported_trap("lassign (no list arg)")
            return
        list_arg = args[0]
        var_names = args[1:]

        # Stash the list value once so we can index into it repeatedly
        # without re-evaluating its expression (which may have side effects
        # via command substitution).
        list_local = self._add_extra_local(prefix="_lassign_list", val_type=ValType.I32)
        self._emit_value(list_arg)
        self._emit_local_set(list_local)

        # Per-variable: write list_index(list, i) into the named variable.
        lindex_idx = self._shared_imports.get("tcl_list_index")
        if lindex_idx is None:
            # Can't emit without the runtime helper — fall back to trap.
            self._emit_unsupported_trap("lassign (missing tcl_list_index)")
            return

        for i, var_name in enumerate(var_names):
            self._emit_local_get(list_local)
            # Index argument is a TclObj holding the integer i.
            self._emit_obj_literal(str(i))
            self._emit_call(lindex_idx)
            self._emit_var_write_obj(var_name)

        # Produce the leftover list (elements from index len(var_names) on).
        ltail_idx = self._shared_imports.get("tcl_list_tail")
        if ltail_idx is None:
            # No tail helper — emit empty list.
            self._emit_i32_const(0)
        else:
            self._emit_local_get(list_local)
            self._emit_obj_literal(str(len(var_names)))
            self._emit_call(ltail_idx)

        if keep_on_stack:
            return
        # Statement context: the leftover list is discarded.  ``defs`` for
        # lassign lists the VAR_WRITE targets (var_names), NOT a place to
        # store the command's return value — ignore it here to avoid
        # overwriting one of the just-assigned locals.
        self._emit(WasmOp.DROP)

    def _emit_cmd_info(self, args: tuple[str, ...], defs: tuple[str, ...]) -> None:
        """``info subcommand ?arg?`` in statement context.

        Emits the subcommand's value, then stores to ``defs[0]`` if given
        or drops.
        """
        if not args:
            self._emit_unsupported_trap("info (no subcommand)")
            return
        self._emit_info_value(args)
        if defs:
            def_idx = self._intern_local(defs[0])
            self._emit_local_set(def_idx)
        else:
            self._emit(WasmOp.DROP)

    def _emit_info_value(self, args: tuple[str, ...]) -> None:
        """Leave the i32 TclObj result of ``info <args>`` on the stack.

        ``info exists varName`` is inlined so that compile-time scope
        information (alias bindings, ``::``-qualified globals) informs the
        lookup; everything else falls through to the runtime
        ``info_dispatch`` helper for subcommands it understands.
        """
        if not args:
            self._emit_i32_const(0)
            return
        subcmd = args[0]

        # ``info level 0`` — return the current proc's invocation as
        # a list (proc-name + all args).  tcltest's ``outputChannel``
        # uses ``[llength [info level 0]] == 1`` as a "was I called
        # without a filename arg?" check; without a real impl the
        # runtime's ``info_dispatch`` returns empty, llength = 0, and
        # the check falls through to the ``open $filename`` branch
        # that traps on the unsupported ``open`` stub.
        #
        # We know the proc's param list at compile time, so emit a
        # list whose length tracks the real invocation: one element
        # per declared parameter, space-joined (elements aren't
        # list-quoted because we only care about the llength here —
        # callers that inspect element contents via ``lindex`` would
        # need proper quoting; extend then).  The proc name itself
        # is the first element.
        if (
            subcmd == "level"
            and len(args) == 2
            and args[1] == "0"
            and self._is_proc
            and self._proc_qname is not None
        ):
            append_idx = self._shared_imports.get("tcl_append")
            # proc-name as the first word (use the unqualified tail
            # rather than the fully qualified name — tcltest's
            # ``outputChannel`` call-site compares against the
            # command name a script would've used, not its
            # namespace-qualified form).
            tail = self._proc_qname.rsplit("::", 1)[-1] or self._proc_qname
            self._emit_obj_literal(tail)
            if append_idx is not None:
                for pname in self._params:
                    # Space separator + param value.
                    self._emit_obj_literal(" ")
                    self._emit_call(append_idx)
                    # Read the local — parameters are the first
                    # N locals in a compiled proc.
                    idx = self._local_index.get(pname)
                    if idx is None:
                        # Shouldn't happen for param names, but fall
                        # back to empty string.
                        self._emit_obj_literal("")
                    else:
                        self._emit_local_get(idx)
                    self._emit_call(append_idx)
            return

        if subcmd == "exists" and len(args) >= 2:
            # ``info exists var`` — resolve the variable reference with
            # compile-time alias info so upvar'd / variable'd / ``::``-
            # qualified names hit the global table rather than searching
            # for an unrelated local.
            var = args[1]
            # ``info exists $dynamicName`` / ``info exists [cmd]`` —
            # the NAME of the variable to check is produced at runtime
            # from a substitution.  Resolve the name value at runtime
            # and dispatch through the runtime ``info_exists`` helper
            # so the check follows the name's actual string, not the
            # literal text after ``$``.
            is_dynamic = (var.startswith("$") or var.startswith("[")) and _parse_array_ref(
                var
            ) is None
            if is_dynamic:
                info_exists_idx = self._shared_imports.get("tcl_info_exists")
                if info_exists_idx is not None:
                    self._emit_value(var)
                    self._emit_call(info_exists_idx)
                    return
                # Helper missing — fall through to the interpreter so
                # the answer is correct even if less efficient.
                self._emit_eval_fallback("info", args)
                return
            # Normalise ``$name`` / ``${name}`` just in case the lowerer
            # hasn't stripped the sigil.
            resolved = self._resolve_var_name(var) or var
            # ``info exists arr(key)`` — array element lookup.  The array
            # name itself may be alias-bound (upvar/variable) so resolve
            # it through _emit_array_name_obj, then probe the element
            # table via tcl_array_element_exists.
            array_ref = _parse_array_ref(resolved)
            if array_ref is not None:
                elem_idx = self._shared_imports.get("tcl_array_element_exists")
                if elem_idx is not None:
                    arr, key = array_ref
                    self._emit_array_name_obj(arr)
                    self._emit_value(key)
                    self._emit_call(elem_idx)
                    return
                # Runtime helper missing — conservative false.
                self._emit_i64_const(0)
                self._emit_box_int()
                return
            gexist_idx = self._shared_imports.get("tcl_global_exists")
            binding = self._aliases.get(resolved)
            if binding is not None and binding[0] == "global" and gexist_idx is not None:
                _, target_idx = binding
                self._emit_local_get(target_idx)
                self._emit_call(gexist_idx)
                return
            if resolved.startswith("::") and gexist_idx is not None:
                self._emit_obj_literal(resolved)
                self._emit_call(gexist_idx)
                return
            # FRAME-only vars (routed through tcl_local_set by the
            # escape-aware writer) don't land in a WASM local, so the
            # pointer-nullness check below would always say "no".
            # Dispatch through the runtime ``tcl_info_exists`` helper,
            # which probes the frame table by name and returns a
            # boxed TclObj 0/1 (matching the other existence paths).
            if self._is_frame_only_var(resolved):
                info_exists_idx = self._shared_imports.get("tcl_info_exists")
                if info_exists_idx is not None:
                    self._emit_obj_literal(resolved)
                    self._emit_call(info_exists_idx)
                    return
            # Plain proc-local: the compiled proc uses WASM locals
            # that are zero-initialised, so "exists" is approximated
            # as "value pointer is non-null" — matches writes that
            # land through _emit_var_write_obj.
            idx = self._local_index.get(resolved)
            if idx is None:
                # Never referenced — always non-existent.  Emit boxed 0.
                self._emit_i64_const(0)
                self._emit_box_int()
                return
            self._emit_local_get(idx)
            self._emit_i32_const(0)
            self._emit(WasmOp.I32_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
            self._emit_box_int()
            return

        # Subcommands dispatched via the runtime's info_dispatch helper.
        info_dispatch_idx = self._shared_imports.get("tcl_info_dispatch")
        if info_dispatch_idx is not None and subcmd in ("body", "args", "exists"):
            self._emit_obj_literal(subcmd)
            if len(args) >= 2:
                self._emit_value(args[1])
            else:
                self._emit_i32_const(0)
            self._emit_call(info_dispatch_idx)
            return

        # Unknown subcommand — fall back to the interpreter.
        self._emit_eval_fallback("info", args)

    def _emit_cmd_runtime(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...],
    ) -> None:
        """Emit a call to an imported runtime function for a known command."""
        import_key, expected_argc = _CMD_RUNTIME[command]
        func_idx = self._shared_imports.get(import_key)
        if func_idx is None:
            self._emit_unsupported_trap(command)
            return

        # Record a diag site for *every* runtime-dispatched command so
        # a trap inside the runtime (stubs raising ``unsupported
        # command: X``, ``lappend`` erroring on a non-list value, etc.)
        # is attributable to the right source location.  Three WASM
        # bytes per call; negligible against the alternative of
        # silent misattribution.
        self._emit_diag_site(command, args=args, kind="runtime")

        spec = _RUNTIME_IMPORTS[import_key]
        param_count = len(spec[2])

        # For commands like append/lappend that mutate a variable,
        # the first arg is the variable name; the runtime receives the
        # current variable value + one new value and returns the
        # updated value.  ``append x a b c`` concatenates a, b, c onto
        # x in order; ``lappend x a b c`` appends each as a separate
        # element.  Both loop per-value and store the running result
        # back between each call.
        mutates_var = command in ("append", "lappend")
        if mutates_var and len(args) >= 2:
            var_name = args[0]
            var_idx = self._intern_local(var_name)
            for value_arg in args[1:]:
                self._emit_local_get(var_idx)  # current value
                self._emit_value(value_arg)  # value to append
                self._emit_call(func_idx)
                self._emit_local_set(var_idx)
            return

        # ``list`` handled outside _emit_cmd_runtime — see
        # ``_emit_list_value`` for the N-arg build.

        # ``format`` takes a format string + up to 3 arg TclObjs —
        # fewer args get zero-padded so the runtime sees empty slots.
        if command == "format":
            if not args:
                self._emit_i32_const(0)
                self._emit_i32_const(0)
                self._emit_i32_const(0)
                self._emit_i32_const(0)
            else:
                self._emit_value(args[0])
                for slot in range(1, 4):
                    if slot < len(args):
                        self._emit_value(args[slot])
                    else:
                        self._emit_i32_const(0)
            self._emit_call(func_idx)
            if spec[3]:
                if defs:
                    def_idx = self._intern_local(defs[0])
                    self._emit_local_set(def_idx)
                else:
                    # Keep on stack if result matters (value context)
                    # — here we're in statement context so drop.
                    self._emit(WasmOp.DROP)
            return

        # fconfigure takes a fd + a variable number of ``-option value``
        # pairs; the runtime fn has a 2-arg signature (fd, opts_obj),
        # so pack args[1:] into a single space-joined string literal
        # here and hand the resulting TclObj to the stub.  For
        # arguments that are variable references (``$value``) we
        # can't fold at compile time; in that case we emit a
        # ``tcl_concat`` chain at runtime.  Most fconfigure call
        # sites in the wild use only literal option names and
        # literal values, so the fast path is common.
        if command == "fconfigure":
            if not args:
                self._emit_i32_const(0)
                self._emit_i32_const(0)
            else:
                self._emit_value(args[0])
                rest = args[1:]
                if not rest:
                    self._emit_i32_const(0)
                elif all(not a.startswith("$") and not a.startswith("[") for a in rest):
                    self._emit_obj_literal(" ".join(rest))
                else:
                    # Mixed literals + refs — build via repeated
                    # ``tcl_concat(acc, " word")``.  tcl_concat is
                    # always imported via the lifecycle set.
                    concat_idx = self._shared_imports.get("tcl_concat")
                    if concat_idx is None:
                        self._emit_obj_literal(" ".join(rest))
                    else:
                        self._emit_obj_literal(rest[0])
                        for word in rest[1:]:
                            self._emit_obj_literal(" ")
                            self._emit_call(concat_idx)
                            self._emit_value(word)
                            self._emit_call(concat_idx)
            self._emit_call(func_idx)
            if spec[3]:
                if defs:
                    def_idx = self._intern_local(defs[0])
                    self._emit_local_set(def_idx)
                else:
                    self._emit(WasmOp.DROP)
            return

        # For puts, handle optional channel argument: puts ?-nonewline? ?channelId? string
        if command == "puts":
            # Use the last argument as the string value
            if args:
                self._emit_value(args[-1])
            else:
                self._emit_i32_const(0)
            self._emit_call(func_idx)
            if spec[3]:
                # puts returns empty string — drop
                self._emit(WasmOp.DROP)
            return

        # concat: variadic — Tcl concat trims whitespace from each arg and
        # joins non-empty results with a single space.
        if command == "concat":

            def _concat_is_lit(a: str) -> bool:
                return (
                    not a.startswith("$")
                    and not a.startswith("[")
                    and not self._has_embedded_subst(a)
                    and a not in self._aliases
                    and a not in self._local_index
                )

            all_lits = all(_concat_is_lit(a) for a in args)
            if not args:
                self._emit_obj_literal("")
            elif all_lits:
                # Compile-time: trim each IR value (already de-braced and
                # backslash-substituted by the lexer), drop empties, join.
                parts = [a.strip() for a in args]
                self._emit_obj_literal(" ".join(p for p in parts if p))
            else:
                # Mixed literals + runtime values: chain tcl_cmd_concat calls.
                # tcl_cmd_concat trims each arg's whitespace before joining.
                self._emit_value(args[0])
                for a in args[1:]:
                    self._emit_value(a)
                    self._emit_call(func_idx)
            if defs and spec[3]:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            elif spec[3]:
                self._emit(WasmOp.DROP)
            return

        # Generic: push args up to param_count (all i32 TclObj pointers)
        for i in range(min(param_count, len(args))):
            self._emit_value(args[i])
        # Pad missing args with null TclObj
        for _ in range(param_count - len(args)):
            self._emit_i32_const(0)

        self._emit_call(func_idx)

        # Store result in def variable if present
        if defs and spec[3]:
            def_idx = self._intern_local(defs[0])
            self._emit_local_set(def_idx)
        elif spec[3]:
            self._emit(WasmOp.DROP)

    def _emit_cmd_proc_call(
        self,
        func_idx: int,
        n_params: int,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        qname: str | None = None,
        tokens: "CommandTokens | None" = None,
    ) -> None:
        """Emit a direct call to a compiled procedure.

        Enforces WASM arity: pushes exactly *n_params* i32 TclObj
        pointers onto the stack — padding with the proc's declared
        defaults (via ``_proc_defaults``) for missing args, or
        boxed zero when a param has no default.  Surplus args are
        dropped (matches Tcl's behaviour for procs without ``args``
        catchall).

        When the callee has an ``args`` variadic tail (its qname is in
        ``_proc_args_tail``), all call-site args beyond the fixed
        positional count are packed into a single list TclObj and
        passed as the last argument.

        *tokens* provides the original parsed tokens for each call-site
        word, letting us distinguish braced (``{…}``) args from
        unbraced ones.  Braced args must skip backslash / interpolation
        substitution at emit time (Tcl semantics: braces suppress all
        substitution).
        """
        defaults: tuple[str | None, ...] = ()
        if qname is not None:
            defaults = self._proc_defaults.get(qname, ())

        has_args_tail = qname is not None and qname in self._proc_args_tail

        def _was_braced(call_arg_idx: int) -> bool:
            """Return True if the call-site word at *call_arg_idx* was braced.

            *call_arg_idx* is 0-based into *args* (the CALLSITE args tuple),
            so ``tokens.argv`` is indexed at ``call_arg_idx + 1`` to skip
            the command-name token.
            """
            if tokens is None or tokens.argv is None:
                return False
            tok_idx = call_arg_idx + 1
            if tok_idx >= len(tokens.argv):
                return False
            return tokens.argv[tok_idx].type == TokenType.STR

        if has_args_tail and n_params > 0:
            # Fixed positional slots: first n_params-1
            fixed = n_params - 1
            for i in range(min(fixed, len(args))):
                self._emit_value(args[i], was_braced=_was_braced(i))
            # Pad missing fixed slots with defaults / null
            for slot in range(len(args), fixed):
                default = defaults[slot] if slot < len(defaults) else None
                if default is None:
                    self._emit_default_arg()
                else:
                    self._emit_obj_literal(default)
            # Last slot: list of all remaining call args.  Pass a
            # per-tail-arg ``was_braced`` probe so braced tokens in
            # the args tail keep their exact source bytes (no
            # backslash subst) when packed into the list.
            tail_args = args[fixed:]
            self._emit_args_list(
                tail_args,
                was_braced_fn=lambda i, base=fixed: _was_braced(base + i),
            )
        else:
            # Push arguments up to the callee's parameter count
            for i in range(min(n_params, len(args))):
                self._emit_value(args[i], was_braced=_was_braced(i))
            # Pad missing args with the declared default or boxed zero.
            # Defaults are LITERAL values from the param spec — ``{$y}``
            # and ``{[clock seconds]}`` must reach the callee unchanged,
            # *not* be substituted at call time.  Emit as an obj literal
            # so no ``$var`` / ``[cmd]`` interpolation happens.
            for slot in range(len(args), n_params):
                default = defaults[slot] if slot < len(defaults) else None
                if default is None:
                    self._emit_default_arg()
                else:
                    self._emit_obj_literal(default)

        self._emit_call(func_idx)

        # Store result in def variable if present
        if defs:
            def_idx = self._intern_local(defs[0])
            self._emit_local_set(def_idx)
        else:
            # Drop unused return value in statement context
            self._emit(WasmOp.DROP)

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
        self._emit_script(init)

        # Structure: block{ loop{ <cond>; block{ <body> }; <next>; br 0 } }
        # continue → br to inner block end (skips rest of body, runs next)
        # break → br to outer block (exits loop entirely)
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # break target
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))  # loop restart

        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)  # break if false

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._emit_script(body)
        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        self._emit_script(next_script)
        self._emit_br(0)  # back to loop

        self._emit(WasmOp.END)  # end loop
        self._emit(WasmOp.END)  # end break block

    def _emit_while(self, condition: ExprNode, body: IRScript) -> None:
        """Emit a while loop."""
        # For while, continue just restarts from the condition.
        # Structure: block{ loop{ <cond>; block{ <body> }; br 0 } }
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._emit_script(body)
        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

    def _emit_foreach(
        self,
        iterators: tuple[tuple[tuple[str, ...], str], ...],
        body: IRScript,
    ) -> None:
        """Emit a foreach loop using real list element access.

        Calls ``list_length`` to get the element count, then iterates
        with a counter calling ``list_index`` on each iteration to
        extract the element and assign it to the loop variable.

        Internal counter/limit are i64; the loop variable and list
        are i32 TclObj pointers.
        """
        counter = self._add_extra_local("_foreach_i")
        limit = self._add_extra_local("_foreach_n")
        list_local = self._add_extra_local("_foreach_list", ValType.I32)

        first_vars, first_list = iterators[0]

        # Store the list TclObj for repeated list_index calls
        self._emit_value(first_list)
        self._emit_local_set(list_local)

        # limit = list_length(list) — unbox to i64
        llength_idx = self._shared_imports.get("tcl_list_length")
        if llength_idx is not None:
            self._emit_local_get(list_local)
            self._emit_call(llength_idx)
            self._emit_unbox_int()
        else:
            # Fallback: use integer value as count
            self._emit_local_get(list_local)
            self._emit_unbox_int()
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

        # element = list_index(list, counter)
        lindex_idx = self._shared_imports.get("tcl_list_index")
        if first_vars:
            var_local = self._intern_local(first_vars[0])
            if lindex_idx is not None:
                self._emit_local_get(list_local)
                self._emit_local_get(counter)
                self._emit_box_int()  # index as TclObj
                self._emit_call(lindex_idx)
            else:
                # Fallback: use counter as element value
                self._emit_local_get(counter)
                self._emit_box_int()
            self._emit_local_set(var_local)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._emit_script(body)
        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        # counter++
        self._emit_local_get(counter)
        self._emit_i64_const(1)
        self._emit(WasmOp.I64_ADD)
        self._emit_local_set(counter)

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

    def _emit_switch(self, subject: str, arms, default_body, *, mode: str = "exact") -> None:
        """Emit a switch statement as a chain of if/else.

        Uses ``string_equal`` for exact matching and ``string_match``
        for glob matching.  Falls back to i64 integer comparison when
        the runtime imports are unavailable.
        """
        # Determine comparison function
        if mode == "glob":
            cmp_import = self._shared_imports.get("tcl_string_match")
        else:
            cmp_import = self._shared_imports.get("tcl_string_equal")

        if cmp_import is not None:
            # String-based comparison: subject is i32 TclObj
            subject_local = self._add_extra_local(f"_switch_{subject}", ValType.I32)
            self._emit_value(subject)
            self._emit_local_set(subject_local)

            arm_count = len(arms)
            for i, arm in enumerate(arms):
                if arm.body is None:
                    continue
                if arm.pattern == "default":
                    # default arm handled after all others
                    continue
                # Compare: for glob, args are (pattern, subject)
                # For exact, args are (subject, pattern)
                if mode == "glob":
                    self._emit_obj_literal(arm.pattern)
                    self._emit_local_get(subject_local)
                else:
                    self._emit_local_get(subject_local)
                    self._emit_obj_literal(arm.pattern)
                self._emit_call(cmp_import)
                # string_equal/string_match returns TclObj wrapping 0 or 1
                self._emit_unbox_int()
                self._emit(WasmOp.I32_WRAP_I64)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                self._emit_script(arm.body)
                if i < arm_count - 1 or default_body:
                    self._emit(WasmOp.ELSE)

            if default_body:
                self._emit_script(default_body)

            # Close all if blocks
            for arm in arms:
                if arm.body is not None and arm.pattern != "default":
                    self._emit(WasmOp.END)
        else:
            # Fallback: integer comparison
            subject_local = self._add_extra_local(f"_switch_{subject}")
            self._emit_value(subject)
            self._emit_unbox_int()
            self._emit_local_set(subject_local)

            arm_count = len(arms)
            for i, arm in enumerate(arms):
                if arm.body is None:
                    continue
                self._emit_local_get(subject_local)
                self._emit_literal(arm.pattern)
                self._emit(WasmOp.I64_EQ)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                self._emit_script(arm.body)
                if i < arm_count - 1 or default_body:
                    self._emit(WasmOp.ELSE)

            if default_body:
                self._emit_script(default_body)

            for arm in arms:
                if arm.body is not None:
                    self._emit(WasmOp.END)

    def _emit_catch(
        self,
        body: IRScript,
        result_var: str | None,
        *,
        keep_on_stack: bool = False,
    ) -> None:
        """Emit a catch block with error-flag based error propagation.

        Inside the catch body, ``error()`` sets a flag and returns
        instead of trapping.  After each statement we check the flag
        and break out of the body block if an error occurred.

        If *keep_on_stack* is True, the catch return code (i32 TclObj
        wrapping 0 or 1) is left on the WASM stack for implicit return.
        """
        enter_idx = self._shared_imports.get("tcl_catch_enter")
        leave_idx = self._shared_imports.get("tcl_catch_leave")
        result_idx = self._shared_imports.get("tcl_catch_result")
        has_error_idx = self._shared_imports.get("tcl_catch_has_error")

        if enter_idx is None or leave_idx is None:
            self._emit_script(body)
            if result_var:
                idx = self._intern_local(result_var)
                self._emit_i32_const(0)
                self._emit_local_set(idx)
            if keep_on_stack:
                self._emit_i32_const(0)
            return

        # Enter catch scope
        self._emit_call(enter_idx)

        # Body inside a block — error check after each statement breaks out.
        # For the LAST statement, when a result variable is needed, emit it in
        # "keep result" mode and record the value via catch_set_ok_result so
        # that catch_result() returns the body's last-command value on success.
        set_ok_idx = self._shared_imports.get("tcl_catch_set_ok_result")
        capture_last = (
            result_var is not None
            and result_idx is not None
            and set_ok_idx is not None
            and len(body.statements) > 0
        )
        self._catch_depth += 1
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        stmts = body.statements
        for i, stmt in enumerate(stmts):
            is_last = capture_last and (i == len(stmts) - 1)
            if is_last:
                # Emit in "keep result" mode, then record for catch_result().
                assert set_ok_idx is not None  # guaranteed by capture_last
                self._emit_stmt_keep_result(stmt)
                self._emit_call(set_ok_idx)  # consumes the i32 result
            else:
                self._emit_stmt(stmt)
            if has_error_idx is not None:
                self._emit_call(has_error_idx)
                self._emit_br_if(0)
        self._emit(WasmOp.END)
        self._catch_depth -= 1

        # Leave catch scope — get return code (0=OK, 1=ERROR)
        self._emit_call(leave_idx)
        if keep_on_stack:
            # Leave the i32 TclObj on stack for implicit return
            pass
        else:
            self._emit(WasmOp.DROP)

        # Store result/error message if requested
        if result_var and result_idx is not None:
            rv_idx = self._intern_local(result_var)
            self._emit_call(result_idx)
            self._emit_local_set(rv_idx)

    def _emit_stmt_keep_result(self, stmt: "IRStatement") -> None:
        """Emit a statement leaving its i32 TclObj result on the WASM stack.

        Used by ``_emit_catch`` to capture the last body statement's value
        for the ``catch body result`` result variable.  Unlike
        ``_emit_stmt``, this variant does NOT drop the result.

        For statements that naturally produce no value (declarations,
        ``unset``, etc.) we push a null TclObj (0) as the result.
        """
        from ..ir import (
            IRBarrier,
            IRCall,
            IRExprEval,
            IRIncr,
        )

        match stmt:
            case IRCall(command=command, args=args, defs=defs, tokens=tokens):
                if (
                    tokens is not None
                    and tokens.expand_word is not None
                    and any(tokens.expand_word)
                    and tokens.argv_texts
                ):
                    # {*} expansion — eval fallback leaves result on stack.
                    ew = tokens.expand_word
                    parts = [
                        (f"{{*}}{t}" if (i < len(ew) and ew[i]) else t)
                        for i, t in enumerate(tokens.argv_texts)
                    ]
                    script = " ".join(parts)
                    self._emit_eval_fallback(command, args, script_override=script)
                    # result is on stack; no DROP here
                else:
                    self._emit_call_stmt_tail(command, args, defs)
            case IRBarrier(command=barrier_cmd, args=barrier_args, reason=reason):
                if barrier_cmd:
                    self._emit_eval_fallback(barrier_cmd, barrier_args)
                    # result is on stack; no DROP
                else:
                    self._emit_eval_fallback(reason)
                    # result is on stack; no DROP
            case IRExprEval(expr=expr):
                self._emit_expr(expr)
                self._emit_box_int()
            case IRIncr(name=name, amount=amount):
                # Mimic the incr emitter (keep value on stack)
                self._emit_var_read_obj(name)
                self._emit_unbox_int()
                amt: int | None = 1
                if amount is not None:
                    try:
                        amt = int(amount)
                    except ValueError:
                        self._emit_value(amount)
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        amt = None  # signal: already handled
                if amt is not None:
                    self._emit_i64_const(amt)
                    self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                self._emit_var_write_obj_keep(name)
            case _:
                # All other statement types (assign, scoping, etc.):
                # emit normally, then push a null result.
                self._emit_stmt(stmt)
                self._emit_i32_const(0)

    def _emit_catch_from_args(
        self,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        *,
        keep_on_stack: bool = False,
    ) -> None:
        """Emit catch from the CFG's IRCall("catch", (body_text, ...)).

        Re-parses the body text to IR and delegates to ``_emit_catch``.
        If *keep_on_stack* is True, the catch return code (i32 TclObj)
        is left on the WASM stack for implicit return.
        """
        body_text = args[0].strip() if args else ""
        # Strip outer braces preserved by _split_command_subst so
        # lower_to_ir receives the raw script, not "{script}".
        if body_text.startswith("{") and body_text.endswith("}"):
            body_text = body_text[1:-1]
        result_var = args[1] if len(args) >= 2 else None

        if not body_text:
            enter_idx = self._shared_imports.get("tcl_catch_enter")
            leave_idx = self._shared_imports.get("tcl_catch_leave")
            if enter_idx is not None and leave_idx is not None:
                self._emit_call(enter_idx)
                self._emit_call(leave_idx)
                if not keep_on_stack:
                    self._emit(WasmOp.DROP)
            elif keep_on_stack:
                self._emit_i32_const(0)
            return

        from ...compiler.lowering import lower_to_ir

        try:
            ir_module = lower_to_ir(body_text)
            body_script = ir_module.top_level
        except Exception:
            self._emit_unsupported_trap("catch (unparseable body)")
            return

        self._emit_catch(body_script, result_var, keep_on_stack=keep_on_stack)

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

    def _is_loop_header(self, header: str, body_start: str) -> bool:
        """Check if *header* is a loop header with a back-edge from the body.

        Returns ``True`` when following goto/branch edges from *body_start*
        reaches *header* again, indicating a loop back-edge.

        Stops the walk at any block in ``self._active_loop_headers``: if
        we reach an OUTER loop's header, we have followed the outer
        loop's back-edge and this is not a new nested loop.
        """
        visited: set[str] = set()
        worklist = [body_start]
        while worklist:
            name = worklist.pop()
            if name == header:
                return True
            if name in visited:
                continue
            # Outer loop header — stop the walk here so we don't wrap
            # around through the outer loop back to `header`.
            if name in self._active_loop_headers and name != header:
                continue
            visited.add(name)
            blk = self._cfg.blocks.get(name)
            if blk is None:
                continue
            match blk.terminator:
                case CFGGoto(target=t):
                    worklist.append(t)
                case CFGBranch(true_target=tt, false_target=ft):
                    worklist.append(tt)
                    worklist.append(ft)
        return False

    def _emit_cfg_loop(
        self,
        header: str,
        condition: ExprNode,
        body_start: str,
        exit_block: str,
    ) -> None:
        """Emit a WASM block/loop for a CFG for/while loop pattern.

        Structure: ``block { loop { br_if(exit) ; body ; br(loop) } }``
        The header block is the loop test; the body is everything
        reachable from *body_start* that eventually loops back to *header*.
        """
        # Mark the header as visited to prevent re-entry from the back-edge
        self._visited.add(header)
        # Track header so nested _is_loop_header walks don't confuse
        # outer back-edges with new nested loops.
        self._active_loop_headers.add(header)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        # Evaluate loop condition — break if false
        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)  # break out of block

        # Wrap body in a block for break/continue support
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)

        # Emit body blocks
        self._emit_loop_body(body_start, header)

        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        # Loop back
        self._emit_br(0)
        self._emit(WasmOp.END)  # end loop
        self._emit(WasmOp.END)  # end block

        self._active_loop_headers.discard(header)

        # Continue with the exit block
        self._emit_block(exit_block)

    def _emit_loop_body(self, start: str, header: str) -> None:
        """Emit all blocks reachable from *start* until reaching *header*.

        Follows goto chains linearly.  When a CFGBranch is encountered,
        checks whether it is a nested loop (back-edge from the true
        branch) and emits ``block{loop{...}}`` for it.  Plain branches
        (if/else) are emitted with ``if/else/end``.  Stops when a goto
        target is the outer loop's *header* (the back-edge).
        """
        current = start
        while current and current != header:
            if current in self._visited:
                return
            self._visited.add(current)

            blk = self._cfg.blocks.get(current)
            if blk is None:
                return

            # Emit statements
            for stmt in blk.statements:
                self._emit_stmt(stmt)

            match blk.terminator:
                case CFGGoto(target=target):
                    current = target
                case CFGBranch(condition=cond, true_target=tt, false_target=ft):
                    # Check for nested loop: the true-target body
                    # eventually loops back to this block.
                    if self._is_loop_header(current, tt):
                        # Emit a nested WASM loop for this inner header.
                        self._active_loop_headers.add(current)
                        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
                        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

                        self._emit_expr(cond)
                        self._emit_i64_const(0)
                        self._emit(WasmOp.I64_EQ)
                        self._emit_br_if(1)  # break if false

                        # Emit inner loop body — stops at current (inner back-edge)
                        self._emit_loop_body(tt, current)

                        self._emit_br(0)  # loop back
                        self._emit(WasmOp.END)  # end loop
                        self._emit(WasmOp.END)  # end block
                        self._active_loop_headers.discard(current)

                        # Continue with the false-target (loop exit)
                        current = ft
                    else:
                        # Plain branch (if/else inside loop body)
                        merge = self._find_merge_block(tt, ft)
                        if merge is not None:
                            self._visited.add(merge)

                        self._emit_expr(cond)
                        self._emit_i64_const(0)
                        self._emit(WasmOp.I64_NE)
                        self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                        self._emit_loop_body(tt, header)
                        self._emit(WasmOp.ELSE)
                        self._emit_loop_body(ft, header)
                        self._emit(WasmOp.END)

                        if merge is not None:
                            self._visited.discard(merge)
                            current = merge
                        else:
                            return
                case CFGReturn(value=value):
                    if value is not None:
                        self._emit_value(value)
                    else:
                        self._emit_i32_const(0)
                    self._emit(WasmOp.RETURN)
                    return
                case _:
                    return

    def _emit_foreach_loop(
        self,
        header_stmts: tuple,
        body_block: str,
        end_block: str,
        *,
        header_block: str = "",
    ) -> None:
        """Emit a list-iteration loop for a foreach CFG pattern.

        Uses ``list_length`` to get element count and ``list_index`` to
        extract each element.  Falls back to counter-as-value when the
        runtime imports are unavailable.

        Internal counter/limit are i64; the loop variable is i32 TclObj.
        """
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
        list_local = self._add_extra_local("_foreach_list", ValType.I32)

        # Store the list TclObj for repeated list_index calls
        if list_var is not None:
            self._emit_value(list_var)
        else:
            self._emit_i32_const(0)
        self._emit_local_set(list_local)

        # limit = list_length(list)
        llength_idx = self._shared_imports.get("tcl_list_length")
        if llength_idx is not None:
            self._emit_local_get(list_local)
            self._emit_call(llength_idx)
            self._emit_unbox_int()
        else:
            self._emit_local_get(list_local)
            self._emit_unbox_int()
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

        # element = list_index(list, counter)
        lindex_idx = self._shared_imports.get("tcl_list_index")
        if loop_var is not None:
            var_local = self._intern_local(loop_var)
            if lindex_idx is not None:
                self._emit_local_get(list_local)
                self._emit_local_get(counter)
                self._emit_box_int()
                self._emit_call(lindex_idx)
            else:
                self._emit_local_get(counter)
                self._emit_box_int()
            self._emit_local_set(var_local)

        # Wrap body in block for break/continue
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)

        # Mark the foreach header as an active outer loop so that
        # _is_loop_header doesn't treat the header→body back-edge as a
        # new nested loop when it walks successor blocks of the body.
        if header_block:
            self._active_loop_headers.add(header_block)
        self._emit_block(body_block)
        if header_block:
            self._active_loop_headers.discard(header_block)

        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

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

        # Detect implicit return pattern: last statement is IRExprEval,
        # IRCall, IRIncr, or an IRBarrier that has a real runtime result
        # (e.g. ``namespace eval :: $cmd $args``), followed by goto to an
        # empty exit block.  In Tcl, the last command's result is the
        # proc's return value.
        stmts = block.statements
        use_implicit_return = False
        if (
            stmts
            and isinstance(stmts[-1], (IRExprEval, IRCall, IRIncr, IRBarrier))
            and isinstance(block.terminator, CFGGoto)
            and self._is_exit_block(block.terminator.target)
            and self._is_proc
        ):
            use_implicit_return = True

        if use_implicit_return:
            for stmt in stmts[:-1]:
                self._emit_stmt(stmt)
            last = stmts[-1]
            # Update diag-site context before the tail-call branches
            # dispatch — otherwise a fallback or unsupported-trap in
            # the tail would stamp a site with whichever range the
            # previous statement happened to leave behind.
            self._record_stmt_context(last)
            if isinstance(last, IRExprEval):
                # Emit the final expression (i64), box to i32 for return
                self._emit_expr(last.expr)
                self._emit_box_int()
            elif isinstance(last, IRCall):
                # Emit the call keeping its i32 result on the stack
                last_tokens = last.tokens
                if (
                    last_tokens is not None
                    and last_tokens.expand_word is not None
                    and any(last_tokens.expand_word)
                    and last_tokens.argv_texts
                ):
                    # ``{*}`` expansion in tail position — reconstruct
                    # command text with ``{*}`` prefixes for eval.
                    ew = last_tokens.expand_word
                    parts = [
                        (f"{{*}}{t}" if (i < len(ew) and ew[i]) else t)
                        for i, t in enumerate(last_tokens.argv_texts)
                    ]
                    script = " ".join(parts)
                    self._emit_eval_fallback(last.command, last.args, script_override=script)
                    if self._optimise:
                        self._const_map.clear()
                else:
                    self._emit_call_stmt_tail(last.command, last.args, last.defs)
            elif isinstance(last, IRBarrier):
                # IRBarrier in tail position — keep result on stack (no DROP).
                # ``namespace eval ns arg1 arg2 ...`` with dynamic args uses
                # WASM-level assembly so compiled-frame aliases resolve.
                barrier_cmd = last.command
                barrier_args = last.args
                if (
                    barrier_cmd == "namespace"
                    and barrier_args
                    and barrier_args[0] == "eval"
                    and len(barrier_args) > 2
                ):
                    if not self._emit_namespace_eval_bridge(barrier_args[2:], drop_result=False):
                        self._emit_eval_fallback(barrier_cmd, barrier_args)
                        # result stays on stack (no DROP)
                else:
                    # Generic barrier in tail position — eval fallback,
                    # result stays on stack.
                    if barrier_cmd:
                        self._emit_eval_fallback(barrier_cmd, barrier_args)
                    else:
                        self._emit_eval_fallback(last.reason)
                if self._optimise:
                    self._const_map.clear()
            elif isinstance(last, IRIncr):
                # Emit incr keeping the new value (i32 TclObj) on stack.
                # Alias-aware: reads/writes route through globals for
                # upvar/variable-bound locals.
                self._emit_var_read_obj(last.name)
                self._emit_unbox_int()
                amt = 1
                if last.amount is not None:
                    try:
                        amt = int(last.amount)
                    except ValueError:
                        self._emit_value(last.amount)
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        self._emit_box_int()
                        if last.name in self._aliases:
                            self._emit_var_write_obj_keep(last.name)
                        else:
                            idx = self._intern_local(last.name)
                            self._emit_local_tee(idx)
                        self._emit(WasmOp.RETURN)
                        return
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                if last.name in self._aliases:
                    self._emit_var_write_obj_keep(last.name)
                else:
                    idx = self._intern_local(last.name)
                    self._emit_local_tee(idx)
            self._emit(WasmOp.RETURN)
            return

        for stmt in stmts:
            self._emit_stmt(stmt)

        match block.terminator:
            case CFGReturn(value=value):
                if value is not None:
                    self._emit_value(value)
                else:
                    self._emit_i32_const(0)
                self._emit(WasmOp.RETURN)

            case CFGBranch(condition=condition, true_target=tt, false_target=ft):
                # Foreach pattern: the CFG builder desugars foreach into
                # a header block with an opaque <foreach_has_next> branch.
                # Detect this and emit a counter-based WASM loop instead.
                if isinstance(condition, ExprRaw) and condition.text == "<foreach_has_next>":
                    self._emit_foreach_loop(stmts, tt, ft, header_block=block_name)
                    return

                # For/while loop pattern: the header block branches on a
                # condition; the true-target body eventually loops back
                # to this header via a goto chain.  Detect back-edges
                # and emit block{loop{...}} instead of if/else.
                if self._is_loop_header(block_name, tt):
                    self._emit_cfg_loop(block_name, condition, tt, ft)
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
        """Generate a complete WASM function.

        All functions take i32 TclObj params and return a single i32
        TclObj result (null pointer 0 for void/empty-string returns).
        """
        # Emit the compiled-proc registration prologue so the
        # runtime's proc table records every compiled proc by name.
        # The func_idx=1 marker tells ``eval_proc_call`` to dispatch
        # through the host bridge (``call_compiled_proc``) rather
        # than try to eval a Tcl source body.
        queue = getattr(self, "_compiled_proc_queue", None)
        if queue:
            reg_idx = self._shared_imports.get("tcl_proc_register_compiled")
            if reg_idx is not None:
                for qname, n_params, has_args_tail in queue:
                    self._emit_obj_literal(qname)
                    self._emit_i32_const(n_params)
                    self._emit_i32_const(1)  # func_idx marker (any non-zero)
                    self._emit_i32_const(1 if has_args_tail else 0)
                    self._emit_call(reg_idx)
                    self._emit(WasmOp.DROP)

        # Compiled-proc entry prologue: push a frame and bind each
        # param into the frame's local-variable table.  This lets
        # eval-fallbacks *inside* the proc body (e.g. ``proc
        # $varName {body}`` where the interpreter needs to resolve
        # ``$varName`` against the compiled proc's locals) see the
        # param values via ``var_resolve``.  ``::top`` isn't a
        # proc — it runs in global scope and doesn't get a frame.
        wants_frame = (
            self._is_proc
            and "tcl_frame_push" in self._shared_imports
            and "tcl_local_set" in self._shared_imports
            and "tcl_frame_pop" in self._shared_imports
        )
        if wants_frame:
            push_idx = self._shared_imports["tcl_frame_push"]
            self._emit_call(push_idx)
            self._emit(WasmOp.DROP)  # discard frame idx
            local_set_idx = self._shared_imports["tcl_local_set"]
            for i, pname in enumerate(self._params):
                if not pname or pname.startswith("_"):
                    continue
                self._emit_obj_literal(pname)
                self._emit_local_get(i)
                self._emit_call(local_set_idx)
                self._emit(WasmOp.DROP)

        # Record where the body begins — we need to walk only the
        # body instructions in the frame_pop post-process, not the
        # prologue we just emitted.
        prologue_end = len(self._body)

        self._emit_block(self._cfg.entry)

        # Ensure function has a return value on all paths (i32 null TclObj)
        if not self._body or self._body[-1].op != WasmOp.RETURN:
            self._emit_i32_const(0)
            self._emit(WasmOp.END)
        else:
            self._emit(WasmOp.END)

        # Compiled-proc exit epilogue: pop the frame we pushed in the
        # prologue.  Inject a CALL to ``tcl_frame_pop`` immediately
        # before every ``WasmOp.RETURN`` and at the natural implicit
        # fall-through (between the last body instruction and the
        # final END).  Confining the walk to instructions *after*
        # ``prologue_end`` means we don't accidentally treat the
        # prologue's own call/drop as a RETURN site.
        if wants_frame:
            pop_idx = self._shared_imports.get("tcl_frame_pop")
            if pop_idx is not None:
                call_pop = WasmInstruction(op=WasmOp.CALL, operands=_leb128_unsigned(pop_idx))
                new_body = list(self._body[:prologue_end])
                body_tail = list(self._body[prologue_end:])
                for instr in body_tail:
                    if instr.op == WasmOp.RETURN:
                        new_body.append(call_pop)
                    new_body.append(instr)
                # Insert pop between the last non-END instruction and
                # the trailing END(s) for the natural fallthrough.
                i = len(new_body) - 1
                while i >= 0 and new_body[i].op == WasmOp.END:
                    i -= 1
                if i >= 0 and new_body[i].op != WasmOp.RETURN:
                    # Natural fallthrough — stack has the implicit
                    # return value at position i.  Insert pop right
                    # after it so the stack is ordered
                    # [return_val, (no change from pop)] when END fires.
                    new_body = new_body[: i + 1] + [call_pop] + new_body[i + 1 :]
                self._body = new_body

        # Apply optimisations
        self._run_optimisations()

        # Separate param types from extra locals
        param_types = [ValType.I32] * len(self._params)
        extra_locals = self._local_types[len(self._params) :]

        return WasmFunction(
            name=self._cfg.name,
            params=param_types,
            results=[ValType.I32],
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
    shared_imports: dict[str, int] | None = None,
    proc_index: dict[str, tuple[int, int]] | None = None,
) -> WasmFunction:
    """Generate a WASM function from a CFG function.

    When *optimise* is ``True``, constant folding, peephole
    optimisation, and dead-code elimination are applied.
    """
    emitter = _WasmEmitter(
        cfg,
        params=params,
        optimise=optimise,
        is_proc=is_proc,
        shared_imports=shared_imports,
        proc_index=proc_index,
    )
    return emitter.generate()


def wasm_codegen_module(
    cfg_module: CFGModule,
    ir_module: IRModule,
    *,
    optimise: bool = False,
    filename: str | None = None,
    diag_map: DiagMap | None = None,
    escape_summaries: dict[str, ProcEscapeSummary] | None = None,
) -> WasmModule:
    """Generate a complete WASM module from a CFG module.

    Args:
        cfg_module: The control-flow-graph module to compile.
        ir_module: The IR module (for procedure metadata).
        optimise: When ``True``, enable optimisation passes
            (constant folding, peephole, dead-code elimination).
            When ``False``, emit straightforward unoptimised code.
        filename: Source filename recorded on every diag site.
            Passing this without a ``diag_map`` allocates a fresh
            map; passing neither disables diag instrumentation
            entirely (no ``tcl_diag_set`` calls, no ``site=<id>``
            prefix on traps).
        diag_map: Optional pre-allocated :class:`DiagMap` to populate.
            Pass one when the caller wants to serialise the sidecar
            alongside the WASM bytes.  Leave as ``None`` (and omit
            *filename*) when diagnostics aren't needed — callers
            that can't resolve ``site=<id>`` prefixes shouldn't see
            them at all.

    Returns:
        A ``WasmModule`` that can be serialised to ``.wasm`` binary
        or inspected as WAT text.  Diag data (when enabled) is
        recorded on the supplied *diag_map* (or on the fresh one
        allocated when a *filename* was passed).
    """
    # Only instrument with diag sites when the caller explicitly
    # asks for them — either by passing a ``DiagMap`` to populate,
    # or by providing a ``filename`` (which is useless without a
    # map to record it in, so we interpret it as an implicit
    # request).  Without a map the emitter skips ``tcl_diag_set``
    # calls entirely — avoids emitting ``site=<id>`` prefixes the
    # caller has no way to resolve.
    if diag_map is None and filename is not None:
        diag_map = DiagMap(filename=filename)
    elif diag_map is not None and filename is not None:
        diag_map.filename = filename

    # Phase 0: Run per-proc var-escape analysis so the emitter can narrow
    # frame-sync work to vars the analysis couldn't prove LOCAL-safe.
    # Runs unconditionally — the analysis is a cheap tree walk, and callers
    # that pre-computed it can pass ``escape_summaries`` to skip the work.
    if escape_summaries is None:
        try:
            escape_summaries = analyse_var_escape(ir_module=ir_module)
        except Exception:  # noqa: BLE001 — analysis failure falls back
            escape_summaries = None

    module = WasmModule()

    # Phase 1: Pre-scan IR to find which runtime imports are needed
    needed_imports = _scan_needed_imports(cfg_module, ir_module)

    # Phase 2: Register needed imports (these occupy the first function indices)
    shared_imports: dict[str, int] = {}
    for import_key in sorted(needed_imports):
        spec = _RUNTIME_IMPORTS.get(import_key)
        if spec is None:
            continue
        mod_name, func_name, params, results = spec
        type_idx = module._intern_type(params, results)
        func_idx = len(module.imports)
        module.imports.append(WasmImport(module=mod_name, name=func_name, type_idx=type_idx))
        shared_imports[import_key] = func_idx

    num_imports = len(module.imports)

    # Phase 3: Build a single ordered list of callables (procs + methods)
    # and the proc name → (func_idx, n_params) map.  This avoids
    # double-indexing methods that appear in cfg_module.procedures but
    # are defined in ir_module.methods rather than ir_module.procedures.
    proc_index: dict[str, tuple[int, int]] = {}
    # Parallel map of proc qname → tuple of default-value strings
    # (or ``None`` per slot without a default).  Used by the codegen
    # to pad missing call-site args with the declared default instead
    # of a boxed zero.
    from ..lowering import _parse_params_with_defaults

    proc_defaults: dict[str, tuple[str | None, ...]] = {}
    callables: list[tuple[str, CFGFunction, tuple[str, ...]]] = []

    for qname, cfg_func in cfg_module.procedures.items():
        ir_proc = ir_module.procedures.get(qname)
        if ir_proc is not None:
            callables.append((qname, cfg_func, ir_proc.params))
            if ir_proc.params_raw:
                defs = _parse_params_with_defaults(ir_proc.params_raw)
                proc_defaults[qname] = tuple(d for _, d in defs)
        elif ir_module.methods and qname in ir_module.methods:
            ir_method = ir_module.methods[qname]
            callables.append((qname, cfg_func, ir_method.params))
        else:
            callables.append((qname, cfg_func, ()))

    # Assign function indices: ::top is at num_imports, callables follow
    func_idx = num_imports + 1  # +1 for ::top
    # Also track which procs have a variadic ``args`` tail parameter.
    proc_args_tail: set[str] = set()
    for qname, _, params in callables:
        proc_index[qname] = (func_idx, len(params))
        if params and params[-1] == "args":
            proc_args_tail.add(qname)
        func_idx += 1

    # Shared string table so data segments from different functions
    # don't collide at offset 0. All emitters share a single list,
    # index dict, and offset counter.
    shared_strings: list[tuple[str, int]] = []
    shared_string_index: dict[str, int] = {}
    shared_string_offset: list[int] = [0]

    # Phase 4: Compile top-level with shared state
    top_escape = escape_summaries.get("::top") if escape_summaries is not None else None
    emitter = _WasmEmitter(
        cfg_module.top_level,
        optimise=optimise,
        is_proc=False,
        shared_imports=shared_imports,
        proc_index=proc_index,
        proc_defaults=proc_defaults,
        proc_args_tail=proc_args_tail,
        shared_strings=shared_strings,
        shared_string_index=shared_string_index,
        shared_string_offset=shared_string_offset,
        diag_map=diag_map,
        escape_summary=top_escape,
    )
    # Register every compiled proc in the runtime proc table with a
    # non-zero func_idx marker so the interpreter knows to dispatch
    # via the host bridge (``call_compiled_proc`` → wasmtime
    # lookup).  We don't ship the Tcl source body — the compiled
    # WASM function IS the body, and the host bridge invokes it by
    # name.  This lets a dynamically-registered proc (``proc
    # $varName {body}``) whose body references a compiled helper
    # like ``Configure`` actually execute instead of trapping with
    # "unknown command".
    emitter.set_compiled_proc_registrations(ir_module)
    top_func = emitter.generate()
    top_func.name = "::top"
    module.functions.append(top_func)
    # ::top occupies the first function index after imports.
    if diag_map is not None:
        diag_map.procs.append((num_imports, "::top"))

    # Phase 5: Compile callables (procedures and methods)
    for qname, cfg_func, params in callables:
        proc_escape = escape_summaries.get(qname) if escape_summaries is not None else None
        callable_emitter = _WasmEmitter(
            cfg_func,
            params=params,
            optimise=optimise,
            is_proc=True,
            shared_imports=shared_imports,
            proc_index=proc_index,
            proc_defaults=proc_defaults,
            proc_args_tail=proc_args_tail,
            shared_strings=shared_strings,
            shared_string_index=shared_string_index,
            shared_string_offset=shared_string_offset,
            proc_qname=qname,
            diag_map=diag_map,
            escape_summary=proc_escape,
        )
        callable_func = callable_emitter.generate()
        callable_func.name = qname
        module.functions.append(callable_func)
        # Record the proc's WASM function index so the sidecar can
        # resolve ``<wasm function N>`` backtraces to proc names.
        if diag_map is not None:
            f_idx, _ = proc_index[qname]
            diag_map.procs.append((f_idx, qname))

    # Emit a single set of data segments from the shared string table.
    for value, offset in shared_strings:
        encoded = value.encode("utf-8")
        data = len(encoded).to_bytes(4, "little") + encoded
        module.data_segments.append(WasmData(offset=offset, data=data))

    # Ensure types are registered
    for func in module.functions:
        module._intern_type(func.params, func.results)

    return module
