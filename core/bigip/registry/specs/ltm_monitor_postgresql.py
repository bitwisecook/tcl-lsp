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
            "ltm_monitor_postgresql",
            module="ltm",
            object_types=("monitor postgresql",),
        ),
        header_types=(("ltm", "monitor postgresql"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="count", value_type="integer"),
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
                references=("ltm_monitor_postgresql",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(name="recv", value_type="string"),
            BigipPropertySpec(name="recv-column", value_type="string"),
            BigipPropertySpec(name="recv-row", value_type="string"),
            BigipPropertySpec(name="send", value_type="string"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(name="username", value_type="reference", allow_none=True),
        ),
    )
