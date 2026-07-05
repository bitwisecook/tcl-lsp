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
