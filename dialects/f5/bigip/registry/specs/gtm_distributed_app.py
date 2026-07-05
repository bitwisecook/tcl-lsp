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
            "gtm_distributed_app",
            module="gtm",
            object_types=("distributed-app",),
        ),
        header_types=(("gtm", "distributed-app"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="dependency-level",
                value_type="enum",
                allow_none=True,
                enum_values=("datacenter", "link", "none", "server", "wideip"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="disabled-contexts",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="persist-cidr-ipv4", value_type="integer"),
            BigipPropertySpec(name="persist-cidr-ipv6", value_type="integer"),
            BigipPropertySpec(
                name="persistence",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="ttl-persistence", value_type="integer", default="3600"),
            BigipPropertySpec(
                name="wideips",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none"),
                default="none",
            ),
        ),
    )
