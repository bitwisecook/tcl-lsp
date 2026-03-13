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
            "sys_log_config_destination_remote_syslog",
            module="sys",
            object_types=("log-config destination remote-syslog",),
        ),
        header_types=(("sys", "log-config destination remote-syslog"),),
        properties=(
            BigipPropertySpec(
                name="default-facility",
                value_type="enum",
                enum_values=(
                    "local0",
                    "local1",
                    "local2",
                    "local3",
                    "local4",
                    "local5",
                    "local6",
                    "local7",
                ),
            ),
            BigipPropertySpec(
                name="default-severity",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="format", value_type="enum", enum_values=("legacy-bigip", "rfc3164", "rfc5424")
            ),
            BigipPropertySpec(name="remote-high-speed-log", value_type="string"),
        ),
    )
