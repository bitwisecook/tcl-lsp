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
            "sys_crypto_csr",
            module="sys",
            object_types=("crypto csr",),
        ),
        header_types=(("sys", "crypto csr"),),
        properties=(
            BigipPropertySpec(
                name="admin-email-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="basic-constraints", value_type="string"),
            BigipPropertySpec(name="challenge-password", value_type="string"),
            BigipPropertySpec(name="city", value_type="string"),
            BigipPropertySpec(name="common-name", value_type="string"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(
                name="email-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="key", value_type="string"),
            BigipPropertySpec(name="key-usage", value_type="string"),
            BigipPropertySpec(name="organization", value_type="string"),
            BigipPropertySpec(name="ou", value_type="string"),
            BigipPropertySpec(name="state", value_type="string"),
            BigipPropertySpec(name="subject-alternative-name", value_type="string"),
        ),
    )
