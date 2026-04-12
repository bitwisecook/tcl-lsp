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
            "sys_crypto_fips_key",
            module="sys",
            object_types=("crypto fips key",),
        ),
        header_types=(("sys", "crypto fips key"),),
    )
