# Enriched from F5 iRules reference documentation.
"""STREAM::encoding -- Specifies non-default content encoding."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__encoding.html"


_av = make_av(_SOURCE)


@register
class StreamEncodingCommand(CommandDef):
    name = "STREAM::encoding"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::encoding",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Specifies non-default content encoding.",
                synopsis=("STREAM::encoding (ascii | utf-8 | unicode)",),
                snippet="Specifies non-default content encoding. The default value is ascii.",
                source=_SOURCE,
                examples=(
                    "when STREAM_MATCHED {\n"
                    "    set stream_match [STREAM::match]\n"
                    '    log local0. "$stream_match"\n'
                    "    STREAM::encoding utf-8\n"
                    "    # The ?/? represents unicode characters.\n"
                    '    if { $stream_match contains "hello?/?" } {\n'
                    '        STREAM::replace "hello hey"\n'
                    '        log local0. "stream match is [STREAM::match]"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::encoding (ascii | utf-8 | unicode)",
                    arg_values={
                        0: (
                            _av(
                                "ascii",
                                "STREAM::encoding ascii",
                                "STREAM::encoding (ascii | utf-8 | unicode)",
                            ),
                            _av(
                                "utf-8",
                                "STREAM::encoding utf-8",
                                "STREAM::encoding (ascii | utf-8 | unicode)",
                            ),
                            _av(
                                "unicode",
                                "STREAM::encoding unicode",
                                "STREAM::encoding (ascii | utf-8 | unicode)",
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
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
