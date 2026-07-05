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
            "net_ipsec_manual_security_association",
            module="net",
            object_types=("ipsec manual-security-association",),
        ),
        header_types=(("net", "ipsec manual-security-association"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="auth-algorithm", value_type="unknown"),
            BigipPropertySpec(name="auth-key", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-address",
                value_type="string",
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="encrypt-algorithm",
                value_type="enum",
                enum_values=("3des", "aes128", "aes192", "aes256", "null"),
            ),
            BigipPropertySpec(name="encrypt-key", value_type="unknown"),
            BigipPropertySpec(
                name="ipsec-policy",
                value_type="reference",
                references=("net_ipsec_ipsec_policy",),
            ),
            BigipPropertySpec(name="protocol", value_type="unknown"),
            BigipPropertySpec(name="source-address", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="spi", value_type="unknown"),
        ),
    )
