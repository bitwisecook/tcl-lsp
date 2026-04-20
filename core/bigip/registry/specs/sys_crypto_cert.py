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
            BigipPropertySpec(name="city", value_type="string"),
            BigipPropertySpec(name="common-name", value_type="string"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(
                name="email-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="key", value_type="string"),
            BigipPropertySpec(name="lifetime", value_type="string"),
            BigipPropertySpec(name="organization", value_type="string"),
            BigipPropertySpec(name="ou", value_type="string"),
            BigipPropertySpec(name="state", value_type="string"),
            BigipPropertySpec(name="subject-alternative-name", value_type="string"),
            BigipPropertySpec(name="install", value_type="string"),
            BigipPropertySpec(
                name="cert-validation-options",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "ocsp"),
            ),
            BigipPropertySpec(name="cert-validators", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="from-local-file", value_type="string"),
            BigipPropertySpec(name="from-url", value_type="string"),
            BigipPropertySpec(name="issuer-cert", value_type="boolean", allow_none=True),
        ),
    )
