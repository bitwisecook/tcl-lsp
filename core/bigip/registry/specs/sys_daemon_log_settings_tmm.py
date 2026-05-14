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
            "sys_daemon_log_settings_tmm",
            module="sys",
            object_types=("daemon-log-settings tmm",),
        ),
        header_types=(("sys", "daemon-log-settings tmm"),),
        properties=(
            BigipPropertySpec(
                name="arp-log-level",
                value_type="enum",
                enum_values=("debug", "error", "informational", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="http-compression-log-level",
                value_type="enum",
                enum_values=("debug", "error", "informational", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="http-log-level",
                value_type="enum",
                enum_values=("debug", "error", "informational", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="ip-log-level",
                value_type="enum",
                enum_values=("debug", "informational", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="irule-log-level",
                value_type="enum",
                enum_values=("debug", "error", "informational", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="layer4-log-level",
                value_type="enum",
                enum_values=("debug", "informational", "notice"),
            ),
            BigipPropertySpec(
                name="net-log-level",
                value_type="enum",
                enum_values=("critical", "debug", "error", "informational", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="os-log-level",
                value_type="enum",
                enum_values=(
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ),
            ),
            BigipPropertySpec(
                name="pva-log-level",
                value_type="enum",
                enum_values=("debug", "informational", "notice"),
            ),
            BigipPropertySpec(
                name="ssl-log-level",
                value_type="enum",
                enum_values=(
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ),
            ),
        ),
    )
