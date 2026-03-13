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
            "sys_crypto_acceleration_strategy",
            module="sys",
            object_types=("crypto acceleration-strategy",),
        ),
        header_types=(("sys", "crypto acceleration-strategy"),),
    )
