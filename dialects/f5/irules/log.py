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

"""log -- Write a message to BIG-IP logging facilities."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/log.html"


def _fac(name: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=f"{name}.",
        detail=f"Syslog facility: {name}",
    )


def _lvl(name: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=name,
        detail=f"Syslog level: {name}",
    )


_FACILITIES = (
    _fac("local0"),
    _fac("local1"),
    _fac("local2"),
    _fac("local3"),
    _fac("local4"),
    _fac("local5"),
    _fac("local6"),
    _fac("local7"),
    _fac("kern"),
    _fac("user"),
    _fac("mail"),
    _fac("daemon"),
    _fac("auth"),
    _fac("syslog"),
    _fac("lpr"),
    _fac("news"),
    _fac("uucp"),
    _fac("cron"),
    _fac("authpriv"),
    _fac("ftp"),
    _fac("ntp"),
    _fac("security"),
    _fac("console"),
)

_LEVELS = (
    _lvl("emerg"),
    _lvl("alert"),
    _lvl("crit"),
    _lvl("err"),
    _lvl("warning"),
    _lvl("notice"),
    _lvl("info"),
    _lvl("debug"),
)


@register
class LogCommand(CommandDef):
    name = "log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="log",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Write a message to BIG-IP logging facilities.",
                synopsis=("log ?facility.level? message",),
                snippet='Common form: `log local0. "message"`.',
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="log ?-noname? ?facility.level? message",
                    options=(
                        OptionSpec(
                            name="-noname",
                            detail="Suppress iRule name prefix in log message.",
                            takes_value=False,
                        ),
                    ),
                    arg_values={
                        0: _FACILITIES,
                    },
                    subcommand_arg_values={
                        # After selecting a facility (e.g. "local0."), offer levels
                        ("local0.", 0): _LEVELS,
                        ("local1.", 0): _LEVELS,
                        ("local2.", 0): _LEVELS,
                        ("local3.", 0): _LEVELS,
                        ("local4.", 0): _LEVELS,
                        ("local5.", 0): _LEVELS,
                        ("local6.", 0): _LEVELS,
                        ("local7.", 0): _LEVELS,
                        ("kern.", 0): _LEVELS,
                        ("user.", 0): _LEVELS,
                        ("mail.", 0): _LEVELS,
                        ("daemon.", 0): _LEVELS,
                        ("auth.", 0): _LEVELS,
                        ("syslog.", 0): _LEVELS,
                        ("lpr.", 0): _LEVELS,
                        ("news.", 0): _LEVELS,
                        ("uucp.", 0): _LEVELS,
                        ("cron.", 0): _LEVELS,
                        ("authpriv.", 0): _LEVELS,
                        ("ftp.", 0): _LEVELS,
                        ("ntp.", 0): _LEVELS,
                        ("security.", 0): _LEVELS,
                        ("console.", 0): _LEVELS,
                    },
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            taint_log_sink="IRULE3003",
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
