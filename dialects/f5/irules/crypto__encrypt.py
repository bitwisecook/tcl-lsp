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
"""CRYPTO::encrypt -- Encrypts data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__encrypt.html"


_av = make_av(_SOURCE)


@register
class CryptoEncryptCommand(CommandDef):
    name = "CRYPTO::encrypt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::encrypt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This iRules command encrypts data.",
                synopsis=("CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",),
                snippet=(
                    "This iRules command encrypts data. A ciphertext encrypted with this\n"
                    "command should be decryptable by third party software.\n"
                    "\n"
                    "CRYPTO::encrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n"
                    '                [-padding <"pkcs" | "oaep" | "none">]\n'
                    "\n"
                    "     * encrypts data based on several parameters\n"
                    "          + alg - algorithm. ASCII string from a given list (see below)\n"
                    "            The spelling is lowercase and the iRule will fail for anything\n"
                    "            not in the list. In ctx mode, alg must be given in the first\n"
                    "            CRYPTO:: command and cannot be modified."
                ),
                source=_SOURCE,
                examples=(
                    "Encrypt an MSISDN header\n"
                    "# Encrypt the MSISDN header for each request.\n"
                    "# The encryption is deliberately designed to be insecure;\n"
                    "# that is, the same MSISDN will always be encrypted to\n"
                    "# the same ciphertext. And since the IV will always be\n"
                    "# the same for each encryption, there's no need to send\n"
                    "# it out with the ciphertext.\n"
                    "#\n"
                    "when SIP_REQUEST {\n"
                    '    set key "abed1ddc04fbb05856bca4a0ca60f21e"\n'
                    '    set iv "d78d86d9084eb9239694c9a733904037"'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
                    options=(
                        OptionSpec(
                            name="-alg",
                            detail="Encryption algorithm.",
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
                                "CRYPTO::encrypt pkcs",
                                "CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "oaep",
                                "CRYPTO::encrypt oaep",
                                "CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "none",
                                "CRYPTO::encrypt none",
                                "CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
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
