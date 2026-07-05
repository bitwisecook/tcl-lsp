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
            "ltm_auth_ssl_crldp",
            module="ltm",
            object_types=("auth ssl-crldp",),
        ),
        header_types=(("ltm", "auth ssl-crldp"),),
        properties=(
            BigipPropertySpec(
                name="cache-timeout",
                value_type="integer",
                default="86400 (24 hours)",
            ),
            BigipPropertySpec(name="connection-timeout", value_type="integer", default="15"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="servers",
                value_type="enum",
                required=True,
                allow_none=True,
                enum_values=("default", "none"),
                default="none",
            ),
            BigipPropertySpec(
                name="update-interval",
                value_type="integer",
                default="0 (zero), which indicates an internal default value is active",
            ),
            BigipPropertySpec(
                name="use-issuer",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
