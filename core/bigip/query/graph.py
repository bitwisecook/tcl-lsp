"""Forward / reverse reference helpers for the query DSL.

The query DSL exposes ``refs(.)`` and ``referenced_by(.)`` builtins
that walk the same graph the ``f5 grep`` verb uses.  Rather than
re-implement the edge enumeration, we forward through to the existing
helpers in :mod:`core.bigip.grep`.
"""

from __future__ import annotations

from .errors import BuiltinError
from .values import ObjectRef


def _root_for(obj: ObjectRef):
    if obj.config_uri == "":
        raise BuiltinError("graph builtins require an object loaded from a config")
    from .runner import _ACTIVE_ROOTS

    root = _ACTIVE_ROOTS.get(obj.config_uri)
    if root is None:
        raise BuiltinError("graph builtins called outside an active query context")
    return root


def forward_refs(obj: ObjectRef) -> list[str]:
    from ..grep import compute_grep

    root = _root_for(obj)
    report = compute_grep(
        sources={root.uri: root.source},
        configs={root.uri: root.config},
        pattern=obj.full_path,
        use_regex=False,
        use_cidr=False,
        direction="forward",
        max_depth=1,
        max_nodes=1024,
        include_body=False,
        recurse=True,
    )
    seen: list[str] = []
    for node in report.related:
        if node.full_path == obj.full_path:
            continue
        seen.append(node.full_path)
    return seen


def reverse_refs(obj: ObjectRef) -> list[str]:
    from ..grep import compute_grep

    root = _root_for(obj)
    report = compute_grep(
        sources={root.uri: root.source},
        configs={root.uri: root.config},
        pattern=obj.full_path,
        use_regex=False,
        use_cidr=False,
        direction="reverse",
        max_depth=1,
        max_nodes=1024,
        include_body=False,
        recurse=True,
    )
    seen: list[str] = []
    for node in report.related:
        if node.full_path == obj.full_path:
            continue
        seen.append(node.full_path)
    return seen
