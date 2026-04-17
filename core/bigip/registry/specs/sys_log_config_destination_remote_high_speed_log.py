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
            "sys_log_config_destination_remote_high_speed_log",
            module="sys",
            object_types=("log-config destination remote-high-speed-log",),
        ),
        header_types=(("sys", "log-config destination remote-high-speed-log"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="distribution",
                value_type="enum",
                enum_values=("adaptive", "balanced", "replicated"),
            ),
            BigipPropertySpec(name="pool-name", value_type="string"),
            BigipPropertySpec(name="protocol", value_type="enum", enum_values=("tcp", "udp")),
        ),
    )
