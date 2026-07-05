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
            "apm_saml_artifact_resolution_service",
            module="apm",
            object_types=("saml artifact-resolution-service",),
        ),
        header_types=(("apm", "saml artifact-resolution-service"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="artifact-resolution-service-host",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="artifact-resolution-service-port", value_type="integer"),
            BigipPropertySpec(
                name="artifact-send-method",
                value_type="enum",
                enum_values=("http-post", "http-redirect"),
                default="http-redirect",
            ),
            BigipPropertySpec(name="artifact-validity", value_type="integer", default="60 seconds"),
            BigipPropertySpec(name="basic-auth-password", value_type="string", allow_none=True),
            BigipPropertySpec(name="basic-auth-username", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="description",
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
            BigipPropertySpec(name="virtual-server-name", value_type="reference"),
            BigipPropertySpec(
                name="want-artifact-resolution-rq-signed",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
        ),
    )
