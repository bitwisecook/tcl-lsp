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
            "sys_icall_istats_trigger",
            module="sys",
            object_types=("icall istats-trigger",),
        ),
        header_types=(("sys", "icall istats-trigger"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="duration", value_type="integer"),
            BigipPropertySpec(name="event-name", value_type="string"),
            BigipPropertySpec(name="istats-key", value_type="string"),
            BigipPropertySpec(name="range-max", value_type="integer"),
            BigipPropertySpec(name="range-min", value_type="integer"),
            BigipPropertySpec(name="repeat", value_type="integer"),
        ),
    )
