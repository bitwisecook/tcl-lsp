# Enriched from F5 iRules reference documentation.
"""HSL::open -- Opens a handle for High Speed Logging communication."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HSL__open.html"


_av = make_av(_SOURCE)


@register
class HslOpenCommand(CommandDef):
    name = "HSL::open"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HSL::open",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Opens a handle for High Speed Logging communication.",
                synopsis=(
                    "HSL::open ('-publisher' | '-pub') PUBLISHER",
                    "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ",
                ),
                snippet=(
                    "Open a handle for High Speed Logging communication. After creating the\n"
                    "connection, send data on the connection using HSL::send."
                ),
                source=_SOURCE,
                examples=(
                    "#2\n"
                    "when CLIENT_ACCEPTED {\n"
                    "    set hsl [HSL::open -publisher /Common/lpAll]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HSL::open ('-publisher' | '-pub') PUBLISHER",
                    options=(
                        OptionSpec(
                            name="-publisher", detail="Option -publisher.", takes_value=True
                        ),
                        OptionSpec(name="-pub", detail="Option -pub.", takes_value=True),
                        OptionSpec(name="-proto", detail="Option -proto.", takes_value=True),
                        OptionSpec(name="-pool", detail="Option -pool.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "UDP",
                                "HSL::open UDP",
                                "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ",
                            ),
                            _av(
                                "TCP",
                                "HSL::open TCP",
                                "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
