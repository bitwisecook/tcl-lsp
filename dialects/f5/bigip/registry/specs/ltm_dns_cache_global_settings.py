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
            "ltm_dns_cache_global_settings",
            module="ltm",
            object_types=("dns cache global-settings",),
        ),
        header_types=(("ltm", "dns cache global-settings"),),
        properties=(
            BigipPropertySpec(name="cache-maximum-ttl", value_type="integer"),
            BigipPropertySpec(name="cache-minimum-ttl", value_type="integer"),
            BigipPropertySpec(name="resolver-edns-buffer-size", value_type="integer"),
            BigipPropertySpec(
                name="serve-expired",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="serve-expired-client-timeout",
                value_type="integer",
                default="1800 milliseconds",
            ),
            BigipPropertySpec(
                name="serve-expired-reply-ttl",
                value_type="integer",
                default="30 seconds",
            ),
            BigipPropertySpec(
                name="serve-expired-ttl",
                value_type="integer",
                default="86400 seconds",
            ),
            BigipPropertySpec(
                name="serve-expired-ttl-reset",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
        ),
    )
