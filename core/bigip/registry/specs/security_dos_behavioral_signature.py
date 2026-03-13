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
            "security_dos_behavioral_signature",
            module="security",
            object_types=("dos behavioral-signature",),
        ),
        header_types=(("security", "dos behavioral-signature"),),
        properties=(
            BigipPropertySpec(name="alias", value_type="string"),
            BigipPropertySpec(name="status", value_type="string"),
        ),
    )
