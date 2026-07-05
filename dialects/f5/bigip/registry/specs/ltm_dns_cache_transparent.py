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
            "ltm_dns_cache_transparent",
            module="ltm",
            object_types=("dns cache transparent",),
        ),
        header_types=(("ltm", "dns cache transparent"),),
        properties=(
            BigipPropertySpec(
                name="answer-default-zones",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="local-zones",
                value_type="list",
                repeated=True,
                list_operators=frozenset(("add",)),
                default="empty",
            ),
            BigipPropertySpec(name="msg-cache-size", value_type="integer", default="1048576"),
            BigipPropertySpec(
                name="response-policy-zones",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify")),
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("response-policy-zones",),
                        enum_values=("nxdomain", "walled-garden"),
                    ),
                    BigipPropertySpec(
                        name="walled-garden",
                        value_type="unknown",
                        in_sections=("response-policy-zones",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("response-policy-zones",),
                enum_values=("nxdomain", "walled-garden"),
            ),
            BigipPropertySpec(
                name="walled-garden",
                value_type="unknown",
                in_sections=("response-policy-zones",),
            ),
            BigipPropertySpec(name="rrset-cache-size", value_type="integer", default="10485760"),
            BigipPropertySpec(
                name="rrset-rotate",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "query-id"),
                default="none",
            ),
        ),
    )
