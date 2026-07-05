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
            "apm_profile_exchange",
            module="apm",
            object_types=("profile exchange",),
        ),
        header_types=(("apm", "profile exchange"),),
        properties=(
            BigipPropertySpec(
                name="active-sync-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="active-sync-sso-config",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="active-sync-url",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="auto-discover-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="auto-discover-sso-config",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="auto-discover-url",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="ntlm-auth-name", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="offline-address-book-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="offline-address-book-sso-config",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="offline-address-book-url",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="rpc-over-http-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="rpc-over-http-sso-config",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="rpc-over-http-url",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="user-agent-pattern-for-utf8",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="web-service-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="web-service-sso-config",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="web-service-url", value_type="string", allow_none=True),
        ),
    )
