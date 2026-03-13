# Enriched from F5 iRules reference documentation.
"""GTP::new -- Creates a new GTP message for given version & request-type."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__new.html"


@register
class GtpNewCommand(CommandDef):
    name = "GTP::new"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::new",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new GTP message for given version & request-type.",
                synopsis=("GTP::new VERSION TYPE",),
                snippet=(
                    "Creates a new GTP message for given version & request-type.\n"
                    "Valid values for version are 1 or 2 only.\n"
                    "Request-type: A value less than 256.\n"
                    'Returns a TCL object of type "GTP-Message"'
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set t2 [GTP::new 2 10]\n"
                    '    log local0. "GTP version [GTP::header version -message $t2]"\n'
                    '    log local0. "GTP type [GTP::header type -message $t2]"\n'
                    "}"
                ),
                return_value='Returns a TCL object of type "GTP-Message"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::new VERSION TYPE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
