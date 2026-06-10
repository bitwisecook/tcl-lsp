# Enriched from F5 iRules reference documentation.
"""SIP::response -- Gets or rewrites the SIP response."""

# Introduced: BIG-IP v10+ (core SIP iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__response.html"


_av = make_av(_SOURCE)


@register
class SipResponseCommand(CommandDef):
    name = "SIP::response"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::response",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or rewrites the SIP response.",
                synopsis=(
                    "SIP::response (code | phrase)",
                    "SIP::response rewrite CODE (PHRASE)?",
                ),
                snippet=(
                    "These commands allow you to get or rewrite the SIP response code or\nphrase."
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_RESPONSE {\n"
                    "  log local0. [SIP::via 0]\n"
                    "  SIP::header remove Via 0\n"
                    '  SIP::response rewrite 123 "no xxx"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::response <subcommand> ?args?",
                    arg_values={
                        0: (
                            _av("code", "Get response code.", "SIP::response code"),
                            _av("phrase", "Get response phrase.", "SIP::response phrase"),
                            _av(
                                "rewrite",
                                "Rewrite response code and phrase.",
                                "SIP::response rewrite <code> ?phrase?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "code": SubCommand(
                    name="code",
                    arity=Arity(0, 0),
                    detail="Get response code.",
                    synopsis="SIP::response code",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.NETWORK_IO,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "phrase": SubCommand(
                    name="phrase",
                    arity=Arity(0, 0),
                    detail="Get response phrase.",
                    synopsis="SIP::response phrase",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.NETWORK_IO,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "rewrite": SubCommand(
                    name="rewrite",
                    arity=Arity(1, 2),
                    detail="Rewrite response code and phrase.",
                    synopsis="SIP::response rewrite <code> ?phrase?",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.NETWORK_IO,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
