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
            "apm_policy_agent_oauth_authz",
            module="apm",
            object_types=("policy agent oauth-authz",),
        ),
        header_types=(("apm", "policy agent oauth-authz"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="audience",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete")),
            ),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="entries",
                value_type="list",
                allow_none=True,
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
                        name="expression",
                        value_type="string",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="id-token-claim-entries",
                        value_type="list",
                        in_sections=("entries",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="jwt-access-token-claim-entries",
                        value_type="list",
                        in_sections=("entries",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="scope-entries",
                        value_type="list",
                        in_sections=("entries",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="userinfo-claim-entries",
                        value_type="list",
                        in_sections=("entries",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
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
                name="expression",
                value_type="string",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="id-token-claim-entries",
                value_type="list",
                in_sections=("entries",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("entries", "id-token-claim-entries"),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="claim-name",
                        value_type="unknown",
                        in_sections=("entries", "id-token-claim-entries"),
                    ),
                    BigipPropertySpec(
                        name="claim-value",
                        value_type="string",
                        in_sections=("entries", "id-token-claim-entries"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("entries", "id-token-claim-entries"),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="claim-name",
                value_type="unknown",
                in_sections=("entries", "id-token-claim-entries"),
            ),
            BigipPropertySpec(
                name="claim-value",
                value_type="string",
                in_sections=("entries", "id-token-claim-entries"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="jwt-access-token-claim-entries",
                value_type="list",
                in_sections=("entries",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("entries", "jwt-access-token-claim-entries"),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="claim-name",
                        value_type="unknown",
                        in_sections=("entries", "jwt-access-token-claim-entries"),
                    ),
                    BigipPropertySpec(
                        name="claim-value",
                        value_type="string",
                        in_sections=("entries", "jwt-access-token-claim-entries"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("entries", "jwt-access-token-claim-entries"),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="claim-name",
                value_type="unknown",
                in_sections=("entries", "jwt-access-token-claim-entries"),
            ),
            BigipPropertySpec(
                name="claim-value",
                value_type="string",
                in_sections=("entries", "jwt-access-token-claim-entries"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="scope-entries",
                value_type="list",
                in_sections=("entries",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("entries", "scope-entries"),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="scope-name",
                        value_type="unknown",
                        in_sections=("entries", "scope-entries"),
                    ),
                    BigipPropertySpec(
                        name="scope-value",
                        value_type="string",
                        in_sections=("entries", "scope-entries"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("entries", "scope-entries"),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="scope-name",
                value_type="unknown",
                in_sections=("entries", "scope-entries"),
            ),
            BigipPropertySpec(
                name="scope-value",
                value_type="string",
                in_sections=("entries", "scope-entries"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="userinfo-claim-entries",
                value_type="list",
                in_sections=("entries",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("entries", "userinfo-claim-entries"),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="claim-name",
                        value_type="unknown",
                        in_sections=("entries", "userinfo-claim-entries"),
                    ),
                    BigipPropertySpec(
                        name="claim-value",
                        value_type="string",
                        in_sections=("entries", "userinfo-claim-entries"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("entries", "userinfo-claim-entries"),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="claim-name",
                value_type="unknown",
                in_sections=("entries", "userinfo-claim-entries"),
            ),
            BigipPropertySpec(
                name="claim-value",
                value_type="string",
                in_sections=("entries", "userinfo-claim-entries"),
                allow_none=True,
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(
                name="prompt-for-authorization",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="subject", value_type="string", allow_none=True),
        ),
    )
