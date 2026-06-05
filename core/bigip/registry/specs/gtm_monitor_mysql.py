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
            "gtm_monitor_mysql",
            module="gtm",
            object_types=("monitor mysql",),
        ),
        header_types=(("gtm", "monitor mysql"),),
        properties=(
            BigipPropertySpec(name="count", value_type="enum", enum_values=("0", "1")),
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
                references=("gtm_monitor_mysql",),
                default="mysql",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="recv", value_type="string", default="none"),
            BigipPropertySpec(name="recv-column", value_type="string", default="none"),
            BigipPropertySpec(name="recv-row", value_type="string", default="none"),
            BigipPropertySpec(name="send", value_type="string"),
            BigipPropertySpec(name="timeout", value_type="integer", default="91 seconds"),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
        ),
    )
