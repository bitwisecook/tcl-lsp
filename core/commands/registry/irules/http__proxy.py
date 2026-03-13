# Enriched from F5 iRules reference documentation.
"""HTTP::proxy -- Controls the application of HTTP proxy when using an Explicit HTTP profile."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__proxy.html"


_av = make_av(_SOURCE)


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
                    synopsis="HTTP::proxy",
                    arg_values={
                        0: (
                            _av(
                                "enable", "HTTP::proxy enable", "HTTP::proxy ('enable' | 'disable')"
                            ),
                            _av(
                                "disable",
                                "HTTP::proxy disable",
                                "HTTP::proxy ('enable' | 'disable')",
                            ),
                            _av(
                                "uri-rewrite",
                                "HTTP::proxy uri-rewrite",
                                "HTTP::proxy 'uri-rewrite' ('enable' | 'disable')",
                            ),
                            _av(
                                "addr",
                                "HTTP::proxy addr",
                                "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')",
                            ),
                            _av(
                                "port",
                                "HTTP::proxy port",
                                "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')",
                            ),
                            _av(
                                "rtdom",
                                "HTTP::proxy rtdom",
                                "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')",
                            ),
                            _av(
                                "exists",
                                "HTTP::proxy exists",
                                "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')",
                            ),
                            _av(
                                "iptuple",
                                "HTTP::proxy iptuple",
                                "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')",
                            ),
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
        )
