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
            "ltm_dns_dnssec_zone",
            module="ltm",
            object_types=("dns dnssec zone",),
        ),
        header_types=(("ltm", "dns dnssec zone"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ds-algorithm",
                value_type="enum",
                enum_values=("sha1", "sha256", "sha384"),
                default="sha1",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="ds-algorithms",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="sha1",
            ),
            BigipPropertySpec(name="ds-records", value_type="unknown"),
            BigipPropertySpec(name="external-delegations", value_type="unknown"),
            BigipPropertySpec(
                name="indicate-authenticated",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="keys", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="nsec3-algorithm", value_type="unknown", default="SHA1"),
            BigipPropertySpec(name="nsec3-iterations", value_type="integer", default="1"),
            BigipPropertySpec(
                name="publish-cds-cdnskey",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="secure",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
