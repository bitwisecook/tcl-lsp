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

"""f5report — interactive HTML reports for F5 BIG-IP configs.

The heavy lifting is done by the F5 BIG-IP *query engine* — the same
jq-flavoured ``f5-query`` DSL that ships with tcl-lsp — exposed to Python as a
native PyO3 extension (:mod:`f5report._engine`). Nothing here shells out to a
subprocess: configs and UCS archives are parsed, queried and projected entirely
in-process.

Typical use::

    import f5report
    sources = f5report.load_paths(["device.ucs"])
    html = f5report.build_report(sources, title="Production LTM")
    open("report.html", "w").write(html)
"""

from __future__ import annotations

from . import _engine
from ._engine import QueryError, load_paths, query, ucs_to_scf
from .report import build_report, collect_model

__all__ = [
    "QueryError",
    "build_report",
    "collect_model",
    "engine_version",
    "load_paths",
    "query",
    "ucs_to_scf",
]

# Taken from the native binding rather than written here: releases are tag-only,
# so a literal in the tree would go stale the moment a tag is cut. The binding
# resolves it at compile time from TCL_LSP_VERSION, else `git describe`.
__version__ = _engine.__version__


def engine_version() -> str:
    """Return the version of the native query-engine binding."""
    return _engine.__version__
