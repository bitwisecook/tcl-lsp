# Enriched from F5 iRules reference documentation.
"""GTP::ie -- This set of commands allows for the parsing and interpretation of GTP IE elements."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__ie.html"


_av = make_av(_SOURCE)


@register
class GtpIeCommand(CommandDef):
    name = "GTP::ie"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::ie",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This set of commands allows for the parsing and interpretation of GTP IE elements.",
                synopsis=(
                    "GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
                    "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                    "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                    "GTP::ie 'get' 'list' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                ),
                snippet=(
                    "This set of commands allows for the parsing and interpretation of GTP\n"
                    "IE elements."
                ),
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_INGRESS {\n"
                    "    if { [GTP::ie exists imsi:0] } {\n"
                    '        log local0. "GTP imsi [GTP::ie get value imsi:0]"\n'
                    "    }\n"
                    '    log local0. "Total number of top level IEs [GTP::ie count]"\n'
                    "    set ie_list [ GTP::ie get list]\n"
                    "    foreach ie $ie_list {\n"
                    '        log local0. "IE $ie"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
                    options=(
                        OptionSpec(name="-message", detail="Option -message.", takes_value=True),
                        OptionSpec(name="-type", detail="Option -type.", takes_value=True),
                        OptionSpec(name="-instance", detail="Option -instance.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "exists",
                                "GTP::ie exists",
                                "GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
                            ),
                            _av(
                                "count",
                                "GTP::ie count",
                                "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                            ),
                            _av(
                                "get",
                                "GTP::ie get",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "instance",
                                "GTP::ie instance",
                                "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                            ),
                            _av(
                                "length",
                                "GTP::ie length",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "encode-type",
                                "GTP::ie encode-type",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "value",
                                "GTP::ie value",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "list",
                                "GTP::ie list",
                                "GTP::ie 'get' 'list' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
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
