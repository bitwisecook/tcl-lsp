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
            "ltm_auth_ssl_cc_ldap",
            module="ltm",
            object_types=("auth ssl-cc-ldap",),
        ),
        header_types=(("ltm", "auth ssl-cc-ldap"),),
        properties=(
            BigipPropertySpec(
                name="admin-dn",
                value_type="reference",
                required=True,
                allow_none=True,
                default="none",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(name="admin-password", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="cache-size",
                value_type="integer",
                default="20000 bytes (20KB)",
            ),
            BigipPropertySpec(name="cache-timeout", value_type="integer", default="300 seconds"),
            BigipPropertySpec(name="certmap-base", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="certmap-key",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="certmap-user-serial",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="group-base", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="group-key",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="group-member-key",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="role-key",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="search-type",
                value_type="enum",
                enum_values=("cert", "certmap", "user"),
            ),
            BigipPropertySpec(
                name="secure",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="servers", value_type="unknown"),
            BigipPropertySpec(name="user-base", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="user-class",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="user-key", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="valid-groups",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="valid-roles",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
        ),
    )
