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
            "net_bwc_traffic_group",
            module="net",
            object_types=("bwc traffic-group",),
        ),
        header_types=(("net", "bwc traffic-group"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dynamic", value_type="boolean"),
            BigipPropertySpec(name="priority-classes", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="weight-percentage", value_type="integer", in_sections=("priority-classes",)
            ),
            BigipPropertySpec(name="net", value_type="reference", references=("cm_traffic_group",)),
            BigipPropertySpec(name="priority-classes", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="weight-percentage", value_type="string", in_sections=("net",)),
        ),
    )
