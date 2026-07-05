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
            "gtm_global_settings_general",
            module="gtm",
            object_types=("global-settings general",),
        ),
        header_types=(("gtm", "global-settings general"),),
        properties=(
            BigipPropertySpec(
                name="allow-nxdomain-override",
                value_type="enum",
                enum_values=("disable", "enable"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="auto-discovery",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="auto-discovery-interval", value_type="integer", default="30"),
            BigipPropertySpec(
                name="automatic-configuration-save-timeout",
                value_type="integer",
                default="15 seconds",
            ),
            BigipPropertySpec(
                name="cache-ldns-servers",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="domain-name-check",
                value_type="enum",
                allow_none=True,
                enum_values=("allow-underscore", "none"),
                default="allow-underscore",
            ),
            BigipPropertySpec(
                name="drain-persistent-requests",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="forward-status",
                value_type="enum",
                enum_values=("disable", "enable"),
            ),
            BigipPropertySpec(
                name="gtm-sets-recursion",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="heartbeat-interval", value_type="integer", default="10"),
            BigipPropertySpec(
                name="ignore-ltm-rate-limit-modes",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="iquery-cipher-list", value_type="string"),
            BigipPropertySpec(
                name="iquery-crl-validation-depth",
                value_type="enum",
                required=True,
                enum_values=("device", "full"),
                default="full",
            ),
            BigipPropertySpec(name="iquery-minimum-tls-version", value_type="string"),
            BigipPropertySpec(
                name="iquery-reverify-on-crl-becoming-active",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="iquery-reverify-on-crl-expiring",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="iquery-reverify-on-crl-file-update",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="iquery-use-expired-crls",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="iquery-use-not-yet-active-crls",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="iquery-use-revoked-certs",
                value_type="enum",
                enum_values=("always", "existing", "never"),
            ),
            BigipPropertySpec(
                name="monitor-disabled-objects",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="nethsm-timeout", value_type="integer", default="20 seconds"),
            BigipPropertySpec(
                name="nsec3-types-bitmap-strict",
                value_type="enum",
                enum_values=("disable", "enable"),
            ),
            BigipPropertySpec(name="peer-leader", value_type="reference"),
            BigipPropertySpec(
                name="send-wildcard-rrs",
                value_type="enum",
                enum_values=("disable", "enable"),
                default="disable",
            ),
            BigipPropertySpec(name="static-persist-cidr-ipv4", value_type="integer", default="32"),
            BigipPropertySpec(name="static-persist-cidr-ipv6", value_type="integer"),
            BigipPropertySpec(
                name="synchronization",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="synchronization-group-name", value_type="reference"),
            BigipPropertySpec(
                name="synchronization-time-tolerance",
                value_type="integer",
                default="10 seconds",
            ),
            BigipPropertySpec(name="synchronization-timeout", value_type="integer", default="180"),
            BigipPropertySpec(
                name="synchronize-zone-files",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="synchronize-zone-files-timeout",
                value_type="integer",
                default="300",
            ),
            BigipPropertySpec(
                name="topology-allow-zero-scores",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="virtuals-depend-on-server-state",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(name="wideip-zone-nameserver", value_type="string"),
        ),
    )
