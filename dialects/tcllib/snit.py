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

"""snit -- Snit's Not Incr Tcl, OO framework (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "tcllib snit package"
_PACKAGE = "snit"


@register
class SnitTypeCommand(CommandDef):
    name = "snit::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a new snit type (class).",
                synopsis=("snit::type name definition",),
                snippet=(
                    "Defines a new object type. The definition body "
                    "contains option, variable, constructor, destructor, "
                    "method, and typemethod declarations."
                ),
                source=_SOURCE,
                examples=(
                    "snit::type Dog {\n"
                    "    option -name Fido\n"
                    '    method bark {} { return "Woof!" }\n'
                    "}"
                ),
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="snit::type name definition"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            arg_roles={1: frozenset({ArgRole.BODY})},
            # The definition body runs in snit's own definition context
            # (it's reinterpreted by snit's parser into a class
            # description), not the caller's scope.
            body_kind=BodyKind.STRUCTURAL,
            creates_dynamic_barrier=True,
            never_inline_body=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class SnitWidgetCommand(CommandDef):
    name = "snit::widget"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a new snit megawidget type.",
                synopsis=("snit::widget name definition",),
                snippet=(
                    "Like snit::type but creates a Tk megawidget. "
                    "Automatically delegates to a hull widget."
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="snit::widget name definition"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            arg_roles={1: frozenset({ArgRole.BODY})},
            # See ``snit::type`` — definition body runs in snit's own
            # definition context.
            body_kind=BodyKind.STRUCTURAL,
            creates_dynamic_barrier=True,
            never_inline_body=True,
        )


@register
class SnitWidgetadaptorCommand(CommandDef):
    name = "snit::widgetadaptor"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a snit widget adaptor that wraps an existing widget.",
                synopsis=("snit::widgetadaptor name definition",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snit::widgetadaptor name definition",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            arg_roles={1: frozenset({ArgRole.BODY})},
            # See ``snit::type`` — definition body runs in snit's own
            # definition context.
            body_kind=BodyKind.STRUCTURAL,
            creates_dynamic_barrier=True,
            never_inline_body=True,
        )


@register
class SnitTypemethodCommand(CommandDef):
    name = "snit::typemethod"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a type method outside a type definition body.",
                synopsis=("snit::typemethod type name arglist body",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snit::typemethod type name arglist body",
                ),
            ),
            validation=ValidationSpec(arity=Arity(4, 4)),
            arg_roles={3: frozenset({ArgRole.BODY})},
            # Type method bodies run in the type's own dispatch context,
            # not the caller's scope — STRUCTURAL keeps the body out of
            # the enclosing block's data flow.
            body_kind=BodyKind.STRUCTURAL,
        )


@register
class SnitMethodCommand(CommandDef):
    name = "snit::method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define an instance method outside a type definition body.",
                synopsis=("snit::method type name arglist body",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snit::method type name arglist body",
                ),
            ),
            validation=ValidationSpec(arity=Arity(4, 4)),
            arg_roles={3: frozenset({ArgRole.BODY})},
            # Instance method bodies run in the object's own dispatch
            # context, not the caller's scope — STRUCTURAL keeps the
            # body out of the enclosing block's data flow.
            body_kind=BodyKind.STRUCTURAL,
        )


@register
class SnitCompileCommand(CommandDef):
    name = "snit::compile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compile a snit type definition into a Tcl script.",
                synopsis=("snit::compile which name body",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="snit::compile which name body"),),
            validation=ValidationSpec(arity=Arity(3, 3)),
            arg_roles={2: frozenset({ArgRole.BODY})},
            # The body is a snit type definition — run through snit's
            # compiler in its own definition context, not the caller's
            # scope.
            body_kind=BodyKind.STRUCTURAL,
            creates_dynamic_barrier=True,
            never_inline_body=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class SnitMacroCommand(CommandDef):
    name = "snit::macro"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Define a snit macro for use in type definitions.",
                synopsis=("snit::macro name arglist body",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="snit::macro name arglist body"),),
            validation=ValidationSpec(arity=Arity(3, 3)),
            arg_roles={2: frozenset({ArgRole.BODY})},
            # Macro bodies are reinterpreted by snit during type
            # definition processing — they don't share the caller's
            # scope.
            body_kind=BodyKind.STRUCTURAL,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
