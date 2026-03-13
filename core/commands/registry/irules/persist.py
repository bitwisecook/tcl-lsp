# Enriched from F5 iRules reference documentation.
"""persist -- Sets the connection persistence type."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/persist.html"


_av = make_av(_SOURCE)


@register
class PersistCommand(CommandDef):
    name = "persist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="persist",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the connection persistence type.",
                synopsis=(
                    "persist none",
                    "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?",
                    "persist source_addr (IPV4_MASK)? (TIMEOUT)?",
                    "persist simple (IPV4_MASK)? (TIMEOUT)?",
                ),
                snippet=(
                    "Causes the system to use the named persistence type to persist the\n"
                    "connection. Also allows direct inspection and manipulation of the\n"
                    "persistence table."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    "   # Persist the client connection based on the SSL session ID\n"
                    "    persist ssl\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="persist none",
                    arg_values={
                        0: (
                            _av(
                                "insert",
                                "persist insert",
                                "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?",
                            ),
                            _av(
                                "rewrite",
                                "persist rewrite",
                                "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?",
                            ),
                            _av(
                                "passive",
                                "persist passive",
                                "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?",
                            ),
                            _av(
                                "hash",
                                "persist hash",
                                "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                client_side=True, flow=True, also_in=frozenset({"PERSIST_DOWN"})
            ),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PERSISTENCE_TABLE,
                    writes=True,
                    scope=StorageScope.PERSISTENCE,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
