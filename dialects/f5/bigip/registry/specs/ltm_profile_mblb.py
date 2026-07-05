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
            "ltm_profile_mblb",
            module="ltm",
            object_types=("profile mblb",),
        ),
        header_types=(("ltm", "profile mblb"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_mblb",),
                default="mblb",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="egress-high", value_type="unknown", default="50"),
            BigipPropertySpec(name="egress-low", value_type="unknown", default="5"),
            BigipPropertySpec(name="ingress-high", value_type="unknown", default="50"),
            BigipPropertySpec(name="ingress-low", value_type="unknown", default="5"),
            BigipPropertySpec(
                name="isolate-abort",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="isolate-client",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="isolate-expire",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="isolate-server",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="min-conn", value_type="unknown", default="0"),
            BigipPropertySpec(name="shutdown-timeout", value_type="unknown", default="5 seconds"),
            BigipPropertySpec(name="tag-ttl", value_type="unknown", default="60"),
        ),
    )
