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
            "sys_log_config_destination_splunk",
            module="sys",
            object_types=("log-config destination splunk",),
        ),
        header_types=(("sys", "log-config destination splunk"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="forward-to", value_type="string"),
        ),
    )
