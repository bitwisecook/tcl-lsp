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
            "sys_autoscale_group",
            module="sys",
            object_types=("autoscale-group",),
        ),
        header_types=(("sys", "autoscale-group"),),
        properties=(
            BigipPropertySpec(name="autoscale-group-id", value_type="integer", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
