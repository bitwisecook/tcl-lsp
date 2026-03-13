# Enriched from F5 iRules reference documentation.
"""GTP::header -- Allows for the parsing of GTP header information."""

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
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__header.html"


_av = make_av(_SOURCE)


@register
class GtpHeaderCommand(CommandDef):
    name = "GTP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows for the parsing of GTP header information.",
                synopsis=(
                    "GTP::header ('version' | 'type') ('-message' MESSAGE)?",
                    "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?",
                    "GTP::header ('teid' | 'npdu' | 'sequence') 'set' ('-message' MESSAGE)? VALUE",
                    "GTP::header ('teid' | 'npdu' | 'sequence') 'remove' ('-message' MESSAGE)?",
                ),
                snippet=(
                    "Allows for the parsing of GTP header information. UINT -- Unsigned\n"
                    "integer value of n bits. For n > 8, appropriate network to host byte\n"
                    "order conversion happens transparently."
                ),
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_INGRESS {\n"
                    '    log local0. "GTP version [GTP::header version]"\n'
                    '    log local0. "GTP type [GTP::header type]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::header ('version' | 'type') ('-message' MESSAGE)?",
                    options=(
                        OptionSpec(name="-message", detail="Option -message.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "version",
                                "GTP::header version",
                                "GTP::header ('version' | 'type') ('-message' MESSAGE)?",
                            ),
                            _av(
                                "type",
                                "GTP::header type",
                                "GTP::header ('version' | 'type') ('-message' MESSAGE)?",
                            ),
                            _av(
                                "teid",
                                "GTP::header teid",
                                "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?",
                            ),
                            _av(
                                "npdu",
                                "GTP::header npdu",
                                "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?",
                            ),
                            _av(
                                "sequence",
                                "GTP::header sequence",
                                "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?",
                            ),
                            _av(
                                "set",
                                "GTP::header set",
                                "GTP::header ('teid' | 'npdu' | 'sequence') 'set' ('-message' MESSAGE)? VALUE",
                            ),
                            _av(
                                "remove",
                                "GTP::header remove",
                                "GTP::header ('teid' | 'npdu' | 'sequence') 'remove' ('-message' MESSAGE)?",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
