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
            "ltm_dns_hpke_profile",
            module="ltm",
            object_types=("dns hpke profile",),
        ),
        header_types=(("ltm", "dns hpke profile"),),
        properties=(
            BigipPropertySpec(name="aead", value_type="string", default="AES-128-GCM"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="expiration-period",
                value_type="integer",
                default="0 (zero), which indicates unset, and thus the hpke key does not expire",
            ),
            BigipPropertySpec(name="kdf", value_type="string", default="HKDF-SHA256"),
            BigipPropertySpec(name="kem", value_type="string", default="X25519"),
            BigipPropertySpec(
                name="rollover-period",
                value_type="integer",
                default="0 (zero), which indicates unset, and thus the hpke key does not roll over",
            ),
        ),
    )
