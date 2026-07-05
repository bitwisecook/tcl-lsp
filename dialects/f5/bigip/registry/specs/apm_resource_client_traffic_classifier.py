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
            "apm_resource_client_traffic_classifier",
            module="apm",
            object_types=("resource client-traffic-classifier",),
        ),
        header_types=(("apm", "resource client-traffic-classifier"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="entries",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("entries",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="client-rate-class",
                        value_type="string",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="dst-ip",
                        value_type="string",
                        in_sections=("entries",),
                        allow_none=True,
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="dst-mask",
                        value_type="integer",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="dst-port",
                        value_type="integer",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="protocol",
                        value_type="integer",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="src-ip",
                        value_type="string",
                        in_sections=("entries",),
                        allow_none=True,
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="src-mask",
                        value_type="integer",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="src-port",
                        value_type="integer",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("entries",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="client-rate-class",
                value_type="string",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="dst-ip",
                value_type="string",
                in_sections=("entries",),
                allow_none=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="dst-mask",
                value_type="integer",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="dst-port",
                value_type="integer",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="integer",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="src-ip",
                value_type="string",
                in_sections=("entries",),
                allow_none=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="src-mask",
                value_type="integer",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="src-port",
                value_type="integer",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
        ),
    )
