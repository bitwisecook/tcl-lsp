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
            "gtm_monitor_wmi",
            module="gtm",
            object_types=("monitor wmi",),
        ),
        header_types=(("gtm", "monitor wmi"),),
        properties=(
            BigipPropertySpec(name="command", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_wmi",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="metrics", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "password"),
            ),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="url", value_type="enum", allow_none=True, enum_values=("none", "url")
            ),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
        ),
    )
