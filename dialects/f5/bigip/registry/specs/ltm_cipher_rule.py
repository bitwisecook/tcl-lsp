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
            "ltm_cipher_rule",
            module="ltm",
            object_types=("cipher rule",),
        ),
        header_types=(("ltm", "cipher rule"),),
        properties=(
            BigipPropertySpec(name="cipher", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dh-groups", value_type="string"),
            BigipPropertySpec(name="signature-algorithms", value_type="string"),
        ),
    )
