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
            "sys_management_ovsdb",
            module="sys",
            object_types=("management-ovsdb",),
        ),
        header_types=(("sys", "management-ovsdb"),),
        properties=(
            BigipPropertySpec(name="bfd-disabled", value_type="unknown"),
            BigipPropertySpec(name="bfd-enabled", value_type="unknown"),
            BigipPropertySpec(name="bfd-route-domain", value_type="unknown"),
            BigipPropertySpec(name="ca-cert-file", value_type="unknown"),
            BigipPropertySpec(name="cert-file", value_type="unknown"),
            BigipPropertySpec(name="cert-key-file", value_type="unknown"),
            BigipPropertySpec(
                name="controller-addresses",
                value_type="string",
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disabled", value_type="unknown"),
            BigipPropertySpec(name="enabled", value_type="unknown"),
            BigipPropertySpec(
                name="flooding-type",
                value_type="enum",
                enum_values=("multipoint", "replicator"),
            ),
            BigipPropertySpec(name="log-level", value_type="unknown"),
            BigipPropertySpec(
                name="logical-routing-type",
                value_type="enum",
                allow_none=True,
                enum_values=("backhaul", "none"),
            ),
            BigipPropertySpec(name="port", value_type="unknown"),
            BigipPropertySpec(
                name="tunnel-floating-addresses",
                value_type="string",
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="tunnel-local-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="tunnel-maintenance-mode",
                value_type="enum",
                enum_values=("active", "passive"),
            ),
        ),
    )
