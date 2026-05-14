from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_crypto_crl",
            module="sys",
            object_types=("crypto crl",),
        ),
        header_types=(("sys", "crypto crl"),),
        properties=(),
    )
