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
            "auth_partition",
            module="auth",
            object_types=("partition",),
        ),
        header_types=(("auth", "partition"),),
        properties=(
            BigipPropertySpec(
                name="default-route-domain",
                value_type="reference",
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
