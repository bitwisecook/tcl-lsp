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
            "sys_log_config_destination_local_database",
            module="sys",
            object_types=("log-config destination local-database",),
        ),
        header_types=(("sys", "log-config destination local-database"),),
        properties=(BigipPropertySpec(name="description", value_type="string"),),
    )
