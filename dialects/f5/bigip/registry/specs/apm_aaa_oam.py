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
            "apm_aaa_oam",
            module="apm",
            object_types=("aaa oam",),
        ),
        header_types=(("apm", "aaa oam"),),
        properties=(
            BigipPropertySpec(
                name="access-server-hostname",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="access-server-name",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="access-server-port",
                value_type="integer",
                allow_none=True,
                default="6021",
            ),
            BigipPropertySpec(
                name="access-server-retries",
                value_type="integer",
                default="0 (zero)",
            ),
            BigipPropertySpec(
                name="accessgate-encrypted-password",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="accessgates",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                enum_values=("config-accessgate", "noop"),
            ),
            BigipPropertySpec(name="admin-id", value_type="string", required=True, allow_none=True),
            BigipPropertySpec(
                name="admin-password",
                value_type="string",
                required=True,
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="enable",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="global-access-protocol-passphrase",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="transport-security-mode",
                value_type="enum",
                enum_values=("cert", "open", "simple"),
            ),
        ),
    )
