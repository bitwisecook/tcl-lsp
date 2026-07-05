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
            "sys_file_ssl_cert",
            module="sys",
            object_types=("file ssl-cert",),
        ),
        header_types=(("sys", "file ssl-cert"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cert-validation-options",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "ocsp"),
            ),
            BigipPropertySpec(
                name="cert-validators",
                value_type="reference",
                references=("sys_crypto_cert_validator_crl", "sys_crypto_cert_validator_ocsp"),
            ),
            BigipPropertySpec(name="issuer-cert", value_type="reference"),
            BigipPropertySpec(name="source-path", value_type="unknown"),
        ),
    )
