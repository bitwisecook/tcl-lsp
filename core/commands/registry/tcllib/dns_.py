"""dns -- DNS client library (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib dns package"
_PACKAGE = "dns"


@register
class DnsResolveCommand(CommandDef):
    name = "dns::resolve"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Perform a DNS lookup.",
                synopsis=(
                    "dns::resolve name ?-type type? ?-class class? ?-server server? ?-timeout ms?",
                ),
                source=_SOURCE,
                examples="set tok [dns::resolve www.example.com]",
                return_value="A DNS query token.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="dns::resolve name ?options?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class DnsNameCommand(CommandDef):
    name = "dns::name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the domain name from a DNS query result.",
                synopsis=("dns::name token",),
                source=_SOURCE,
                return_value="A list of domain names.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::name token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class DnsAddressCommand(CommandDef):
    name = "dns::address"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the IP addresses from a DNS query result.",
                synopsis=("dns::address token",),
                source=_SOURCE,
                return_value="A list of IP addresses.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::address token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class DnsCleanupCommand(CommandDef):
    name = "dns::cleanup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Clean up resources associated with a DNS query.",
                synopsis=("dns::cleanup token",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::cleanup token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
