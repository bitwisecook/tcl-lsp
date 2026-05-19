"""Forward / reverse reference helpers for the query DSL.

The query DSL exposes ``refs(.)`` and ``referenced_by(.)`` builtins
that walk the same graph the ``f5 grep`` verb uses.  Rather than
re-implement the edge enumeration, we forward through to the existing
helpers in :mod:`dialects.f5.bigip.grep`.
"""

from __future__ import annotations

from .errors import BuiltinError
from .values import ObjectRef


def _root_for(obj: ObjectRef):
    if obj.config_uri == "":
        raise BuiltinError("graph builtins require an object loaded from a config")
    from .runner import _lookup_active_root

    root = _lookup_active_root(obj.config_uri)
    if root is None:
        raise BuiltinError("graph builtins called outside an active query context")
    return root


def _grep_inputs(obj: ObjectRef) -> tuple[dict[str, str], dict]:
    """Return ``(sources, configs)`` for :func:`compute_grep`.

    In ``--merge`` mode every loaded source is fed in so references
    cross files (a gtm pool pointing into ltm resolves transparently).
    Outside merge mode we stay scoped to the originating source — two
    unrelated configs loaded for parallel inspection should not have
    their reference graphs joined behind the user's back.
    """
    from .runner import _is_merge_active, _lookup_active_roots

    if _is_merge_active():
        roots = _lookup_active_roots()
        if roots:
            return (
                {uri: r.source for uri, r in roots.items()},
                {uri: r.config for uri, r in roots.items()},
            )
    root = _root_for(obj)
    return ({root.uri: root.source}, {root.uri: root.config})


def forward_refs(obj: ObjectRef) -> list[str]:
    from dialects.f5.bigip.grep import compute_grep

    sources, configs = _grep_inputs(obj)
    # Exact-path seed: ``refs(/Common/p)`` must not also match
    # ``/Common/p2`` and inflate the result with edges from a
    # different object.
    report = compute_grep(
        sources=sources,
        configs=configs,
        pattern=obj.full_path,
        use_regex=False,
        use_cidr=False,
        use_exact=True,
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
    from dialects.f5.bigip.grep import compute_grep

    sources, configs = _grep_inputs(obj)
    # Exact-path seed: ``referenced_by(/Common/p)`` returns only
    # referrers of /Common/p, not of /Common/p2.
    report = compute_grep(
        sources=sources,
        configs=configs,
        pattern=obj.full_path,
        use_regex=False,
        use_cidr=False,
        use_exact=True,
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
