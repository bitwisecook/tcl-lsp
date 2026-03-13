# Enriched from F5 iRules reference documentation.
"""IVS_ENTRY::result -- Sends a result code to the IVS client."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IVS_ENTRY__result.html"


_av = make_av(_SOURCE)


@register
class IvsEntryResultCommand(CommandDef):
    name = "IVS_ENTRY::result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IVS_ENTRY::result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends a result code to the IVS client.",
                synopsis=("IVS_ENTRY::result (noop | modified | response)",),
                snippet=(
                    "Send a result code to the IVS (Internal Virtual Server) client\n"
                    "(usually ADAPT). The intent is to allow an IVS to be used in a\n"
                    'user-defined way without a specific IVS profile like "icap". If an\n'
                    '"icap" profile is present, IVS_ENTRY::result should not be used as\n'
                    "it would cause a second result to be sent to the IVS client\n"
                    "(usually ADAPT), with undefined effect."
                ),
                source=_SOURCE,
                examples=(
                    "when IVS_ENTRY_REQUEST {\n"
                    "                # Tell primary virtual the IVS will not handle this request\n"
                    "                IVS_ENTRY::result noop\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IVS_ENTRY::result (noop | modified | response)",
                    arg_values={
                        0: (
                            _av(
                                "noop",
                                "IVS_ENTRY::result noop",
                                "IVS_ENTRY::result (noop | modified | response)",
                            ),
                            _av(
                                "modified",
                                "IVS_ENTRY::result modified",
                                "IVS_ENTRY::result (noop | modified | response)",
                            ),
                            _av(
                                "response",
                                "IVS_ENTRY::result response",
                                "IVS_ENTRY::result (noop | modified | response)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ICAP", "IVS_ENTRY"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
