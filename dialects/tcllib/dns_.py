# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""dns -- DNS client library (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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


@register
class DnsConfigureCommand(CommandDef):
    name = "dns::configure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Get or set DNS client configuration options.",
                synopsis=("dns::configure ?options?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::configure ?options?"),),
            validation=ValidationSpec(arity=Arity(0)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class DnsCnameCommand(CommandDef):
    name = "dns::cname"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the canonical name from a DNS query result.",
                synopsis=("dns::cname token",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::cname token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class DnsStatusCommand(CommandDef):
    name = "dns::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the status of a DNS query.",
                synopsis=("dns::status token",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::status token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class DnsResetCommand(CommandDef):
    name = "dns::reset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Reset a DNS query.",
                synopsis=("dns::reset token ?reason? ?message?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(kind=FormKind.DEFAULT, synopsis="dns::reset token ?reason? ?message?"),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class DnsWaitCommand(CommandDef):
    name = "dns::wait"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Wait for a DNS query to complete.",
                synopsis=("dns::wait token",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::wait token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class DnsErrorcodeCommand(CommandDef):
    name = "dns::errorcode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the error code from a DNS query.",
                synopsis=("dns::errorcode token",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::errorcode token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class DnsErrorCommand(CommandDef):
    name = "dns::error"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the error message from a DNS query.",
                synopsis=("dns::error token",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::error token"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class DnsResultCommand(CommandDef):
    name = "dns::result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the result of a DNS query.",
                synopsis=("dns::result token ?options?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::result token ?options?"),),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class DnsDumpCommand(CommandDef):
    name = "dns::dump"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Dump the contents of a DNS query token for debugging.",
                synopsis=("dns::dump token ?channel?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="dns::dump token ?channel?"),),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )
