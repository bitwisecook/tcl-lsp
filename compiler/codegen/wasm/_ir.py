"""WASM IR dataclasses, enums, and WAT formatting.

Holds every data structure the code generator builds at compile time:
value types, section IDs, the opcode enum, per-function and per-module
containers, and the diagnostic-site sidecar. Also contains the
self-contained WAT (text format) renderer used by the compiler
explorer and by human-readable dumps in tests.

No dependency on the ``_WasmEmitter`` class; only on ``_encoding`` for
LEB128 and string encoding. Safe to import from anywhere inside the
package.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from enum import IntEnum
from typing import TYPE_CHECKING

from ._encoding import (
    _encode_string,
    _encode_vector,
    _leb128_signed,
    _leb128_unsigned,
)

if TYPE_CHECKING:
    from core.analysis.semantic_model import Range

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
    """A single WASM instruction with optional operands.

    ``range`` captures the source range of the originating Tcl statement
    (when the instruction was emitted from inside ``_emit_stmt``) so the
    compiler explorer can offer click-to-source navigation and render
    per-statement comments above each instruction group.  ``label`` is a
    free-form hint set by callers that want to attach a human-readable
    tag to a structural instruction — for example the emitter tags
    ``block``/``loop``/``if`` opens with the Tcl command that produced
    them (``foreach`` / ``while`` / ``if``).  Both fields are ignored by
    the binary encoder and WAT formatter; only the explorer view
    consumes them.
    """

    op: int
    operands: bytes = b""
    range: Range | None = None
    label: str | None = None

    def encode(self) -> bytes:
        return bytes([self.op]) + self.operands


@dataclass(slots=True)
class WasmFunction:
    """Compiled WASM function.

    ``source_range`` records the proc's original source range (from
    :class:`~core.compiler.ir.IRProcedure`) so the explorer can place
    the cursor inside the proc body when the user clicks on a call
    target that resolves to this function.  ``kind`` distinguishes the
    synthetic ``::top`` wrapper from real Tcl procs and methods.
    """

    name: str
    params: list[ValType]
    results: list[ValType]
    locals: list[ValType]
    body: list[WasmInstruction]
    local_names: list[str] = field(default_factory=list)
    exported: bool = False
    source_range: Range | None = None
    kind: str = "proc"  # "top" | "proc" | "method"

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

    def to_explorer_json(self) -> list[dict]:
        """Structured per-instruction view for the compiler explorer.

        Returns a list of function entries (one per function in the
        module; the synthetic ``(module)`` entry carries types, imports
        and data).  Each function entry carries its instruction list
        with:

        - resolved ``call`` target names (imports and internal
          functions),
        - resolved ``br`` / ``br_if`` targets (opening/closing of the
          enclosing structured-control-flow construct),
        - a ``range`` dict of the originating Tcl statement (when
          available), for click-to-source navigation, and
        - any explorer label attached by the emitter
          (``"foreach"``, ``"if"``, ``"catch body"``, …).

        The shape matches what ``explorer/serialise.py`` ships to the
        frontend.  Indices into the returned instruction list are
        stable, so the UI can cross-link a branch instruction to its
        matching block/loop/if open/close.
        """
        # Indices: imports occupy 0..N-1, defined functions follow.
        num_imports = len(self.imports)

        def _func_label(idx: int) -> dict:
            if idx < num_imports:
                imp = self.imports[idx]
                return {
                    "kind": "import",
                    "name": imp.name,
                    "module": imp.module,
                    "funcIdx": idx,
                    "defIdx": None,
                }
            def_idx = idx - num_imports
            if 0 <= def_idx < len(self.functions):
                f = self.functions[def_idx]
                return {
                    "kind": "top" if f.kind == "top" else f.kind,
                    "name": f.name,
                    "module": None,
                    "funcIdx": idx,
                    "defIdx": def_idx,
                }
            return {
                "kind": "unknown",
                "name": f"<func {idx}>",
                "module": None,
                "funcIdx": idx,
                "defIdx": None,
            }

        module_header = {
            "name": "(module)",
            "kind": "module",
            "funcIdx": None,
            "params": [],
            "results": [],
            "locals": [],
            "sourceRange": None,
            "instrCount": 0,
            "instructions": [],
            "imports": [
                {
                    "module": imp.module,
                    "name": imp.name,
                    "typeIdx": imp.type_idx,
                    "funcIdx": i,
                }
                for i, imp in enumerate(self.imports)
            ],
            "types": [
                {
                    "index": i,
                    "params": [_valtype_name(p) for p in params],
                    "results": [_valtype_name(r) for r in results],
                }
                for i, (params, results) in enumerate(self._types)
            ],
            "dataSegments": [
                {"offset": seg.offset, "size": len(seg.data)} for seg in self.data_segments
            ],
        }

        entries: list[dict] = [module_header]
        for def_idx, func in enumerate(self.functions):
            entries.append(
                _function_to_explorer_json(
                    func,
                    func_idx=num_imports + def_idx,
                    resolve_func=_func_label,
                )
            )
        return entries

    def to_wat(self) -> str:
        """Generate a human-readable WAT (WebAssembly Text) representation."""
        lines: list[str] = ["(module"]

        # Types
        for i, (params, results) in enumerate(self._types):
            p = " ".join(f"(param {_valtype_name(pt)})" for pt in params)
            r = " ".join(f"(result {_valtype_name(rt)})" for rt in results)
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


def _format_wat_instr(instr: WasmInstruction, indent: int) -> str | None:
    """Format a single instruction for WAT output."""
    prefix = "    " * indent
    op = instr.op

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


# Explorer view helpers


def _range_to_explorer_dict(rng: Range | None) -> dict | None:
    """Serialise a Range for the explorer frontend."""
    if rng is None:
        return None
    return {
        "startLine": rng.start.line,
        "startCol": rng.start.character,
        "startOffset": rng.start.offset,
        "endLine": rng.end.line,
        "endCol": rng.end.character,
        "endOffset": rng.end.offset,
    }


def _function_to_explorer_json(
    func: WasmFunction,
    *,
    func_idx: int,
    resolve_func,
) -> dict:
    """Build a structured explorer view for a single ``WasmFunction``.

    Resolves each instruction's display text, its ``call`` / ``br``
    target (when any), its indent level, and emits a stable index so
    the UI can cross-link a branch to the control-flow construct it
    targets.  ``resolve_func`` is a callable taking a 0-based WASM
    function index (including imports) and returning a label dict.
    """
    local_names = list(func.local_names)
    param_count = len(func.params)

    def _local_label(idx: int) -> str:
        if 0 <= idx < len(local_names):
            return local_names[idx]
        return f"$l{idx}"

    # First pass: pair BLOCK/LOOP/IF opens with their matching END (and
    # ELSE).  Control-stack entry: (opcode, open_idx, else_idx | None,
    # end_idx | None, label).  Built by a single linear walk.
    stack: list[dict] = []
    open_info: dict[int, dict] = {}  # open_idx → info

    for i, instr in enumerate(func.body):
        if instr.op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
            entry = {
                "op": instr.op,
                "openIdx": i,
                "elseIdx": None,
                "endIdx": None,
                "label": instr.label,
            }
            stack.append(entry)
            open_info[i] = entry
        elif instr.op == WasmOp.ELSE:
            if stack:
                stack[-1]["elseIdx"] = i
        elif instr.op == WasmOp.END:
            if stack:
                entry = stack.pop()
                entry["endIdx"] = i

    # Resolve each instruction
    instructions: list[dict] = []
    indent = 0
    ctrl_stack: list[dict] = []  # mirrors the open stack while walking forward

    for i, instr in enumerate(func.body):
        op = instr.op
        mnemonic = _WAT_NAMES.get(op, f"0x{op:02x}")

        # Structural tracking first so indent reflects the instruction
        # we're about to emit rather than the one after it.
        this_indent = indent
        block_label = None
        block_kind = None
        if op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
            # Opens are rendered at the outer indent.  New frame pushed
            # after we record the instruction so following body ops
            # render one level deeper.
            info = open_info.get(i)
            if info is not None:
                block_label = f"$L{i}"
            if op == WasmOp.BLOCK:
                block_kind = "block"
            elif op == WasmOp.LOOP:
                block_kind = "loop"
            elif op == WasmOp.IF:
                block_kind = "if"
            ctrl_stack.append(info or {"op": op, "openIdx": i, "endIdx": None, "label": None})
            indent += 1
        elif op == WasmOp.ELSE:
            # Render at one less indent (matches structured CFG style).
            this_indent = max(0, indent - 1)
        elif op == WasmOp.END:
            if ctrl_stack:
                ctrl_stack.pop()
            indent = max(0, indent - 1)
            this_indent = indent

        # Operand decoding
        operand_text = ""
        full_text = mnemonic
        call_target = None
        branch_target = None
        local_index: int | None = None

        if op in (
            WasmOp.LOCAL_GET,
            WasmOp.LOCAL_SET,
            WasmOp.LOCAL_TEE,
        ):
            if instr.operands:
                idx = _decode_leb128_unsigned(instr.operands)
                local_index = idx
                name = _local_label(idx)
                operand_text = str(idx)
                full_text = (
                    f"{mnemonic} {idx} {name}" if name != f"$l{idx}" else f"{mnemonic} {idx}"
                )
        elif op in (WasmOp.GLOBAL_GET, WasmOp.GLOBAL_SET):
            if instr.operands:
                idx = _decode_leb128_unsigned(instr.operands)
                operand_text = str(idx)
                full_text = f"{mnemonic} {idx}"
        elif op == WasmOp.CALL:
            if instr.operands:
                idx = _decode_leb128_unsigned(instr.operands)
                operand_text = str(idx)
                call_target = resolve_func(idx)
                full_text = f"{mnemonic} {idx}"
        elif op in (WasmOp.BR, WasmOp.BR_IF):
            if instr.operands:
                depth = _decode_leb128_unsigned(instr.operands)
                operand_text = str(depth)
                full_text = f"{mnemonic} {depth}"
                # Resolve target: depth-th from top of ctrl_stack, or
                # the function body when depth == len(ctrl_stack).
                if depth < len(ctrl_stack):
                    frame = ctrl_stack[len(ctrl_stack) - 1 - depth]
                    target_op = frame.get("op")
                    if target_op == WasmOp.LOOP:
                        target_idx = frame.get("openIdx")
                        target_kind = "loop_header"
                    else:
                        target_idx = frame.get("endIdx")
                        target_kind = "block_end" if target_op == WasmOp.BLOCK else "if_end"
                    branch_target = {
                        "depth": depth,
                        "targetIdx": target_idx,
                        "kind": target_kind,
                        "label": frame.get("label"),
                    }
                else:
                    branch_target = {
                        "depth": depth,
                        "targetIdx": None,
                        "kind": "function_return",
                        "label": None,
                    }
        elif op == WasmOp.I32_CONST:
            if instr.operands:
                val = _decode_leb128_signed(instr.operands)
                operand_text = str(val)
                full_text = f"{mnemonic} {val}"
        elif op == WasmOp.I64_CONST:
            if instr.operands:
                val = _decode_leb128_signed(instr.operands)
                operand_text = str(val)
                full_text = f"{mnemonic} {val}"
        elif op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
            if instr.operands:
                bt = instr.operands[0]
                if bt == _BLOCK_VOID:
                    full_text = mnemonic
                else:
                    full_text = f"{mnemonic} (result {_valtype_name(bt)})"
            else:
                full_text = mnemonic

        instructions.append(
            {
                "idx": i,
                "indent": this_indent,
                "op": mnemonic,
                "opcode": int(op),
                "operandText": operand_text,
                "fullText": full_text,
                "range": _range_to_explorer_dict(instr.range),
                "label": instr.label,
                "callTarget": call_target,
                "branchTarget": branch_target,
                "blockLabel": block_label,
                "blockKind": block_kind,
                "localIndex": local_index,
            }
        )

    # Attach closing metadata onto BLOCK/LOOP/IF opens and their END
    # counterparts so the UI can cross-link them.
    for i, instr in enumerate(instructions):
        op_int = instr["opcode"]
        if op_int in (int(WasmOp.BLOCK), int(WasmOp.LOOP), int(WasmOp.IF)):
            info = open_info.get(i)
            if info is not None:
                instr["endIdx"] = info["endIdx"]
                instr["elseIdx"] = info["elseIdx"]
        elif op_int == int(WasmOp.END):
            # Find the matching open via open_info iteration.
            for info in open_info.values():
                if info["endIdx"] == i:
                    instr["openIdx"] = info["openIdx"]
                    instructions[info["openIdx"]].setdefault("endIdx", i)
                    break
        elif op_int == int(WasmOp.ELSE):
            for info in open_info.values():
                if info["elseIdx"] == i:
                    instr["openIdx"] = info["openIdx"]
                    instr["endIdx"] = info["endIdx"]
                    break

    return {
        "name": func.name,
        "kind": func.kind,
        "funcIdx": func_idx,
        "exported": func.exported,
        "params": [
            {"name": local_names[i] if i < len(local_names) else f"$p{i}", "type": _valtype_name(p)}
            for i, p in enumerate(func.params)
        ],
        "results": [_valtype_name(r) for r in func.results],
        "locals": [
            {
                "name": local_names[param_count + i]
                if (param_count + i) < len(local_names)
                else f"$l{i}",
                "type": _valtype_name(lt),
            }
            for i, lt in enumerate(func.locals)
        ],
        "sourceRange": _range_to_explorer_dict(func.source_range),
        "instrCount": len(func.body),
        "instructions": instructions,
    }
