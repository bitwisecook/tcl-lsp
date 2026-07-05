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
"""CRYPTO::hash -- Generates a hash on a piece of data."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__hash.html"


_av = make_av(_SOURCE)


@register
class CryptoHashCommand(CommandDef):
    name = "CRYPTO::hash"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::hash",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Generates a hash on a piece of data.",
                synopsis=(
                    "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                ),
                snippet=(
                    "This iRules command generates a hash on a piece of data\n"
                    "\n"
                    "CRYPTO::hash [-alg <>] [-ctx <> [-final]] [<data>]\n"
                    "\n"
                    "     * Generates a hash on a piece of data\n"
                    "\n"
                    "Algorithm List\n"
                    "\n"
                    "     * md5\n"
                    "     * ripemd160\n"
                    "     * sha1\n"
                    "     * sha224\n"
                    "     * sha256\n"
                    "     * sha384\n"
                    "     * sha512"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "if {[class match [b64encode [CRYPTO::hash -alg sha384 [HTTP::host][HTTP::path]]] equals HASH ]} {\n"
                    '    log local0. " this FQDN + PATH is mathing - [HTTP::host][HTTP::path]"\n'
                    "}\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                    options=(
                        OptionSpec(
                            name="-alg",
                            detail="Hash algorithm.",
                            takes_value=True,
                            value_hint="ALG",
                        ),
                        OptionSpec(
                            name="-ctx",
                            detail="Context variable for multi-step operations.",
                            takes_value=True,
                            value_hint="CTX_VAR",
                        ),
                        OptionSpec(
                            name="-final",
                            detail="Finalize context-based operation.",
                            takes_value=False,
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "md5",
                                "CRYPTO::hash md5",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "ripemd160",
                                "CRYPTO::hash ripemd160",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha1",
                                "CRYPTO::hash sha1",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha224",
                                "CRYPTO::hash sha224",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha256",
                                "CRYPTO::hash sha256",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha384",
                                "CRYPTO::hash sha384",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
