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
            "apm_aaa_f5_service_connector",
            module="apm",
            object_types=("aaa f5-service-connector",),
        ),
        header_types=(("apm", "aaa f5-service-connector"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="customer-id", value_type="string", required=True),
            BigipPropertySpec(name="customer-key", value_type="string", required=True),
            BigipPropertySpec(
                name="dns-resolver",
                value_type="reference",
                required=True,
                references=("net_dns_resolver",),
            ),
            BigipPropertySpec(name="serverssl-profile", value_type="reference", required=True),
            BigipPropertySpec(name="service-url", value_type="string", required=True),
        ),
    )
