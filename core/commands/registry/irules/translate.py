# Enriched from F5 iRules reference documentation.
"""translate -- Enables, disables, or queries (as specified) destination address or port translation."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/translate.html"


_av = make_av(_SOURCE)


@register
class TranslateCommand(CommandDef):
    name = "translate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="translate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables, disables, or queries (as specified) destination address or port translation.",
                synopsis=(
                    "translate (address | port | service)",
                    "translate (address | port | service) ((enable | disable)",
                ),
                snippet=(
                    "Enables, disables, or queries (as specified) destination address or\n"
                    "port translation"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { [IP::addr [IP::remote_addr] equals 10.0.8.0/24] } {\n"
                    "        translate address disable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="translate (address | port | service)",
                    arg_values={
                        0: (
                            _av(
                                "address",
                                "translate address",
                                "translate (address | port | service)",
                            ),
                            _av("port", "translate port", "translate (address | port | service)"),
                            _av(
                                "service",
                                "translate service",
                                "translate (address | port | service)",
                            ),
                            _av(
                                "enable",
                                "translate enable",
                                "translate (address | port | service) ((enable | disable)",
                            ),
                            _av(
                                "disable",
                                "translate disable",
                                "translate (address | port | service) ((enable | disable)",
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
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
