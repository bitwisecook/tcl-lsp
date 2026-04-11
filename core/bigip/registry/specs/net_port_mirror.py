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
            "net_port_mirror",
            module="net",
            object_types=("port-mirror",),
        ),
        header_types=(("net", "port-mirror"),),
        properties=(
            BigipPropertySpec(
                name="interfaces",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none"),
            ),
        ),
    )
