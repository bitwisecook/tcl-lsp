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
            "apm_aaa_oauth_provider",
            module="apm",
            object_types=("aaa oauth-provider",),
        ),
        header_types=(("apm", "aaa oauth-provider"),),
        properties=(
            BigipPropertySpec(
                name="allow-self-signed-jwk-cert",
                value_type="unknown",
                default="yes",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="authentication-uri", value_type="string", allow_none=True),
            BigipPropertySpec(name="auto-jwt-config-name", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="ignore-expired-cert", value_type="unknown", default="false"),
            BigipPropertySpec(name="last-discovery-time", value_type="unknown"),
            BigipPropertySpec(name="manual-jwt-config-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="max-json-nesting-layers", value_type="integer", default="8"),
            BigipPropertySpec(name="max-response-size", value_type="integer", default="128kb"),
            BigipPropertySpec(name="openid-cfg-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="save-json-payload",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(name="token-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="token-validation-scope-uri",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="trusted-ca-bundle", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("custom", "f5", "facebook", "google", "ping"),
            ),
            BigipPropertySpec(name="use-auto-jwt-config", value_type="unknown", default="true"),
            BigipPropertySpec(name="userinfo-request-uri", value_type="string", allow_none=True),
        ),
    )
