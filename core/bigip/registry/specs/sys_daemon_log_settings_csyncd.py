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
            "sys_daemon_log_settings_csyncd",
            module="sys",
            object_types=("daemon-log-settings csyncd",),
        ),
        header_types=(("sys", "daemon-log-settings csyncd"),),
        properties=(BigipPropertySpec(name="log-level", value_type="boolean"),),
    )
