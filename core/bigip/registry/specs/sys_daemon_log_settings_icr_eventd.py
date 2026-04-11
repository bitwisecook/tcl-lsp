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
            "sys_daemon_log_settings_icr_eventd",
            module="sys",
            object_types=("daemon-log-settings icr-eventd",),
        ),
        header_types=(("sys", "daemon-log-settings icr-eventd"),),
        properties=(
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("critical", "debug", "error", "informational", "notice", "warning"),
            ),
        ),
    )
