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
            "apm_aaa_saml_idp_automation",
            module="apm",
            object_types=("aaa saml-idp-automation",),
        ),
        header_types=(("apm", "aaa saml-idp-automation"),),
        properties=(
            BigipPropertySpec(name="aaa-saml-server", value_type="string"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="connection-properties",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("connection-properties",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="dns-resolver-name",
                        value_type="string",
                        in_sections=("connection-properties",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="serverssl-profile-name",
                        value_type="string",
                        in_sections=("connection-properties",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("connection-properties",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="dns-resolver-name",
                value_type="string",
                in_sections=("connection-properties",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="serverssl-profile-name",
                value_type="string",
                in_sections=("connection-properties",),
                allow_none=True,
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="frequency", value_type="integer", default="60"),
            BigipPropertySpec(name="idp-matching-source", value_type="string"),
            BigipPropertySpec(name="idp-obj-name-tag", value_type="string"),
            BigipPropertySpec(name="metadata-matching-tag", value_type="string"),
            BigipPropertySpec(name="metadata-urls", value_type="list"),
        ),
    )
