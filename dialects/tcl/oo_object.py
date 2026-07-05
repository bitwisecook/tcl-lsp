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

# Scaffolded from object.n -- refine and commit
"""oo::object -- root class of the class hierarchy."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page object.n"


_av = make_av(_SOURCE)


@register
class OoObjectCommand(CommandDef):
    name = "oo::object"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::object",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="root class of the class hierarchy",
                synopsis=("oo::object method ?arg ...?",),
                snippet="The oo::object class is the root class of the object hierarchy; every object is an instance of this class.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::object method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "destroy",
                                "This method destroys the object, obj, that it is invoked upon, invoking any destructors on the object's class in the process.",
                                "obj destroy",
                            ),
                            _av(
                                "eval",
                                "This method concatenates the arguments, arg, as if with concat, and then evaluates the resulting script in the namespace that is uniquely associated with obj, returning the result of the evaluation.",
                                "obj eval ?arg ...?",
                            ),
                            _av(
                                "unknown",
                                "This method is called when an attempt to invoke the method methodName on object obj fails.",
                                "obj unknown ?methodName? ?arg ...?",
                            ),
                            _av(
                                "variable",
                                "This method arranges for each variable called varName to be linked from the object obj's unique namespace into the caller's context.",
                                "obj variable ?varName ...?",
                            ),
                            _av(
                                "varname",
                                "This method returns the globally qualified name of the variable varName in the unique namespace for the object obj.",
                                "obj varname varName",
                            ),
                            _av(
                                "<cloned>",
                                "This method is used by oo::copy to copy state from one object to another. It copies procedures and variables from the source object's namespace.",
                                "obj <cloned> sourceObjectName",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
