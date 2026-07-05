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

"""lsearch -- Search a list for an element matching a pattern."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    OptionSpec,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register
from .shimmer_resolvers import resolve_lsearch


@register
class LsearchCommand(CommandDef):
    name = "lsearch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lsearch",
            byte_compiled=True,
            frameless_runtime=True,
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lsearch ?options? list pattern",
                    options=(
                        OptionSpec(name="-exact"),
                        OptionSpec(name="-glob"),
                        OptionSpec(name="-regexp"),
                        OptionSpec(name="-sorted"),
                        OptionSpec(name="-all"),
                        OptionSpec(name="-inline"),
                        OptionSpec(name="-not"),
                        OptionSpec(name="-start", takes_value=True, value_hint="index"),
                        OptionSpec(name="-ascii"),
                        OptionSpec(name="-dictionary"),
                        OptionSpec(name="-integer"),
                        OptionSpec(name="-real"),
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-increasing"),
                        OptionSpec(name="-decreasing"),
                        # -bisect: Tcl 8.6+ (rejected as "bad option" in 8.4/8.5).
                        OptionSpec(
                            name="-bisect",
                            detail="Binary search a sorted list (Tcl 8.6+).",
                            dialects=frozenset({"tcl8.6", "tcl9.0"}),
                        ),
                        # -index: Tcl 8.5+ (rejected in 8.4).
                        OptionSpec(
                            name="-index",
                            takes_value=True,
                            value_hint="index",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                        # -stride: TIP 351 was accepted but only landed in
                        # Tcl 9.0 — tclsh 8.5/8.6 reject it with "bad option
                        # \"-stride\"".  Verified against built tclsh 8.5/8.6.
                        OptionSpec(
                            name="-stride",
                            takes_value=True,
                            value_hint="strideLength",
                            detail="Treat the list as groups of N elements (TIP 351, Tcl 9.0+).",
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        # -subindices: Tcl 8.5+ (rejected in 8.4).
                        OptionSpec(
                            name="-subindices",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.INT,
            arg_type_resolver=resolve_lsearch,
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_search",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
