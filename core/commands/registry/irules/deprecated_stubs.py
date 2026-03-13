"""Stub definitions for deprecated iRules commands without full defs.

These commands are deprecated and have recommended replacements.  They only
need enough metadata to mark them as deprecated so the registry's
``command_status()`` returns ``DEPRECATED`` and ``deprecated_replacement``
carries the recommended replacement text.
"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet
from ..namespace_models import EventRequires
from ._base import _IRULES_ONLY, register
from .classify__application import ClassifyApplicationCommand
from .ip__addr import IpAddrCommand as _IpAddrReplacement
from .profile__http import ProfileHttpCommand
from .tcp__collect import TcpCollectCommand
from .tcp__local_port import TcpLocalPortCommand
from .tcp__remote_port import TcpRemotePortCommand


@register
class AccumulateCommand(CommandDef):
    name = "accumulate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="accumulate",
            deprecated_replacement=TcpCollectCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: use TCP::collect instead"),
            event_requires=EventRequires(),
        )


@register
class HttpClassCommand(CommandDef):
    name = "HTTP::class"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::class",
            deprecated_replacement=ClassifyApplicationCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: use CLASSIFY::application instead"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class DeprecatedIpAddrCommand(CommandDef):
    name = "ip_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ip_addr",
            deprecated_replacement=_IpAddrReplacement,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: use IP::addr instead"),
            event_requires=EventRequires(),
        )


@register
class LocalPortCommand(CommandDef):
    name = "local_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="local_port",
            deprecated_replacement=TcpLocalPortCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: use TCP::local_port instead"),
            event_requires=EventRequires(),
        )


@register
class RemotePortCommand(CommandDef):
    name = "remote_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="remote_port",
            deprecated_replacement=TcpRemotePortCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: use TCP::remote_port instead"),
            event_requires=EventRequires(),
        )


@register
class PluginDisableCommand(CommandDef):
    name = "PLUGIN::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PLUGIN::disable",
            deprecated_replacement="(removed)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: removed"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class PluginEnableCommand(CommandDef):
    name = "PLUGIN::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PLUGIN::enable",
            deprecated_replacement="(removed)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: removed"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class ProfileHttpclassCommand(CommandDef):
    name = "PROFILE::httpclass"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::httpclass",
            deprecated_replacement=ProfileHttpCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: use PROFILE::http instead"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )


@register
class XmlAddressCommand(CommandDef):
    name = "XML::address"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::address",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlCollectCommand(CommandDef):
    name = "XML::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::collect",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlElementCommand(CommandDef):
    name = "XML::element"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::element",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlEventCommand(CommandDef):
    name = "XML::event"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::event",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlEventidCommand(CommandDef):
    name = "XML::eventid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::eventid",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlParseCommand(CommandDef):
    name = "XML::parse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::parse",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlReleaseCommand(CommandDef):
    name = "XML::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::release",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlSoapCommand(CommandDef):
    name = "XML::soap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::soap",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )


@register
class XmlSubscribeCommand(CommandDef):
    name = "XML::subscribe"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XML::subscribe",
            deprecated_replacement="(removed — XML profile deprecated)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(summary="Deprecated: XML profile deprecated"),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
