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
            "sys_crypto_cert_order_manager",
            module="sys",
            object_types=("crypto cert-order-manager",),
        ),
        header_types=(("sys", "crypto cert-order-manager"),),
        properties=(
            BigipPropertySpec(
                name="additional-headers",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="authority",
                value_type="enum",
                enum_values=("comodo", "digicert", "godaddy", "symantec"),
            ),
            BigipPropertySpec(
                name="auto-renew",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="base-url",
                value_type="enum",
                allow_none=True,
                enum_values=("URL", "none"),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="ca-cert", value_type="unknown"),
            BigipPropertySpec(
                name="client-cert",
                value_type="enum",
                required=True,
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="client-key",
                value_type="enum",
                required=True,
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="client-key-passphrase",
                value_type="string",
                required=True,
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="edit-order-info", value_type="unknown"),
            BigipPropertySpec(name="internal-proxy", value_type="unknown"),
            BigipPropertySpec(
                name="login-name",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="login-password",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(name="order-info", value_type="string"),
            BigipPropertySpec(
                name="validity-days",
                value_type="enum",
                allow_none=True,
                enum_values=("days", "none"),
                default="365 days",
            ),
        ),
    )
