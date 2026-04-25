"""Code lens provider -- reference counts on procs.

Emits lenses in two phases:

1. ``get_code_lenses`` produces lightweight lenses with a ``data`` payload and
   no command; the client must call ``codeLens/resolve`` for the final title
   and command.
2. ``resolve_code_lens`` looks up the cached counts from the workspace index
   and returns a fully populated ``CodeLens``.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Protocol

from lsprotocol import types

from core.analysis import analyse
from core.analysis.semantic_model import AnalysisResult, ProcDef
from core.common.lsp import to_lsp_range


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
        counts = workspace_index.proc_usage_counts()
        count = counts.get(data.qname, 0)
        title = f"{count} reference" if count == 1 else f"{count} references"
        locations = find_references(data.uri, data.qname) if find_references else []
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
