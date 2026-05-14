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
            "gtm_monitor_wap",
            module="gtm",
            object_types=("monitor wap",),
        ),
        header_types=(("gtm", "monitor wap"),),
        properties=(
            BigipPropertySpec(name="accounting-node", value_type="reference"),
            BigipPropertySpec(
                name="accounting-port",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="call-id", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="check-until-up",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
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
                references=("gtm_monitor_wap",),
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
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="10 seconds"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="recv", value_type="string", default="none"),
            BigipPropertySpec(name="secret", value_type="unknown", default="none"),
            BigipPropertySpec(name="send", value_type="string", default="none"),
            BigipPropertySpec(name="server-id", value_type="unknown", default="none"),
            BigipPropertySpec(name="session-id", value_type="unknown", default="none"),
            BigipPropertySpec(name="timeout", value_type="integer", default="31 seconds"),
        ),
    )
