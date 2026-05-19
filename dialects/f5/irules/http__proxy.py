# Enriched from F5 iRules reference documentation.
"""HTTP::proxy -- Controls the application of HTTP proxy when using an Explicit HTTP profile."""

# Introduced: BIG-IP v9+ (core HTTP iRules command) (approximate, from F5 documentation)

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

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__proxy.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.HTTP_URI, reads=True, connection_side=ConnectionSide.BOTH),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.HTTP_URI,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)


@register
class HttpProxyCommand(CommandDef):
    name = "HTTP::proxy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::proxy",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls the application of HTTP proxy when using an Explicit HTTP profile.",
                synopsis=(
                    "HTTP::proxy",
                    "HTTP::proxy ('enable' | 'disable')",
                    "HTTP::proxy 'uri-rewrite' ('enable' | 'disable')",
                    "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')",
                    "HTTP::proxy chain ?args?",
                ),
                snippet=(
                    "When an Explicit HTTP profile is applied to a virtual server, HTTP::proxy allows control of whether the BIG-IP will handle the proxy of the connection locally or send it to a downstream pool for processing instead.\n"
                    "\n"
                    "This functionality was introduced in v11.6, and is available for v11.5.1 via an Engineering Hotfix.\n"
                    "\n"
                    "HTTP::proxy allows inspection of the results of the DNS lookup used in the Explicit HTTP Proxy.\n"
                    "\n"
                    "When a HTTP Proxy Chaining profile is applied to a virtual server, HTTP::proxy chain may be used to control the CONNECT request used to connect to the next proxy in the chain."
                ),
                source=_SOURCE,
                examples=('when HTTP_REQUEST {\n    log local0. "[HTTP::method] [HTTP::uri]"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::proxy ?subcommand? ?args?",
                    arg_values={
                        0: (
                            _av("enable", "Enable HTTP proxy.", "HTTP::proxy enable"),
                            _av("disable", "Disable HTTP proxy.", "HTTP::proxy disable"),
                            _av(
                                "uri-rewrite",
                                "Control URI rewriting.",
                                "HTTP::proxy uri-rewrite ?enable|disable?",
                            ),
                            _av("addr", "Get proxy destination address.", "HTTP::proxy addr"),
                            _av("port", "Get proxy destination port.", "HTTP::proxy port"),
                            _av("rtdom", "Get proxy route domain.", "HTTP::proxy rtdom"),
                            _av("exists", "Check if proxy is active.", "HTTP::proxy exists"),
                            _av("iptuple", "Get proxy IP tuple.", "HTTP::proxy iptuple"),
                            _av("chain", "Control proxy chaining.", "HTTP::proxy chain ?args?"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "enable": SubCommand(
                    name="enable",
                    arity=Arity(0, 0),
                    detail="Enable HTTP proxy.",
                    synopsis="HTTP::proxy enable",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "disable": SubCommand(
                    name="disable",
                    arity=Arity(0, 0),
                    detail="Disable HTTP proxy.",
                    synopsis="HTTP::proxy disable",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "uri-rewrite": SubCommand(
                    name="uri-rewrite",
                    arity=Arity(1, 1),
                    detail="Control URI rewriting.",
                    synopsis="HTTP::proxy uri-rewrite ?enable|disable?",
                    mutator=True,
                    arg_values={
                        0: (
                            _av(
                                "enable", "Enable URI rewriting.", "HTTP::proxy uri-rewrite enable"
                            ),
                            _av(
                                "disable",
                                "Disable URI rewriting.",
                                "HTTP::proxy uri-rewrite disable",
                            ),
                        )
                    },
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "addr": SubCommand(
                    name="addr",
                    arity=Arity(0, 0),
                    detail="Get proxy destination address.",
                    synopsis="HTTP::proxy addr",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "port": SubCommand(
                    name="port",
                    arity=Arity(0, 0),
                    detail="Get proxy destination port.",
                    synopsis="HTTP::proxy port",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "rtdom": SubCommand(
                    name="rtdom",
                    arity=Arity(0, 0),
                    detail="Get proxy route domain.",
                    synopsis="HTTP::proxy rtdom",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(0, 0),
                    detail="Check if proxy is active.",
                    synopsis="HTTP::proxy exists",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "iptuple": SubCommand(
                    name="iptuple",
                    arity=Arity(0, 0),
                    detail="Get proxy IP tuple.",
                    synopsis="HTTP::proxy iptuple",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "chain": SubCommand(
                    name="chain",
                    arity=Arity(0),
                    detail="Control proxy chaining.",
                    synopsis="HTTP::proxy chain ?args?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )
