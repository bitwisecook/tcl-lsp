# Enriched from F5 iRules reference documentation.
"""SDP::media -- Get or set SDP media information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SDP__media.html"


_av = make_av(_SOURCE)


@register
class SdpMediaCommand(CommandDef):
    name = "SDP::media"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SDP::media",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set SDP media information.",
                synopsis=(
                    "SDP::media (count | MEDIA_INDEX)?",
                    "SDP::media (type | transport) (MEDIA_INDEX)?",
                    "SDP::media attr (MEDIA_INDEX (ATTR_INDEX)?)?",
                    "SDP::media port (MEDIA_INDEX (NEW_PORT)?)?",
                ),
                snippet=(
                    "This command allows you to get or set different aspects of the media\n"
                    "information for your SDP connection."
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    '    log local0. "SDP media count: [SDP::media count]"\n'
                    '    log local0. "SDP media transport: [SDP::media transport 0]"\n'
                    '    log local0. "SDP media port: [SDP::media port 0]"\n'
                    '    log local0. "SDP media connection: [SDP::media conn 0]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SDP::media (count | MEDIA_INDEX)?",
                    arg_values={
                        0: (
                            _av("count", "SDP::media count", "SDP::media (count | MEDIA_INDEX)?"),
                            _av(
                                "type",
                                "SDP::media type",
                                "SDP::media (type | transport) (MEDIA_INDEX)?",
                            ),
                            _av(
                                "transport",
                                "SDP::media transport",
                                "SDP::media (type | transport) (MEDIA_INDEX)?",
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
