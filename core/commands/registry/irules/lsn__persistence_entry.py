# Enriched from F5 iRules reference documentation.
"""LSN::persistence-entry -- Create or lookup LSN translation address."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__persistence-entry.html"


_av = make_av(_SOURCE)


@register
class LsnPersistenceEntryCommand(CommandDef):
    name = "LSN::persistence-entry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::persistence-entry",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Create or lookup LSN translation address.",
                synopsis=(
                    "LSN::persistence-entry (delete|get) CLIENT_ADDR",
                    "LSN::persistence-entry create (-override)? LSN_POOL CLIENT_ADDR TRANSLATION_ADDR (TIMEOUT)?",
                ),
                snippet=(
                    "Create or lookup LSN translation address. Those commands are linked to CGNAT module introduced in 11.3. You need to license and provision this module to use this command.\n"
                    "\n"
                    "LSN::persistence-entry create [-override] <client_address>[:<client_port>] [<translation_address>[:<translation_port>]]\n"
                    "LSN::persistence-entry get <client_address>[:<client_port>]\n"
                    "\n"
                    "v11.4+\n"
                    "LSN::persistence-entry create [-override] <lsn_pool>  <client_address>[:<port>] <translation_address>[:<port>]]  [timeout]\n"
                    "\n"
                    "v11.5+\n"
                    "LSN::persistence-entry delete <client_address>"
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    set clientIP [IP::client_addr]\n}"),
                return_value="LSN::persistence-entry create",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::persistence-entry (delete|get) CLIENT_ADDR",
                    options=(
                        OptionSpec(name="-override", detail="Option -override.", takes_value=False),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "delete",
                                "LSN::persistence-entry delete",
                                "LSN::persistence-entry (delete|get) CLIENT_ADDR",
                            ),
                            _av(
                                "get",
                                "LSN::persistence-entry get",
                                "LSN::persistence-entry (delete|get) CLIENT_ADDR",
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
                    target=SideEffectTarget.LSN_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
