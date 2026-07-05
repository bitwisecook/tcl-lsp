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
"""CRYPTO::keygen -- Generates keys that can be used to encrypt and sign data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__keygen.html"


_av = make_av(_SOURCE)


@register
class CryptoKeygenCommand(CommandDef):
    name = "CRYPTO::keygen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::keygen",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Generates keys that can be used to encrypt and sign data.",
                synopsis=("CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",),
                snippet=(
                    "This iRules command is used to generate keys that can be used to\n"
                    "encrypt and sign data.\n"
                    "\n"
                    "CRYPTO::keygen -alg <> -len <> [-passphrase <> -salt[hex] <> -rounds <>]\n"
                    "\n"
                    "     * Used to generate keys that can be used to encrypt and sign data.\n"
                    "          + -alg (Two options: random or pbkdf2-md5)\n"
                    "          + -len (Must be a multiple of 8, e.g."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
                    options=(
                        OptionSpec(
                            name="-alg",
                            detail="Key generation algorithm.",
                            takes_value=True,
                            value_hint="ALG",
                        ),
                        OptionSpec(
                            name="-len",
                            detail="Key length (must be multiple of 8).",
                            takes_value=True,
                            value_hint="LENGTH",
                        ),
                        OptionSpec(
                            name="-exp",
                            detail="Exponent (for RSA).",
                            takes_value=True,
                            value_hint="EXPONENT",
                        ),
                        OptionSpec(
                            name="-passphrase",
                            detail="Passphrase for key derivation.",
                            takes_value=True,
                            value_hint="PASSPHRASE",
                        ),
                        OptionSpec(
                            name="-salt", detail="Binary salt.", takes_value=True, value_hint="SALT"
                        ),
                        OptionSpec(
                            name="-salthex",
                            detail="Hex-encoded salt.",
                            takes_value=True,
                            value_hint="SALT_HEX",
                        ),
                        OptionSpec(
                            name="-rounds",
                            detail="Rounds for PBKDF2.",
                            takes_value=True,
                            value_hint="ROUNDS",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "random",
                                "CRYPTO::keygen random",
                                "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
                            ),
                            _av(
                                "pbkdf2-md5",
                                "CRYPTO::keygen pbkdf2-md5",
                                "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
                            ),
                            _av(
                                "rsa",
                                "CRYPTO::keygen rsa",
                                "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
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
