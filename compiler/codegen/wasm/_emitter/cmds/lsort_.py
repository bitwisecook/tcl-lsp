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

"""WASM emit hook for ``lsort`` — trailing-positional list sort.

Plain ``lsort LIST`` keeps the fast path via ``tcl_cmd_list_sort``.
Once any options arrive (``-integer``, ``-decreasing``, ``-dictionary``,
``-index``, ``-stride``, ``-unique``, ``-indices``, ``-nocase``,
``-command``) the call has to thread through the runtime's
``eval_lsort`` so the option matrix is honoured — the single-arg
runtime helper would silently drop the switches and return the wrong
order.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _smart_quote_arg(a: str) -> str:
    """Quote *a* as a single Tcl word for ``tcl_eval``.

    Mirrors the eval-fallback logic without the double-brace
    wrapping that ``tcl_list_quote`` applies for safety: we know
    each arg here came from a parsed source word (literal,
    substitution, or option) so we can use the simpler rule of
    ``{a}`` when *a* has whitespace and balanced braces, ``"a"``
    after escaping when *a* has unbalanced braces.

    The IR / split-command-subst form often hands us the value
    *with* outer braces still attached (``{a b}``).  Pass those
    through verbatim — they're already a valid Tcl word.
    """
    if not a:
        return "{}"
    if a.startswith("$") or a.startswith("["):
        return a
    if len(a) >= 2 and a[0] == "{" and a[-1] == "}":
        # Already a brace-delimited word from ``_split_command_subst``.
        # Verify the braces are matched without going negative — if
        # they are, it's a single Tcl word already and we should not
        # re-wrap.
        depth = 0
        balanced = True
        for ch in a:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth < 0:
                    balanced = False
                    break
        if balanced and depth == 0:
            return a
    needs_quote = False
    depth = 0
    balanced = True
    for ch in a:
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth < 0:
                balanced = False
        elif ch in " \t\n\r\v\f;":
            needs_quote = True
    if depth != 0:
        balanced = False
    if not needs_quote and balanced and not a.startswith("#"):
        return a
    if balanced:
        return "{" + a + "}"
    # Fall back to dquote escaping when braces are unbalanced.
    out: list[str] = ['"']
    for ch in a:
        if ch in '"\\$[':
            out.append("\\" + ch)
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def _emit_lsort(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    prep = emitter._runtime_prep("lsort", args)
    if prep is None:
        return False
    func_idx, rimp = prep
    param_count = len(rimp.params)

    if len(args) > param_count:
        # Option-bearing form — build an eval script with proper
        # quoting and route through ``tcl_eval`` so the runtime's
        # ``eval_lsort`` parses the full option matrix.  We build the
        # script ourselves rather than calling ``_emit_eval_fallback``
        # because that helper uses list-quoting (which double-wraps a
        # literal word into ``{{word}}``) and we don't always have
        # ``_current_call_tokens`` populated in value context.
        script = "lsort " + " ".join(_smart_quote_arg(a) for a in args)
        emitter._emit_eval_fallback("lsort", args, script_override=script)
        if context is EmitContext.STATEMENT:
            from ..._ir import WasmOp

            emitter._emit(WasmOp.DROP)
        return True

    for i in range(min(param_count, len(args))):
        emitter._emit_value(args[i])
    for _ in range(param_count - len(args)):
        emitter._emit_i32_const(0)

    emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("lsort", _emit_lsort)
