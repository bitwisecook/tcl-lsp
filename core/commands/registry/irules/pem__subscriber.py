# Enriched from F5 iRules reference documentation.
"""PEM::subscriber -- This command allows you to create, delete or retreive information of a PEM subscriber session with subscriber-id and subscriber-id-type in the PEM Session DB."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PEM__subscriber.html"


_av = make_av(_SOURCE)


@register
class PemSubscriberCommand(CommandDef):
    name = "PEM::subscriber"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PEM::subscriber",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to create, delete or retreive information of a PEM subscriber session with subscriber-id and subscriber-id-type in the PEM Session DB.",
                synopsis=(
                    "PEM::subscriber config policy ( (get SUBS_ID (PEM_SUBS_TYPE2))",
                    "PEM::subscriber create SUBS_ID ( (subscriber-id-type | subscriber-type) (PEM_SUBS_TYPE2)) (ip-address IP_ADDR)+",
                ),
                snippet=(
                    "Each PEM session carries the following standard attributes: imsi, imeisv, tower-id, rat-type, user-name, subscriber-type, subscriber-id, state, aaa-reporting-interval, provision.\n"
                    "\n"
                    "Details (Syntax):\n"
                    "PEM::subscriber create <subscriber-id> subscriber-type <e164 | imsi | nai | private> ip-address <IP_ADDR> [imsi <sring>] [user-name <string>] [tower-id <string>] [imeisv <string>] [provision <yes | no>] [<custom attr> <custom value>] [policy <string1> ..."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    PEM::subscriber create 4086007577 subscriber-type e164 ip-address 10.10.10.10\n"
                    "    PEM::subscriber config policy referential set 4086007577 e164 policy1\n"
                    "    PEM::subscriber ip 4086007577 e164 10.0.0.92 2001::101\n"
                    "    PEM::subscriber info 4086007577 e164 imsi 1223456\n"
                    "    PEM::subscriber info 4086007577 e164 imeisv 2534535\n"
                    "    PEM::subscriber info 4086007577 e164 tower-id 8248249\n"
                    "    PEM::subscriber info 4086007577 e164 user-name 2139449"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PEM::subscriber config policy ( (get SUBS_ID (PEM_SUBS_TYPE2))",
                    arg_values={
                        0: (
                            _av(
                                "subscriber-id-type",
                                "PEM::subscriber subscriber-id-type",
                                "PEM::subscriber create SUBS_ID ( (subscriber-id-type | subscriber-type) (PEM_SUBS_TYPE2)) (ip-address IP_ADDR)+",
                            ),
                            _av(
                                "subscriber-type",
                                "PEM::subscriber subscriber-type",
                                "PEM::subscriber create SUBS_ID ( (subscriber-id-type | subscriber-type) (PEM_SUBS_TYPE2)) (ip-address IP_ADDR)+",
                            ),
                            _av(
                                "ip-address",
                                "PEM::subscriber ip-address",
                                "PEM::subscriber create SUBS_ID ( (subscriber-id-type | subscriber-type) (PEM_SUBS_TYPE2)) (ip-address IP_ADDR)+",
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
