"""Formatting helpers for bytecode disassembly output."""

from __future__ import annotations

from typing import TYPE_CHECKING

from .opcodes import _INDEX_END, _JUMP_OPS, _LVT_OPS, _STR_CLASS_NAMES, Op

if TYPE_CHECKING:
    from ._types import FunctionAsm


def _esc(text: str, limit: int = 40) -> str:
    """Escape a literal value for disassembly comments.

    Matches Tcl's disassembler (``PrintSourceToObj`` in ``tclDisassemble.c``):
    backslashes are NOT doubled; ``"`` and the named whitespace controls
    (``\\n \\t \\r \\v \\f``) use short forms; every other codepoint below
    0x20 or at/above 0x7F is escaped as ``\\uXXXX`` (or ``\\UXXXXXXXX`` for
    supplementary planes). Without this, raw control bytes like STX leak
    into the HTML pane and render as unprintable/tofu glyphs.
    """
    parts: list[str] = []
    for ch in text:
        cp = ord(ch)
        if ch == '"':
            parts.append('\\"')
        elif ch == "\n":
            parts.append("\\n")
        elif ch == "\t":
            parts.append("\\t")
        elif ch == "\r":
            parts.append("\\r")
        elif ch == "\v":
            parts.append("\\v")
        elif ch == "\f":
            parts.append("\\f")
        elif cp > 0xFFFF:
            parts.append(f"\\U{cp:08x}")
        elif cp < 0x20 or cp >= 0x7F:
            parts.append(f"\\u{cp:04x}")
        else:
            parts.append(ch)
    s = "".join(parts)
    if len(s) > limit:
        return s[: limit - 3] + "..."
    return s


def format_function_asm(asm) -> str:
    """Render FunctionAsm to disassembly text."""
    lines: list[str] = []

    total_bytes = sum(i.size for i in asm.instructions)
    lines.append(
        f"ByteCode {asm.name}, {len(asm.instructions)} instructions, "
        f"{total_bytes} bytes, {len(asm.literals)} literals, "
        f"{len(asm.lvt)} variables"
    )

    if asm.literals.entries():
        lines.append("  Literals:")
        for i, lit in enumerate(asm.literals.entries()):
            lines.append(f'    {i}: "{_esc(lit)}"')

    if asm.lvt.entries():
        lines.append("  Local variables:")
        for i, var in enumerate(asm.lvt.entries()):
            lines.append(f'    %v{i}: "{var}"')

    lines.append("  Instructions:")

    off2labels: dict[int, list[str]] = {}
    for label, off in asm.labels.items():
        off2labels.setdefault(off, []).append(label)

    for instr in asm.instructions:
        if instr.offset in off2labels:
            for lbl in off2labels[instr.offset]:
                lines.append(f"  # {lbl}:")

        parts: list[str] = []
        jump_comment = ""
        for j, operand in enumerate(instr.operands):
            if isinstance(operand, str) and instr.op in _JUMP_OPS:
                target = asm.labels.get(operand, 0)
                relative = target - instr.offset
                parts.append(f"+{relative}" if relative >= 0 else str(relative))
                jump_comment = f"\t# pc {target}"
            elif isinstance(operand, str) and instr.op == Op.START_CMD and j == 0:
                target = asm.labels.get(operand, 0)
                relative = target - instr.offset
                parts.append(f"+{relative}" if relative >= 0 else str(relative))
                count = instr.operands[1] if len(instr.operands) > 1 else 1
                jump_comment = f"\t# next cmd at pc {target}, {count} cmds start here"
            elif isinstance(operand, str):
                target = asm.labels.get(operand, 0)
                parts.append(f"pc {target}")
            elif instr.op in _LVT_OPS and j == 0:
                parts.append(f"%v{operand}")
            elif instr.op in (Op.DICT_SET, Op.DICT_UNSET, Op.DICT_INCR_IMM) and j == 1:
                parts.append(f"%v{operand}")
            elif instr.op in (
                Op.INCR_SCALAR1_IMM,
                Op.INCR_STK_IMM,
                Op.INCR_ARRAY_STK_IMM,
                Op.DICT_INCR_IMM,
                Op.STR_MATCH,
                Op.REGEXP,
            ):
                parts.append(f"+{operand}" if operand >= 0 else str(operand))
            elif instr.op in (Op.RETURN_IMM, Op.SYNTAX) and j == 0:
                parts.append(f"+{operand}" if operand >= 0 else str(operand))
            elif instr.op == Op.STR_CLASS and isinstance(operand, int):
                parts.append(_STR_CLASS_NAMES.get(operand, str(operand)))
            elif (
                instr.op in (Op.LIST_INDEX_IMM, Op.LIST_RANGE_IMM, Op.STR_RANGE_IMM)
                and isinstance(operand, int)
                and operand <= _INDEX_END
            ):
                if operand == _INDEX_END:
                    parts.append("end")
                else:
                    parts.append(f"end{operand - _INDEX_END}")
            else:
                parts.append(str(operand))

        ops_str = " ".join(parts)
        spacer = " " if ops_str else ""
        comment = (
            jump_comment if jump_comment else (f"\t# {instr.comment}" if instr.comment else "")
        )
        lines.append(f"    ({instr.offset}) {instr.mnemonic}{spacer}{ops_str}{comment}")

        if instr.op == Op.JUMP_TABLE and instr.jump_table:
            entries = []
            for pattern, label in instr.jump_table.items():
                target_pc = asm.labels.get(label, 0)
                entries.append(f'"{_esc(pattern)}"->pc {target_pc}')
            lines.append(f"\t\t[{', '.join(entries)}]")

    return "\n".join(lines)


