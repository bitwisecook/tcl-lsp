# Enriched from F5 iRules reference documentation.
"""LSN::persistence -- Set the translation address and port selection mode for the current connection, and the translation entry timeout."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__persistence.html"


_av = make_av(_SOURCE)


@register
class LsnPersistenceCommand(CommandDef):
    name = "LSN::persistence"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::persistence",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the translation address and port selection mode for the current connection, and the translation entry timeout.",
                synopsis=(
                    "LSN::persistence none (TIMEOUT)?",
                    "LSN::persistence (address | address-port) TIMEOUT",
                ),
                snippet=(
                    "Set the translation address and port selection mode for the current connection, and the translation entry timeout.\n"
                    "\n"
                    "LSN::persistence <none|address|address-port|strict-address-port> <timeout>"
                ),
                source=_SOURCE,
                return_value="LSN::persistence none",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::persistence none (TIMEOUT)?",
                    arg_values={
                        0: (
                            _av(
                                "address",
                                "LSN::persistence address",
                                "LSN::persistence (address | address-port) TIMEOUT",
                            ),
                            _av(
                                "address-port",
                                "LSN::persistence address-port",
                                "LSN::persistence (address | address-port) TIMEOUT",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=_LSN_EVENT_REQUIRES,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
