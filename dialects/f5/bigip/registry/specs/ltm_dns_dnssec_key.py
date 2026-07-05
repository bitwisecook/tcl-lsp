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
            "ltm_dns_dnssec_key",
            module="ltm",
            object_types=("dns dnssec key",),
        ),
        header_types=(("ltm", "dns dnssec key"),),
        properties=(
            BigipPropertySpec(
                name="algorithm",
                value_type="enum",
                enum_values=(
                    "ecdsap256sha256",
                    "ecdsap384sha384",
                    "rsasha1",
                    "rsasha256",
                    "rsasha512",
                ),
                default="RSASHA1",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="bitwidth",
                value_type="enum",
                enum_values=("1024", "2048", "4096", "512"),
                default="1024",
            ),
            BigipPropertySpec(name="certificate-file", value_type="string", required=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="expiration-period",
                value_type="integer",
                default="0 (zero), which indicates unset, and thus the key does not expire",
            ),
            BigipPropertySpec(
                name="generation",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="expiration",
                        value_type="unknown",
                        in_sections=("generation",),
                    ),
                    BigipPropertySpec(
                        name="rollover",
                        value_type="unknown",
                        in_sections=("generation",),
                    ),
                ),
            ),
            BigipPropertySpec(name="expiration", value_type="unknown", in_sections=("generation",)),
            BigipPropertySpec(name="rollover", value_type="unknown", in_sections=("generation",)),
            BigipPropertySpec(name="key-file", value_type="string", required=True),
            BigipPropertySpec(
                name="key-type",
                value_type="enum",
                enum_values=("ksk", "zsk"),
                default="zsk",
            ),
            BigipPropertySpec(
                name="rollover-period",
                value_type="integer",
                default="0 (zero), which indicates unset, and thus the key does not roll over",
            ),
            BigipPropertySpec(
                name="signature-pub-period",
                value_type="integer",
                default="403200 seconds",
            ),
            BigipPropertySpec(
                name="signature-valid-period",
                value_type="integer",
                default="604800 seconds",
            ),
            BigipPropertySpec(name="ttl", value_type="integer", default="86400"),
            BigipPropertySpec(
                name="use-fips",
                value_type="enum",
                allow_none=True,
                enum_values=("external", "internal", "none"),
                default="none",
            ),
        ),
    )
