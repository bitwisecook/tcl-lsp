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
            "gtm_monitor_soap",
            module="gtm",
            object_types=("monitor soap",),
        ),
        header_types=(("gtm", "monitor soap"),),
        properties=(
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_soap",),
                default="soap",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string", shape_kind="endpoint"),
            BigipPropertySpec(
                name="expect-fault",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
            BigipPropertySpec(name="method", value_type="string"),
            BigipPropertySpec(
                name="namespace",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="parameter-name",
                value_type="reference",
                allow_none=True,
                default="bool",
            ),
            BigipPropertySpec(name="parameter-type", value_type="string", default="none"),
            BigipPropertySpec(name="parameter-value", value_type="integer", default="none"),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="protocol", value_type="unknown", default="none"),
            BigipPropertySpec(name="return-type", value_type="string", default="bool"),
            BigipPropertySpec(name="return-value", value_type="integer", default="none"),
            BigipPropertySpec(name="soap-action", value_type="string", default="the empty string"),
            BigipPropertySpec(name="timeout", value_type="integer", default="120 seconds"),
            BigipPropertySpec(name="url-path", value_type="string", default="none"),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
        ),
    )
