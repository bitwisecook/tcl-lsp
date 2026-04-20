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
            "sys_daemon_log_settings_clusterd",
            module="sys",
            object_types=("daemon-log-settings clusterd",),
        ),
        header_types=(("sys", "daemon-log-settings clusterd"),),
        properties=(BigipPropertySpec(name="log-level", value_type="boolean"),),
    )
