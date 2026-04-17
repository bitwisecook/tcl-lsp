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
            "ltm_monitor_virtual_location",
            module="ltm",
            object_types=("monitor virtual-location",),
        ),
        header_types=(("ltm", "monitor virtual-location"),),
        properties=(
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_virtual_location",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
        ),
    )
