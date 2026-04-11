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
            "sys_sflow_global_settings_interface",
            module="sys",
            object_types=("sflow global-settings interface",),
        ),
        header_types=(("sys", "sflow global-settings interface"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer"),
        ),
    )
