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
            "sys_crypto_cert_validation_response_ocsp",
            module="sys",
            object_types=("crypto cert-validation-response ocsp",),
        ),
        header_types=(("sys", "crypto cert-validation-response ocsp"),),
        properties=(BigipPropertySpec(name="certificate", value_type="string"),),
    )
