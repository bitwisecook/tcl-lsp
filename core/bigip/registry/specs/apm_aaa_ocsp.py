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
            "apm_aaa_ocsp",
            module="apm",
            object_types=("aaa ocsp",),
        ),
        header_types=(("apm", "aaa ocsp"),),
        properties=(
            BigipPropertySpec(
                name="allow-certs",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ca-file",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ca-path",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cert-id-digest", value_type="unknown"),
            BigipPropertySpec(
                name="chain",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="check-certs",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="explicit-ocsp",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="ignore-aia",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="intern",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="nonce",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(name="sign-digest", value_type="unknown", default="sha1"),
            BigipPropertySpec(
                name="sign-key",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="sign-key-passphrase",
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
            BigipPropertySpec(name="status-age", value_type="unknown", default="0 (zero)"),
            BigipPropertySpec(
                name="trust-other",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="url",
                value_type="unknown",
                required=True,
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="va-file",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="validity-period",
                value_type="unknown",
                required=True,
                default="300",
                usage_flags=frozenset(("not_synced",)),
            ),
            BigipPropertySpec(
                name="verify",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="verify-cert",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="verify-other",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="verify-sig",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
        ),
    )
