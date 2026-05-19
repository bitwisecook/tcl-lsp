"""ip -- IP address manipulation (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

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
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::normalize address ?Ip4inIp6?"),),
            validation=ValidationSpec(arity=Arity(1, 2)),
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


@register
class IpIsCommand(CommandDef):
    name = "ip::is"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Test whether a value is a valid IP address of the given class.",
                synopsis=("ip::is class address",),
                source=_SOURCE,
                return_value="1 if the address matches the class, 0 otherwise.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::is class address"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
            return_type=TclType.BOOLEAN,
        )


@register
class IpTypeCommand(CommandDef):
    name = "ip::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the type of an IP address.",
                synopsis=("ip::type address",),
                source=_SOURCE,
                return_value="The address type string.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::type address"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class IpMaskCommand(CommandDef):
    name = "ip::mask"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the network mask for an address.",
                synopsis=("ip::mask address",),
                source=_SOURCE,
                return_value="The network mask string.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::mask address"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class IpCollapseCommand(CommandDef):
    name = "ip::collapse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Collapse a list of IP addresses or subnets into the minimal set.",
                synopsis=("ip::collapse addressList",),
                source=_SOURCE,
                return_value="A list of collapsed address ranges.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::collapse addressList"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
            return_type=TclType.LIST,
        )


@register
class IpSubtractCommand(CommandDef):
    name = "ip::subtract"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Subtract address ranges from a list of hosts.",
                synopsis=("ip::subtract addressList",),
                source=_SOURCE,
                return_value="A list of remaining address ranges.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="ip::subtract addressList"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
            return_type=TclType.LIST,
        )
