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
            "ltm_profile_web_acceleration",
            module="ltm",
            object_types=("profile web-acceleration",),
        ),
        header_types=(("ltm", "profile web-acceleration"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cache-aging-rate", value_type="integer", default="9"),
            BigipPropertySpec(
                name="cache-client-cache-control-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "max-age", "none"),
                default="all",
            ),
            BigipPropertySpec(
                name="cache-insert-age-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="cache-max-age", value_type="integer", default="3600 seconds"),
            BigipPropertySpec(name="cache-max-entries", value_type="integer", default="10000"),
            BigipPropertySpec(
                name="cache-object-max-size",
                value_type="integer",
                default="50000 bytes",
            ),
            BigipPropertySpec(
                name="cache-object-min-size",
                value_type="integer",
                default="500 bytes",
            ),
            BigipPropertySpec(name="cache-size", value_type="integer", default="100 megabytes"),
            BigipPropertySpec(
                name="cache-uri-exclude",
                value_type="unknown",
                allow_none=True,
                default="none and specifies that no URI will be excluded",
            ),
            BigipPropertySpec(name="cache-uri-include", value_type="unknown"),
            BigipPropertySpec(
                name="cache-uri-include-override",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cache-uri-pinned",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_web_acceleration",),
                default="webacceleration",
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
