"""ip -- IP address manipulation (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib ip package"
_PACKAGE = "ip"


@register
class IpNormaliseCommand(CommandDef):
    name = "ip::normalize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Normalise an IP address to its canonical form.",
                synopsis=("ip::normalize address",),
                source=_SOURCE,
                examples="set norm [ip::normalize 192.168.001.001]",
                return_value="The normalised IP address string.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::normalize address"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class IpPrefixCommand(CommandDef):
    name = "ip::prefix"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the network prefix of an address/mask.",
                synopsis=("ip::prefix address/mask",),
                source=_SOURCE,
                examples="set net [ip::prefix 192.168.1.5/24]",
                return_value="The network prefix address.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::prefix address/mask"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class IpContractCommand(CommandDef):
    name = "ip::contract"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Contract an IPv6 address to its shortest form.",
                synopsis=("ip::contract address",),
                source=_SOURCE,
                return_value="The contracted IPv6 address string.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::contract address"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class IpEqualCommand(CommandDef):
    name = "ip::equal"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Test if two IP addresses or subnets are equal.",
                synopsis=("ip::equal address1 address2",),
                source=_SOURCE,
                return_value="1 if equal, 0 otherwise.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::equal address1 address2"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
        )


@register
class IpVersionCommand(CommandDef):
    name = "ip::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the IP version of an address (4 or 6).",
                synopsis=("ip::version address",),
                source=_SOURCE,
                return_value="4 or 6, or -1 if not a valid IP address.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::version address"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )
