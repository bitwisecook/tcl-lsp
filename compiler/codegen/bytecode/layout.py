# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Instruction layout and jump-size optimisation helpers."""

from __future__ import annotations


def optimise_jumps(
    instrs, labels: dict[str, int], jump4_to_jump1: dict, *, max_iters: int = 10
) -> None:
    """Replace 4-byte jumps with 1-byte jumps when relative offset fits."""
    for _ in range(max_iters):
        offset = 0
        for instr in instrs:
            instr.offset = offset
            offset += instr.size

        label_offsets: dict[str, int] = {}
        for label, instr_idx in labels.items():
            if instr_idx < len(instrs):
                label_offsets[label] = instrs[instr_idx].offset
            else:
                label_offsets[label] = offset

        changed = False
        for instr in instrs:
            short_op = jump4_to_jump1.get(instr.op)
            if short_op is None:
                continue
            target = instr.operands[0]
            if isinstance(target, str):
                if target.startswith(("switch_", "proc_exit_")):
                    continue
                if instr.comment in ("break", "continue", "try_on"):
                    continue
                target_off = label_offsets.get(target)
                if target_off is None:
                    continue
            else:
                target_off = target

            rel = target_off - instr.offset
            if -128 <= rel <= 127:
                instr.op = short_op
                changed = True

        if not changed:
            break


def resolve_layout(instrs, labels: dict[str, int]) -> dict[str, int]:
    """Assign final byte offsets and return label->offset mapping."""
    offset = 0
    for instr in instrs:
        instr.offset = offset
        offset += instr.size

    label_offsets: dict[str, int] = {}
    for label, instr_idx in labels.items():
        if instr_idx < len(instrs):
            label_offsets[label] = instrs[instr_idx].offset
        else:
            label_offsets[label] = offset
    return label_offsets
