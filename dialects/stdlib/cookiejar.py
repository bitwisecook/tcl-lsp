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

"""cookiejar -- Tcl stdlib HTTP cookie jar (package require cookiejar)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_PKG = "cookiejar"
_SOURCE = "Tcl stdlib cookiejar package"


@register
class HttpCookiejar(CommandDef):
    name = "http::cookiejar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http::cookiejar",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Create or configure an HTTP cookie jar (TclOO class).",
                synopsis=(
                    "http::cookiejar create name ?filename?",
                    "http::cookiejar new ?filename?",
                ),
                snippet=(
                    "A TclOO class implementing RFC 6265 cookie management.  "
                    "Instances track cookies received in HTTP responses and "
                    "automatically attach them to subsequent requests."
                ),
                source=_SOURCE,
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
class TclIdnaEncode(CommandDef):
    name = "tcl::idna::encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::idna::encode",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Encode a hostname to IDNA (Internationalised Domain Names) format.",
                synopsis=("tcl::idna::encode hostname",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class TclIdnaDecode(CommandDef):
    name = "tcl::idna::decode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::idna::decode",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Decode a hostname from IDNA format to Unicode.",
                synopsis=("tcl::idna::decode hostname",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )
