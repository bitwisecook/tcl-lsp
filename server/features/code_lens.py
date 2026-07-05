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

"""Code lens provider -- reference counts on procs.

Emits lenses in two phases:

1. ``get_code_lenses`` produces lightweight lenses with a ``data`` payload and
   no command; the client must call ``codeLens/resolve`` for the final title
   and command.
2. ``resolve_code_lens`` resolves the proc's references and returns a fully
   populated ``CodeLens`` whose count is ``len(references)`` — the same
   resolution that backs the peek, so the title and the peek can never drift
   (issue #637). When no resolver is supplied (unit tests) it falls back to the
   cached workspace usage counts.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Protocol

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult, ProcDef
from server._lsp_conv import to_lsp_range


class _WorkspaceLike(Protocol):
    def proc_usage_counts(self) -> dict[str, int]: ...


# ``find_references(uri, qname) -> list[Location]`` — supplies the location
# list for the ``tcl-lsp.showReferences`` wrapper command, which converts
# the arguments to vscode.Uri/Position/Location instances before delegating
# to the built-in ``editor.action.showReferences``. Optional for callers
# (e.g. unit tests) that don't need a working peek.
FindReferences = Callable[[str, str], list[types.Location]]


@dataclass(slots=True)
class _LensData:
    kind: str
    uri: str
    qname: str

    def to_dict(self) -> dict[str, str]:
        return {"kind": self.kind, "uri": self.uri, "qname": self.qname}

    @classmethod
    def from_dict(cls, payload: dict) -> _LensData:
        return cls(
            kind=str(payload.get("kind", "")),
            uri=str(payload.get("uri", "")),
            qname=str(payload.get("qname", "")),
        )


def _proc_ref_lens(uri: str, proc: ProcDef) -> types.CodeLens:
    name_range = to_lsp_range(proc.name_range)
    return types.CodeLens(
        range=name_range,
        data=_LensData(
            kind="proc_ref_count",
            uri=uri,
            qname=proc.qualified_name,
        ).to_dict(),
    )


def get_code_lenses(
    source: str,
    uri: str,
    analysis: AnalysisResult | None,
) -> list[types.CodeLens]:
    """Return unresolved code lenses for every proc in ``analysis``.

    When ``analysis`` is ``None`` the function runs a throwaway
    :func:`analyse` inline so callers can pass through unprepared document
    state (e.g. immediately after a fire-and-forget ``didOpen``).
    """
    if analysis is None:
        analysis = analyse(source)
    lenses: list[types.CodeLens] = []
    for _qname, proc in analysis.all_procs.items():
        lenses.append(_proc_ref_lens(uri, proc))
    return lenses


def resolve_code_lens(
    lens: types.CodeLens,
    workspace_index: _WorkspaceLike,
    find_references: FindReferences | None = None,
) -> types.CodeLens:
    """Populate ``title``/``command`` on ``lens`` using cached usage counts.

    The command is ``tcl-lsp.showReferences``, a thin client-side wrapper
    the VS Code extension registers to convert the URI, position, and
    locations from their JSON-RPC shapes into the ``vscode.Uri``,
    ``vscode.Position``, and ``vscode.Location`` instances that the
    built-in ``editor.action.showReferences`` command requires. Passing
    ``find_references=None`` (e.g. in tests) yields an empty locations
    list but still produces a well-formed command.
    """
    payload = lens.data if isinstance(lens.data, dict) else {}
    data = _LensData.from_dict(payload)
    if data.kind == "proc_ref_count":
        # The count must equal the number of locations the peek resolves, so
        # derive it from the same ``find_references`` pass rather than the
        # parallel ``proc_usage_counts`` aggregate. The two used to drift
        # (issue #637): a forward or cross-file call resolves to ``None`` at
        # analysis time, so it is absent from ``proc_usage_counts`` (keyed by
        # the resolved qualified name) yet still matched by name in the
        # reference resolution — giving "0 references" above a proc the peek
        # listed as used. ``proc_usage_counts`` remains the fallback only for
        # callers (e.g. unit tests) that supply no resolver.
        if find_references is not None:
            locations = find_references(data.uri, data.qname)
            count = len(locations)
        else:
            locations = []
            counts = workspace_index.proc_usage_counts()
            count = counts.get(data.qname, 0)
        title = f"{count} reference" if count == 1 else f"{count} references"
        return types.CodeLens(
            range=lens.range,
            command=types.Command(
                title=title,
                command="tcl-lsp.showReferences",
                arguments=[data.uri, lens.range.start, locations],
            ),
            data=lens.data,
        )
    # Unknown lens kind — return as-is so the client just ignores it.
    return lens
