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
            "sys_sflow_global_settings_http",
            module="sys",
            object_types=("sflow global-settings http",),
        ),
        header_types=(("sys", "sflow global-settings http"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer"),
            BigipPropertySpec(name="sampling-rate", value_type="integer"),
        ),
    )
