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
            "net_self_allow",
            module="net",
            object_types=("self-allow",),
        ),
        header_types=(("net", "self-allow"),),
        properties=(
            BigipPropertySpec(
                name="defaults", value_type="enum", allow_none=True, enum_values=("all", "none")
            ),
        ),
    )
