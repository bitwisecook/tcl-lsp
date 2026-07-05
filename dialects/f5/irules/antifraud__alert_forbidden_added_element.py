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

# Generated from F5 iRules reference documentation -- do not edit manually.
"""ANTIFRAUD::alert_forbidden_added_element -- Deprecated: For the external_sources alert: returns forbidden added HTML element and its content, in an escaped base64 format."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .antifraud__alert_details import AntifraudAlertDetailsCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_forbidden_added_element.html"


@register
class AntifraudAlertForbiddenAddedElementCommand(CommandDef):
    name = "ANTIFRAUD::alert_forbidden_added_element"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_forbidden_added_element",
            deprecated_replacement=AntifraudAlertDetailsCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: For the external_sources alert: returns forbidden added HTML element and its content, in an escaped base64 format.",
                synopsis=("ANTIFRAUD::alert_forbidden_added_element",),
                snippet="For the external_sources alert: returns forbidden added HTML element and its content, in an escaped base64 format.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_forbidden_added_element",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ANTIFRAUD"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
