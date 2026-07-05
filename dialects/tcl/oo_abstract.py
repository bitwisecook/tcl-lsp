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

# Scaffolded from class.n -- refine and commit
"""oo::abstract -- metaclass for abstract classes."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register
from .oo_class import _oo_metaclass_arg_roles

_SOURCE = "Tcl man page abstract.n"


_av = make_av(_SOURCE)


@register
class OoAbstractCommand(CommandDef):
    name = "oo::abstract"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::abstract",
            is_oo_metaclass=True,
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="metaclass for abstract classes",
                synopsis=("oo::abstract method ?arg ...?",),
                snippet="The oo::abstract command creates a class that cannot be directly instantiated. Only subclasses of an abstract class may be instantiated.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::abstract method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "This creates a new abstract class called name. Note: create is not exported on instances of oo::abstract; abstract classes cannot be directly instantiated.",
                                "cls create name ?arg ...?",
                            ),
                            _av(
                                "new",
                                "This creates a new abstract class with a unique name. Note: new is not exported on instances of oo::abstract; abstract classes cannot be directly instantiated.",
                                "cls new ?arg ...?",
                            ),
                            _av(
                                "createWithNamespace",
                                "This creates a new abstract class with an explicitly chosen namespace. Note: createWithNamespace is not exported on instances of oo::abstract.",
                                "cls createWithNamespace name nsName ?arg ...?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            arg_role_resolver=_oo_metaclass_arg_roles,
            # See ``oo::class`` — the metaclass body runs in the class's
            # own definition context, not the caller's scope.
            body_kind=BodyKind.STRUCTURAL,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
