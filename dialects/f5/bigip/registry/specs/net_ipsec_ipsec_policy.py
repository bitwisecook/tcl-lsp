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

from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "net_ipsec_ipsec_policy",
            module="net",
            object_types=("ipsec ipsec-policy",),
        ),
        header_types=(("net", "ipsec ipsec-policy"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ike-phase2-auth-algorithm",
                value_type="enum",
                enum_values=(
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "sha1",
                    "sha256",
                    "sha384",
                    "sha512",
                ),
                default="aes-gcm128",
            ),
            BigipPropertySpec(
                name="ike-phase2-encrypt-algorithm",
                value_type="enum",
                enum_values=(
                    "3des",
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "aes128",
                    "aes192",
                    "aes256",
                    "null",
                ),
                default="aes-gcm128",
            ),
            BigipPropertySpec(name="ike-phase2-lifetime", value_type="integer"),
            BigipPropertySpec(name="ike-phase2-lifetime-kilobytes", value_type="integer"),
            BigipPropertySpec(
                name="ike-phase2-perfect-forward-secrecy",
                value_type="enum",
                enum_values=(
                    "modp1024",
                    "modp1536",
                    "modp2048",
                    "modp3072",
                    "modp4096",
                    "modp6144",
                    "modp768",
                    "modp8192",
                ),
            ),
            BigipPropertySpec(
                name="ipcomp",
                value_type="enum",
                allow_none=True,
                enum_values=("deflate", "none", "null"),
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("interface", "tunnel")),
            BigipPropertySpec(name="protocol", value_type="unknown"),
            BigipPropertySpec(
                name="tunnel-local-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="tunnel-remote-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
        ),
    )
