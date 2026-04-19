# Enriched from F5 iRules reference documentation.
"""PEM::session -- This command allows you to create, delete or retreive information of a PEM session using session IP address in the PEM Session DB."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PEM__session.html"


_av = make_av(_SOURCE)


@register
class PemSessionCommand(CommandDef):
    name = "PEM::session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PEM::session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to create, delete or retreive information of a PEM session using session IP address in the PEM Session DB.",
                synopsis=(
                    "PEM::session config policy ((get IP_ADDR) |",
                    "PEM::session delete IP_ADDR",
                ),
                snippet=(
                    "This command allows you to create, delete or retreive information of a PEM Session in the PEM Session DB.\n"
                    "Each PEM session carries the following standard attributes: imsi, imeisv, tower-id, rat-type, user-name, state, aaa-reporting-interval, provision.\n"
                    "\n"
                    "Details (Syntax):\n"
                    "PEM::session create <framed ip> [subscriber-id <string> subscriber-type <e164 | imsi | nai | private>] [imsi <sring>] [user-name <string>] [tower-id <string>] [imeisv <string>] [provision <yes | no>] [<custom attr> <custom value>] [policy <string1> ..."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    PEM::session create 10.10.10.10 subscriber-id 12345 subscriber-type e164 policy pem-policy1 pem-policy2\n"
                    "\n"
                    "    set polisy_var [PEM::session config policy get 10.10.10.10]\n"
                    "    set ip_var [PEM::session ip 12345 e164]\n"
                    "    set id_var [PEM::session info 10.10.10.10 subscriber-id]\n"
                    "\n"
                    "    PEM::session delete 10.10.10.10\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PEM::session config policy ((get IP_ADDR) |",
                    arg_values={
                        0: (
                            _av(
                                "get",
                                "PEM::session get",
                                "PEM::session config policy ((get IP_ADDR) |",
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
