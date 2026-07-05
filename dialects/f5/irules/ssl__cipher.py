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

# Enriched from F5 iRules reference documentation.
"""SSL::cipher -- Returns SSL cipher information."""

# Introduced: BIG-IP v9+ (core SSL iRules command) (approximate, from F5 documentation)

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
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__cipher.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(
        target=SideEffectTarget.SSL_STATE,
        reads=True,
        connection_side=ConnectionSide.BOTH,
        scope=StorageScope.EVENT,
    ),
)


@register
class SslCipherCommand(CommandDef):
    name = "SSL::cipher"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::cipher",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns SSL cipher information.",
                synopsis=(
                    "SSL::cipher bits",
                    "SSL::cipher name",
                    "SSL::cipher version",
                    "SSL::cipher clientlist",
                ),
                snippet="Returns an SSL cipher name, its version, and the number of secret bits used.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    # Check encryption strength\n"
                    "    if { [SSL::cipher bits] >= 128 } {\n"
                    "        pool web_servers\n"
                    "    } else {\n"
                    "        # Client is using a weak cipher\n"
                    "        # Use one of the destination commands\n"
                    "\n"
                    "        # Either specify a pool\n"
                    "        pool sorry_servers\n"
                    "\n"
                    "        # or to a specific node\n"
                    "        node 10.10.10.10\n"
                    "\n"
                    "        # or send a 302 response to redirect to a specific URL\n"
                    "        # Set cache control headers to prevent proxies from caching the response."
                ),
                return_value='SSL::cipher name Returns the current SSL cipher name using the format of the L<OpenSSL SSL_CIPHER_get_name() function|https://www.openssl.org/docs/ssl/SSL_CIPHER_get_name.html> (e.g. "EDH-RSA-DES-CBC3-SHA" or "RC4-MD5").',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::cipher <subcommand>",
                    arg_values={
                        0: (
                            _av("bits", "Get number of secret bits.", "SSL::cipher bits"),
                            _av("name", "Get cipher name.", "SSL::cipher name"),
                            _av("version", "Get cipher version.", "SSL::cipher version"),
                            _av("clientlist", "Get client cipher list.", "SSL::cipher clientlist"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
            subcommands={
                sub: SubCommand(
                    name=sub,
                    arity=Arity(0, 0),
                    detail=detail,
                    synopsis=f"SSL::cipher {sub}",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                )
                for sub, detail in (
                    ("bits", "Get number of secret bits used."),
                    ("name", "Get cipher name."),
                    ("version", "Get cipher version."),
                    ("clientlist", "Get client cipher list."),
                )
            },
        )