def format_module_asm(module) -> str:
    """Format entire module assembly."""
    parts = [format_function_asm(module.top_level)]
    for name in sorted(module.procedures):
        parts.append("")
        parts.append(format_function_asm(module.procedures[name]))
    return "\n".join(parts)


# Structured explorer view.  ``_range_to_explorer_dict`` is defined
# once in ``wasm/_ir.py`` and re-used here so the ASM and WASM
# explorer payloads stay in lockstep — any future change to the range
# wire format lands in a single spot.
from .wasm._ir import _range_to_explorer_dict  # noqa: E402


def format_function_explorer(asm: FunctionAsm, *, func_name: str | None = None) -> dict:
    """Structured per-instruction explorer view for one FunctionAsm.

    Mirrors the shape produced by ``WasmModule.to_explorer_json()`` so
    the frontend renderer can reuse the same DOM structure and
    interaction wiring.  Each instruction carries:

    - resolved jump targets (``jumpTarget.targetIdx`` is the index of
      the matching label's defining instruction),
    - the literal / LVT / immediate operand decoded as text
      (``operandText``),
    - the source range of the originating Tcl statement
      (``range``) for click-to-source, and
    - the label attached to its offset (``blockLabel``) so label rows
      render as named anchors in the UI.

    Labels not bound to any instruction (end-of-block labels that land
    on the byte just past the last instruction) are emitted as
    synthetic ``label`` rows at the tail of the instruction list so
    forward-jumps to "after the last op" still have a visible anchor
    for the UI to point the arrow at.
    """
    name = func_name or asm.name
    instructions = asm.instructions

    # Build offset → instruction index map and offset → labels[] map.
    offset_to_idx: dict[int, int] = {
        instr.offset: i for i, instr in enumerate(instructions) if instr.offset >= 0
    }
    # End-of-function byte offset so a label pointing past the last
    # instruction still resolves to a synthetic tail anchor.
    end_offset = 0
    if instructions:
        last = instructions[-1]
        end_offset = last.offset + last.size

    # Labels keyed by offset (preserving emission order).
    off2labels: dict[int, list[str]] = {}
    for label, off in asm.labels.items():
        off2labels.setdefault(off, []).append(label)

    def _resolve_label(label: str) -> tuple[int | None, int | None]:
        """Return (idx, offset) for a label.  idx is None when the
        label points past the last real instruction (end-of-proc
        synthetic anchor, rendered at the tail)."""
        if label not in asm.labels:
            return None, None
        off = asm.labels[label]
        if off in offset_to_idx:
            return offset_to_idx[off], off
        return None, off

    result_instructions: list[dict] = []

    for i, instr in enumerate(instructions):
        # Emit label-row markers for every label bound to this offset.
        for lbl in off2labels.get(instr.offset, ()):
            result_instructions.append(
                {
                    "idx": len(result_instructions),
                    "kind": "label",
                    "label": lbl,
                    "targetIdx": i,  # first instr at that offset
                    "offset": instr.offset,
                }
            )

        # Decode operand text the same way format_function_asm does so
        # the frontend can render consistent text.  The key difference:
        # we also populate a structured ``jumpTarget`` when the op's
        # operand is a label ref.
        parts: list[str] = []
        jump_target = None
        lvt_ref = None
        for j, operand in enumerate(instr.operands):
            if isinstance(operand, str) and instr.op in _JUMP_OPS:
                tgt_off = asm.labels.get(operand, 0)
                rel = tgt_off - instr.offset
                parts.append(f"+{rel}" if rel >= 0 else str(rel))
                tgt_idx, _ = _resolve_label(operand)
                jump_target = {
                    "label": operand,
                    "offset": tgt_off,
                    "targetIdx": tgt_idx,
                    "relative": rel,
                }
            elif isinstance(operand, str) and instr.op == Op.START_CMD and j == 0:
                tgt_off = asm.labels.get(operand, 0)
                rel = tgt_off - instr.offset
                parts.append(f"+{rel}" if rel >= 0 else str(rel))
                tgt_idx, _ = _resolve_label(operand)
                jump_target = {
                    "label": operand,
                    "offset": tgt_off,
                    "targetIdx": tgt_idx,
                    "relative": rel,
                    "kind": "start_cmd_end",
                }
            elif isinstance(operand, str):
                tgt_off = asm.labels.get(operand, 0)
                parts.append(f"pc {tgt_off}")
            elif instr.op in _LVT_OPS and j == 0:
                parts.append(f"%v{operand}")
                lvt_ref = int(operand)
            elif instr.op in (Op.DICT_SET, Op.DICT_UNSET, Op.DICT_INCR_IMM) and j == 1:
                parts.append(f"%v{operand}")
                lvt_ref = int(operand)
            elif instr.op in (
                Op.INCR_SCALAR1_IMM,
                Op.INCR_STK_IMM,
                Op.INCR_ARRAY_STK_IMM,
                Op.DICT_INCR_IMM,
                Op.STR_MATCH,
                Op.REGEXP,
            ):
                parts.append(f"+{operand}" if operand >= 0 else str(operand))
            elif instr.op in (Op.RETURN_IMM, Op.SYNTAX) and j == 0:
                parts.append(f"+{operand}" if operand >= 0 else str(operand))
            elif instr.op == Op.STR_CLASS and isinstance(operand, int):
                parts.append(_STR_CLASS_NAMES.get(operand, str(operand)))
            elif (
                instr.op in (Op.LIST_INDEX_IMM, Op.LIST_RANGE_IMM, Op.STR_RANGE_IMM)
                and isinstance(operand, int)
                and operand <= _INDEX_END
            ):
                if operand == _INDEX_END:
                    parts.append("end")
                else:
                    parts.append(f"end{operand - _INDEX_END}")
            else:
                parts.append(str(operand))

        operand_text = " ".join(parts)

        # JumpTable entries list (pattern → label → target index).
        jump_table_entries: list[dict] | None = None
        if instr.op == Op.JUMP_TABLE and instr.jump_table:
            jump_table_entries = []
            for pattern, label in instr.jump_table.items():
                tgt_off = asm.labels.get(label, 0)
                tgt_idx, _ = _resolve_label(label)
                jump_table_entries.append(
                    {
                        "pattern": pattern,
                        "label": label,
                        "offset": tgt_off,
                        "targetIdx": tgt_idx,
                    }
                )

        result_instructions.append(
            {
                "idx": len(result_instructions),
                "kind": "instr",
                "seq": i,  # emission order (not line number)
                "op": instr.mnemonic,
                "operandText": operand_text,
                "fullText": f"{instr.mnemonic} {operand_text}".rstrip(),
                "offset": instr.offset,
                "size": instr.size,
                "range": _range_to_explorer_dict(instr.source_range),
                "sourceLine": instr.source_line,
                "comment": instr.comment,
                "jumpTarget": jump_target,
                "jumpTable": jump_table_entries,
                "lvtRef": lvt_ref,
            }
        )

    # Synthetic tail label rows for labels pointing past the last real
    # instruction (end-of-proc anchor for forward jumps).  De-dup
    # against labels we already placed inline.
    placed_labels = {lbl for lbls in off2labels.values() for lbl in lbls}
    tail_labels: list[str] = sorted(
        lbl for lbl, off in asm.labels.items() if lbl not in placed_labels and off >= end_offset
    )
    for lbl in tail_labels:
        off = asm.labels[lbl]
        idx = len(result_instructions)
        result_instructions.append(
            {
                "idx": idx,
                "kind": "label",
                "label": lbl,
                "targetIdx": idx,  # points at itself
                "offset": off,
            }
        )

    # Resolve jumpTarget.targetIdx from asm.instructions-index to
    # result_instructions-index so the UI can cross-link directly.
    # Prefer pointing to the label row (if one exists at the target
    # offset) rather than the first instruction at that offset — the
    # label row IS the visible anchor the arrow should point to.
    label_row_by_name: dict[str, int] = {
        row["label"]: row["idx"] for row in result_instructions if row["kind"] == "label"
    }
    asm_idx_to_result_idx: dict[int, int] = {}
    for row in result_instructions:
        if row["kind"] == "instr":
            asm_idx_to_result_idx[row["seq"]] = row["idx"]

    def _retarget(tgt: dict) -> None:
        lbl = tgt.get("label")
        if lbl and lbl in label_row_by_name:
            tgt["targetIdx"] = label_row_by_name[lbl]
            return
        old = tgt.get("targetIdx")
        if old is not None and old in asm_idx_to_result_idx:
            tgt["targetIdx"] = asm_idx_to_result_idx[old]

    for row in result_instructions:
        if row["kind"] != "instr":
            continue
        jt = row.get("jumpTarget")
        if jt:
            _retarget(jt)
        for entry in row.get("jumpTable") or []:
            _retarget(entry)

    return {
        "name": name,
        "kind": "proc" if func_name else "top",
        "instrCount": len(instructions),
        "byteCount": sum(i.size for i in instructions),
        "literals": list(asm.literals.entries()),
        "locals": list(asm.lvt.entries()),
        "instructions": result_instructions,
        "text": format_function_asm(asm),
    }


def format_module_explorer(module) -> list[dict]:
    """Structured explorer view for a whole ModuleAsm.

    Preserves ``module.procedures`` declaration order — the WASM
    explorer (:meth:`WasmModule.to_explorer_json`) emits entries in
    declaration order too, so the ASM and WASM tabs line up in the
    same order for side-by-side comparison.  The legacy plain-text
    ``format_module_asm`` still sorts alphabetically because it
    mirrors ``tcl::unsupported::disassemble`` output.
    """
    entries: list[dict] = []
    entries.append(format_function_explorer(module.top_level))
    # Ensure ``::top`` is tagged as such even when callers pass the
    # top-level in via a synthetic name.
    entries[0]["kind"] = "top"
    for name, proc_asm in module.procedures.items():
        entries.append(format_function_explorer(proc_asm, func_name=name))
    return entries
