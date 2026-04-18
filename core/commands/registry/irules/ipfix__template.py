# Enriched from F5 iRules reference documentation.
"""IPFIX::template -- IPFIX::template Provides the ability to create and delete IPFIX message templates that may be used to generate IPFIX messages based on processing in the iRule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IPFIX__template.html"


_av = make_av(_SOURCE)


@register
class IpfixTemplateCommand(CommandDef):
    name = "IPFIX::template"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IPFIX::template",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="IPFIX::template Provides the ability to create and delete IPFIX message templates that may be used to generate IPFIX messages based on processing in the iRule.",
                synopsis=("IPFIX::template ( (create TEMPLATE_STRING) |",),
                snippet=(
                    "This command provides the ability to create and delete user defined IPFIX\n"
                    "message templates that may be used to send IPFIX messages to a specified\n"
                    "destination."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    '    set static::http_track_dest ""\n'
                    '    set static::http_track_tmplt ""\n'
                    "}"
                ),
                return_value="IPFIX::template create TEMPLATE_STRING returns an IPFIX template object that is used by the IPFIX::msg create command and IPFIX::template delete command.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IPFIX::template ( (create TEMPLATE_STRING) |",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "IPFIX::template create",
                                "IPFIX::template ( (create TEMPLATE_STRING) |",
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
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
