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
"""CRYPTO::decrypt -- Decrypts data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__decrypt.html"


_av = make_av(_SOURCE)


@register
class CryptoDecryptCommand(CommandDef):
    name = "CRYPTO::decrypt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::decrypt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This iRules command decrypts data.",
                synopsis=("CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",),
                snippet=(
                    "This iRules command decrypts data.\n"
                    "\n"
                    "CRYPTO::decrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n"
                    '                [-padding <"pkcs" | "oaep" | "none">]\n'
                    "\n"
                    "     * decrypts data based on several parameters\n"
                    "          + alg - algorithm. ASCII string from a given list (see below)\n"
                    "            The spelling is lowercase and the iRule will fail for anything\n"
                    "            not in the list. In ctx mode, alg must be given in the first\n"
                    "            CRYPTO::command and cannot be modified."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
                    options=(
                        OptionSpec(
                            name="-alg",
                            detail="Decryption algorithm.",
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
                        OptionSpec(
                            name="-key", detail="Binary key.", takes_value=True, value_hint="KEY"
                        ),
                        OptionSpec(
                            name="-keyhex",
                            detail="Hex-encoded key.",
                            takes_value=True,
                            value_hint="KEY_HEX",
                        ),
                        OptionSpec(
                            name="-iv",
                            detail="Initialization vector (binary).",
                            takes_value=True,
                            value_hint="IV",
                        ),
                        OptionSpec(
                            name="-ivhex",
                            detail="Initialization vector (hex).",
                            takes_value=True,
                            value_hint="IV_HEX",
                        ),
                        OptionSpec(
                            name="-padding",
                            detail="Padding mode (pkcs, oaep, none).",
                            takes_value=True,
                            value_hint="PADDING",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "pkcs",
                                "CRYPTO::decrypt pkcs",
                                "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "oaep",
                                "CRYPTO::decrypt oaep",
                                "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "none",
                                "CRYPTO::decrypt none",
                                "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
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
