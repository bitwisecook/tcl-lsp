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
            "sys_daemon_log_settings_icrd",
            module="sys",
            object_types=("daemon-log-settings icrd",),
        ),
        header_types=(("sys", "daemon-log-settings icrd"),),
        properties=(
            BigipPropertySpec(
                name="audit",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "modifications", "all"),
            ),
        ),
    )
