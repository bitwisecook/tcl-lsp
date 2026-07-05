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
            "wom_local_endpoint",
            module="wom",
            object_types=("local-endpoint",),
        ),
        header_types=(("wom", "local-endpoint"),),
        properties=(
            BigipPropertySpec(
                name="addresses",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="allow-nat",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="endpoint",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="internal-forwarding",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="ip-encap-mtu", value_type="integer"),
            BigipPropertySpec(
                name="ip-encap-profile",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="ip-encap-type",
                value_type="enum",
                allow_none=True,
                enum_values=("gre", "ipip", "ipsec", "none"),
                default="none",
            ),
            BigipPropertySpec(
                name="no-route",
                value_type="enum",
                enum_values=("drop", "passthru"),
                default="passthru",
            ),
            BigipPropertySpec(
                name="server-ssl",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
                references=("ltm_profile_server_ssl",),
                default="none",
            ),
            BigipPropertySpec(
                name="snat",
                value_type="enum",
                allow_none=True,
                enum_values=("local", "none", "remote"),
                default="none",
            ),
            BigipPropertySpec(name="tunnel-port", value_type="integer", default="443"),
        ),
    )
