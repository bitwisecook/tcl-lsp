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
            "ltm_monitor_mssql",
            module="ltm",
            object_types=("monitor mssql",),
        ),
        header_types=(("ltm", "monitor mssql"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="count", value_type="integer", default="zero"),
            BigipPropertySpec(
                name="database",
                value_type="reference",
                allow_none=True,
                references=("sys_log_config_destination_local_database",),
                default="none",
            ),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_mssql",),
                default="mssql",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(name="recv", value_type="string", default="none"),
            BigipPropertySpec(name="recv-column", value_type="string", default="none"),
            BigipPropertySpec(name="recv-row", value_type="string", default="none"),
            BigipPropertySpec(name="send", value_type="string"),
            BigipPropertySpec(name="time-until-up", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="timeout", value_type="integer", default="91 seconds"),
            BigipPropertySpec(
                name="up-interval",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
        ),
    )
