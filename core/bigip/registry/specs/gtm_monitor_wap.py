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
            BigipPropertySpec(name="accounting-port", value_type="integer", allow_none=True),
            BigipPropertySpec(name="call-id", value_type="unknown"),
            BigipPropertySpec(
                name="check-until-up",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_wap",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="framed-address", value_type="unknown"),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="recv", value_type="string"),
            BigipPropertySpec(name="secret", value_type="unknown"),
            BigipPropertySpec(name="send", value_type="string"),
            BigipPropertySpec(name="server-id", value_type="unknown"),
            BigipPropertySpec(name="session-id", value_type="unknown"),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
