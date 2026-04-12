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
            "net_cmetrics",
            module="net",
            object_types=("cmetrics",),
        ),
        header_types=(("net", "cmetrics"),),
        properties=(BigipPropertySpec(name="dest-addr", value_type="string"),),
    )
