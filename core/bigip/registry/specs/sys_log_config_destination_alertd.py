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
            "sys_log_config_destination_alertd",
            module="sys",
            object_types=("log-config destination alertd",),
        ),
        header_types=(("sys", "log-config destination alertd"),),
        properties=(BigipPropertySpec(name="description", value_type="string"),),
    )
