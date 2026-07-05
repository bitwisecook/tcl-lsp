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
            "ltm_auth_ocsp_responder",
            module="ltm",
            object_types=("auth ocsp-responder",),
        ),
        header_types=(("ltm", "auth ocsp-responder"),),
        properties=(
            BigipPropertySpec(
                name="allow-certs",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ca-file",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ca-path",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cert-id-digest",
                value_type="enum",
                enum_values=("md5", "sha1"),
                default="sha1",
            ),
            BigipPropertySpec(
                name="chain",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="check-certs",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="explicit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="ignore-aia",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="intern",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="nonce",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="sign-digest",
                value_type="enum",
                required=True,
                enum_values=("md5", "sha1"),
                default="sha1",
            ),
            BigipPropertySpec(
                name="sign-key",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="sign-key-pass-phrase",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="sign-other",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="signer", value_type="unknown", allow_none=True, default="none"),
            BigipPropertySpec(name="status-age", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="trust-other",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="value disabled",
            ),
            BigipPropertySpec(name="url", value_type="unknown", required=True, default="none"),
            BigipPropertySpec(
                name="va-file",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="validity-period", value_type="integer"),
            BigipPropertySpec(
                name="verify",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="verify-cert",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="verify-other",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="verify-sig",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
