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
            "util_serverssl_ciphers",
            module="util",
            object_types=("serverssl-ciphers",),
        ),
        header_types=(("util", "serverssl-ciphers"),),
        properties=(),
    )
