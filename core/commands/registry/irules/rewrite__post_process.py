# Enriched from F5 iRules reference documentation.
"""REWRITE::post_process -- Toggle post processing functionality."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/REWRITE__post_process.html"


@register
class RewritePostProcessCommand(CommandDef):
    name = "REWRITE::post_process"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="REWRITE::post_process",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggle post processing functionality.",
                synopsis=("REWRITE::post_process (SWITCH)?",),
                snippet=(
                    "When REWRITE::post_process is called (without any arguments), it\n"
                    'will return a "0" to signify that it is off, or an "1" to signify that\n'
                    'it is on. By default, it is off. Use the command "REWRITE::post_process\n'
                    '1" to turn on the post process functionality and "REWRITE::post_process\n'
                    '0" to turn it off. When post_process is on, the\n'
                    "REWRITE_RESPONSE_DONE event is triggered. Otherwise, the\n"
                    "REWRITE_RESPONSE_DONE event is ignored."
                ),
                source=_SOURCE,
                examples=(
                    "when REWRITE_REQUEST_DONE {\n"
                    '  if { "[HTTP::host][HTTP::path]" eq "www.external.com/contents.php" } {\n'
                    "    # Found the file we wanted to modify\n"
                    "    REWRITE::post_process 1\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="REWRITE::post_process (SWITCH)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"REWRITE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
