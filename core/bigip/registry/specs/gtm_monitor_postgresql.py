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
            "gtm_monitor_postgresql",
            module="gtm",
            object_types=("monitor postgresql",),
        ),
        header_types=(("gtm", "monitor postgresql"),),
        properties=(
            BigipPropertySpec(name="count", value_type="enum", enum_values=("0", "1")),
            BigipPropertySpec(
                name="database",
                value_type="reference",
                allow_none=True,
                references=("sys_log_config_destination_local_database",),
            ),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_postgresql",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="recv", value_type="string"),
            BigipPropertySpec(name="recv-column", value_type="string"),
            BigipPropertySpec(name="recv-row", value_type="string"),
            BigipPropertySpec(name="send", value_type="string"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="username", value_type="reference", allow_none=True),
        ),
    )
