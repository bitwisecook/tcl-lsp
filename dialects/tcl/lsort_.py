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

"""lsort -- Sort the elements of a list."""

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
from compiler.side_effects import StorageType
from compiler.types import TclType

from ._base import register
from .shimmer_resolvers import resolve_lsort


@register
class LsortCommand(CommandDef):
    name = "lsort"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lsort",
            byte_compiled=True,
            frameless_runtime=True,
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lsort ?options? list",
                    options=(
                        OptionSpec(name="-ascii"),
                        OptionSpec(name="-dictionary"),
                        OptionSpec(name="-integer"),
                        OptionSpec(name="-real"),
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-increasing"),
                        OptionSpec(name="-decreasing"),
                        # -indices: Tcl 8.5+ (rejected in 8.4).
                        OptionSpec(
                            name="-indices",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                        OptionSpec(name="-unique"),
                        OptionSpec(name="-command", takes_value=True, value_hint="cmdPrefix"),
                        OptionSpec(name="-index", takes_value=True, value_hint="index"),
                        # -stride: Tcl 8.6+ (rejected in 8.4/8.5).
                        OptionSpec(
                            name="-stride",
                            takes_value=True,
                            value_hint="length",
                            dialects=frozenset({"tcl8.6", "tcl9.0"}),
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.LIST,
            arg_type_resolver=resolve_lsort,
            inferred_storage_type=StorageType.LIST,
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_sort",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
