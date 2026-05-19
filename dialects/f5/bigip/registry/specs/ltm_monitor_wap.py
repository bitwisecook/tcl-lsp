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
            "ltm_monitor_wap",
            module="ltm",
            object_types=("monitor wap",),
        ),
        header_types=(("ltm", "monitor wap"),),
        properties=(
            BigipPropertySpec(name="accounting-node", value_type="reference"),
            BigipPropertySpec(
                name="accounting-port",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="call-id", value_type="unknown", default="none"),
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
                references=("ltm_monitor_wap",),
                default="wap",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(name="framed-address", value_type="unknown", default="none"),
            BigipPropertySpec(name="interval", value_type="integer", default="10 seconds"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="recv", value_type="string", default="none"),
            BigipPropertySpec(name="secret", value_type="unknown", default="none"),
            BigipPropertySpec(name="send", value_type="string", default="none"),
            BigipPropertySpec(name="server-id", value_type="unknown", default="none"),
            BigipPropertySpec(name="session-id", value_type="unknown", default="none"),
            BigipPropertySpec(name="time-until-up", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="timeout", value_type="integer", default="31 seconds"),
            BigipPropertySpec(
                name="up-interval",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
        ),
    )
