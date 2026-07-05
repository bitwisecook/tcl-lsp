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
            "ltm_profile_http2",
            module="ltm",
            object_types=("profile http2",),
        ),
        header_types=(("ltm", "profile http2"),),
        properties=(
            BigipPropertySpec(
                name="activation-modes",
                value_type="list",
                repeated=True,
                default="{ alpn }",
            ),
            BigipPropertySpec(name="concurrent-streams-per-connection", value_type="integer"),
            BigipPropertySpec(name="connection-idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_http2",),
                default="http2",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="enforce-tls-requirements",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="frame-size", value_type="integer", default="2048"),
            BigipPropertySpec(name="header-table-size", value_type="integer", default="4096 bytes"),
            BigipPropertySpec(
                name="insert-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="insert-header-name", value_type="unknown", default='"X-HTTP2"'),
            BigipPropertySpec(name="receive-window", value_type="integer", default="32"),
            BigipPropertySpec(name="write-size", value_type="integer", default="16384"),
        ),
    )
