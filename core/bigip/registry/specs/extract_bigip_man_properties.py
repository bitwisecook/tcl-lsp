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
            "extract_bigip_man_properties",
            module="extract",
            object_types=("bigip man properties",),
        ),
        header_types=(("extract", "bigip man properties"),),
        properties=(),
    )
