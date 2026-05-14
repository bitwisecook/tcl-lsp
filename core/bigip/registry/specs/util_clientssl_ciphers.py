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
            "util_clientssl_ciphers",
            module="util",
            object_types=("clientssl-ciphers",),
        ),
        header_types=(("util", "clientssl-ciphers"),),
        properties=(),
    )
