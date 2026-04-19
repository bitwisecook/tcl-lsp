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
            "sys_crypto_pkcs12",
            module="sys",
            object_types=("crypto pkcs12",),
        ),
        header_types=(("sys", "crypto pkcs12"),),
        properties=(
            BigipPropertySpec(name="install", value_type="string"),
            BigipPropertySpec(name="from-local-file", value_type="string"),
            BigipPropertySpec(name="from-url", value_type="string"),
            BigipPropertySpec(name="passphrase", value_type="string"),
        ),
    )
