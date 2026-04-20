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
            "sys_ipfix_destination",
            module="sys",
            object_types=("ipfix destination",),
        ),
        header_types=(("sys", "ipfix destination"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
