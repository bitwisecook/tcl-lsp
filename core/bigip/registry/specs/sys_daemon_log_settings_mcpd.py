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
            "sys_daemon_log_settings_mcpd",
            module="sys",
            object_types=("daemon-log-settings mcpd",),
        ),
        header_types=(("sys", "daemon-log-settings mcpd"),),
        properties=(
            BigipPropertySpec(
                name="audit",
                value_type="enum",
                enum_values=("all", "disabled", "enabled", "verbose"),
            ),
            BigipPropertySpec(name="log-level", value_type="string"),
            BigipPropertySpec(name="informational", value_type="boolean"),
        ),
    )
