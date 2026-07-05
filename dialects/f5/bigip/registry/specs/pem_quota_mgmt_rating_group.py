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
            "pem_quota_mgmt_rating_group",
            module="pem",
            object_types=("quota-mgmt rating-group",),
        ),
        header_types=(("pem", "quota-mgmt rating-group"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="default-breach-action",
                value_type="enum",
                enum_values=("allow", "redirect", "terminate"),
            ),
            BigipPropertySpec(name="default-forwarding-endpoint", value_type="reference"),
            BigipPropertySpec(
                name="default-quota",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="interval",
                        value_type="integer",
                        in_sections=("default-quota",),
                    ),
                    BigipPropertySpec(
                        name="time",
                        value_type="unknown",
                        in_sections=("default-quota",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="volume",
                        value_type="unknown",
                        in_sections=("default-quota",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(name="default-quota-holding-time", value_type="integer"),
            BigipPropertySpec(
                name="interval",
                value_type="integer",
                in_sections=("default-quota",),
            ),
            BigipPropertySpec(
                name="time",
                value_type="unknown",
                in_sections=("default-quota",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="consumption-time",
                        value_type="unknown",
                        in_sections=("default-quota", "time"),
                    ),
                    BigipPropertySpec(
                        name="usage-time",
                        value_type="unknown",
                        in_sections=("default-quota", "time"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="consumption-time",
                value_type="unknown",
                in_sections=("default-quota", "time"),
            ),
            BigipPropertySpec(
                name="usage-time",
                value_type="unknown",
                in_sections=("default-quota", "time"),
            ),
            BigipPropertySpec(
                name="volume",
                value_type="unknown",
                in_sections=("default-quota",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="input-octets",
                        value_type="unknown",
                        in_sections=("default-quota", "volume"),
                    ),
                    BigipPropertySpec(
                        name="output-octets",
                        value_type="unknown",
                        in_sections=("default-quota", "volume"),
                    ),
                    BigipPropertySpec(
                        name="total-octets",
                        value_type="unknown",
                        in_sections=("default-quota", "volume"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="input-octets",
                value_type="unknown",
                in_sections=("default-quota", "volume"),
            ),
            BigipPropertySpec(
                name="output-octets",
                value_type="unknown",
                in_sections=("default-quota", "volume"),
            ),
            BigipPropertySpec(
                name="total-octets",
                value_type="unknown",
                in_sections=("default-quota", "volume"),
            ),
            BigipPropertySpec(name="default-threshold", value_type="integer"),
            BigipPropertySpec(name="default-validity-time", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="initial-quota-request",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="interval",
                        value_type="integer",
                        in_sections=("initial-quota-request",),
                    ),
                    BigipPropertySpec(
                        name="volume",
                        value_type="unknown",
                        in_sections=("initial-quota-request",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="interval",
                value_type="integer",
                in_sections=("initial-quota-request",),
            ),
            BigipPropertySpec(
                name="volume",
                value_type="unknown",
                in_sections=("initial-quota-request",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="input-octets",
                        value_type="unknown",
                        in_sections=("initial-quota-request", "volume"),
                    ),
                    BigipPropertySpec(
                        name="output-octets",
                        value_type="unknown",
                        in_sections=("initial-quota-request", "volume"),
                    ),
                    BigipPropertySpec(
                        name="total-octets",
                        value_type="unknown",
                        in_sections=("initial-quota-request", "volume"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="input-octets",
                value_type="unknown",
                in_sections=("initial-quota-request", "volume"),
            ),
            BigipPropertySpec(
                name="output-octets",
                value_type="unknown",
                in_sections=("initial-quota-request", "volume"),
            ),
            BigipPropertySpec(
                name="total-octets",
                value_type="unknown",
                in_sections=("initial-quota-request", "volume"),
            ),
            BigipPropertySpec(name="rating-group-id", value_type="integer"),
            BigipPropertySpec(
                name="request-on-install",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
        ),
    )
