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
            "sys_crypto_cert",
            module="sys",
            object_types=("crypto cert",),
        ),
        header_types=(("sys", "crypto cert"),),
        properties=(
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
            BigipPropertySpec(name="city", value_type="string"),
            BigipPropertySpec(name="common-name", value_type="string"),
            BigipPropertySpec(name="consumer", value_type="unknown"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(name="email-address", value_type="string"),
            BigipPropertySpec(name="issuer-cert", value_type="reference"),
            BigipPropertySpec(name="key", value_type="string"),
            BigipPropertySpec(name="lifetime", value_type="unknown"),
            BigipPropertySpec(name="organization", value_type="string"),
            BigipPropertySpec(name="ou", value_type="string"),
            BigipPropertySpec(name="state", value_type="string"),
            BigipPropertySpec(name="subject-alternative-name", value_type="string"),
        ),
    )
