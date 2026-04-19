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
            BigipPropertySpec(name="accounting-node", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="accounting-port",
                value_type="integer",
                allow_none=True,
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="call-id", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="check-until-up", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_wap",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="framed-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="recv", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="secret", value_type="enum", allow_none=True, enum_values=("none", "password")
            ),
            BigipPropertySpec(name="send", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="server-id", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-id", value_type="integer", allow_none=True),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
