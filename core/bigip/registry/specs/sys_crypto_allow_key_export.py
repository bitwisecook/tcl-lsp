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
            "sys_crypto_allow_key_export",
            module="sys",
            object_types=("crypto allow-key-export",),
        ),
        header_types=(("sys", "crypto allow-key-export"),),
        properties=(
            BigipPropertySpec(name="value", value_type="enum", enum_values=("enabled", "disabled")),
        ),
    )
