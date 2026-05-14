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
            "sys_daemon_ha",
            module="sys",
            object_types=("daemon-ha",),
        ),
        header_types=(("sys", "daemon-ha"),),
        properties=(
            BigipPropertySpec(
                name="heartbeat",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="heartbeat-action",
                value_type="enum",
                enum_values=(
                    "go-offline",
                    "go-offline-downlinks-restart",
                    "go-offline-restart",
                    "reboot",
                    "restart",
                    "restart-all",
                ),
            ),
            BigipPropertySpec(
                name="running",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
