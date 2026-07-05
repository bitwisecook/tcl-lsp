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
            "sys_crypto_key",
            module="sys",
            object_types=("crypto key",),
        ),
        header_types=(("sys", "crypto key"),),
        properties=(
            BigipPropertySpec(name="admin-email-address", value_type="string"),
            BigipPropertySpec(
                name="cert-order-manager",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                usage_flags=frozenset(("optional",)),
                block=(
                    BigipPropertySpec(
                        name="check-status",
                        value_type="enum",
                        in_sections=("cert-order-manager",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="order-id",
                        value_type="enum",
                        in_sections=("cert-order-manager",),
                        required=True,
                        allow_none=True,
                        enum_values=("none",),
                    ),
                    BigipPropertySpec(
                        name="order-passphrase",
                        value_type="enum",
                        in_sections=("cert-order-manager",),
                        allow_none=True,
                        enum_values=("none",),
                    ),
                    BigipPropertySpec(
                        name="order-type",
                        value_type="enum",
                        in_sections=("cert-order-manager",),
                        enum_values=("cancel", "new", "renew", "revoke"),
                    ),
                    BigipPropertySpec(
                        name="revoke-reason",
                        value_type="enum",
                        in_sections=("cert-order-manager",),
                        enum_values=(
                            "AACompromise",
                            "CACompromise",
                            "affiliationChanged",
                            "certificateHold",
                            "cessationOfOperation",
                            "keyCompromise",
                            "privilegeWithdrawn",
                            "removeFromCRL",
                            "superseded",
                            "unspecified",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="check-status",
                value_type="enum",
                in_sections=("cert-order-manager",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="order-id",
                value_type="enum",
                in_sections=("cert-order-manager",),
                required=True,
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="order-passphrase",
                value_type="enum",
                in_sections=("cert-order-manager",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="order-type",
                value_type="enum",
                in_sections=("cert-order-manager",),
                enum_values=("cancel", "new", "renew", "revoke"),
            ),
            BigipPropertySpec(
                name="revoke-reason",
                value_type="enum",
                in_sections=("cert-order-manager",),
                enum_values=(
                    "AACompromise",
                    "CACompromise",
                    "affiliationChanged",
                    "certificateHold",
                    "cessationOfOperation",
                    "keyCompromise",
                    "privilegeWithdrawn",
                    "removeFromCRL",
                    "superseded",
                    "unspecified",
                ),
            ),
            BigipPropertySpec(name="challenge-password", value_type="string"),
            BigipPropertySpec(name="city", value_type="string"),
            BigipPropertySpec(name="common-name", value_type="string"),
            BigipPropertySpec(name="consumer", value_type="unknown"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(
                name="curve-name",
                value_type="enum",
                enum_values=("prime256v1", "secp384r1", "secp521r1"),
                default="prime256v1",
            ),
            BigipPropertySpec(name="email-address", value_type="string"),
            BigipPropertySpec(
                name="key-size",
                value_type="enum",
                enum_values=("1024", "2048", "4096", "512"),
            ),
            BigipPropertySpec(
                name="key-type",
                value_type="enum",
                enum_values=("dsa-private", "ec-private", "rsa-private"),
                default="rsa-private",
            ),
            BigipPropertySpec(name="lifetime", value_type="unknown"),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="organization", value_type="string"),
            BigipPropertySpec(name="ou", value_type="string"),
            BigipPropertySpec(
                name="passphrase",
                value_type="unknown",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="prompt-for-password", value_type="unknown"),
            BigipPropertySpec(
                name="security-type",
                value_type="enum",
                enum_values=("fips", "nethsm", "normal", "password"),
            ),
            BigipPropertySpec(name="state", value_type="string"),
            BigipPropertySpec(name="subject-alternative-name", value_type="string"),
        ),
    )
