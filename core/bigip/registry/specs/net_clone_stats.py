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
            "net_clone_stats",
            module="net",
            object_types=("clone-stats",),
        ),
        header_types=(("net", "clone-stats"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
