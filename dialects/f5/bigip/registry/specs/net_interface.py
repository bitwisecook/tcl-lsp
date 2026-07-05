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
            "net_interface",
            module="net",
            object_types=("interface",),
        ),
        header_types=(("net", "interface"),),
        properties=(
            BigipPropertySpec(
                name="bundle",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="bundle-speed", value_type="enum", enum_values=("100G", "40G")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="flow-control", value_type="unknown", default="tx-rx"),
            BigipPropertySpec(
                name="force-gigabit-fiber",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="forward-error-correction",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="lacp-port-priority", value_type="integer", default="33768"),
            BigipPropertySpec(
                name="link-traps-enabled",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="lldp-admin",
                value_type="enum",
                enum_values=("disable", "rxonly", "txonly", "txrx"),
                default="txonly",
            ),
            BigipPropertySpec(name="lldp-tlvmap", value_type="integer", default="130943"),
            BigipPropertySpec(name="media", value_type="enum", enum_values=("auto", "no-phy")),
            BigipPropertySpec(
                name="media-fixed",
                value_type="enum",
                enum_values=("auto", "no-phy"),
            ),
            BigipPropertySpec(
                name="media-sfp",
                value_type="enum",
                allow_none=True,
                enum_values=("auto", "no-phy", "none"),
            ),
            BigipPropertySpec(name="no-mgmt", value_type="unknown"),
            BigipPropertySpec(
                name="port-fwd-mode",
                value_type="enum",
                enum_values=("l3", "passive", "virtual-wire"),
                default="l3",
            ),
            BigipPropertySpec(
                name="prefer-port",
                value_type="enum",
                enum_values=("fixed", "sfp"),
                default="sfp",
            ),
            BigipPropertySpec(name="qinq-ethertype", value_type="string", default="set to 0x8100"),
            BigipPropertySpec(
                name="sflow",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="poll-interval",
                        value_type="integer",
                        in_sections=("sflow",),
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="poll-interval-global",
                        value_type="enum",
                        in_sections=("sflow",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                        default="yes",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="poll-interval",
                value_type="integer",
                in_sections=("sflow",),
                default="0",
            ),
            BigipPropertySpec(
                name="poll-interval-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="span-mode",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="stp",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="stp-auto-edge-port",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="stp-edge-port",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="stp-link-type",
                value_type="enum",
                enum_values=("auto", "p2p", "shared"),
                default="auto",
            ),
            BigipPropertySpec(name="stp-reset", value_type="unknown"),
        ),
    )
