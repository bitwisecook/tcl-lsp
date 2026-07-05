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
            "saas_csd_profile",
            module="saas",
            object_types=("csd profile",),
        ),
        header_types=(("saas", "csd profile"),),
        properties=(
            BigipPropertySpec(
                name="add-connecting-ip",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="api-domain-pool",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="api-hostname", value_type="string", default="us"),
            BigipPropertySpec(name="api-ivs-ssl", value_type="reference", allow_none=True),
            BigipPropertySpec(name="api-js-path", value_type="string"),
            BigipPropertySpec(
                name="api-proxy-destination",
                value_type="string",
                default="https://us",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="connecting-ip-header",
                value_type="string",
                default="x-iapp-real-ip",
            ),
            BigipPropertySpec(name="customer-id", value_type="string"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("saas_csd_profile",),
                default="csd",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="irules",
                value_type="list",
                repeated=True,
                allow_none=True,
                references=("apm_policy_agent_irule_event", "pem_irule"),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="js-inject-exclude-paths",
                value_type="list",
                repeated=True,
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="js-inject-exclude-paths-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="js-inject-include-paths",
                value_type="list",
                repeated=True,
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="js-inject-include-paths-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="js-inject-location",
                value_type="enum",
                enum_values=("body", "head"),
                default="after head",
            ),
            BigipPropertySpec(
                name="js-inject-script-attribute",
                value_type="enum",
                enum_values=("async", "async-defer", "defer", "sync"),
                default="async-defer",
            ),
            BigipPropertySpec(
                name="proxy-password",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="proxy-pool", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="proxy-username",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="telemetry-domain-pool",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="telemetry-hostname", value_type="string", default="csd"),
            BigipPropertySpec(name="telemetry-ivs-ssl", value_type="reference", allow_none=True),
            BigipPropertySpec(name="telemetry-path", value_type="string", default="/csd/oob"),
            BigipPropertySpec(
                name="telemetry-proxy-destination",
                value_type="string",
                default="https://csd",
            ),
            BigipPropertySpec(
                name="use-proxy-server",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="use-sni",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
