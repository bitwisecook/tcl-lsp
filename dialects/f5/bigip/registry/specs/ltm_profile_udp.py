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
            "ltm_profile_udp",
            module="ltm",
            object_types=("profile udp",),
        ),
        header_types=(("ltm", "profile udp"),),
        properties=(
            BigipPropertySpec(
                name="allow-no-payload",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="buffer-max-bytes", value_type="integer", default="655350"),
            BigipPropertySpec(name="buffer-max-packets", value_type="integer", default="0"),
            BigipPropertySpec(
                name="datagram-load-balancing",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_udp",),
                default="udp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="idle-timeout",
                value_type="integer",
                enum_values=("immediate", "indefinite"),
                default="60 seconds",
            ),
            BigipPropertySpec(
                name="ip-df-mode",
                value_type="enum",
                enum_values=("clear", "pmtu", "preserve", "set"),
            ),
            BigipPropertySpec(name="ip-tos-to-client", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="ip-ttl-mode",
                value_type="enum",
                enum_values=("decrement", "preserve", "proxy", "set"),
            ),
            BigipPropertySpec(name="ip-ttl-v4", value_type="integer"),
            BigipPropertySpec(name="ip-ttl-v6", value_type="integer"),
            BigipPropertySpec(name="link-qos-to-client", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="no-checksum",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="proxy-mss",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="send-buffer-size", value_type="integer", default="655350"),
        ),
    )
