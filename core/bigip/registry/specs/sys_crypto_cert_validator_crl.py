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
            "sys_crypto_cert_validator_crl",
            module="sys",
            object_types=("crypto cert-validator crl",),
        ),
        header_types=(("sys", "crypto cert-validator crl"),),
        properties=(
            BigipPropertySpec(name="internal-proxy", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="strict-revocation-check",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
