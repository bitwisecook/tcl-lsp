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
            "ltm_profile_dns",
            module="ltm",
            object_types=("profile dns",),
        ),
        header_types=(("ltm", "profile dns"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cache", value_type="string"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_dns",),
                default="dns",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dns-security", value_type="string"),
            BigipPropertySpec(
                name="dns64",
                value_type="enum",
                enum_values=("disabled", "immediate", "secondary", "v4-only"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="dns64-additional-section-rewrite",
                value_type="enum",
                enum_values=("any", "disabled", "v4-only", "v6-only"),
                default="disabled",
            ),
            BigipPropertySpec(name="dns64-prefix", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(
                name="edns0-client-subnet-insert",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="enable-cache",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="enable-dns-express",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="enable-dns-firewall",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="enable-dnssec",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="enable-gtm",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="enable-hardware-query-validation",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="enable-hardware-response-cache",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="enable-logging",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="enable-odoh",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="enable-rapid-response",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="log-profile", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="process-rd",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="process-xfr",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="rapid-response-last-action",
                value_type="enum",
                enum_values=("allow", "drop", "noerror", "nxdomain", "refuse", "truncate"),
                default="drop",
            ),
            BigipPropertySpec(
                name="unhandled-query-action",
                value_type="enum",
                enum_values=("allow", "drop", "hint", "noerror", "reject"),
                default="allow",
            ),
            BigipPropertySpec(
                name="use-local-bind",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
        ),
    )
