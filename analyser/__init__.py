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

from __future__ import annotations

_EXPORTS = frozenset(
    {"Analyser", "AnalyserSnapshot", "AnalysisResult", "parse_param_list", "analyse"}
)

__all__ = list(_EXPORTS)


def __getattr__(name: str):
    if name in _EXPORTS:
        from ._analyser import (  # noqa: PLC0415
            Analyser,
            AnalyserSnapshot,
            AnalysisResult,
            analyse,
            parse_param_list,
        )

        g = globals()
        g["Analyser"] = Analyser
        g["AnalyserSnapshot"] = AnalyserSnapshot
        g["AnalysisResult"] = AnalysisResult
        g["parse_param_list"] = parse_param_list
        g["analyse"] = analyse
        return g[name]
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
