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
            "security_debug_register",
            module="security",
            object_types=("debug register",),
        ),
        header_types=(("security", "debug register"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="unknown",
                        in_sections=("destination",),
                    ),
                    BigipPropertySpec(
                        name="port", value_type="unknown", in_sections=("destination",)
                    ),
                ),
            ),
            BigipPropertySpec(name="address", value_type="unknown", in_sections=("destination",)),
            BigipPropertySpec(name="port", value_type="unknown", in_sections=("destination",)),
            BigipPropertySpec(
                name="match-ip-version",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(name="protocol", value_type="unknown", default="any"),
            BigipPropertySpec(
                name="source",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="address", value_type="unknown", in_sections=("source",)
                    ),
                    BigipPropertySpec(name="port", value_type="unknown", in_sections=("source",)),
                    BigipPropertySpec(
                        name="vlan",
                        value_type="reference",
                        in_sections=("source",),
                        references=(
                            "net_fdb_vlan",
                            "net_vlan",
                            "net_vlan_allowed",
                            "net_vlan_group",
                            "sys_sflow_data_source_vlan",
                            "sys_sflow_global_settings_vlan",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(name="address", value_type="unknown", in_sections=("source",)),
            BigipPropertySpec(name="port", value_type="unknown", in_sections=("source",)),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                in_sections=("source",),
                references=(
                    "net_fdb_vlan",
                    "net_vlan",
                    "net_vlan_allowed",
                    "net_vlan_group",
                    "sys_sflow_data_source_vlan",
                    "sys_sflow_global_settings_vlan",
                ),
            ),
        ),
    )
