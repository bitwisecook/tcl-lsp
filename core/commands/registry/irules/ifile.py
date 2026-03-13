# Enriched from F5 iRules reference documentation.
"""ifile -- Returns content and attributes from external files on the BIG-IP system."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ifile.html"


_av = make_av(_SOURCE)


@register
class IfileCommand(CommandDef):
    name = "ifile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ifile",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns content and attributes from external files on the BIG-IP system.",
                synopsis=(
                    "ifile 'listall'",
                    "ifile (",
                ),
                snippet=(
                    "This iRules command returns content and attributes from external files\n"
                    "on the BIG-IP system"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "   # Retrieve the file contents, send it in an HTTP 200 response and clear the temporary variable\n"
                    '   set ifileContent [ifile get "/Common/iFile-index.html"]\n'
                    "   HTTP::respond 200 content $ifileContent\n"
                    "   unset ifileContent\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ifile 'listall'",
                    arg_values={0: (_av("listall", "ifile listall", "ifile 'listall'"),)},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
