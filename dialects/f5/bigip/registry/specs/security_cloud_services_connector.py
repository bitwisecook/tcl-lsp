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
            "security_cloud_services_connector",
            module="security",
            object_types=("cloud-services connector",),
        ),
        header_types=(("security", "cloud-services connector"),),
        properties=(
            BigipPropertySpec(name="activation-note", value_type="string"),
            BigipPropertySpec(name="activation-time", value_type="unknown"),
            BigipPropertySpec(name="clientside-key", value_type="string"),
            BigipPropertySpec(name="clientside-token", value_type="string"),
            BigipPropertySpec(name="clientside-url", value_type="string"),
            BigipPropertySpec(name="control-key", value_type="string"),
            BigipPropertySpec(name="control-token", value_type="string"),
            BigipPropertySpec(name="control-url", value_type="string"),
            BigipPropertySpec(name="deployment-id", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="expiration-time", value_type="unknown"),
            BigipPropertySpec(name="params", value_type="string"),
            BigipPropertySpec(
                name="services",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="centralized-device-id",
                        value_type="unknown",
                        in_sections=("services",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="centralized-device-id",
                value_type="unknown",
                in_sections=("services",),
                shape_kind="object",
            ),
        ),
    )
