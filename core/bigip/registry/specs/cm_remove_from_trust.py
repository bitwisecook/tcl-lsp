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
            "cm_remove_from_trust",
            module="cm",
            object_types=("remove-from-trust",),
        ),
        header_types=(("cm", "remove-from-trust"),),
        properties=(BigipPropertySpec(name="device-name", value_type="string"),),
    )
