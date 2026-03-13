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
            "net_bwc_priority_group",
            module="net",
            object_types=("bwc priority-group",),
        ),
        header_types=(("net", "bwc priority-group"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="priority-classes", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="description", value_type="string", in_sections=("priority-classes",)
            ),
            BigipPropertySpec(
                name="weight-percentage", value_type="integer", in_sections=("priority-classes",)
            ),
            BigipPropertySpec(name="net", value_type="string"),
            BigipPropertySpec(name="priority-classes", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="weight-percentage", value_type="string", in_sections=("net",)),
        ),
    )
